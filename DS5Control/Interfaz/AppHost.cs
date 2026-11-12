using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Management;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Interop;
using System.Windows.Media;
using DS5Control;
using DS5ControlApp.Interfaz.ViewModels;
using H.NotifyIcon;
using H.NotifyIcon.Core;
using NonFormTimer = System.Timers.Timer;

namespace DS5ControlApp.Interfaz
{
    /// <summary>
    /// Invisible window that owns everything living outside the interface: the tray icon, the
    /// Windows messages for device changes and IPC, suspend and resume, hotkeys and automatic
    /// profiles.
    /// </summary>
    public sealed class AppHost : Window
    {
        /// <summary>IPC clients find the running application by this title.</summary>
        private const string IpcWindowTitle = "DS4Windows";
        private const string ReleasesPage = "https://github.com/anxo0/DS5Control/releases/latest";

        /// <summary>Gives the engine time to start before using the network.</summary>
        private const int UpdateCheckDelay = 5000;

        private const int SmallIconWidth = 49;
        private const int SmallIconHeight = 50;

        [DllImport("user32.dll")]
        private static extern int GetSystemMetrics(int index);

        private const int POWER_RESUME = 7;
        private const int POWER_SUSPEND = 4;

        private readonly ProfileList profileListHolder = new ProfileList();
        private readonly ControllerListViewModel conLvViewModel;
        private readonly TrayIconViewModel trayIconVM;
        private readonly AutoProfileHolder autoProfileHolder = new AutoProfileHolder();
        private readonly AutoProfileChecker autoprofileChecker;
        private readonly bool startMinimized;
        private H.NotifyIcon.TaskbarIcon notifyIcon;
        private IntPtr regHandle = new IntPtr();
        private ManagementEventWatcher managementEvWatcher;
        private bool wasrunning;
        private NonFormTimer hotkeysTimer;
        private NonFormTimer autoProfilesTimer;

        public AutoProfileHolder AutoProfiles => autoProfileHolder;

        public ProfileList Profiles => profileListHolder;

        public bool StartMinimized => startMinimized;

        /// <summary>Whether the user is looking at the interface, so no notification is needed.</summary>
        public Func<bool> IsInterfaceActive { get; set; } = () => false;

        public Func<bool> IsInterfaceVisible { get; set; } = () => false;

        /// <summary>Tells the open window an update is about to close it; done when it has been seen.</summary>
        public Func<Version, Task> AnnounceUpdate { get; set; } = _ => Task.CompletedTask;

        private bool InterfaceActive => IsInterfaceActive();

        /// <summary>Brings the interface to the front.</summary>
        public Action OpenInterface { get; set; } = () => { };

        public AppHost(ArgumentParser parser)
        {
            Title = IpcWindowTitle;
            Width = 0;
            Height = 0;
            WindowStyle = WindowStyle.None;
            ShowInTaskbar = false;
            ShowActivated = false;

            profileListHolder.Refresh();
            conLvViewModel = new ControllerListViewModel(App.rootHub, profileListHolder);
            trayIconVM = new TrayIconViewModel(App.rootHub, profileListHolder);
            CreateTrayIcon();

            startMinimized = Global.StartMinimized || parser.Mini;
            autoprofileChecker = new AutoProfileChecker(autoProfileHolder);
            SetupEvents();

            // Don't tie timers to main thread
            Thread timerThread = new Thread(() =>
            {
                hotkeysTimer = new NonFormTimer { Interval = 20, AutoReset = false };
                autoProfilesTimer = new NonFormTimer { Interval = 1000, AutoReset = false };
            });
            timerThread.IsBackground = true;
            timerThread.Priority = ThreadPriority.Lowest;
            timerThread.Start();
            // Wait for thread tasks to finish before continuing
            timerThread.Join();

            IntPtr handle = new WindowInteropHelper(this).EnsureHandle();
            HwndSource source = HwndSource.FromHwnd(handle);
            HookWindowMessages(source);
            source.AddHook(WndProc);
        }

