/*
DS4Windows
Copyright (C) 2023  Travis Nickles

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

using NLog;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.IO.MemoryMappedFiles;
using System.Net.Http;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;
using WPFLocalizeExtension.Engine;

namespace DS5ControlApp
{
    /// <summary>
    /// Interaction logic for App.xaml
    /// </summary>
    [System.Security.SuppressUnmanagedCodeSecurity]
    public partial class App : Application
    {
        [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
        private static extern int GetClassName(IntPtr hWnd, StringBuilder lpClassName, int nMaxCount);

        [DllImport("user32.dll", EntryPoint = "FindWindow")]
        private static extern IntPtr FindWindow(string sClass, string sWindow);

        [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = false)]
        private static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, ref COPYDATASTRUCT lParam);

        [StructLayout(LayoutKind.Sequential)]
        public struct COPYDATASTRUCT
        {
            public IntPtr dwData;
            public int cbData;
            public IntPtr lpData;
        }

        private Thread controlThread;
        public static DS5Control.ControlService rootHub;
        public static HttpClient requestClient
        {
            get => DS5Control.SharedHttpClient.Instance;
            set => DS5Control.SharedHttpClient.Instance = value;
        }
        private bool skipSave;
        private bool runShutdown;
        private bool exitApp;
        private Thread testThread;
        private bool exitComThread = false;
        private const string SingleAppComEventName = "{a52b5b20-d9ee-4f32-8518-307fa14aa0c6}";
        private EventWaitHandle threadComEvent = null;
        private Timer collectTimer;
        private static LoggerHolder logHolder;

        private MemoryMappedFile ipcClassNameMMF = null; // MemoryMappedFile for inter-process communication used to hold className of DS5Form window
        private MemoryMappedFile ipcResultDataMMF = null; // MemoryMappedFile for inter-process communication used to exchange string result data between cmdline client process and the background running DS5Control app

        private Interfaz.WebUi.WebUiWindow interfaceWindow;

        private void Application_Startup(object sender, StartupEventArgs e)
        {
            runShutdown = true;
            skipSave = true;

            ArgumentParser parser = new ArgumentParser();
            parser.Parse(e.Args);
            CheckOptions(parser);

            if (exitApp)
            {
                return;
            }

            try
            {
                Process.GetCurrentProcess().PriorityClass =
                    ProcessPriorityClass.High;
            }
            catch { } // Ignore problems raising the priority.

            // Force Normal IO Priority
            IntPtr ioPrio = new IntPtr(2);
            DS5Control.Util.NtSetInformationProcess(Process.GetCurrentProcess().Handle,
                DS5Control.Util.PROCESS_INFORMATION_CLASS.ProcessIoPriority, ref ioPrio, 4);

            // Force Normal Page Priority
            IntPtr pagePrio = new IntPtr(5);
            DS5Control.Util.NtSetInformationProcess(Process.GetCurrentProcess().Handle,
                DS5Control.Util.PROCESS_INFORMATION_CLASS.ProcessPagePriority, ref pagePrio, 4);

            // another instance is already running if TryOpenExisting returns true.
            try
            {
                if (EventWaitHandleAcl.TryOpenExisting(SingleAppComEventName,
                System.Security.AccessControl.EventWaitHandleRights.Synchronize |
                System.Security.AccessControl.EventWaitHandleRights.Modify,
                out EventWaitHandle tempComEvent))
                {
                    tempComEvent.Set();  // signal the other instance.
                    tempComEvent.Close();

                    runShutdown = false;
                    Current.Shutdown();    // Quit temp instance
                    return;
                }
            }
            catch (System.UnauthorizedAccessException)
            {
                // Ignore exception
            }

            // Allow sleep time durations less than 16 ms
            DS5Control.Util.timeBeginPeriod(1);

            // Retrieve info about installed ViGEmBus device if found
            DS5Control.Global.RefreshViGEmBusInfo();

            // Create the Event handle
            threadComEvent = new EventWaitHandle(false, EventResetMode.ManualReset, SingleAppComEventName);
            CreateTempWorkerThread();

            CreateControlService(parser);
            RenderOptions.ProcessRenderMode = RenderMode.SoftwareOnly;

            MoveOldSettings();
            DS5Control.Global.FindConfigLocation();
            bool firstRun = DS5Control.Global.firstRun;

            // Could not find unique profile location; does not exist or multiple places.
            // Advise user to specify where DS5Control should save its configuation files
            // and profiles
            if (firstRun)
            {
                DS5Control.Global.SaveWhere(DS5Control.Global.appDataPpath);
            }

            // Exit if base configuration could not be generated
            if (firstRun && !CreateConfDirSkeleton())
            {
                MessageBox.Show($"Cannot create config folder structure in {DS5Control.Global.appdatapath}. Exiting",
                    "DS5Control", MessageBoxButton.OK, MessageBoxImage.Error);
                Current.Shutdown(1);
                return;
            }

            logHolder = new LoggerHolder(rootHub);
            DispatcherUnhandledException += App_DispatcherUnhandledException;
            AppDomain.CurrentDomain.UnhandledException += CurrentDomain_UnhandledException;
            Logger logger = logHolder.Logger;
            string version = DS5Control.Global.exeversion;
            logger.Info($"DS5Control version {version}");
            logger.Info($"DS5Control exe file: {DS5Control.Global.exeFileName}");
            logger.Info($"DS5Control Assembly Architecture: {(Environment.Is64BitProcess ? "x64" : "x86")}");
            logger.Info($"OS Version: {Environment.OSVersion}");
            logger.Info($"OS Product Name: {DS5Control.Util.GetOSProductName()}");
            logger.Info($"OS Release ID: {DS5Control.Util.GetOSReleaseId()}");
            logger.Info($"System Architecture: {(Environment.Is64BitOperatingSystem ? "x64" : "x86")}");
            logger.Info("Logger created");

            bool readAppConfig = DS5Control.Global.Load();
            if (!firstRun && !readAppConfig)
            {
                logger.Info($@"Profiles.xml not read at location ${DS5Control.Global.appdatapath}\Profiles.xml. Using default app settings");
            }

            // Without this, a clean install would ignore a DualSense.
            if (firstRun)
            {
                DS5Control.Global.DeviceOptions.DualSenseOpts.Enabled = true;
                DS5Control.Global.Save();
            }

            if (firstRun)
            {
                logger.Info("No config found. Creating default config");
                AttemptSave();

                DS5Control.Global.SaveAsNewProfile(0, "Default");
                for (int i = 0; i < DS5Control.ControlService.MAX_DS5_CONTROLLER_COUNT; i++)
                {
                    DS5Control.Global.ProfilePath[i] = DS5Control.Global.OlderProfilePath[i] = "Default";
                }

                logger.Info("Default config created");
            }

            skipSave = false;

            if (!DS5Control.Global.LoadActions())
            {
                DS5Control.Global.CreateStdActions();
            }

            // Have app use selected culture
            SetUICulture(DS5Control.Global.UseLang);

            DS5Control.Global.LoadLinkedProfiles();
            Interfaz.AppHost host = new Interfaz.AppHost(parser);
            MainWindow = host;
            interfaceWindow = new Interfaz.WebUi.WebUiWindow(rootHub, host);
            interfaceWindow.StartFailed += (_, _) =>
            {
                MessageBox.Show("DS5Control necesita WebView2 Runtime. Instálalo desde Microsoft y vuelve a abrir el programa.",
                    "DS5Control", MessageBoxButton.OK, MessageBoxImage.Error);
                host.Close();
            };
            if (!host.StartMinimized) interfaceWindow.Show();

            // Set up hooks for IPC command calls
            CreateIPCClassNameMMF(new WindowInteropHelper(host).Handle);

            bool runningAsAdmin = DS5Control.Global.IsAdministrator();
            rootHub.LogDebug($"Running as {(runningAsAdmin ? "Admin" : "User")}");

            if (DS5Control.Global.hidHideInstalled)
            {
                rootHub.CheckHidHidePresence();
            }

            rootHub.LoadPermanentSlotsConfig();
            host.LateChecks(parser);
        }

        private void CurrentDomain_UnhandledException(object sender, UnhandledExceptionEventArgs e)
        {
            if (!Current.Dispatcher.CheckAccess())
            {
                Logger logger = logHolder.Logger;
                Exception exp = e.ExceptionObject as Exception;
                logger.Error($"Thread App Crashed with message {exp.Message}");
                logger.Error(exp.ToString());
                //LogManager.Flush();
                //LogManager.Shutdown();
                if (e.IsTerminating)
                {
                    Dispatcher.Invoke(() =>
                    {
                        rootHub?.PrepareAbort();
                        CleanShutdown();
                    });
                }
            }
            else
            {
                Logger logger = logHolder.Logger;
                Exception exp = e.ExceptionObject as Exception;
                if (e.IsTerminating)
                {
                    logger.Error($"Thread Crashed with message {exp.Message}");
                    logger.Error(exp.ToString());

                    rootHub?.PrepareAbort();
                    CleanShutdown();
                }
            }
        }

        private void App_DispatcherUnhandledException(object sender, System.Windows.Threading.DispatcherUnhandledExceptionEventArgs e)
        {
            //Debug.WriteLine("App Crashed");
            //Debug.WriteLine(e.Exception.StackTrace);
            Logger logger = logHolder.Logger;
            logger.Error($"Thread Crashed with message {e.Exception.Message}");
            logger.Error(e.Exception.ToString());
            //LogManager.Flush();
            //LogManager.Shutdown();
        }

        private bool CreateConfDirSkeleton()
        {
            bool result = true;
            try
            {
                Directory.CreateDirectory(DS5Control.Global.appdatapath);
                Directory.CreateDirectory(DS5Control.Global.appdatapath + @"\Profiles\");
                Directory.CreateDirectory(DS5Control.Global.appdatapath + @"\Logs\");
                //Directory.CreateDirectory(DS5Control.Global.appdatapath + @"\Macros\");
            }
            catch (UnauthorizedAccessException)
            {
                result = false;
            }


            return result;
        }

        private void AttemptSave()
        {
            if (!DS5Control.Global.Save()) //if can't write to file
            {
                if (MessageBox.Show("Cannot write at current location\nCopy Settings to appdata?", "DS5Control",
                    MessageBoxButton.YesNo, MessageBoxImage.Warning) == MessageBoxResult.Yes)
                {
                    try
                    {
                        Directory.CreateDirectory(DS5Control.Global.appDataPpath);
                        File.Copy(DS5Control.Global.exedirpath + "\\Profiles.xml",
                            DS5Control.Global.appDataPpath + "\\Profiles.xml");
                        File.Copy(DS5Control.Global.exedirpath + "\\Auto Profiles.xml",
                            DS5Control.Global.appDataPpath + "\\Auto Profiles.xml");
                        Directory.CreateDirectory(DS5Control.Global.appDataPpath + "\\Profiles");
                        foreach (string s in Directory.GetFiles(DS5Control.Global.exedirpath + "\\Profiles"))
                        {
                            File.Copy(s, DS5Control.Global.appDataPpath + "\\Profiles\\" + Path.GetFileName(s));
                        }
                    }
                    catch { }
                    MessageBox.Show("Copy complete, please relaunch DS5Control and remove settings from Program Directory",
                        "DS5Control");
                }
                else
                {
                    MessageBox.Show("DS5Control cannot edit settings here, This will now close",
                        "DS5Control");
                }

                DS5Control.Global.appdatapath = null;
                skipSave = true;
                Current.Shutdown();
                return;
            }
        }

        private void CheckOptions(ArgumentParser parser)
        {
            if (parser.HasErrors)
            {
                runShutdown = false;
                exitApp = true;
                Current.Shutdown(1);
            }
            else if (parser.Driverinstall)
            {
                runShutdown = false;
                exitApp = true;
                Current.Shutdown();
            }
            else if (parser.ReenableDevice)
            {
                DS5Control.DS5Devices.reEnableDevice(parser.DeviceInstanceId);
                runShutdown = false;
                exitApp = true;
                Current.Shutdown();
            }
            else if (parser.Runtask)
            {
                StartupMethods.LaunchOldTask();
                runShutdown = false;
                exitApp = true;
                Current.Shutdown();
            }
            else if (parser.Command)
            {
                IntPtr hWndDS5WindowsForm = IntPtr.Zero;
                hWndDS5WindowsForm = FindWindow(ReadIPCClassNameMMF(), "DS4Windows");
                if (hWndDS5WindowsForm != IntPtr.Zero)
                {
                    bool bDoSendMsg = true;
                    bool bWaitResultData = false;
                    bool bOwnsMutex = false;
                    Mutex ipcSingleTaskMutex = null;
                    EventWaitHandle ipcNotifyEvent = null;

                    COPYDATASTRUCT cds;
                    cds.lpData = IntPtr.Zero;

                    try
                    {
                        if (parser.CommandArgs.ToLower().StartsWith("query."))
                        {
                            // Query.device# (1..4) command returns a string result via memory mapped file. The cmd is sent to the background DS5Control 
                            // process (via WM_COPYDATA wnd msg), then this client process waits for the availability of the result and prints it to console output pipe.
                            // Use mutex obj to make sure that concurrent client calls won't try to write and read the same MMF result file at the same time.
                            ipcSingleTaskMutex = new Mutex(false, "DS5Windows_IPCResultData_SingleTaskMtx");
                            try
                            {
                                bOwnsMutex = ipcSingleTaskMutex.WaitOne(10000);
                            }
                            catch (AbandonedMutexException)
                            {
                                bOwnsMutex = true;
                            }

                            if (bOwnsMutex)
                            {
                                // This process owns the inter-process sync mutex obj. Let's proceed with creating the output MMF file and waiting for a result.
                                bWaitResultData = true;
                                CreateIPCResultDataMMF();
                                ipcNotifyEvent = new EventWaitHandle(false, EventResetMode.AutoReset, "DS5Windows_IPCResultData_ReadyEvent");
                            }
                            else
                                // If the mtx failed then something must be seriously wrong. Cannot do anything in that case because MMF file may be modified by concurrent processes.
                                bDoSendMsg = false;
                        }

                        if (bDoSendMsg)
                        {
                            cds.dwData = IntPtr.Zero;
                            cds.cbData = parser.CommandArgs.Length;
                            cds.lpData = Marshal.StringToHGlobalAnsi(parser.CommandArgs);
                            SendMessage(hWndDS5WindowsForm, Interfaz.AppHost.WM_COPYDATA, IntPtr.Zero, ref cds);

                            if (bWaitResultData)
                                Console.WriteLine(WaitAndReadIPCResultDataMMF(ipcNotifyEvent));
                        }
                    }
                    finally
                    {
                        // Release the result MMF file in the client process before releasing the mtx and letting other client process to proceed with the same MMF file
                        if (ipcResultDataMMF != null) ipcResultDataMMF.Dispose();
                        ipcResultDataMMF = null;

                        // If this was "Query.xxx" cmdline client call then release the inter-process mutex and let other concurrent clients to proceed (if there are anyone waiting for the MMF result file)
                        if (bOwnsMutex && ipcSingleTaskMutex != null)
                            ipcSingleTaskMutex.ReleaseMutex();

                        if (cds.lpData != IntPtr.Zero)
                            Marshal.FreeHGlobal(cds.lpData);
                    }
                }

                runShutdown = false;
                exitApp = true;
                Current.Shutdown();
            }
        }

        private void CreateControlService(ArgumentParser parser)
        {
            controlThread = new Thread(() =>
            {
                rootHub = new DS5Control.ControlService(parser);

                DS5Control.Program.rootHub = rootHub;
                requestClient = new HttpClient();
                requestClient.DefaultRequestHeaders.Add("User-Agent", "DS4Windows");
                collectTimer = new Timer(GarbageTask, null, 30000, 30000);

            });
            controlThread.Priority = ThreadPriority.Normal;
            controlThread.IsBackground = true;
            controlThread.Start();
            while (controlThread.IsAlive)
                Thread.SpinWait(500);
        }

        private void CreateBaseThread()
        {
            controlThread = new Thread(() =>
            {
                DS5Control.Program.rootHub = rootHub;
                requestClient = new HttpClient();
                requestClient.DefaultRequestHeaders.Add("User-Agent", "DS4Windows");
                collectTimer = new Timer(GarbageTask, null, 30000, 30000);
            });
            controlThread.Priority = ThreadPriority.Normal;
            controlThread.IsBackground = true;
            controlThread.Start();
            while (controlThread.IsAlive)
                Thread.SpinWait(500);
        }

        private void GarbageTask(object state)
        {
            GC.Collect(0, GCCollectionMode.Forced, false);
        }

        private void CreateTempWorkerThread()
        {
            testThread = new Thread(SingleAppComThread_DoWork);
            testThread.Priority = ThreadPriority.Lowest;
            testThread.IsBackground = true;
            testThread.Start();
        }

        private void SingleAppComThread_DoWork()
        {
            while (!exitComThread)
            {
                // check for a signal.
                if (threadComEvent.WaitOne())
                {
                    threadComEvent.Reset();
                    // The user tried to start another instance. We can't allow that,
                    // so bring the other instance back into view and enable that one.
                    // That form is created in another thread, so we need some thread sync magic.
                    if (!exitComThread)
                    {
                        Dispatcher.BeginInvoke((Action)(() =>
                        {
                            interfaceWindow?.Reveal();
                        }));
                    }
                }
            }
        }

        public void CreateIPCClassNameMMF(IntPtr hWnd)
        {
            if (ipcClassNameMMF != null) return; // Already holding a handle to MMF file. No need to re-write the data

            try
            {
                StringBuilder wndClassNameStr = new StringBuilder(128);
                if (GetClassName(hWnd, wndClassNameStr, wndClassNameStr.Capacity) != 0 && wndClassNameStr.Length > 0)
                {
                    byte[] buffer = ASCIIEncoding.ASCII.GetBytes(wndClassNameStr.ToString());

                    ipcClassNameMMF = MemoryMappedFile.CreateNew("DS5Windows_IPCClassName.dat", 128);
                    MemoryMappedViewAccessor ipcClassNameMMA_Now = ipcClassNameMMF.CreateViewAccessor(0, buffer.Length);
                    ipcClassNameMMA_Now.WriteArray(0, buffer, 0, buffer.Length);
                    ipcClassNameMMA_Now?.Dispose();
                    // The MMF file is alive as long this process holds the file handle open
                }
            }
            catch (Exception)
            {
                /* Eat all exceptions because errors here are not fatal for DS5Win */
            }
        }

        private string ReadIPCClassNameMMF()
        {
            MemoryMappedFile mmf = null;
            MemoryMappedViewAccessor mma = null;

            try
            {
                byte[] buffer = new byte[128];
                mmf = MemoryMappedFile.OpenExisting("DS5Windows_IPCClassName.dat");
                mma = mmf.CreateViewAccessor(0, 128);
                mma.ReadArray(0, buffer, 0, buffer.Length);
                return ASCIIEncoding.ASCII.GetString(buffer);
            }
            catch (Exception)
            {
                // Eat all exceptions
            }
            finally
            {
                if (mma != null) mma.Dispose();
                if (mmf != null) mmf.Dispose();
            }

            return null;
        }

        private void CreateIPCResultDataMMF()
        {
            // Cmdline client process calls this to create the MMF file used in inter-process-communications. The background DS5Control process 
            // uses WriteIPCResultDataMMF method to write a command result and the client process reads the result from the same MMF file.
            if (ipcResultDataMMF != null) return; // Already holding a handle to MMF file. No need to re-write the data

            try
            {
                ipcResultDataMMF = MemoryMappedFile.CreateNew("DS5Windows_IPCResultData.dat", 256);
                // The MMF file is alive as long this process holds the file handle open
            }
            catch (Exception)
            {
                /* Eat all exceptions because errors here are not fatal for DS5Win */
            }
        }

        private string WaitAndReadIPCResultDataMMF(EventWaitHandle ipcNotifyEvent)
        {
            if (ipcResultDataMMF != null)
            {
                // Wait until the inter-process-communication (IPC) result data is available and read the result
                try
                {
                    // Wait max 10 secs and if the result is still not available then timeout and return "empty" result
                    if (ipcNotifyEvent == null || ipcNotifyEvent.WaitOne(10000))
                    {
                        int strNullCharIdx;
                        byte[] buffer = new byte[256];
                        MemoryMappedViewAccessor ipcResultDataMMA = ipcClassNameMMF.CreateViewAccessor(0, buffer.Length);
                        ipcResultDataMMA.ReadArray(0, buffer, 0, buffer.Length);
                        strNullCharIdx = Array.FindIndex(buffer, byteVal => byteVal == 0);
                        return ASCIIEncoding.ASCII.GetString(buffer, 0, (strNullCharIdx <= 1 ? 1 : strNullCharIdx));
                    }
                }
                catch (Exception)
                {
                    /* Eat all exceptions because errors here are not fatal for DS5Win */
                }
            }

            return String.Empty;
        }

        public void WriteIPCResultDataMMF(string dataStr)
        {
            // The background DS5Control process calls this method to write out the result of "-command QueryProfile.device#" command.
            // The cmdline client process reads the result from the DS5Windows_IPCResultData.dat MMF file and sends the result to console output pipe.
            MemoryMappedFile mmf = null;
            MemoryMappedViewAccessor mma = null;
            EventWaitHandle ipcNotifyEvent = null;

            try
            {
                ipcNotifyEvent = EventWaitHandle.OpenExisting("DS5Windows_IPCResultData_ReadyEvent");

                byte[] buffer = ASCIIEncoding.ASCII.GetBytes(dataStr);
                mmf = MemoryMappedFile.OpenExisting("DS5Windows_IPCResultData.dat");
                mma = mmf.CreateViewAccessor(0, 256);
                mma.WriteArray(0, buffer, 0, (buffer.Length >= 256 ? 256 : buffer.Length));
            }
            catch (Exception)
            {
                // Eat all exceptions
            }
            finally
            {
                if (mma != null) mma.Dispose();
                if (mmf != null) mmf.Dispose();

                if (ipcNotifyEvent != null) ipcNotifyEvent.Set();
            }
        }

        private void SetUICulture(string culture)
        {
            try
            {
                //CultureInfo ci = new CultureInfo("ja");
                CultureInfo ci = CultureInfo.GetCultureInfo(culture);
                LocalizeDictionary.Instance.SetCurrentThreadCulture = true;
                LocalizeDictionary.Instance.Culture = ci;
                // fixes the culture in threads
                CultureInfo.DefaultThreadCurrentCulture = ci;
                CultureInfo.DefaultThreadCurrentUICulture = ci;
                //DS5ControlApp.Properties.Resources.Culture = ci;
                Thread.CurrentThread.CurrentCulture = ci;
                Thread.CurrentThread.CurrentUICulture = ci;
            }
            catch (CultureNotFoundException) { /* Skip setting culture that we cannot set */ }
        }

        /// <summary>Brings the settings of the versions that still used the old folder name.</summary>
        private static void MoveOldSettings()
        {
            string old = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "DS4Windows");
            string current = DS5Control.Global.appDataPpath;
            if (!Directory.Exists(old) || Directory.Exists(current)) return;

            try
            {
                Directory.Move(old, current);
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                MessageBox.Show($"No se pudieron mover los ajustes a {current}: {exception.Message}",
                    "DS5Control", MessageBoxButton.OK, MessageBoxImage.Warning);
            }
        }

        private void Application_Exit(object sender, ExitEventArgs e)
        {
            if (runShutdown)
            {
                Logger logger = logHolder.Logger;
                logger.Info("Request App Shutdown");
                CleanShutdown();
            }
        }

        private void Application_SessionEnding(object sender, SessionEndingCancelEventArgs e)
        {
            Logger logger = logHolder.Logger;
            logger.Info("User Session Ending");
            CleanShutdown();
        }

        private void CleanShutdown()
        {
            if (runShutdown)
            {
                if (rootHub != null)
                {
                    Task.Run(() =>
                    {
                        if (rootHub.running)
                        {
                            rootHub.Stop(immediateUnplug: true);
                            rootHub.ShutDown();
                        }
                    }).Wait();
                }

                if (!skipSave)
                {
                    DS5Control.Global.Save();
                }

                // Reset timer
                DS5Control.Util.timeEndPeriod(1);

                exitComThread = true;
                if (threadComEvent != null)
                {
                    threadComEvent.Set();  // signal the other instance.
                    while (testThread.IsAlive)
                        Thread.SpinWait(500);
                    threadComEvent.Close();
                }

                if (ipcClassNameMMF != null) ipcClassNameMMF.Dispose();

                LogManager.Flush();
                LogManager.Shutdown();
            }
        }
    }
}