        private void CreateTrayIcon()
        {
            notifyIcon = new H.NotifyIcon.TaskbarIcon
            {
                DataContext = trayIconVM,
                CustomName = Global.exelocation,
                MenuActivation = PopupActivationMode.RightClick,
            };
            notifyIcon.SetBinding(H.NotifyIcon.TaskbarIcon.IconSourceProperty, new Binding(nameof(TrayIconViewModel.IconSource)) { Mode = BindingMode.OneWay });
            notifyIcon.SetBinding(H.NotifyIcon.TaskbarIcon.ToolTipTextProperty, new Binding(nameof(TrayIconViewModel.TooltipText)) { Mode = BindingMode.OneWay });
            notifyIcon.TrayRightMouseUp += (_, _) => notifyIcon.ContextMenu = trayIconVM.ContextMenu;
            notifyIcon.TrayMiddleMouseDown += (_, _) => Close();
            notifyIcon.TrayMouseDoubleClick += (_, _) => OpenInterface();

            try
            {
                notifyIcon.ForceCreate(enablesEfficiencyMode: false);
            }
            catch (Exception exception)
            {
                AppLogger.LogToGui($"Could not create the tray icon: {exception.Message}", true);
            }

            trayIconVM.UseModernMenu();
        }

        public void SetTrayMenuTheme(bool dark) => trayIconVM.SetMenuTheme(dark);

        public void SetHotkeysEnabled(bool enabled)
        {
            if (App.rootHub.running) ChangeHotkeysStatus(enabled);
        }

        public void LateChecks(ArgumentParser parser)
        {
            if (!parser.Stop)
            {
                Task startup = Task.Run(() =>
                {
                    Thread.Sleep(1000);
                    App.rootHub.Start();
                });
                Util.LogAssistBackgroundTask(startup);
            }

            Version updated = AppUpdater.TakeNotice(Global.appdatapath, typeof(AppHost).Assembly.GetName().Version);
            if (updated != null)
            {
                string message = $"DS5Control se ha actualizado a la versión {Short(updated)}.";
                AppLogger.LogToGui(message, false);
                ShowUpdateNotification(message);
            }

            if (Global.CheckWhen > 0)
            {
                Task updates = Task.Delay(UpdateCheckDelay).ContinueWith(_ => UpdateAsync()).Unwrap();
                Util.LogAssistBackgroundTask(updates);
            }
        }

        /// <summary>
        /// Moves to the latest release if there is a newer one, and reopens the program.
        /// Returns false when this is already the latest version.
        /// </summary>
        public async Task<bool> UpdateAsync()
        {
#if DEBUG
            // A development build lives in bin: it must never be replaced by a release.
            AppLogger.LogToGui("Las compilaciones de desarrollo no se actualizan.", false);
            return await Task.FromResult(false);
#else
            try
            {
                using var client = new System.Net.Http.HttpClient();
                client.DefaultRequestHeaders.UserAgent.ParseAdd("DS5Control");

                AppUpdate update = await AppUpdater.FindAsync(client, typeof(AppHost).Assembly.GetName().Version);
                if (update == null) return false;

                if (!AppUpdater.CanWriteTo(Global.exedirpath))
                {
                    AppLogger.LogToGui($"Hay una versión nueva ({update.Version}), pero no se puede escribir en {Global.exedirpath}. Descárgala de {ReleasesPage}", true);
                    return true;
                }

                string files = await AppUpdater.DownloadAsync(client, update);

                // Closing and reopening on its own looks like a crash unless it is said first.
                string version = Short(update.Version);
                AppLogger.LogToGui($"Actualizando a la versión {version}...", false);
                bool visible = await Dispatcher.InvokeAsync(() => IsInterfaceVisible());
                await Dispatcher.InvokeAsync(() => ShowUpdateNotification(
                    $"Actualizando a la versión {version}. DS5Control se cerrará y volverá a abrirse solo en unos segundos."));
                await await Dispatcher.InvokeAsync(() => AnnounceUpdate(update.Version));

                AppUpdater.SaveNotice(Global.appdatapath, update.Version);
                using Process install = AppUpdater.StartInstall(files, Global.exedirpath, Environment.ProcessId,
                    toTray: !visible);

                await Dispatcher.InvokeAsync(Close);
                return true;
            }
            catch (Exception exception) when (exception is System.Net.Http.HttpRequestException or TaskCanceledException
                or IOException or UnauthorizedAccessException or System.Text.Json.JsonException)
            {
                AppLogger.LogToGui($"No se pudo actualizar: {exception.Message}", true);
                return true;
            }
#endif
        }

        private static string Short(Version version) => version.ToString(3);

        /// <summary>Shown whatever the notification setting: the user has to know why the program closes.</summary>
        private void ShowUpdateNotification(string message)
        {
            notifyIcon?.ShowNotification(TrayIconViewModel.ballonTitle, message, H.NotifyIcon.Core.NotificationIcon.Info);
        }

        public async void RestartService()
        {
            Task restart = Task.Run(() =>
            {
                App.rootHub.Stop();
                App.rootHub.Start();
            });
            Util.LogAssistBackgroundTask(restart);
            await restart;
        }

        public async void ChangeService()
        {
            ControlService service = App.rootHub;
            Task serviceTask = Task.Run(() =>
            {
                if (service.running)
                    service.Stop(immediateUnplug: true);
                else
                    service.Start();
            });

            // Log exceptions that might occur
            Util.LogAssistBackgroundTask(serviceTask);
            await serviceTask;
        }

        /// <summary>Follows connections and mode changes; HidHide is only called when something changed.</summary>
        private readonly System.Windows.Threading.DispatcherTimer hidHideSync = new System.Windows.Threading.DispatcherTimer { Interval = TimeSpan.FromSeconds(2) };

        private void SetupEvents()
        {
            hidHideSync.Tick += (_, _) => HidHideControl.Apply(HidHideControl.EmulatedControllers());
            hidHideSync.Start();
            App.rootHub.ServiceStarted += ControlServiceStarted;
            App.rootHub.PreServiceStop += PrepareForServiceStop;
            conLvViewModel.ControllerCol.CollectionChanged += ControllerCol_CollectionChanged;
            AppLogger.TrayIconLog += ShowNotification;
            trayIconVM.RequestShutdown += (_, _) => Close();
            trayIconVM.ProfileSelected += TrayIconVM_ProfileSelected;
            trayIconVM.RequestOpen += (_, _) => OpenInterface();
            trayIconVM.RequestServiceChange += (_, _) => ChangeService();
            autoprofileChecker.RequestServiceChange += AutoprofileChecker_RequestServiceChange;
            autoProfileHolder.AutoProfileColl.CollectionChanged += AutoProfileColl_CollectionChanged;

            bool wmiConnected = false;
            WqlEventQuery q = new WqlEventQuery();
            ManagementScope scope = new ManagementScope("root\\CIMV2");
            q.EventClassName = "Win32_PowerManagementEvent";

            try
            {
                scope.Connect();
            }
            catch (COMException) { }
            catch (ManagementException) { }

            if (scope.IsConnected)
            {
                wmiConnected = true;
                managementEvWatcher = new ManagementEventWatcher(scope, q);
                managementEvWatcher.EventArrived += PowerEventArrive;
                try
                {
                    managementEvWatcher.Start();
                }
                catch (ManagementException) { wmiConnected = false; }
                catch (COMException) { wmiConnected = false; }
            }

            if (!wmiConnected)
            {
                AppLogger.LogToGui(@"Could not connect to Windows Management Instrumentation service.
Suspend support not enabled.", true);
            }
        }

        private void TrayIconVM_ProfileSelected(TrayIconViewModel sender, ControllerHolder item, string profile)
        {
            if (conLvViewModel.ControllerDict.TryGetValue(item.Index, out CompositeDeviceModel device))
            {
                device.ChangeSelectedProfile(profile);
            }
        }

        private void PrepareForServiceStop(object sender, EventArgs e)
        {
            Dispatcher.BeginInvoke(() => trayIconVM.ClearContextMenu());
            ChangeHotkeysStatus(false);
        }

        private void ControllerCol_CollectionChanged(object sender,
            System.Collections.Specialized.NotifyCollectionChangedEventArgs e)
        {
            Dispatcher.BeginInvoke(() =>
            {
                if (e.NewItems != null)
                {
                    foreach (CompositeDeviceModel item in e.NewItems)
                    {
                        item.Device.SyncChange += DS5Device_SyncChange;
                    }
                }

                if (App.rootHub.running) trayIconVM.PopulateContextMenu();
            });
        }

        protected override void OnClosed(EventArgs e)
        {
            hidHideSync.Stop();
            // The controller has to be visible again for every program once DS5Control is gone.
            HidHideControl.Apply(Array.Empty<string>());
            hotkeysTimer.Stop();
            autoProfilesTimer.Stop();
            Util.UnregisterNotify(regHandle);

            // Attempt to dispose of notify icon early
            notifyIcon?.Dispose();
            notifyIcon = null;

            base.OnClosed(e);
            Application.Current.Shutdown();
        }

        private void ShowHotkeyNotification(string message)
        {
            if (!InterfaceActive && Global.Notifications == 2)
            {
                notifyIcon.ShowNotification(TrayIconViewModel.ballonTitle, message, H.NotifyIcon.Core.NotificationIcon.Info);
            }
        }

        private void ShowNotification(object sender, DS5Control.DebugEventArgs e)
        {
            Dispatcher.BeginInvoke((Action)(() =>
            {

                if (!InterfaceActive && (Global.Notifications == 2 ||
                    (Global.Notifications == 1 && e.Warning)))
                {
                    if (notifyIcon.IsCreated)
                    {
                        try
                        {
                            notifyIcon.ShowNotification(TrayIconViewModel.ballonTitle,
                            e.Data, !e.Warning ? H.NotifyIcon.Core.NotificationIcon.Info :
                            H.NotifyIcon.Core.NotificationIcon.Warning);
                        }
                        catch (System.InvalidOperationException)
                        {
                            // Ignore
                        }
                    }
                }
            }));
        }

        private void ControlServiceStarted(object sender, EventArgs e)
        {
            if (Global.SwipeProfiles)
            {
                ChangeHotkeysStatus(true);
            }

            CheckAutoProfileStatus();
        }

        private void AutoprofileChecker_RequestServiceChange(AutoProfileChecker sender, bool state)
        {
            Dispatcher.BeginInvoke((Action)(() =>
            {
                ChangeService();
            }));
        }

        private void AutoProfileColl_CollectionChanged(object sender,
            System.Collections.Specialized.NotifyCollectionChangedEventArgs e)
        {
            CheckAutoProfileStatus();
        }

        private void PowerEventArrive(object sender, EventArrivedEventArgs e)
        {
            short evType = Convert.ToInt16(e.NewEvent.GetPropertyValue("EventType"));
            switch (evType)
            {
                // Wakeup from Suspend
                case POWER_RESUME:
                    {
                        DS5LightBar.shuttingdown = false;
                        App.rootHub.suspending = false;

                        if (wasrunning)
                        {
                            wasrunning = false;

                            Program.rootHub.LogDebug(DS5ControlApp.Translations.Strings.WakeupFromSuspend);
                            //Program.rootHub.LogDebug($"{Thread.CurrentThread.ManagedThreadId}");

                            //Thread.Sleep(60000);
                            //App.rootHub.Start();

                            //Task startupTask = Task.Run(() =>
                            Task startupTask = Task.Delay(5000).ContinueWith(t =>
                            {
                                App.rootHub.Start();
                            });

                            // Log exceptions that might occur
                            Util.LogAssistBackgroundTask(startupTask);
                        }
                    }

                    break;
                // Entering Suspend
                case POWER_SUSPEND:
                    {
                        DS5LightBar.shuttingdown = true;
                        Program.rootHub.suspending = true;

                        if (App.rootHub.running)
                        {
                            App.rootHub.Stop(immediateUnplug: true);
                            wasrunning = true;

                            Thread.Sleep(1000);
                        }
                    }

                    break;

                default: break;
            }
        }

        private void ChangeHotkeysStatus(bool state)
        {
            if (state)
            {
                hotkeysTimer.Elapsed += HotkeysTimer_Elapsed;
                hotkeysTimer.Start();
            }
            else
            {
                hotkeysTimer.Stop();
                hotkeysTimer.Elapsed -= HotkeysTimer_Elapsed;
            }
        }

        private void HotkeysTimer_Elapsed(object sender, System.Timers.ElapsedEventArgs e)
        {
            hotkeysTimer.Stop();

            if (Global.SwipeProfiles)
            {
                foreach (CompositeDeviceModel item in conLvViewModel.ControllerCol)
                //for (int i = 0; i < 4; i++)
                {
                    string slide = App.rootHub.TouchpadSlide(item.DevIndex);
                    if (slide == "left")
                    {
                        //int ind = i;
                        Dispatcher.BeginInvoke((Action)(() =>
                        {
                            if (item.SelectedIndex <= 0)
                            {
                                item.SelectedIndex = item.ProfileListCol.Count - 1;
                            }
                            else
                            {
                                item.SelectedIndex--;
                            }
                        }));
                    }
                    else if (slide == "right")
                    {
                        //int ind = i;
                        Dispatcher.BeginInvoke((Action)(() =>
                        {
                            if (item.SelectedIndex == (item.ProfileListCol.Count - 1))
                            {
                                item.SelectedIndex = 0;
                            }
                            else
                            {
                                item.SelectedIndex++;
                            }
                        }));
                    }

                    if (slide.Contains("t"))
                    {
                        //int ind = i;
                        Dispatcher.BeginInvoke((Action)(() =>
                        {
                            string temp = string.Format(Properties.Resources.UsingProfile, (item.DevIndex + 1).ToString(), item.SelectedProfile, $"{item.Device.Battery}");
                            ShowHotkeyNotification(temp);
                        }));
                    }
                }
            }

            hotkeysTimer.Start();
        }

        private void ChangeAutoProfilesStatus(bool state)
        {
            if (state)
            {
                autoProfilesTimer.Elapsed += AutoProfilesTimer_Elapsed;
                autoProfilesTimer.Start();
                autoprofileChecker.Running = true;
            }
            else
            {
                autoProfilesTimer.Stop();
                autoProfilesTimer.Elapsed -= AutoProfilesTimer_Elapsed;
                autoprofileChecker.Running = false;
            }
        }

        private void CheckAutoProfileStatus()
        {
            int pathCount = autoProfileHolder.AutoProfileColl.Count;
            bool timerEnabled = autoprofileChecker.Running;
            if (pathCount > 0 && !timerEnabled)
            {
                ChangeAutoProfilesStatus(true);
            }
            else if (pathCount == 0 && timerEnabled)
            {
                ChangeAutoProfilesStatus(false);
            }
        }

        private void AutoProfilesTimer_Elapsed(object sender, System.Timers.ElapsedEventArgs e)
        {
            autoProfilesTimer.Stop();
            //Console.WriteLine("Event triggered");
            autoprofileChecker.Process();

            if (autoprofileChecker.Running)
            {
                autoProfilesTimer.Start();
            }
        }

        private void DS5Device_SyncChange(object sender, EventArgs e)
        {
            Dispatcher.BeginInvoke((Action)(() =>
            {
                trayIconVM.PopulateContextMenu();
            }));
        }

        private void InnerHotplug2()
        {
            inHotPlug = true;

            bool loopHotplug = false;

            lock (hotplugCounterLock)
            {
                loopHotplug = hotplugCounter > 0;
                hotplugCounter = 0;
            }

            Program.rootHub.UpdateHidHiddenAttributes();
            while (loopHotplug == true)
            {
                Thread.Sleep(HOTPLUG_CHECK_DELAY);
                Program.rootHub.HotPlug();

                lock (hotplugCounterLock)
                {
                    loopHotplug = hotplugCounter > 0;
                    hotplugCounter = 0;
                }
            }

            inHotPlug = false;
        }

        private void HookWindowMessages(HwndSource source)
        {
            Guid hidGuid = new Guid();
            NativeMethods.HidD_GetHidGuid(ref hidGuid);
            bool result = Util.RegisterNotify(source.Handle, hidGuid, ref regHandle);
            if (!result)
            {
                App.Current.Shutdown();
            }
        }

        public void SetAppIcon(string palette)
        {
            string icon = $"{Global.RESOURCES_PREFIX}/Logo/ds5control-{palette}.ico";
            Global.iconChoiceResources[TrayIconChoice.Default] = icon;
            Global.iconChoiceResources[TrayIconChoice.Colored] = icon;
            Global.iconChoiceResources[TrayIconChoice.Battery] = icon;
            if (Global.UseIconChoice != TrayIconChoice.White && Global.UseIconChoice != TrayIconChoice.Black)
            {
                // Set directly: the conversion from IconSource leaves this icon empty.
                string file = Path.Combine(AppContext.BaseDirectory, "iconos", $"ds5control-{palette}.ico");
                if (File.Exists(file))
                {
                    notifyIcon.Icon = new System.Drawing.Icon(file, GetSystemMetrics(SmallIconWidth), GetSystemMetrics(SmallIconHeight));
                }
            }
        }

        private bool inHotPlug = false;
        private int hotplugCounter = 0;
        private object hotplugCounterLock = new object();
        private const int DBT_DEVNODES_CHANGED = 0x0007;
        private const int DBT_DEVICEARRIVAL = 0x8000;
        private const int DBT_DEVICEREMOVECOMPLETE = 0x8004;
        public const int WM_COPYDATA = 0x004A;
        private const int HOTPLUG_CHECK_DELAY = 2000;

        private IntPtr WndProc(IntPtr hwnd, int msg, IntPtr wParam,
            IntPtr lParam, ref bool handled)
        {
            // Handle messages...
            switch (msg)
            {
                case Util.WM_DEVICECHANGE:
                {
                    if (Global.runHotPlug)
                    {
                        Int32 Type = wParam.ToInt32();
                        if (Type == DBT_DEVICEARRIVAL ||
                            Type == DBT_DEVICEREMOVECOMPLETE)
                        {
                            lock (hotplugCounterLock)
                            {
                                hotplugCounter++;
                            }

                            if (!inHotPlug)
                            {
                                inHotPlug = true;
                                Task hotplugTask = Task.Run(() => { InnerHotplug2(); });
                                // Log exceptions that might occur
                                Util.LogAssistBackgroundTask(hotplugTask);
                            }
                        }
                    }
                    break;
                }
                case WM_COPYDATA:
                {
                    // Received InterProcessCommunication (IPC) message. DS5Win command is embedded as a string value in lpData buffer
                    try
                    {
                        App.COPYDATASTRUCT cds = (App.COPYDATASTRUCT)Marshal.PtrToStructure(lParam, typeof(App.COPYDATASTRUCT));
                        if (cds.cbData >= 4 && cds.cbData <= 256)
                        {
                            int tdevice = -1;

                            byte[] buffer = new byte[cds.cbData];
                            Marshal.Copy(cds.lpData, buffer, 0, cds.cbData);
                            string[] strData = Encoding.ASCII.GetString(buffer).Split('.');

                            if (strData.Length >= 1)
                            {
                                strData[0] = strData[0].ToLower();

                                if (strData[0] == "start")
                                { 
                                    if(!Program.rootHub.running) 
                                        ChangeService();
                                }
                                else if (strData[0] == "stop")
                                {    
                                    if (Program.rootHub.running)
                                        ChangeService();
                                }
                                else if (strData[0] == "cycle")
                                {
                                    ChangeService();
                                }
                                else if (strData[0] == "shutdown")
                                {
                                    // Force disconnect all gamepads before closing the app to avoid "Are you sure you want to close the app" messagebox
                                    if (Program.rootHub.running)
                                        ChangeService();

                                    Close();
                                }
                                else if (strData[0] == "disconnect")
                                {
                                    // Command syntax: Disconnect[.device#] (fex Disconnect.1)
                                    // Disconnect all wireless controllers. ex. (Disconnect)
                                    if (strData.Length == 1)
                                    {
                                        // Attempt to disconnect all wireless controllers
                                        // Opt to make copy of Dictionary before iterating over contents
                                        var dictCopy = new Dictionary<int, CompositeDeviceModel>(conLvViewModel.ControllerDict);
                                        foreach(KeyValuePair<int, CompositeDeviceModel> pair in dictCopy)
                                        {
                                            pair.Value.RequestDisconnect();
                                        }
                                    }
                                    else
                                    {
                                        // Attempt to disconnect one wireless controller
                                        if (int.TryParse(strData[1], out tdevice)) tdevice--;

                                        if (conLvViewModel.ControllerDict.TryGetValue(tdevice, out CompositeDeviceModel model))
                                        {
                                            model.RequestDisconnect();
                                        }
                                    }
                                }
                                else if ((strData[0] == "changeledcolor") && strData.Length >= 5)
                                {
                                        // Command syntax: changeledcolor.device#.red.gree.blue (ex changeledcolor.1.255.0.0)
                                   if (int.TryParse(strData[1], out tdevice))
                                        tdevice--;
                                    if (tdevice >= 0 && tdevice < ControlService.MAX_DS5_CONTROLLER_COUNT)
                                    {
                                        byte.TryParse(strData[2], out byte red);
                                        byte.TryParse(strData[3], out byte green);
                                        byte.TryParse(strData[4], out byte blue);

                                        conLvViewModel.ControllerCol[tdevice].UpdateCustomLightColor(Color.FromRgb(red, green, blue));
                                    }

                                }
                                else if ((strData[0] == "loadprofile" || strData[0] == "loadtempprofile") && strData.Length >= 3)
                                {
                                    // Command syntax: LoadProfile.device#.profileName (fex LoadProfile.1.GameSnake or LoadTempProfile.1.WebBrowserSet)
                                    if (int.TryParse(strData[1], out tdevice)) tdevice--;

                                    if (tdevice >= 0 && tdevice < ControlService.MAX_DS5_CONTROLLER_COUNT &&
                                            File.Exists(Global.appdatapath + "\\Profiles\\" + strData[2] + ".xml"))
                                    {
                                        if (strData[0] == "loadprofile")
                                        {
                                            int idx = profileListHolder.ProfileListCol.Select((item, index) => new { item, index }).
                                                    Where(x => x.item.Name == strData[2]).Select(x => x.index).DefaultIfEmpty(-1).First();

                                            if (idx >= 0 && tdevice < conLvViewModel.ControllerCol.Count)
                                            {
                                                conLvViewModel.ControllerCol[tdevice].ChangeSelectedProfile(strData[2]);
                                            }
                                            else
                                            {
                                                // Preset profile name for later loading
                                                Global.ProfilePath[tdevice] = strData[2];
                                                //Global.LoadProfile(tdevice, true, Program.rootHub);
                                            }
                                        }
                                        else
                                        {
                                            Task.Run(() =>
                                            {
                                                DS5Device device = conLvViewModel.ControllerCol[tdevice].Device;
                                                if (device != null)
                                                {
                                                    device.HaltReportingRunAction(() =>
                                                    {
                                                        Global.LoadTempProfile(tdevice, strData[2], true, Program.rootHub);
                                                    });
                                                }
                                            }).Wait();
                                        }

                                        DS5Device device = conLvViewModel.ControllerCol[tdevice].Device;
                                        if (device != null)
                                        {
                                            string prolog = string.Format(Properties.Resources.UsingProfile, (tdevice + 1).ToString(), strData[2], $"{device.Battery}");
                                            Program.rootHub.LogDebug(prolog);
                                        }
                                    }
                                }
                                else if (strData[0] == "outputslot" && strData.Length >= 3)
                                {
                                    // Command syntax: 
                                    //    OutputSlot.slot#.Unplug
                                    //    OutputSlot.slot#.PlugDS5
                                    //    OutputSlot.slot#.PlugX360
                                    if (int.TryParse(strData[1], out tdevice))
                                        tdevice--;

                                    if (tdevice >= 0 && tdevice < ControlService.MAX_DS5_CONTROLLER_COUNT)
                                    {
                                        strData[2] = strData[2].ToLower();
                                        Motor.OutSlotDevice slotDevice = Program.rootHub.OutputslotMan.OutputSlots[tdevice];
                                        if (strData[2] == "unplug")
                                            Program.rootHub.DetachUnboundOutDev(slotDevice);
                                        else if (strData[2] == "plugds4")
                                            Program.rootHub.AttachUnboundOutDev(slotDevice, OutContType.DS4);
                                        else if (strData[2] == "plugx360")
                                            Program.rootHub.AttachUnboundOutDev(slotDevice, OutContType.X360);
                                    }
                                }
                                else if (strData[0] == "query" && strData.Length >= 3)
                                {
                                    string propName;
                                    string propValue = String.Empty;

                                    // Command syntax: QueryProfile.device#.Name (fex "Query.1.ProfileName" would print out the name of the active profile in controller 1)
                                    if (int.TryParse(strData[1], out tdevice))
                                        tdevice--;

                                    if (tdevice >= 0 && tdevice < ControlService.MAX_DS5_CONTROLLER_COUNT)
                                    {
                                        // Name of the property to query from a profile or DS5Control app engine
                                        propName = strData[2].ToLower();

                                            if (propName == "profilename")
                                            {
                                                if (Global.useTempProfile[tdevice])
                                                    propValue = Global.tempprofilename[tdevice];
                                                else
                                                    propValue = Global.ProfilePath[tdevice];
                                            }
                                            else if (propName == "outconttype")
                                                propValue = Global.OutContType[tdevice].ToString();
                                            else if (propName == "activeoutdevtype")
                                                propValue = Global.activeOutDevType[tdevice].ToString();
                                            else if (propName == "usedinputonly")
                                                propValue = Global.useDInputOnly[tdevice].ToString();

                                            else if (propName == "devicevidpid" && App.rootHub.DS5Controllers[tdevice] != null)
                                                propValue = $"VID={App.rootHub.DS5Controllers[tdevice].HidDevice.Attributes.VendorHexId}, PID={App.rootHub.DS5Controllers[tdevice].HidDevice.Attributes.ProductHexId}";
                                            else if (propName == "devicepath" && App.rootHub.DS5Controllers[tdevice] != null)
                                                propValue = App.rootHub.DS5Controllers[tdevice].HidDevice.DevicePath;
                                            else if (propName == "macaddress" && App.rootHub.DS5Controllers[tdevice] != null)
                                                propValue = App.rootHub.DS5Controllers[tdevice].MacAddress;
                                            else if (propName == "displayname" && App.rootHub.DS5Controllers[tdevice] != null)
                                                propValue = App.rootHub.DS5Controllers[tdevice].DisplayName;
                                            else if (propName == "conntype" && App.rootHub.DS5Controllers[tdevice] != null)
                                                propValue = App.rootHub.DS5Controllers[tdevice].ConnectionType.ToString();
                                            else if (propName == "exclusivestatus" && App.rootHub.DS5Controllers[tdevice] != null)
                                                propValue = App.rootHub.DS5Controllers[tdevice].CurrentExclusiveStatus.ToString();
                                            else if (propName == "battery" && App.rootHub.DS5Controllers[tdevice] != null)
                                                propValue = App.rootHub.DS5Controllers[tdevice].Battery.ToString();
                                            else if (propName == "charging" && App.rootHub.DS5Controllers[tdevice] != null)
                                                propValue = App.rootHub.DS5Controllers[tdevice].Charging.ToString();
                                            else if (propName == "outputslottype")
                                                propValue = App.rootHub.OutputslotMan.OutputSlots[tdevice].CurrentType.ToString();
                                            else if (propName == "outputslotpermanenttype")
                                                propValue = App.rootHub.OutputslotMan.OutputSlots[tdevice].PermanentType.ToString();
                                            else if (propName == "outputslotattachedstatus")
                                                propValue = App.rootHub.OutputslotMan.OutputSlots[tdevice].CurrentAttachedStatus.ToString();
                                            else if (propName == "outputslotinputbound")
                                                propValue = App.rootHub.OutputslotMan.OutputSlots[tdevice].CurrentInputBound.ToString();

                                            else if (propName == "apprunning")
                                                propValue = App.rootHub.running.ToString(); // Controller idx value is ignored, but it still needs to be in 1..4 range in a cmdline call
                                    }

                                    // Write out the property value to MMF result data file and notify a client process that the data is available
                                    ((Application.Current) as App).WriteIPCResultDataMMF(propValue);
                                }
                            }
                        }
                    }
                    catch
                    {
                        // Eat all exceptions in WM_COPYDATA because exceptions here are not fatal for DS5Control background app
                    }
                    break;
                }
                default: break;
            }

            return IntPtr.Zero;
        }
    }
}
