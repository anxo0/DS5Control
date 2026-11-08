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

using DS5Control.Motor;
using DS5ControlApp.Motor;
using Microsoft.Win32;
using Nefarius.ViGEm.Client;
using Sensorit.Base;
using SharpOSC;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Windows.Threading;
using static DS5Control.Global;

namespace DS5Control
{
    public class ControlService
    {
        public ViGEmClient vigemTestClient = null;
        // Might be useful for ScpVBus build
        public const int EXPANDED_CONTROLLER_COUNT = 8;
        public const int MAX_DS5_CONTROLLER_COUNT = Global.MAX_DS5_CONTROLLER_COUNT;
#if FORCE_4_INPUT
        public static int CURRENT_DS5_CONTROLLER_LIMIT = Global.OLD_XINPUT_CONTROLLER_COUNT;
#else
        public static int CURRENT_DS5_CONTROLLER_LIMIT = Global.IsWin8OrGreater() ? MAX_DS5_CONTROLLER_COUNT : Global.OLD_XINPUT_CONTROLLER_COUNT;
#endif
        public static bool USING_MAX_CONTROLLERS = CURRENT_DS5_CONTROLLER_LIMIT == EXPANDED_CONTROLLER_COUNT;
        public DS5Device[] DS5Controllers = new DS5Device[MAX_DS5_CONTROLLER_COUNT];
        public int activeControllers = 0;
        public Mouse[] touchPad = new Mouse[MAX_DS5_CONTROLLER_COUNT];
        public bool running = false;
        public bool loopControllers = true;
        public bool inServiceTask = false;
        private DS5State[] MappedState = new DS5State[MAX_DS5_CONTROLLER_COUNT];
        private DS5State[] CurrentState = new DS5State[MAX_DS5_CONTROLLER_COUNT];
        private DS5State[] PreviousState = new DS5State[MAX_DS5_CONTROLLER_COUNT];
        private DS5State[] TempState = new DS5State[MAX_DS5_CONTROLLER_COUNT];
        public DS5StateExposed[] ExposedState = new DS5StateExposed[MAX_DS5_CONTROLLER_COUNT];
        public ControllerSlotManager slotManager = new ControllerSlotManager();
        public bool recordingMacro = false;
        public event EventHandler<DebugEventArgs> Debug = null;
        bool[] buttonsdown = new bool[MAX_DS5_CONTROLLER_COUNT] { false, false, false, false, false, false, false, false };
        bool[] held = new bool[MAX_DS5_CONTROLLER_COUNT];
        int[] oldmouse = new int[MAX_DS5_CONTROLLER_COUNT] { -1, -1, -1, -1, -1, -1, -1, -1 };
        public OutputDevice[] outputDevices = new OutputDevice[MAX_DS5_CONTROLLER_COUNT] { null, null, null, null, null, null, null, null };
        private OneEuroFilter3D[] udpEuroPairAccel = new OneEuroFilter3D[UdpServer.NUMBER_SLOTS]
        {
            new OneEuroFilter3D(), new OneEuroFilter3D(),
            new OneEuroFilter3D(), new OneEuroFilter3D(),
        };
        private OneEuroFilter3D[] udpEuroPairGyro = new OneEuroFilter3D[UdpServer.NUMBER_SLOTS]
        {
            new OneEuroFilter3D(), new OneEuroFilter3D(),
            new OneEuroFilter3D(), new OneEuroFilter3D(),
        };
        Thread tempThread;
        Thread tempBusThread;
        Thread eventDispatchThread;
        // Not interface code: System.Windows.Threading.Dispatcher (WindowsBase) is used here only as
        // the ordered work queue of the "ControlService Events" thread. It has no window behind it.
        // HID discovery and hotplug run on this thread and depend on that ordering.
        Dispatcher eventDispatcher;
        public bool suspending;

        private UdpServer _udpServer;
        private OutputSlotManager outputslotMan;

        private HashSet<string> hidDeviceHidingAffectedDevs = new HashSet<string>();
        private HashSet<string> hidDeviceHidingExemptedDevs = new HashSet<string>();
        private bool hidDeviceHidingForced = false;
        private bool hidDeviceHidingEnabled = false;

        private ControlServiceDeviceOptions deviceOptions;
        public ControlServiceDeviceOptions DeviceOptions { get => deviceOptions; }

        private DS5ControlApp.ArgumentParser cmdParser;

        public event EventHandler ServiceStarted;
        public event EventHandler PreServiceStop;
        public event EventHandler ServiceStopped;
        public event EventHandler RunningChanged;
        //public event EventHandler HotplugFinished;
        public delegate void HotplugControllerHandler(ControlService sender, DS5Device device, int index);
        public event HotplugControllerHandler HotplugController;
        public delegate void MuteButtonPressedHandler(ControlService sender, DS5Device device, int index);
        /// <summary>Raised on the input thread of the device: handlers must return at once.</summary>
        public event MuteButtonPressedHandler MuteButtonPressed;

        private byte[][] udpOutBuffers = new byte[UdpServer.NUMBER_SLOTS][]
        {
            new byte[UdpServer.DATA_RSP_PACKET_LEN], new byte[UdpServer.DATA_RSP_PACKET_LEN],
            new byte[UdpServer.DATA_RSP_PACKET_LEN], new byte[UdpServer.DATA_RSP_PACKET_LEN],
        };

        private DS5State[] oscState = new DS5State[MAX_DS5_CONTROLLER_COUNT];
        public HandleOscPacket oscCallback;

        public UDPListener oscListener;
        public UDPSender oscSender;

        void GetPadDetailForIdx(int padIdx, ref DualShockPadMeta meta)
        {
            //meta = new DualShockPadMeta();
            meta.PadId = (byte)padIdx;
            meta.Model = DsModel.DS4;

            var d = DS5Controllers[padIdx];
            if (d == null)
            {
                meta.PadMacAddress = null;
                meta.PadState = DsState.Disconnected;
                meta.ConnectionType = DsConnection.None;
                meta.Model = DsModel.None;
                meta.BatteryStatus = 0;
                meta.IsActive = false;
                return;
                //return meta;
            }

            bool isValidSerial = false;
            string stringMac = d.getMacAddress();
            if (!string.IsNullOrEmpty(stringMac))
            {
                stringMac = string.Join("", stringMac.Split(':'));
                //stringMac = stringMac.Replace(":", "").Trim();
                meta.PadMacAddress = System.Net.NetworkInformation.PhysicalAddress.Parse(stringMac);
                isValidSerial = d.isValidSerial();
            }

            if (!isValidSerial)
            {
                //meta.PadMacAddress = null;
                meta.PadState = DsState.Disconnected;
            }
            else
            {
                if (d.isSynced() || d.IsAlive())
                    meta.PadState = DsState.Connected;
                else
                    meta.PadState = DsState.Reserved;
            }

            meta.ConnectionType = (d.getConnectionType() == ConnectionType.USB) ? DsConnection.Usb : DsConnection.Bluetooth;
            meta.IsActive = !d.isDS5Idle();

            int batteryLevel = d.getBattery();
            if (d.isCharging() && batteryLevel >= 100)
                meta.BatteryStatus = DsBattery.Charged;
            else
            {
                if (batteryLevel >= 95)
                    meta.BatteryStatus = DsBattery.Full;
                else if (batteryLevel >= 70)
                    meta.BatteryStatus = DsBattery.High;
                else if (batteryLevel >= 50)
                    meta.BatteryStatus = DsBattery.Medium;
                else if (batteryLevel >= 20)
                    meta.BatteryStatus = DsBattery.Low;
                else if (batteryLevel >= 5)
                    meta.BatteryStatus = DsBattery.Dying;
                else
                    meta.BatteryStatus = DsBattery.None;
            }

            //return meta;
        }

        public ControlService(DS5ControlApp.ArgumentParser cmdParser)
        {
            this.cmdParser = cmdParser;

            Crc32Algorithm.InitializeTable(DS5Device.DefaultPolynomial);

            eventDispatchThread = new Thread(() =>
            {
                Dispatcher currentDis = Dispatcher.CurrentDispatcher;
                eventDispatcher = currentDis;
                Dispatcher.Run();
            });
            eventDispatchThread.IsBackground = true;
            eventDispatchThread.Priority = ThreadPriority.BelowNormal;
            eventDispatchThread.Name = "ControlService Events";
            eventDispatchThread.Start();

            for (int i = 0, arlength = DS5Controllers.Length; i < arlength; i++)
            {
                MappedState[i] = new DS5State();
                CurrentState[i] = new DS5State();
                TempState[i] = new DS5State();
                PreviousState[i] = new DS5State();
                ExposedState[i] = new DS5StateExposed(CurrentState[i]);
                oscState[i] = new DS5State();

                int tempDev = i;
                Global.L2OutputSettings[i].TwoStageModeChanged += (sender, e) =>
                {
                    Mapping.l2TwoStageMappingData[tempDev].Reset();
                };

                Global.R2OutputSettings[i].TwoStageModeChanged += (sender, e) =>
                {
                    Mapping.r2TwoStageMappingData[tempDev].Reset();
                };
            }

            outputslotMan = new OutputSlotManager();
            //outputslotMan.SlotAssigned += OutputslotMan_SlotAssigned;
            deviceOptions = Global.DeviceOptions;

            DS5Devices.RequestElevation += DS5Devices_RequestElevation;
            DS5Devices.PrepareDS5Init = PrepareDS5DeviceInit;
            DS5Devices.PostDS5Init = PostDS5DeviceInit;
            DS5Devices.PreparePendingDevice = CheckForSupportedDevice;
            outputslotMan.ViGEmFailure += OutputslotMan_ViGEmFailure;

            Global.UDPServerSmoothingMincutoffChanged += ChangeUdpSmoothingAttrs;
            Global.UDPServerSmoothingBetaChanged += ChangeUdpSmoothingAttrs;

            CreateOSCCallback();

            SystemEvents.DisplaySettingsChanged += SystemEvents_DisplaySettingsChanged;
            //oscListener = new UDPListener(Global.getOSCServerPortNum(), callback: oscCallback);
            //AppLogger.LogToGui("OSC LISTENER STARTED", false);
        }

        private void SystemEvents_DisplaySettingsChanged(object sender, EventArgs e)
        {
            Global.PrepareAbsMonitorBounds(string.Empty);
        }

        //private void OutputslotMan_SlotAssigned(OutputSlotManager sender, int slotNum, OutSlotDevice outSlotDev)
        //{
        //    LogDebug($"Associated input controller #{outSlotDev.InputIndex + 1} ({outSlotDev.InputDisplayString}) to virtual {outSlotDev.OutputDevice.GetDeviceType()} Controller in{(outSlotDev.PermanentType != OutContType.None ? " permanent" : "")} output slot #{outSlotDev.Index + 1}");
        //}

        private string[] MapMonitoringOscMessageToCommand(string[] command)
        {
            // Overwrite "monitor" with the controller Id
            command[2] = command[3];

            switch (command[4])
            {
                case "battery":
                    command[3] = "battery";
                    break;
                case "l2":
                case "r2":
                    command[3] = "trigger";
                    break;
                case "rx":
                case "ry":
                case "lx":
                case "ly":
                    command[3] = "stick";
                    break;
                default:
                    command[3] = "press";
                    break;
            }

            return command;
        }

        private void CreateOSCCallback()
        {
            oscCallback = delegate (OscPacket packet)
            {
                var messageReceived = (OscMessage)packet;

                // If typecase fails, exit
                if (messageReceived == null)
                {
                    return;
                }

                string[] command = null;
                try
                {
                    command = messageReceived.Address.Split("/");
                }
                catch (Exception e)
                {
                    AppLogger.LogToGui("Error Receiving OSC Message: " + e.Message, false, true);
                }

                if (command == null)
                {
                    return;
                }

                if (command[1] != "ds5control")
                {
                    return;
                }

                if (command[2] == "monitor")
                {
                    if (Global.isInterpretingOscMonitoring())
                    {
                        command = MapMonitoringOscMessageToCommand(command);
                    }
                    else
                    {
                        return;
                    }
                }

                int stateInd = -1;
                if (!int.TryParse(command[2], out stateInd))
                {
                    stateInd = -1;
                }

                if (stateInd == -1)
                {
                    AppLogger.LogToGui("Received malformed OSC address: " + messageReceived.Address, false);
                    return;
                }

                if (command[3] == "battery")
                {
                    if (!isUsingOSCSender())
                    {
                        AppLogger.LogToGui("Battery level requested, but the OSC Sender isn't active. Turn it on in Settings.", false);
                    }
                    else
                    {
                        oscSender.Send(new SharpOSC.OscMessage("/ds5control/monitor/" + stateInd + "/battery", oscState[stateInd].Battery));
                    }
                    return;
                }
                else if (command[3] == "press")
                {
                    int messageValue = Convert.ToInt32(messageReceived.Arguments[0]);
                    bool buttonBool = messageValue == 1 ? true : false;

                    switch (command[4])
                    {
                        case "cross":
                            oscState[stateInd].Cross = buttonBool;
                            break;
                        case "square":
                            oscState[stateInd].Square = buttonBool;
                            break;
                        case "circle":
                            oscState[stateInd].Circle = buttonBool;
                            break;
                        case "triangle":
                            oscState[stateInd].Triangle = buttonBool;
                            break;
                        case "r1":
                            oscState[stateInd].R1 = buttonBool;
                            break;
                        case "r2":
                            oscState[stateInd].R2 = Convert.ToByte(buttonBool ? 255 : 0);
                            break;
                        case "r3":
                            oscState[stateInd].R3 = buttonBool;
                            break;
                        case "l1":
                            oscState[stateInd].L1 = buttonBool;
                            break;
                        case "l2":
                            oscState[stateInd].L2 = Convert.ToByte(buttonBool ? 255 : 0);
                            break;
                        case "l3":
                            oscState[stateInd].L3 = buttonBool;
                            break;
                        case "dpadup":
                        case "dup":
                            oscState[stateInd].DpadUp = buttonBool;
                            break;
                        case "dpaddown":
                        case "ddown":
                            oscState[stateInd].DpadDown = buttonBool;
                            break;
                        case "dpadleft":
                        case "dleft":
                            oscState[stateInd].DpadLeft = buttonBool;
                            break;
                        case "dpadright":
                        case "dright":
                            oscState[stateInd].DpadRight = buttonBool;
                            break;
                        case "options":
                            oscState[stateInd].Options = buttonBool;
                            break;
                        case "share":
                            oscState[stateInd].Share = buttonBool;
                            break;
                    }
                }
                else if (command[3] == "stick" && messageReceived.Arguments.Count == 1)
                {
                    switch (command[4])
                    {
                        case "lx":
                            oscState[stateInd].LX = Convert.ToByte(Convert.ToSingle(messageReceived.Arguments[0]));
                            break;
                        case "ly":
                            oscState[stateInd].LY = Convert.ToByte(Convert.ToSingle(messageReceived.Arguments[0]));
                            break;
                        case "rx":
                            oscState[stateInd].RX = Convert.ToByte(Convert.ToSingle(messageReceived.Arguments[0]));
                            break;
                        case "ry":
                            oscState[stateInd].RY = Convert.ToByte(Convert.ToSingle(messageReceived.Arguments[0]));
                            break;
                    }
                }
                else if (command[3] == "stick" && messageReceived.Arguments.Count == 2)
                {
                    float xValue = Convert.ToSingle(messageReceived.Arguments[0]);
                    float yValue = Convert.ToSingle(messageReceived.Arguments[1]);

                    if (command[4] == "left")
                    {
                        oscState[stateInd].LX = Convert.ToByte(xValue * 255);
                        oscState[stateInd].LY = Convert.ToByte(yValue * 255);
                    }
                    else if (command[4] == "right")
                    {
                        oscState[stateInd].RX = Convert.ToByte(xValue * 255);
                        oscState[stateInd].RY = Convert.ToByte(yValue * 255);
                    }
                }
                else if (command[3] == "trigger")
                {
                    switch (command[4])
                    {
                        case "r2":
                            oscState[stateInd].R2 = Convert.ToByte(Convert.ToSingle(messageReceived.Arguments[0]));
                            break;
                        case "l2":
                            oscState[stateInd].L2 = Convert.ToByte(Convert.ToSingle(messageReceived.Arguments[0]));
                            break;
                    }
                }
            };
        }

        public void RefreshOutputKBMHandler()
        {
            if (Global.outputKBMHandler != null)
            {
                Global.outputKBMHandler.Disconnect();
                Global.outputKBMHandler = null;
            }

            if (Global.outputKBMMapping != null)
            {
                Global.outputKBMMapping = null;
            }

            InitOutputKBMHandler();
        }

        private void InitOutputKBMHandler()
        {
            string attemptVirtualkbmHandler = cmdParser.VirtualkbmHandler;
            Global.InitOutputKBMHandler(attemptVirtualkbmHandler);

            bool handlerConnected = false;
            try
            {
                handlerConnected = Global.outputKBMHandler.Connect();
            }
            catch { }

            if (!handlerConnected &&
                attemptVirtualkbmHandler != VirtualKBMFactory.GetFallbackHandlerIdentifier())
            {
                Global.outputKBMHandler = VirtualKBMFactory.GetFallbackHandler();
            }
            else
            {
                // Connection was made. Check if version number should get populated
                if (outputKBMHandler.GetIdentifier() == FakerInputHandler.IDENTIFIER)
                {
                    Global.outputKBMHandler.Version = Global.fakerInputVersion;
                }
            }

            Global.InitOutputKBMMapping(Global.outputKBMHandler.GetIdentifier());
            Global.outputKBMMapping.PopulateConstants();
            Global.outputKBMMapping.PopulateMappings();
        }

        private void OutputslotMan_ViGEmFailure(object sender, int errorCode)
        {
            eventDispatcher.BeginInvoke((Action)(() =>
            {
                loopControllers = false;
                while (inServiceTask)
                    Thread.SpinWait(1000);

                LogDebug(string.Format(DS5ControlApp.Translations.Strings.ViGEmPluginFailure, errorCode),
                    true);
                Stop();
            }));
        }

        public void PostDS5DeviceInit(DS5Device device)
        {
            if (device.DeviceType == InputDevices.InputDeviceType.JoyConL ||
                device.DeviceType == InputDevices.InputDeviceType.JoyConR)
            {
                if (deviceOptions.JoyConDeviceOpts.LinkedMode == JoyConDeviceOptions.LinkMode.Joined)
                {
                    InputDevices.JoyConDevice tempJoyDev = device as InputDevices.JoyConDevice;
                    tempJoyDev.PerformStateMerge = true;

                    if (device.DeviceType == InputDevices.InputDeviceType.JoyConL)
                    {
                        tempJoyDev.PrimaryDevice = true;
                        if (deviceOptions.JoyConDeviceOpts.JoinGyroProv == JoyConDeviceOptions.JoinedGyroProvider.JoyConL)
                        {
                            tempJoyDev.OutputMapGyro = true;
                        }
                        else
                        {
                            tempJoyDev.OutputMapGyro = false;
                        }
                    }
                    else
                    {
                        tempJoyDev.PrimaryDevice = false;
                        if (deviceOptions.JoyConDeviceOpts.JoinGyroProv == JoyConDeviceOptions.JoinedGyroProvider.JoyConR)
                        {
                            tempJoyDev.OutputMapGyro = true;
                        }
                        else
                        {
                            tempJoyDev.OutputMapGyro = false;
                        }
                    }
                }
            }
        }

        private void PrepareDS5DeviceSettingHooks(DS5Device device)
        {
            if (device.DeviceType == InputDevices.InputDeviceType.DualSense)
            {
                InputDevices.DualSenseDevice tempDSDev = device as InputDevices.DualSenseDevice;

                DualSenseControllerOptions dSOpts = tempDSDev.NativeOptionsStore;
                dSOpts.LedModeChanged += (sender, e) => { tempDSDev.CheckControllerNumDeviceSettings(activeControllers); };
            }
            else if (device.DeviceType == InputDevices.InputDeviceType.JoyConL ||
                device.DeviceType == InputDevices.InputDeviceType.JoyConR)
            {
            }
        }

        public bool CheckForSupportedDevice(HidDevice device, VidPidInfo metaInfo)
        {
            bool result = false;
            switch (metaInfo.inputDevType)
            {
                case InputDevices.InputDeviceType.DS4:
                    result = deviceOptions.DS5DeviceOpts.Enabled;
                    break;
                case InputDevices.InputDeviceType.DualSense:
                    result = deviceOptions.DualSenseOpts.Enabled;
                    break;
                case InputDevices.InputDeviceType.SwitchPro:
                    result = deviceOptions.SwitchProDeviceOpts.Enabled;
                    break;
                case InputDevices.InputDeviceType.JoyConL:
                case InputDevices.InputDeviceType.JoyConR:
                case InputDevices.InputDeviceType.JoyConGrip:
                    result = deviceOptions.JoyConDeviceOpts.Enabled;
                    break;
                case InputDevices.InputDeviceType.DS3:
                    result = deviceOptions.DS3DeviceOpts.Enabled;
                    break;
                default:
                    break;
            }

            return result;
        }

        public void PrepareDS5DeviceInit(DS5Device device)
        {
            // Does nothing now
        }

        public void ShutDown()
        {
            outputslotMan.ShutDown();
            OutputSlotPersist.WriteConfig(outputslotMan);

            eventDispatcher.InvokeShutdown();
            eventDispatcher = null;

            eventDispatchThread.Join();
            eventDispatchThread = null;
        }

        private void DS5Devices_RequestElevation(RequestElevationArgs args)
        {
            // Launches an elevated child process to re-enable device
            ProcessStartInfo startInfo =
                new ProcessStartInfo(Global.exelocation);
            startInfo.Verb = "runas";
            startInfo.Arguments = "re-enabledevice " + args.InstanceId;
            startInfo.UseShellExecute = true;

            try
            {
                Process child = Process.Start(startInfo);
                if (!child.WaitForExit(30000))
                {
                    child.Kill();
                }
                else
                {
                    args.StatusCode = child.ExitCode;
                }
                child.Dispose();
            }
            catch { }
        }

        public void CheckHidHidePresence(string ExePath = "", string ExeName = "Autoprofile Exe", bool AddExe = true) // Default value for D4W Startup
        {
            if (Global.hidHideInstalled)
            {
                LogDebug("HidHide control device found");
                using (HidHideAPIDevice hidHideDevice = new HidHideAPIDevice())
                {
                    if (!hidHideDevice.IsOpen())
                    {
                        return;
                    }
                    // Catch Blank Values and initialize for Startup. Also catches empty Values.
                    // Also Catches Empty values in auto-profiler, and defaults to trying to re-add D4W. Will fail harmlessly later.
                    if (ExePath == "") { ExePath = Global.exelocation; ExeName = "DS4Windows"; AddExe = true; }

                    // Check for inverse application cloak. If setting is being used in HidHide,
                    // skip checking HidHide whitelist for DS5Control.
                    bool inverseAppCloak = hidHideDevice.GetWhiteListInverseState();
                    if (inverseAppCloak)
                    {
                        return;
                    }


                    List<string> dosPaths = hidHideDevice.GetWhitelist();

                    int maxPathCheckLength = 512;
                    StringBuilder sb = new StringBuilder(maxPathCheckLength);

                    DirectoryInfo dirInfo = new DirectoryInfo(Path.GetDirectoryName(ExePath));
                    // Check if exe is placed in a junction symlink directory (done with Scoop).
                    // Good enough
                    if (dirInfo.Attributes.HasFlag(FileAttributes.ReparsePoint) &&
                        dirInfo.LinkTarget != null)
                    {
                        // App directory is a junction. Find real directory and get proper path
                        // for inserting into HidHide
                        ExePath = Path.Combine(dirInfo.LinkTarget, Path.GetFileName(ExePath));
                    }

                    string driveLetter = Path.GetPathRoot(ExePath).Replace("\\", "");
                    uint _ = NativeMethods.QueryDosDevice(driveLetter, sb, maxPathCheckLength);
                    //int error = Marshal.GetLastWin32Error();

                    string dosDrivePath = sb.ToString();
                    // Strip a possible \??\ prefix.
                    if (dosDrivePath.StartsWith(@"\??\"))
                    {
                        dosDrivePath = dosDrivePath.Remove(0, 4);
                    }

                    string partial = ExePath.Replace(driveLetter, "");
                    // Need to trim starting '\\' from path2 or Path.Combine will
                    // treat it as an absolute path and only return path2
                    string realPath = Path.Combine(dosDrivePath, partial.TrimStart('\\'));
                    bool exists = dosPaths.Contains(realPath);
                    if (!exists && AddExe)
                    {
                        LogDebug($"{ExeName} not found in HidHide whitelist. Adding to list");
                        dosPaths.Add(realPath);
                        hidHideDevice.SetWhitelist(dosPaths);
                    }
                    if (exists && !AddExe)
                    {
                        LogDebug($"{ExeName} found in HidHide whitelist. Removing from list");
                        dosPaths.Remove(realPath);
                        hidHideDevice.SetWhitelist(dosPaths);
                    }
                }
            }
        }

        public void LoadPermanentSlotsConfig()
        {
            OutputSlotPersist.ReadConfig(outputslotMan);
        }

        public void UpdateHidHideAttributes()
        {
            if (Global.hidHideInstalled)
            {
                hidDeviceHidingAffectedDevs.Clear();
                hidDeviceHidingExemptedDevs.Clear(); // No known equivalent in HidHide
                hidDeviceHidingForced = false; // No known equivalent in HidHide
                hidDeviceHidingEnabled = false;

                using (HidHideAPIDevice hidHideDevice = new HidHideAPIDevice())
                {
                    if (!hidHideDevice.IsOpen())
                    {
                        return;
                    }

                    bool active = hidHideDevice.GetActiveState();
                    List<string> instances = hidHideDevice.GetBlacklist();

                    hidDeviceHidingEnabled = active;
                    foreach (string instance in instances)
                    {
                        hidDeviceHidingAffectedDevs.Add(instance.ToUpper());
                    }
                }
            }
        }

        public void UpdateHidHiddenAttributes()
        {
            if (Global.hidHideInstalled)
            {
                UpdateHidHideAttributes();
            }
        }

        private bool CheckAffected(DS5Device dev)
        {
            bool result = false;
            if (dev != null && hidDeviceHidingEnabled)
            {
                string deviceInstanceId = Global.GetInstanceIdFromDevicePath(dev.HidDevice.DevicePath);
                if (Global.hidHideInstalled)
                {
                    result = Global.CheckHidHideAffectedStatus(deviceInstanceId,
                        hidDeviceHidingAffectedDevs, hidDeviceHidingExemptedDevs, hidDeviceHidingForced);
                }
            }

            return result;
        }

        /// <summary>
        /// Obtain extra mappable controls not on a DS4 that should be added
        /// to the checked inputs list. Keeps Mapping class from having to check
        /// extra Switch Pro and JoyCon buttons for DS4 controllers
        /// </summary>
        /// <param name="dev">Instance of input device</param>
        /// <returns>List of extra controls to check in Mapping class</returns>
        private List<DS5Controls> GetKnownExtraButtons(DS5Device dev)
        {
            List<DS5Controls> result = new List<DS5Controls>();
            switch (dev.DeviceType)
            {
                case InputDevices.InputDeviceType.DualSense:
                    {
                        InputDevices.DualSenseDevice tempDev = dev as InputDevices.DualSenseDevice;
                        if (tempDev != null &&
                            tempDev.SubType == InputDevices.DualSenseDevice.DeviceSubType.DSEdge)
                        {
                            // Added extra DualSense Edge buttons as extra in the mapper.
                            // Keeps from checking non-existent buttons on other device types.
                            result.AddRange(new DS5Controls[] { DS5Controls.FnL, DS5Controls.FnR, DS5Controls.BLP, DS5Controls.BRP });
                        }
                    }

                    break;
                case InputDevices.InputDeviceType.JoyConL:
                case InputDevices.InputDeviceType.JoyConR:
                    result.AddRange(new DS5Controls[] { DS5Controls.Capture, DS5Controls.SideL, DS5Controls.SideR, DS5Controls.FnL, DS5Controls.FnR });
                    break;
                case InputDevices.InputDeviceType.SwitchPro:
                    result.AddRange(new DS5Controls[] { DS5Controls.Capture });
                    break;
                default:
                    break;
            }

            return result;
        }

        private void ChangeExclusiveStatus(DS5Device dev)
        {
            if (Global.hidHideInstalled)
            {
                dev.CurrentExclusiveStatus = DS5Device.ExclusiveStatus.HidHideAffected;
            }
        }

        private void TestQueueBus(Action temp)
        {
            eventDispatcher.BeginInvoke(() =>
            {
                temp?.Invoke();
            });
        }

        public void ChangeUDPStatus(bool state, bool openPort = true)
        {

            if (state && _udpServer == null)
            {
                udpChangeStatus = true;
                TestQueueBus(() =>
                {
                    _udpServer = new UdpServer(GetPadDetailForIdx);
                    if (openPort)
                    {
                        // Change thread affinity of object to have normal priority
                        Task.Run(() =>
                        {
                            var UDP_SERVER_PORT = Global.getUDPServerPortNum();
                            var UDP_SERVER_LISTEN_ADDRESS = Global.getUDPServerListenAddress();

                            try
                            {
                                _udpServer.Start(UDP_SERVER_PORT, UDP_SERVER_LISTEN_ADDRESS);
                                LogDebug($"UDP server listening on address {UDP_SERVER_LISTEN_ADDRESS} port {UDP_SERVER_PORT}");
                            }
                            catch (System.Net.Sockets.SocketException ex)
                            {
                                var errMsg = String.Format("Couldn't start UDP server on address {0}:{1}, outside applications won't be able to access pad data ({2})", UDP_SERVER_LISTEN_ADDRESS, UDP_SERVER_PORT, ex.SocketErrorCode);

                                LogDebug(errMsg, true);
                                AppLogger.LogToTray(errMsg, true, true);
                            }
                        }).Wait();
                    }

                    udpChangeStatus = false;
                });
            }
            else if (!state && _udpServer != null)
            {
                TestQueueBus(() =>
                {
                    udpChangeStatus = true;
                    _udpServer.Stop();
                    _udpServer = null;
                    AppLogger.LogToGui("Closed UDP server", false);
                    udpChangeStatus = false;

                    for (int i = 0; i < UdpServer.NUMBER_SLOTS; i++)
                    {
                        ResetUdpSmoothingFilters(i);
                    }
                });
            }
        }

        public void ChangeOSCListenerStatus(bool state)
        {
            if (state)
            {
                oscListener = new UDPListener(Global.getOSCServerPortNum(), callback: oscCallback);

                AppLogger.LogToGui("OSC LISTENER STARTED AT PORT: " + Global.getOSCServerPortNum(), false);
            }
            else
            {
                oscListener.Close();
                oscListener = null;
                AppLogger.LogToGui("OSC LISTENER STOPPED", false);
            }
        }

        public void ChangeOSCSenderStatus(bool state)
        {
            if (state)
            {
                AppLogger.LogToGui("OSC SENDER STARTED AT IP: " + Global.getOSCSenderAddress() + " PORT: " + Global.getOSCSenderPortNum(), false);
                oscSender = new UDPSender(Global.getOSCSenderAddress(), Global.getOSCSenderPortNum());
            }
            else
            {
                AppLogger.LogToGui("OSC SENDER STOPPED", false);
                if (oscSender == null) { return; }
                oscSender.Close();
                oscSender = null;
            }
        }

        public void ChangeMotionEventStatus(bool state)
        {
            IEnumerable<DS5Device> devices = DS5Devices.getDS5Controllers();
            if (state)
            {
                int i = 0;
                foreach (DS5Device dev in devices)
                {
                    int tempIdx = i;
                    dev.queueEvent(() =>
                    {
                        if (i < UdpServer.NUMBER_SLOTS)
                        {
                            PrepareDevUDPMotion(dev, tempIdx);
                        }
                    });

                    i++;
                }
            }
            else
            {
                foreach (DS5Device dev in devices)
                {
                    dev.queueEvent(() =>
                    {
                        if (dev.MotionEvent != null)
                        {
                            dev.Report -= dev.MotionEvent;
                            dev.MotionEvent = null;
                        }
                    });
                }
            }
        }

        private bool udpChangeStatus = false;
        public bool changingUDPPort = false;
        public async void UseUDPPort()
        {
            changingUDPPort = true;
            IEnumerable<DS5Device> devices = DS5Devices.getDS5Controllers();
            foreach (DS5Device dev in devices)
            {
                dev.queueEvent(() =>
                {
                    if (dev.MotionEvent != null)
                    {
                        dev.Report -= dev.MotionEvent;
                    }
                });
            }

            await Task.Delay(100);

            var UDP_SERVER_PORT = Global.getUDPServerPortNum();
            var UDP_SERVER_LISTEN_ADDRESS = Global.getUDPServerListenAddress();

            try
            {
                _udpServer.Start(UDP_SERVER_PORT, UDP_SERVER_LISTEN_ADDRESS);
                foreach (DS5Device dev in devices)
                {
                    dev.queueEvent(() =>
                    {
                        if (dev.MotionEvent != null)
                        {
                            dev.Report += dev.MotionEvent;
                        }
                    });
                }
                LogDebug($"UDP server listening on address {UDP_SERVER_LISTEN_ADDRESS} port {UDP_SERVER_PORT}");
            }
            catch (System.Net.Sockets.SocketException ex)
            {
                var errMsg = String.Format("Couldn't start UDP server on address {0}:{1}, outside applications won't be able to access pad data ({2})", UDP_SERVER_LISTEN_ADDRESS, UDP_SERVER_PORT, ex.SocketErrorCode);

                LogDebug(errMsg, true);
                AppLogger.LogToTray(errMsg, true, true);
            }

            changingUDPPort = false;
        }

        private void WarnExclusiveModeFailure(DS5Device device)
        {
            if (DS5Devices.isExclusiveMode && !device.isExclusive())
            {
                string message = DS5ControlApp.Properties.Resources.CouldNotOpenDS5.Replace("*Mac address*", device.getMacAddress()) + " " +
                    DS5ControlApp.Properties.Resources.QuitOtherPrograms;
                LogDebug(message, true);
                AppLogger.LogToTray(message, true);
            }
        }

        private void StartViGEm()
        {
            // Refresh internal ViGEmBus info
            Global.RefreshViGEmBusInfo();
            if (Global.IsRunningSupportedViGEmBus())
            {
                tempThread = new Thread(() =>
                {
                    try
                    {
                        vigemTestClient = new ViGEmClient();
                    }
                    catch { }
                });
                tempThread.Priority = ThreadPriority.AboveNormal;
                tempThread.IsBackground = true;
                tempThread.Start();
                while (tempThread.IsAlive)
                {
                    Thread.SpinWait(500);
                }
            }

            tempThread = null;
        }

        private void StopViGEm()
        {
            if (vigemTestClient != null)
            {
                vigemTestClient.Dispose();
                vigemTestClient = null;
            }
        }

        public void AssignInitialDevices()
        {
            foreach (OutSlotDevice slotDevice in outputslotMan.OutputSlots)
            {
                if (slotDevice.CurrentReserveStatus ==
                    OutSlotDevice.ReserveStatus.Permanent)
                {
                    OutputDevice outDevice = EstablishOutDevice(0, slotDevice.PermanentType);
                    outputslotMan.DeferredPlugin(outDevice, -1, "", outputDevices, slotDevice.PermanentType);
                }
            }
            /*OutSlotDevice slotDevice =
                outputslotMan.FindExistUnboundSlotType(OutContType.X360);

            if (slotDevice == null)
            {
                slotDevice = outputslotMan.FindOpenSlot();
                slotDevice.CurrentReserveStatus = OutSlotDevice.ReserveStatus.Permanent;
                slotDevice.PermanentType = OutContType.X360;
                OutputDevice outDevice = EstablishOutDevice(0, OutContType.X360);
                Xbox360OutDevice tempXbox = outDevice as Xbox360OutDevice;
                outputslotMan.DeferredPlugin(tempXbox, -1, outputDevices, OutContType.X360);
            }
            */

            /*slotDevice = outputslotMan.FindExistUnboundSlotType(OutContType.X360);
            if (slotDevice == null)
            {
                slotDevice = outputslotMan.FindOpenSlot();
                slotDevice.CurrentReserveStatus = OutSlotDevice.ReserveStatus.Permanent;
                slotDevice.DesiredType = OutContType.X360;
                OutputDevice outDevice = EstablishOutDevice(1, OutContType.X360);
                Xbox360OutDevice tempXbox = outDevice as Xbox360OutDevice;
                outputslotMan.DeferredPlugin(tempXbox, 1, outputDevices);
            }*/
        }

        private OutputDevice EstablishOutDevice(int index, OutContType contType)
        {
            OutputDevice temp = null;
            temp = outputslotMan.AllocateController(contType, vigemTestClient);
            return temp;
        }

        public void EstablishOutFeedback(int index, OutContType contType,
            OutputDevice outDevice, DS5Device device)
        {
            int devIndex = index;

            if (contType == OutContType.X360)
            {
                Xbox360OutDevice tempXbox = outDevice as Xbox360OutDevice;
                Nefarius.ViGEm.Client.Targets.Xbox360FeedbackReceivedEventHandler p = (sender, args) =>
                {
                    //Trace.WriteLine(string.Format("Rumble ({0}, {1}) {2}",
                    //    args.LargeMotor, args.SmallMotor, DateTime.Now.ToString("hh:mm:ss.FFFF")));
                    SetDevRumble(device, args.LargeMotor, args.SmallMotor, devIndex);
                };
                tempXbox.cont.FeedbackReceived += p;
                tempXbox.forceFeedbacksDict.Add(index, p);
            }
            else if (contType == OutContType.DS4)
            {
                DS4OutDevice tempDS5 = outDevice as DS4OutDevice;
                if (tempDS5.CanUseAwaitOutputBuffer)
                {
                    DS4OutDeviceExt.ReceivedOutBufferHandler processOutBuffAction = (DS4OutDeviceExt sender, byte[] reportData) =>
                    {
                        /*
                        //DS4OutputBufferData outputData = new DS4OutputBufferData();
                        DS4OutputBufferData outputData =
                            DS4OutDeviceExtras.ConvertOutputBufferArrayToStruct(reportData);

                        bool useRumble = false; bool useLight = false;
                        byte flashOn = 0; byte flashOff = 0;
                        DS5Color? color = null;

                        //Trace.WriteLine(string.Join(" ", reportData));

                        if ((outputData.featureFlags & DS4OutDevice.RUMBLE_FEATURE_FLAG) != 0)
                        {
                            useRumble = true;
                            device.setRumble(outputData.rightFastRumble, outputData.leftSlowRumble);
                            //SetDevRumble(device, devour[4], devour[5], devIndex);
                        }

                        if ((outputData.featureFlags & DS4OutDevice.LIGHTBAR_FEATURE_FLAG) != 0)
                        {
                            useLight = true;
                            color = new DS5Color(outputData.lightbarRedColor,
                                outputData.lightbarGreenColor,
                                outputData.lightbarBlueColor);
                        }
                        else
                        {
                            color = device.LightBarColor;
                        }

                        if ((outputData.featureFlags & DS4OutDevice.FLASH_FEATURE_FLAG) != 0)
                        {
                            useLight = true;
                            flashOn = outputData.flashOnDuration;
                            flashOff = outputData.flashOffDuration;
                        }
                        else
                        {
                            ref DS5LightbarState currentLight =
                                ref device.GetLightbarStateRef();

                            flashOn = currentLight.LightBarFlashDurationOn;
                            flashOff = currentLight.LightBarFlashDurationOff;
                        }

                        if (useLight)
                        {
                            DS5LightbarState lightState = new DS5LightbarState
                            {
                                LightBarColor = (DS5Color)color,
                                LightBarFlashDurationOn = flashOn,
                                LightBarFlashDurationOff = flashOff,
                            };
                            device.SetLightbarState(ref lightState);
                        }
                        //*/

                        //*
                        unchecked
                        {
                            //Trace.WriteLine($"INDEX: {devIndex}");
                            //Trace.WriteLine(string.Join(" ", reportData));
                            //Trace.WriteLine("");

                            bool useRumble = false; bool useLight = false;
                            byte flashOn = 0; byte flashOff = 0;
                            DS5Color? color = null;
                            if ((reportData[1] & DS4OutDevice.RUMBLE_FEATURE_FLAG) != 0)
                            {
                                useRumble = true;
                                SetDevRumble(device, reportData[5], reportData[4], devIndex);
                            }

                            if ((reportData[1] & DS4OutDevice.LIGHTBAR_FEATURE_FLAG) != 0)
                            {
                                useLight = true;
                                color = new DS5Color(reportData[6],
                                    reportData[7],
                                    reportData[8]);
                            }
                            else
                            {
                                color = device.LightBarColor;
                            }

                            if ((reportData[1] & DS4OutDevice.FLASH_FEATURE_FLAG) != 0)
                            {
                                useLight = true;
                                flashOn = reportData[9];
                                flashOff = reportData[10];
                            }
                            else
                            {
                                ref DS5LightbarState currentLight =
                                    ref device.GetLightbarStateRef();

                                flashOn = currentLight.LightBarFlashDurationOn;
                                flashOff = currentLight.LightBarFlashDurationOff;
                            }

                            if (useLight)
                            {
                                DS5LightbarState lightState = new DS5LightbarState
                                {
                                    LightBarColor = (DS5Color)color,
                                    LightBarFlashDurationOn = flashOn,
                                    LightBarFlashDurationOff = flashOff,
                                };
                                device.SetLightbarState(ref lightState);
                            }
                        }
                        //*/
                    };

                    DS4OutDeviceExt tempDS5Ext = tempDS5 as DS4OutDeviceExt;
                    tempDS5Ext.ReceivedOutBuffer += processOutBuffAction;
                    tempDS5Ext.outBufferFeedbacksDict.TryAdd(index, processOutBuffAction);
                    tempDS5Ext.StartOutputBufferThread();
                }
            }
            //else if (contType == OutContType.DS4)
            //{
            //    DS4OutDevice tempDS5 = outDevice as DS4OutDevice;
            //    LightbarSettingInfo deviceLightbarSettingsInfo = Global.LightbarSettingsInfo[devIndex];

            //    Nefarius.ViGEm.Client.Targets.DualShock4FeedbackReceivedEventHandler p = (sender, args) =>
            //    {
            //        bool useRumble = false; bool useLight = false;
            //        byte largeMotor = args.LargeMotor;
            //        byte smallMotor = args.SmallMotor;
            //        //SetDevRumble(device, largeMotor, smallMotor, devIndex);
            //        DS5Color color = new DS5Color(args.LightbarColor.Red,
            //                args.LightbarColor.Green,
            //                args.LightbarColor.Blue);

            //        //Console.WriteLine("IN EVENT");
            //        //Console.WriteLine("Rumble ({0}, {1}) | Light ({2}, {3}, {4}) {5}",
            //        //    largeMotor, smallMotor, color.red, color.green, color.blue, DateTime.Now.ToString("hh:mm:ss.FFFF"));

            //        if (largeMotor != 0 || smallMotor != 0)
            //        {
            //            useRumble = true;
            //        }

            //        // Let games to control lightbar only when the mode is Passthru (otherwise DS5Control controls the light)
            //        if (deviceLightbarSettingsInfo.Mode == LightbarMode.Passthru && (color.red != 0 || color.green != 0 || color.blue != 0))
            //        {
            //            useLight = true;
            //        }

            //        if (!useRumble && !useLight)
            //        {
            //            //Console.WriteLine("Fallback");
            //            if (device.LeftHeavySlowRumble != 0 || device.RightLightFastRumble != 0)
            //            {
            //                useRumble = true;
            //            }
            //            else if (deviceLightbarSettingsInfo.Mode == LightbarMode.Passthru &&
            //                (device.LightBarColor.red != 0 ||
            //                device.LightBarColor.green != 0 ||
            //                device.LightBarColor.blue != 0))
            //            {
            //                useLight = true;
            //            }
            //        }

            //        if (useRumble)
            //        {
            //            //Console.WriteLine("Perform rumble");
            //            SetDevRumble(device, largeMotor, smallMotor, devIndex);
            //        }

            //        if (useLight)
            //        {
            //            //Console.WriteLine("Change lightbar color");
            //            /*DS5HapticState haptics = new DS5HapticState
            //            {
            //                LightBarColor = color,
            //            };
            //            device.SetHapticState(ref haptics);
            //            */

            //            DS5LightbarState lightState = new DS5LightbarState
            //            {
            //                LightBarColor = color,
            //            };
            //            device.SetLightbarState(ref lightState);
            //        }

            //        //Console.WriteLine();
            //    };

            //    tempDS5.cont.FeedbackReceived += p;
            //    tempDS5.forceFeedbacksDict.Add(index, p);
            //}
        }

        public void RemoveOutFeedback(OutContType contType, OutputDevice outDevice, int inIdx)
        {
            if (contType == OutContType.X360)
            {
                Xbox360OutDevice tempXbox = outDevice as Xbox360OutDevice;
                tempXbox.RemoveFeedback(inIdx);
                //tempXbox.cont.FeedbackReceived -= tempXbox.forceFeedbackCall;
                //tempXbox.forceFeedbackCall = null;
            }
            else if (contType == OutContType.DS4)
            {
                DS4OutDevice tempDS5 = outDevice as DS4OutDevice;
                tempDS5.RemoveFeedback(inIdx);
            }
            //else if (contType == OutContType.DS4)
            //{
            //    DS4OutDevice tempDS5 = outDevice as DS4OutDevice;
            //    tempDS5.RemoveFeedback(inIdx);
            //    //tempDS5.cont.FeedbackReceived -= tempDS5.forceFeedbackCall;
            //    //tempDS5.forceFeedbackCall = null;
            //}
        }

        public void AttachNewUnboundOutDev(OutContType contType)
        {
            OutSlotDevice slotDevice = outputslotMan.FindOpenSlot();
            if (slotDevice != null &&
                slotDevice.CurrentAttachedStatus == OutSlotDevice.AttachedStatus.UnAttached)
            {
                OutputDevice outDevice = EstablishOutDevice(-1, contType);
                outputslotMan.DeferredPlugin(outDevice, -1, "", outputDevices, contType);
            }
        }

        public void AttachUnboundOutDev(OutSlotDevice slotDevice, OutContType contType)
        {
            if (slotDevice.CurrentAttachedStatus == OutSlotDevice.AttachedStatus.UnAttached &&
                slotDevice.CurrentInputBound == OutSlotDevice.InputBound.Unbound)
            {
                OutputDevice outDevice = EstablishOutDevice(-1, contType);
                outputslotMan.DeferredPlugin(outDevice, -1, "", outputDevices, contType);
            }
        }

        public void DetachUnboundOutDev(OutSlotDevice slotDevice)
        {
            if (slotDevice.CurrentInputBound == OutSlotDevice.InputBound.Unbound)
            {
                OutputDevice dev = slotDevice.OutputDevice;
                string tempType = dev.GetDeviceType();
                slotDevice.CurrentInputBound = OutSlotDevice.InputBound.Unbound;
                outputslotMan.DeferredRemoval(dev, -1, outputDevices, false);
            }
        }

        public void PluginOutDev(int index, DS5Device device)
        {
            OutContType contType = Global.OutContType[index];

            OutSlotDevice slotDevice = null;
            if (!getDInputOnly(index))
            {
                slotDevice = outputslotMan.FindExistUnboundSlotType(contType);
            }

            if (useDInputOnly[index])
            {
                bool success = false;
                if (contType == OutContType.X360)
                {
                    activeOutDevType[index] = OutContType.X360;

                    if (slotDevice == null)
                    {
                        slotDevice = outputslotMan.FindOpenSlot();
                        if (slotDevice != null)
                        {
                            Xbox360OutDevice tempXbox = EstablishOutDevice(index, OutContType.X360)
                            as Xbox360OutDevice;
                            //outputDevices[index] = tempXbox;

                            // Enable ViGem feedback callback handler only if lightbar/rumble data output is enabled (if those are disabled then no point enabling ViGem callback handler call)
                            if (Global.EnableOutputDataToDS5[index])
                            {
                                EstablishOutFeedback(index, OutContType.X360, tempXbox, device);

                                if (device.JointDeviceSlotNumber != -1)
                                {
                                    DS5Device tempDS5Device = DS5Controllers[device.JointDeviceSlotNumber];
                                    if (tempDS5Device != null)
                                    {
                                        EstablishOutFeedback(device.JointDeviceSlotNumber, OutContType.X360, tempXbox, tempDS5Device);
                                    }
                                }
                            }

                            outputslotMan.DeferredPlugin(tempXbox, index, $"{device.DisplayName} [{device.MacAddress}]", outputDevices, contType);
                            //slotDevice.CurrentInputBound = OutSlotDevice.InputBound.Bound;

                            success = true;
                        }
                        else
                        {
                            LogDebug("Failed. No open output slot found");
                        }
                    }
                    else
                    {
                        slotDevice.CurrentInputBound = OutSlotDevice.InputBound.Bound;
                        Xbox360OutDevice tempXbox = slotDevice.OutputDevice as Xbox360OutDevice;

                        // Enable ViGem feedback callback handler only if lightbar/rumble data output is enabled (if those are disabled then no point enabling ViGem callback handler call)
                        if (Global.EnableOutputDataToDS5[index])
                        {
                            EstablishOutFeedback(index, OutContType.X360, tempXbox, device);

                            if (device.JointDeviceSlotNumber != -1)
                            {
                                DS5Device tempDS5Device = DS5Controllers[device.JointDeviceSlotNumber];
                                if (tempDS5Device != null)
                                {
                                    EstablishOutFeedback(device.JointDeviceSlotNumber, OutContType.X360, tempXbox, tempDS5Device);
                                }
                            }
                        }

                        outputDevices[index] = tempXbox;
                        slotDevice.CurrentType = contType;
                        success = true;
                    }

                    //tempXbox.Connect();
                    //LogDebug("X360 Controller #" + (index + 1) + " connected");
                }
                else if (contType == OutContType.DS4)
                {
                    activeOutDevType[index] = OutContType.DS4;
                    if (slotDevice == null)
                    {
                        slotDevice = outputslotMan.FindOpenSlot();
                        if (slotDevice != null)
                        {
                            DS4OutDevice tempDS5 = EstablishOutDevice(index, OutContType.DS4)
                            as DS4OutDevice;

                            // Enable ViGem feedback callback handler only if DS4 lightbar/rumble data output is enabled (if those are disabled then no point enabling ViGem callback handler call)
                            if (Global.EnableOutputDataToDS5[index])
                            {
                                EstablishOutFeedback(index, OutContType.DS4, tempDS5, device);

                                if (device.JointDeviceSlotNumber != -1)
                                {
                                    DS5Device tempDS5Device = DS5Controllers[device.JointDeviceSlotNumber];
                                    if (tempDS5Device != null)
                                    {
                                        EstablishOutFeedback(device.JointDeviceSlotNumber, OutContType.DS4, tempDS5, tempDS5Device);
                                    }
                                }
                            }

                            outputslotMan.DeferredPlugin(tempDS5, index, $"{device.DisplayName} [{device.MacAddress}]", outputDevices, contType);
                            //slotDevice.CurrentInputBound = OutSlotDevice.InputBound.Bound;

                            success = true;
                        }
                        else
                        {
                            LogDebug("Failed. No open output slot found");
                        }
                    }
                    else
                    {
                        slotDevice.CurrentInputBound = OutSlotDevice.InputBound.Bound;
                        DS4OutDevice tempDS5 = slotDevice.OutputDevice as DS4OutDevice;

                        // Enable ViGem feedback callback handler only if lightbar/rumble data output is enabled (if those are disabled then no point enabling ViGem callback handler call)
                        if (Global.EnableOutputDataToDS5[index])
                        {
                            EstablishOutFeedback(index, OutContType.DS4, tempDS5, device);

                            if (device.JointDeviceSlotNumber != -1)
                            {
                                DS5Device tempDS5Device = DS5Controllers[device.JointDeviceSlotNumber];
                                if (tempDS5Device != null)
                                {
                                    EstablishOutFeedback(device.JointDeviceSlotNumber, OutContType.DS4, tempDS5, tempDS5Device);
                                }
                            }
                        }

                        outputDevices[index] = tempDS5;
                        slotDevice.CurrentType = contType;
                        success = true;
                    }

                    //DS4OutDevice tempDS5 = new DS4OutDevice(vigemTestClient);
                    //DS4OutDevice tempDS5 = outputslotMan.AllocateController(OutContType.DS4, vigemTestClient)
                    //    as DS4OutDevice;
                    //outputDevices[index] = tempDS5;

                    //tempDS5.Connect();
                    //LogDebug("DS4 Controller #" + (index + 1) + " connected");
                }

                // Need to check for possible ViGEmBus failure here
                if (success && slotDevice.OutputDevice != null)
                {
                    LogDebug($"Associated input controller #{index + 1} ({device.DisplayName}) to virtual {slotDevice.OutputDevice.GetDeviceType()} Controller in{(slotDevice.PermanentType != OutContType.None ? " permanent" : "")} output slot #{slotDevice.Index + 1}");
                    useDInputOnly[index] = false;
                }
            }
        }

        public void UnplugOutDev(int index, DS5Device device, bool immediate = false, bool force = false)
        {
            if (!useDInputOnly[index])
            {
                //OutContType contType = Global.OutContType[index];
                OutputDevice dev = outputDevices[index];
                OutSlotDevice slotDevice = outputslotMan.GetOutSlotDevice(dev);
                if (dev != null && slotDevice != null)
                {
                    string tempType = dev.GetDeviceType();
                    LogDebug($"Disassociated virtual {tempType} Controller in{(slotDevice.CurrentReserveStatus == OutSlotDevice.ReserveStatus.Permanent ? " permanent" : "")} output slot #{slotDevice.Index + 1} from input controller #{index + 1} ({device.DisplayName})", false);

                    OutContType currentType = activeOutDevType[index];
                    outputDevices[index] = null;
                    activeOutDevType[index] = OutContType.None;
                    if ((slotDevice.CurrentAttachedStatus == OutSlotDevice.AttachedStatus.Attached &&
                        slotDevice.CurrentReserveStatus == OutSlotDevice.ReserveStatus.Dynamic) || force)
                    {
                        //slotDevice.CurrentInputBound = OutSlotDevice.InputBound.Unbound;
                        outputslotMan.DeferredRemoval(dev, index, outputDevices, immediate);
                    }
                    else if (slotDevice.CurrentAttachedStatus == OutSlotDevice.AttachedStatus.Attached)
                    {
                        slotDevice.CurrentInputBound = OutSlotDevice.InputBound.Unbound;
                        dev.ResetState();
                        dev.RemoveFeedbacks();
                        //RemoveOutFeedback(currentType, dev);
                    }
                    //dev.Disconnect();
                    //LogDebug(tempType + " Controller # " + (index + 1) + " unplugged");
                }

                useDInputOnly[index] = true;
            }
        }

        public bool Start(bool showlog = true)
        {
            inServiceTask = true;
            StartViGEm();
            if (vigemTestClient != null)
            //if (x360Bus.Open() && x360Bus.Start())
            {
                // Initialize output KBM handler at start of ControlService
                InitOutputKBMHandler();

                if (showlog)
                    LogDebug(DS5ControlApp.Properties.Resources.Starting);

                Thread.Sleep(2000);

                bool runningAsAdmin = Global.IsAdministrator();
                if (Global.outputKBMHandler.GetIdentifier() != FakerInputHandler.IDENTIFIER && !runningAsAdmin)
                {
                    string helpURL = @"https://ryochan7.github.io/ds5control-site/troubleshooting/kb-mouse-issues/#windows-not-responding-to-ds5ws-kb-m-commands-in-some-situations";
                    LogDebug($"Some applications may block controller inputs. (Windows UAC Conflictions). Please go to {helpURL} for more information and workarounds.");
                }

                LogDebug($"Using output KB+M handler: {Global.outputKBMHandler.GetFullDisplayName()}");
                LogDebug($"Connection to ViGEmBus {Global.vigembusVersion} established");

                DS5Devices.isExclusiveMode = getUseExclusiveMode(); //Re-enable Exclusive Mode

                UpdateHidHiddenAttributes();

                if (showlog)
                {
                    LogDebug(DS5ControlApp.Properties.Resources.SearchingController);
                    LogDebug(DS5Devices.isExclusiveMode ? DS5ControlApp.Properties.Resources.UsingExclusive : DS5ControlApp.Properties.Resources.UsingShared);
                }

                if (isUsingOSCServer() && oscListener == null)
                {
                    ChangeOSCListenerStatus(true);
                }

                if (isUsingOSCSender() && oscSender == null)
                {
                    ChangeOSCSenderStatus(true);
                }

                if (isUsingUDPServer() && _udpServer == null)
                {
                    ChangeUDPStatus(true, false);
                    while (udpChangeStatus == true)
                    {
                        Thread.SpinWait(500);
                    }
                }

                try
                {
                    loopControllers = true;
                    AssignInitialDevices();

                    eventDispatcher.Invoke(() =>
                    {
                        DS5Devices.findControllers();
                    });

                    IEnumerable<DS5Device> devices = DS5Devices.getDS5Controllers();
                    int numControllers = devices.Count();
                    activeControllers = numControllers;
                    DS5LightBar.defaultLight = false;
                    int i = 0;
                    InputDevices.JoyConDevice tempPrimaryJoyDev = null;
                    for (var devEnum = devices.GetEnumerator();
                        devEnum.MoveNext() && loopControllers; i++)
                    {
                        DS5Device device = devEnum.Current;

                        BeginPrepareConnectedInputController(device, showlog: true);

                        if (deviceOptions.JoyConDeviceOpts.LinkedMode == JoyConDeviceOptions.LinkMode.Joined)
                        {
                            if ((device.DeviceType == InputDevices.InputDeviceType.JoyConL ||
                                device.DeviceType == InputDevices.InputDeviceType.JoyConR) && device.PerformStateMerge)
                            {
                                if (tempPrimaryJoyDev == null)
                                {
                                    tempPrimaryJoyDev = device as InputDevices.JoyConDevice;
                                }
                                else
                                {
                                    InputDevices.JoyConDevice currentJoyDev = device as InputDevices.JoyConDevice;
                                    tempPrimaryJoyDev.JointDevice = currentJoyDev;
                                    currentJoyDev.JointDevice = tempPrimaryJoyDev;

                                    tempPrimaryJoyDev.JointState = currentJoyDev.JointState;

                                    InputDevices.JoyConDevice parentJoy = tempPrimaryJoyDev;
                                    tempPrimaryJoyDev.Removal += (sender, args) =>
                                    {
                                        currentJoyDev.JointDevice = null;
                                    };
                                    currentJoyDev.Removal += (sender, args) =>
                                    {
                                        parentJoy.JointDevice = null;
                                    };

                                    tempPrimaryJoyDev = null;
                                }
                            }
                        }

                        DS5Controllers[i] = device;
                        device.DeviceSlotNumber = i;
                        PrepareConnectedInputControllerSettingEvents(numControllers, device, index: i);

                        if (i >= CURRENT_DS5_CONTROLLER_LIMIT) // out of Xinput devices!
                            break;
                    }
                }
                catch (Exception e)
                {
                    LogDebug(e.Message, true);
                    AppLogger.LogToTray(e.Message, true);
                }

                running = true;

                if (_udpServer != null)
                {
                    //var UDP_SERVER_PORT = 26760;
                    var UDP_SERVER_PORT = Global.getUDPServerPortNum();
                    var UDP_SERVER_LISTEN_ADDRESS = Global.getUDPServerListenAddress();

                    try
                    {
                        _udpServer.Start(UDP_SERVER_PORT, UDP_SERVER_LISTEN_ADDRESS);
                        LogDebug($"UDP server listening on address {UDP_SERVER_LISTEN_ADDRESS} port {UDP_SERVER_PORT}");
                    }
                    catch (System.Net.Sockets.SocketException ex)
                    {
                        var errMsg = string.Format("Couldn't start UDP server on address {0}:{1}, outside applications won't be able to access pad data ({2})", UDP_SERVER_LISTEN_ADDRESS, UDP_SERVER_PORT, ex.SocketErrorCode);

                        LogDebug(errMsg, true);
                        AppLogger.LogToTray(errMsg, true, true);
                    }
                }
            }
            else
            {
                string logMessage = string.Empty;
                if (!vigemInstalled)
                {
                    logMessage = "ViGEmBus is not installed";
                }
                else if (!Global.IsRunningSupportedViGEmBus())
                {
                    logMessage = string.Format("Unsupported ViGEmBus found ({0}). Please install at least ViGEmBus 1.17.333.0", Global.vigembusVersion);
                }
                else
                {
                    logMessage = "Could not connect to ViGEmBus. Please check the status of the System device in Device Manager and if Visual C++ 2017 Redistributable is installed.";
                }

                LogDebug(logMessage);
                AppLogger.LogToTray(logMessage);
            }

            inServiceTask = false;
            runHotPlug = true;
            ServiceStarted?.Invoke(this, EventArgs.Empty);
            RunningChanged?.Invoke(this, EventArgs.Empty);
            using var process = Process.GetCurrentProcess();
            process.PriorityClass = ProcessPriorityOptions.Classes[Global.ProcessPriority];
            return true;
        }

        private void PrepareDevUDPMotion(DS5Device device, int index)
        {
            int tempIdx = index;
            DS5Device.ReportHandler<EventArgs> tempEvnt = (sender, args) =>
            {
                DualShockPadMeta padDetail = new DualShockPadMeta();
                GetPadDetailForIdx(tempIdx, ref padDetail);
                DS5State stateForUdp = TempState[tempIdx];

                CurrentState[tempIdx].CopyTo(stateForUdp);
                if (Global.IsUsingUDPServerSmoothing())
                {
                    if (stateForUdp.elapsedTime == 0)
                    {
                        // No timestamp was found. Exit out of routine
                        return;
                    }

                    double rate = 1.0 / stateForUdp.elapsedTime;
                    OneEuroFilter3D accelFilter = udpEuroPairAccel[tempIdx];
                    stateForUdp.Motion.accelXG = accelFilter.axis1Filter.Filter(stateForUdp.Motion.accelXG, rate);
                    stateForUdp.Motion.accelYG = accelFilter.axis2Filter.Filter(stateForUdp.Motion.accelYG, rate);
                    stateForUdp.Motion.accelZG = accelFilter.axis3Filter.Filter(stateForUdp.Motion.accelZG, rate);

                    OneEuroFilter3D gyroFilter = udpEuroPairGyro[tempIdx];
                    stateForUdp.Motion.angVelYaw = gyroFilter.axis1Filter.Filter(stateForUdp.Motion.angVelYaw, rate);
                    stateForUdp.Motion.angVelPitch = gyroFilter.axis2Filter.Filter(stateForUdp.Motion.angVelPitch, rate);
                    stateForUdp.Motion.angVelRoll = gyroFilter.axis3Filter.Filter(stateForUdp.Motion.angVelRoll, rate);
                }

                _udpServer?.NewReportIncoming(ref padDetail, stateForUdp, udpOutBuffers[tempIdx]);
            };

            device.MotionEvent = tempEvnt;
            device.Report += tempEvnt;
        }

        private void CheckQuickCharge(object sender, EventArgs e)
        {
            DS5Device device = sender as DS5Device;
            if (device.ConnectionType == ConnectionType.BT && getQuickCharge() &&
                device.Charging)
            {
                // Set disconnect flag here. Later Hotplug event will check
                // for presence of flag and remove the device then
                device.ReadyQuickChargeDisconnect = true;
            }
        }

        public void PrepareAbort()
        {
            for (int i = 0, arlength = DS5Controllers.Length; i < arlength; i++)
            {
                DS5Device tempDevice = DS5Controllers[i];
                if (tempDevice != null)
                {
                    tempDevice.PrepareAbort();
                }
            }
        }

        public bool Stop(bool showlog = true, bool immediateUnplug = false)
        {
            if (running)
            {
                running = false;
                runHotPlug = false;
                inServiceTask = true;
                PreServiceStop?.Invoke(this, EventArgs.Empty);

                if (showlog)
                    LogDebug(DS5ControlApp.Properties.Resources.StoppingX360);

                LogDebug("Closing connection to ViGEmBus");

                bool anyUnplugged = false;
                for (int i = 0, arlength = DS5Controllers.Length; i < arlength; i++)
                {
                    DS5Device tempDevice = DS5Controllers[i];
                    if (tempDevice != null)
                    {
                        if ((DCBTatStop && !tempDevice.isCharging()) || suspending)
                        {
                            if (tempDevice.getConnectionType() == ConnectionType.BT)
                            {
                                tempDevice.StopUpdate();
                                tempDevice.DisconnectBT(true);
                            }
                            else if (tempDevice.getConnectionType() == ConnectionType.SONYWA)
                            {
                                // Controller disconnect will complete on next attempted read.
                                // Do not use StopUpdate here
                                tempDevice.DisconnectDongle(true);
                            }
                            else
                            {
                                tempDevice.StopUpdate();
                            }
                        }
                        else
                        {
                            DS5LightBar.forcelight[i] = false;
                            DS5LightBar.forcedFlash[i] = 0;
                            DS5LightBar.defaultLight = true;
                            DS5LightBar.updateLightBar(DS5Controllers[i], i);
                            tempDevice.IsRemoved = true;
                            tempDevice.StopUpdate();
                            DS5Devices.RemoveDevice(tempDevice);
                            Thread.Sleep(50);
                        }

                        CurrentState[i].Battery = PreviousState[i].Battery = 0; // Reset for the next connection's initial status change.
                        OutputDevice tempout = outputDevices[i];
                        if (tempout != null)
                        {
                            UnplugOutDev(i, tempDevice, immediate: immediateUnplug, force: true);
                            anyUnplugged = true;
                        }

                        //outputDevices[i] = null;
                        //useDInputOnly[i] = true;
                        //Global.activeOutDevType[i] = OutContType.None;
                        useDInputOnly[i] = true;
                        DS5Controllers[i] = null;
                        oscState[i] = new DS5State();
                        touchPad[i] = null;
                        lag[i] = false;
                        inWarnMonitor[i] = false;
                    }
                }

                if (showlog)
                    LogDebug(DS5ControlApp.Properties.Resources.StoppingDS5);

                DS5Devices.stopControllers();
                slotManager.ClearControllerList();

                if (oscListener != null)
                {
                    ChangeOSCListenerStatus(false);
                }

                if (oscSender != null)
                {
                    ChangeOSCSenderStatus(false);
                }

                if (_udpServer != null)
                {
                    ChangeUDPStatus(false);
                }

                if (showlog)
                    LogDebug(DS5ControlApp.Properties.Resources.StoppedDS5Windows);

                while (outputslotMan.RunningQueue)
                {
                    Thread.SpinWait(500);
                }
                outputslotMan.Stop(true);

                if (anyUnplugged)
                {
                    Thread.Sleep(OutputSlotManager.DELAY_TIME);
                }

                StopViGEm();

                // Disconnect from KBM system when stopping ControlService
                LogDebug($"Closing connection to output handler {outputKBMHandler.GetDisplayName()}");
                outputKBMHandler.Disconnect();
                inServiceTask = false;
                activeControllers = 0;
            }

            runHotPlug = false;
            ServiceStopped?.Invoke(this, EventArgs.Empty);
            RunningChanged?.Invoke(this, EventArgs.Empty);
            return true;
        }

        public bool HotPlug()
        {
            if (running)
            {
                inServiceTask = true;
                loopControllers = true;
                eventDispatcher.Invoke(() =>
                {
                    DS5Devices.findControllers();
                });

                IEnumerable<DS5Device> devices = DS5Devices.getDS5Controllers();
                int numControllers = devices.Count();
                activeControllers = numControllers;
                InputDevices.JoyConDevice tempPrimaryJoyDev = null;
                InputDevices.JoyConDevice tempSecondaryJoyDev = null;

                if (deviceOptions.JoyConDeviceOpts.LinkedMode == JoyConDeviceOptions.LinkMode.Joined)
                {
                    tempPrimaryJoyDev = devices.Where(d =>
                        (d.DeviceType == InputDevices.InputDeviceType.JoyConL || d.DeviceType == InputDevices.InputDeviceType.JoyConR)
                         && d.PrimaryDevice && d.JointDeviceSlotNumber == -1).FirstOrDefault() as InputDevices.JoyConDevice;

                    tempSecondaryJoyDev = devices.Where(d =>
                        (d.DeviceType == InputDevices.InputDeviceType.JoyConL || d.DeviceType == InputDevices.InputDeviceType.JoyConR)
                        && !d.PrimaryDevice && d.JointDeviceSlotNumber == -1).FirstOrDefault() as InputDevices.JoyConDevice;
                }

                for (var devEnum = devices.GetEnumerator(); devEnum.MoveNext() && loopControllers;)
                {
                    DS5Device device = devEnum.Current;

                    if (device.isDisconnectingStatus())
                        continue;

                    // Use local method rather than Func
                    bool checkAlreadyExists()
                    {
                        for (int Index = 0, arlength = DS5Controllers.Length; Index < arlength; Index++)
                        {
                            if (DS5Controllers[Index] != null &&
                                DS5Controllers[Index].getMacAddress() == device.getMacAddress())
                            {
                                device.CheckControllerNumDeviceSettings(numControllers);
                                return true;
                            }
                        }

                        return false;
                    }

                    if (checkAlreadyExists())
                    {
                        continue;
                    }

                    for (int Index = 0, arlength = DS5Controllers.Length;
                        Index < arlength && Index < CURRENT_DS5_CONTROLLER_LIMIT; Index++)
                    {
                        if (DS5Controllers[Index] == null)
                        {
                            BeginPrepareConnectedInputController(device);

                            if (deviceOptions.JoyConDeviceOpts.LinkedMode == JoyConDeviceOptions.LinkMode.Joined)
                            {
                                if ((device.DeviceType == InputDevices.InputDeviceType.JoyConL ||
                                    device.DeviceType == InputDevices.InputDeviceType.JoyConR) && device.PerformStateMerge)
                                {
                                    if (device.PrimaryDevice &&
                                        tempSecondaryJoyDev != null)
                                    {
                                        InputDevices.JoyConDevice currentJoyDev = device as InputDevices.JoyConDevice;
                                        tempSecondaryJoyDev.JointDevice = currentJoyDev;
                                        currentJoyDev.JointDevice = tempSecondaryJoyDev;

                                        tempSecondaryJoyDev.JointState = currentJoyDev.JointState;

                                        InputDevices.JoyConDevice secondaryJoy = tempSecondaryJoyDev;
                                        secondaryJoy.Removal += (sender, args) =>
                                        {
                                            currentJoyDev.JointDevice = null;
                                        };
                                        currentJoyDev.Removal += (sender, args) =>
                                        {
                                            secondaryJoy.JointDevice = null;
                                        };

                                        tempSecondaryJoyDev = null;
                                        tempPrimaryJoyDev = null;
                                    }
                                    else if (!device.PrimaryDevice &&
                                        tempPrimaryJoyDev != null)
                                    {
                                        InputDevices.JoyConDevice currentJoyDev = device as InputDevices.JoyConDevice;
                                        tempPrimaryJoyDev.JointDevice = currentJoyDev;
                                        currentJoyDev.JointDevice = tempPrimaryJoyDev;

                                        tempPrimaryJoyDev.JointState = currentJoyDev.JointState;

                                        InputDevices.JoyConDevice parentJoy = tempPrimaryJoyDev;
                                        tempPrimaryJoyDev.Removal += (sender, args) =>
                                        {
                                            currentJoyDev.JointDevice = null;
                                        };
                                        currentJoyDev.Removal += (sender, args) =>
                                        {
                                            parentJoy.JointDevice = null;
                                        };

                                        tempPrimaryJoyDev = null;
                                    }
                                }
                            }

                            DS5Controllers[Index] = device;
                            device.DeviceSlotNumber = Index;
                            PrepareConnectedInputControllerSettingEvents(numControllers, device, Index);

                            HotplugController?.Invoke(this, device, Index);
                            break;
                        }
                    }
                }

                inServiceTask = false;
            }

            return true;
        }

        private void PrepareConnectedInputControllerSettingEvents(int numControllers, DS5Device device, int index)
        {
            Global.RefreshExtrasButtons(index, GetKnownExtraButtons(device));
            Global.LoadControllerConfigs(device);
            device.LoadStoreSettings();
            device.CheckControllerNumDeviceSettings(numControllers);

            slotManager.AddController(device, index);
            if (isUsingOSCSender())
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/plug", 1));
            }
            device.Removal += this.On_DS5Removal;
            device.Removal += DS5Devices.On_Removal;
            device.SyncChange += this.On_SyncChange;
            device.SyncChange += DS5Devices.UpdateSerial;
            device.SerialChange += this.On_SerialChange;
            device.ChargingChanged += CheckQuickCharge;

            touchPad[index] = new Mouse(index, device);
            bool profileLoaded = false;
            bool useAutoProfile = useTempProfile[index];
            if (!useAutoProfile)
            {
                if (device.isValidSerial() && containsLinkedProfile(device.getMacAddress()))
                {
                    ProfilePath[index] = getLinkedProfile(device.getMacAddress());
                    Global.linkedProfileCheck[index] = true;
                }
                else
                {
                    ProfilePath[index] = OlderProfilePath[index];
                    Global.linkedProfileCheck[index] = false;
                }

                // Now attempt to load requested profile and settings
                profileLoaded = LoadProfile(index, false, this, false, false);
            }

            if (profileLoaded || useAutoProfile)
            {
                device.LightBarColor = getMainColor(index);

                if (!getDInputOnly(index) && device.isSynced())
                {
                    if (device.PrimaryDevice)
                    {
                        PluginOutDev(index, device);
                    }
                    else if (device.JointDeviceSlotNumber != DS5Device.DEFAULT_JOINT_SLOT_NUMBER)
                    {
                        int otherIdx = device.JointDeviceSlotNumber;
                        OutputDevice tempOutDev = outputDevices[otherIdx];
                        if (tempOutDev != null)
                        {
                            OutContType tempConType = activeOutDevType[otherIdx];
                            EstablishOutFeedback(index, tempConType, tempOutDev, device);
                            outputDevices[index] = tempOutDev;
                            Global.activeOutDevType[index] = tempConType;
                        }
                    }
                }
                else
                {
                    useDInputOnly[index] = true;
                    Global.activeOutDevType[index] = OutContType.None;
                }

                if (device.PrimaryDevice && device.OutputMapGyro)
                {
                    TouchPadOn(index, device);
                }
                else if (device.JointDeviceSlotNumber != DS5Device.DEFAULT_JOINT_SLOT_NUMBER)
                {
                    int otherIdx = device.JointDeviceSlotNumber;
                    DS5Device tempDev = DS5Controllers[otherIdx];
                    if (tempDev != null)
                    {
                        int mappedIdx = tempDev.PrimaryDevice ? otherIdx : index;
                        DS5Device gyroDev = device.OutputMapGyro ? device : (tempDev.OutputMapGyro ? tempDev : null);
                        if (gyroDev != null)
                        {
                            TouchPadOn(mappedIdx, gyroDev);
                        }
                    }
                }

                CheckProfileOptions(index, device);
                SetupInitialHookEvents(index, device);
            }

            int tempIdx = index;
            device.Report += (sender, e) =>
            {
                this.On_Report(sender, e, tempIdx);
            };

            if (_udpServer != null && index < UdpServer.NUMBER_SLOTS)
            {
                PrepareDevUDPMotion(device, tempIdx);
            }

            device.StartUpdate();
        }

        private void BeginPrepareConnectedInputController(DS5Device device, bool showlog = false)
        {
            if (hidDeviceHidingEnabled && CheckAffected(device))
            {
                //device.CurrentExclusiveStatus = DS5Device.ExclusiveStatus.HidGuardAffected;
                ChangeExclusiveStatus(device);
            }

            //Task task = new Task(() => { Thread.Sleep(5); WarnExclusiveModeFailure(device); });
            //task.Start();

            PrepareDS5DeviceSettingHooks(device);
        }

        public void ResetUdpSmoothingFilters(int idx)
        {
            if (idx < UdpServer.NUMBER_SLOTS)
            {
                OneEuroFilter3D temp = udpEuroPairAccel[idx] = new OneEuroFilter3D();
                temp.SetFilterAttrs(Global.UDPServerSmoothingMincutoff, Global.UDPServerSmoothingBeta);

                temp = udpEuroPairGyro[idx] = new OneEuroFilter3D();
                temp.SetFilterAttrs(Global.UDPServerSmoothingMincutoff, Global.UDPServerSmoothingBeta);
            }
        }

        private void ChangeUdpSmoothingAttrs(object sender, EventArgs e)
        {
            for (int i = 0; i < udpEuroPairAccel.Length; i++)
            {
                OneEuroFilter3D temp = udpEuroPairAccel[i];
                temp.SetFilterAttrs(Global.UDPServerSmoothingMincutoff, Global.UDPServerSmoothingBeta);
            }

            for (int i = 0; i < udpEuroPairGyro.Length; i++)
            {
                OneEuroFilter3D temp = udpEuroPairGyro[i];
                temp.SetFilterAttrs(Global.UDPServerSmoothingMincutoff, Global.UDPServerSmoothingBeta);
            }
        }

        public void CheckProfileOptions(int ind, DS5Device device, bool startUp = false)
        {
            device.ModifyFeatureSetFlag(VidPidFeatureSet.NoOutputData, !getEnableOutputDataToDS5(ind));
            if (!getEnableOutputDataToDS5(ind))
                LogDebug("Output data to DS4 disabled. Lightbar and rumble events are not written to DS4 gamepad. If the gamepad is connected over BT then IdleDisconnect option is recommended to let DS5Control to close the connection after long period of idling.");

            device.setIdleTimeout(getIdleDisconnectTimeout(ind));
            device.setBTPollRate(getBTPollRate(ind));

            touchPad[ind].ResetTrackAccel(getTrackballFriction(ind));
            touchPad[ind].ResetToggleGyroModes();

            //Global.TouchOutMode[ind] = TouchpadOutMode.MouseJoystick;
            touchPad[ind].PostSetup();

            Global.L2OutputSettings[ind].TrigEffectSettings.maxValue = (byte)(Math.Max(Global.L2ModInfo[ind].maxOutput, Global.L2ModInfo[ind].maxZone) / 100.0 * 255);
            Global.R2OutputSettings[ind].TrigEffectSettings.maxValue = (byte)(Math.Max(Global.R2ModInfo[ind].maxOutput, Global.R2ModInfo[ind].maxZone) / 100.0 * 255);

            device.PrepareTriggerEffect(InputDevices.TriggerId.LeftTrigger, Global.L2OutputSettings[ind].TriggerEffect,
                Global.L2OutputSettings[ind].TrigEffectSettings);
            device.PrepareTriggerEffect(InputDevices.TriggerId.RightTrigger, Global.R2OutputSettings[ind].TriggerEffect,
                Global.R2OutputSettings[ind].TrigEffectSettings);

            device.RumbleAutostopTime = getRumbleAutostopTime(ind);
            device.setRumble(0, 0);
            device.LightBarColor = Global.getMainColor(ind);

            // DualSense specific profile settings
            if (device is InputDevices.DualSenseDevice dualsense)
            {
                switch (DualSenseRumbleEmulationMode[ind])
                {
                    case InputDevices.DualSenseDevice.RumbleEmulationMode.Disabled:
                        dualsense.UseRumble = false;
                        dualsense.UseAccurateRumble = false;
                        break;
                    case InputDevices.DualSenseDevice.RumbleEmulationMode.Legacy:
                        dualsense.UseRumble = true;
                        dualsense.UseAccurateRumble = false;
                        break;
                    case InputDevices.DualSenseDevice.RumbleEmulationMode.Accurate:
                    default:
                        dualsense.UseRumble = true;
                        dualsense.UseAccurateRumble = true;
                        break;
                }
                dualsense.HapticPowerLevel = DualSenseHapticPowerLevel[ind];
            }

            if (!startUp)
            {
                CheckLauchProfileOption(ind, device);
            }
        }

        private void CheckLauchProfileOption(int ind, DS5Device device)
        {
            string programPath = LaunchProgram[ind];
            if (programPath != string.Empty)
            {
                Process[] localAll = Process.GetProcesses();
                bool procFound = false;
                for (int procInd = 0, procsLen = localAll.Length; !procFound && procInd < procsLen; procInd++)
                {
                    try
                    {
                        string temp = localAll[procInd].MainModule.FileName;
                        if (temp == programPath)
                        {
                            procFound = true;
                        }
                    }
                    // Ignore any process for which this information
                    // is not exposed
                    catch { }
                }

                if (!procFound)
                {
                    Task processTask = new Task(() =>
                    {
                        Thread.Sleep(5000);
                        Process tempProcess = new Process();
                        tempProcess.StartInfo.FileName = programPath;
                        tempProcess.StartInfo.WorkingDirectory = new FileInfo(programPath).Directory.ToString();
                        //tempProcess.StartInfo.UseShellExecute = false;
                        try { tempProcess.Start(); }
                        catch { }
                    });

                    processTask.Start();
                }
            }
        }

        private void SetupInitialHookEvents(int ind, DS5Device device)
        {
            ResetUdpSmoothingFilters(ind);

            // Set up filter for new input device
            OneEuroFilter tempFilter = new OneEuroFilter(OneEuroFilterPair.DEFAULT_WHEEL_CUTOFF,
                OneEuroFilterPair.DEFAULT_WHEEL_BETA);
            Mapping.wheelFilters[ind] = tempFilter;

            // Carry over initial profile wheel smoothing values to filter instances.
            // Set up event hooks to keep values in sync
            SteeringWheelSmoothingInfo wheelSmoothInfo = WheelSmoothInfo[ind];
            wheelSmoothInfo.SetFilterAttrs(tempFilter);
            wheelSmoothInfo.SetRefreshEvents(tempFilter);

            FlickStickSettings flickStickSettings = Global.LSOutputSettings[ind].outputSettings.flickSettings;
            flickStickSettings.RemoveRefreshEvents();
            flickStickSettings.SetRefreshEvents(Mapping.flickMappingData[ind].flickFilter);

            flickStickSettings = Global.RSOutputSettings[ind].outputSettings.flickSettings;
            flickStickSettings.RemoveRefreshEvents();
            flickStickSettings.SetRefreshEvents(Mapping.flickMappingData[ind].flickFilter);

            int tempIdx = ind;
            Global.L2OutputSettings[ind].ResetEvents();
            Global.L2ModInfo[ind].ResetEvents();
            Global.L2OutputSettings[ind].TriggerEffectChanged += (sender, e) =>
            {
                device.PrepareTriggerEffect(InputDevices.TriggerId.LeftTrigger, Global.L2OutputSettings[tempIdx].TriggerEffect,
                    Global.L2OutputSettings[tempIdx].TrigEffectSettings);
            };
            Global.L2ModInfo[ind].MaxOutputChanged += (sender, e) =>
            {
                TriggerDeadZoneZInfo tempInfo = sender as TriggerDeadZoneZInfo;
                L2OutputSettings[tempIdx].TrigEffectSettings.maxValue = (byte)(Math.Max(tempInfo.maxOutput, tempInfo.maxZone) / 100.0 * 255.0);

                // Refresh trigger effect
                device.PrepareTriggerEffect(InputDevices.TriggerId.LeftTrigger, Global.L2OutputSettings[tempIdx].TriggerEffect,
                    Global.L2OutputSettings[tempIdx].TrigEffectSettings);
            };
            Global.L2ModInfo[ind].MaxZoneChanged += (sender, e) =>
            {
                TriggerDeadZoneZInfo tempInfo = sender as TriggerDeadZoneZInfo;
                L2OutputSettings[tempIdx].TrigEffectSettings.maxValue = (byte)(Math.Max(tempInfo.maxOutput, tempInfo.maxZone) / 100.0 * 255.0);

                // Refresh trigger effect
                device.PrepareTriggerEffect(InputDevices.TriggerId.LeftTrigger, Global.L2OutputSettings[tempIdx].TriggerEffect,
                    Global.L2OutputSettings[tempIdx].TrigEffectSettings);
            };

            Global.R2OutputSettings[ind].ResetEvents();
            Global.R2OutputSettings[ind].TriggerEffectChanged += (sender, e) =>
            {
                device.PrepareTriggerEffect(InputDevices.TriggerId.RightTrigger, Global.R2OutputSettings[tempIdx].TriggerEffect,
                    Global.R2OutputSettings[tempIdx].TrigEffectSettings);
            };
            Global.R2ModInfo[ind].MaxOutputChanged += (sender, e) =>
            {
                TriggerDeadZoneZInfo tempInfo = sender as TriggerDeadZoneZInfo;
                R2OutputSettings[tempIdx].TrigEffectSettings.maxValue = (byte)(tempInfo.maxOutput / 100.0 * 255.0);

                // Refresh trigger effect
                device.PrepareTriggerEffect(InputDevices.TriggerId.RightTrigger, Global.R2OutputSettings[tempIdx].TriggerEffect,
                    Global.R2OutputSettings[tempIdx].TrigEffectSettings);
            };
            Global.R2ModInfo[ind].MaxZoneChanged += (sender, e) =>
            {
                TriggerDeadZoneZInfo tempInfo = sender as TriggerDeadZoneZInfo;
                R2OutputSettings[tempIdx].TrigEffectSettings.maxValue = (byte)(tempInfo.maxOutput / 100.0 * 255.0);

                // Refresh trigger effect
                device.PrepareTriggerEffect(InputDevices.TriggerId.RightTrigger, Global.R2OutputSettings[tempIdx].TriggerEffect,
                    Global.R2OutputSettings[tempIdx].TrigEffectSettings);
            };
        }

        /// <summary>
        /// Perform Mapping property resetting as needed before loading profile settings
        /// </summary>
        /// <param name="device">Input device instance</param>
        public void PreLoadReset(int ind)
        {
            //DS5Device inputDevice = DS5Controllers[ind];
            //if (inputDevice == null)
            //{
            //    return;
            //}
            // Skip running for test profile with no mapping data
            if (ind >= Global.TEST_PROFILE_INDEX)
            {
                return;
            }

            // Reset current flick stick progress from previous profile
            Mapping.flickMappingData[ind].Reset();

            // Reset delta accel processors for sticks
            Mapping.deltaAccelProcessors[ind].LSProcessor.Reset();
            Mapping.deltaAccelProcessors[ind].RSProcessor.Reset();

            // Reset absolute mouse state data
            Mapping.absMouseOutputState[ind].Reset();

            // Reset some elements of current Mouse instance
            touchPad[ind]?.Reset();
        }

        public void TouchPadOn(int ind, DS5Device device)
        {
            Mouse tPad = touchPad[ind];
            //ITouchpadBehaviour tPad = touchPad[ind];
            device.Touchpad.TouchButtonDown += tPad.touchButtonDown;
            device.Touchpad.TouchButtonUp += tPad.touchButtonUp;
            device.Touchpad.TouchesBegan += tPad.touchesBegan;
            device.Touchpad.TouchesBegan += tPad.TouchStartedOrEnded;
            device.Touchpad.TouchesMoved += tPad.touchesMoved;
            device.Touchpad.TouchesEnded += tPad.touchesEnded;
            device.Touchpad.TouchesEnded += tPad.TouchStartedOrEnded;
            device.Touchpad.TouchUnchanged += tPad.touchUnchanged;
            //device.Touchpad.PreTouchProcess += delegate { touchPad[ind].populatePriorButtonStates(); };
            device.Touchpad.PreTouchProcess += (sender, args) => { touchPad[ind].populatePriorButtonStates(); };
            device.SixAxis.SixAccelMoved += tPad.sixaxisMoved;
            //LogDebug("Touchpad mode for " + device.MacAddress + " is now " + tmode.ToString());
            //Log.LogToTray("Touchpad mode for " + device.MacAddress + " is now " + tmode.ToString());
        }

        public string GetDS5Battery(int index)
        {
            DS5Device d = DS5Controllers[index];
            if (d != null)
            {
                string battery;
                if (!d.IsAlive())
                    battery = "...";

                if (d.isCharging())
                {
                    if (d.getBattery() >= 100)
                        battery = DS5ControlApp.Properties.Resources.Full;
                    else
                        battery = d.getBattery() + "%+";
                }
                else
                {
                    battery = d.getBattery() + "%";
                }

                return battery;
            }
            else
                return DS5ControlApp.Properties.Resources.NA;
        }

        protected void On_SerialChange(object sender, EventArgs e)
        {
            DS5Device device = (DS5Device)sender;
            int ind = -1;
            for (int i = 0, arlength = MAX_DS5_CONTROLLER_COUNT; ind == -1 && i < arlength; i++)
            {
                DS5Device tempDev = DS5Controllers[i];
                if (tempDev != null && device == tempDev)
                    ind = i;
            }

            if (ind >= 0)
            {
                OnDeviceSerialChange(this, ind, device.getMacAddress());
            }
        }

        protected void On_SyncChange(object sender, EventArgs e)
        {
            DS5Device device = (DS5Device)sender;
            int ind = -1;
            for (int i = 0, arlength = CURRENT_DS5_CONTROLLER_LIMIT; ind == -1 && i < arlength; i++)
            {
                DS5Device tempDev = DS5Controllers[i];
                if (tempDev != null && device == tempDev)
                    ind = i;
            }

            if (ind >= 0)
            {
                bool synced = device.isSynced();

                if (!synced)
                {
                    if (!useDInputOnly[ind])
                    {
                        Global.activeOutDevType[ind] = OutContType.None;
                        UnplugOutDev(ind, device);
                    }
                }
                else
                {
                    if (!getDInputOnly(ind))
                    {
                        touchPad[ind].ReplaceOneEuroFilterPair();
                        //touchPad[ind].ReplaceOneEuroFilterPair();

                        touchPad[ind].Cursor.ReplaceOneEuroFilterPair();
                        touchPad[ind].Cursor.SetupLateOneEuroFilters();
                        PluginOutDev(ind, device);
                    }
                }
            }
        }

        // Called when DS4 is disconnected or timed out
        protected void On_DS5Removal(object sender, EventArgs e)
        {
            DS5Device device = (DS5Device)sender;
            int ind = -1;
            for (int i = 0, arlength = DS5Controllers.Length; ind == -1 && i < arlength; i++)
            {
                if (DS5Controllers[i] != null && device.getMacAddress() == DS5Controllers[i].getMacAddress())
                    ind = i;
            }

            if (ind != -1)
            {
                bool removingStatus = false;
                lock (device.removeLocker)
                {
                    if (!device.IsRemoving)
                    {
                        removingStatus = true;
                        device.IsRemoving = true;
                    }
                }

                if (removingStatus)
                {
                    CurrentState[ind].Battery = PreviousState[ind].Battery = 0; // Reset for the next connection's initial status change.
                    muteButtonEdges[ind].Reset();
                    if (!useDInputOnly[ind])
                    {
                        UnplugOutDev(ind, device);
                    }
                    else if (!device.PrimaryDevice)
                    {
                        OutputDevice outDev = outputDevices[ind];
                        if (outDev != null)
                        {
                            outDev.RemoveFeedback(ind);
                            outputDevices[ind] = null;
                        }
                    }

                    // Use Task to reset device synth state and commit it
                    Task.Run(() =>
                    {
                        Mapping.Commit(ind);
                    }).Wait();

                    string removed = DS5ControlApp.Properties.Resources.ControllerWasRemoved.Replace("*Mac address*", (ind + 1).ToString());
                    if (device.getBattery() <= 20 &&
                        device.getConnectionType() == ConnectionType.BT && !device.isCharging())
                    {
                        removed += ". " + DS5ControlApp.Properties.Resources.ChargeController;
                    }

                    LogDebug(removed);
                    AppLogger.LogToTray(removed);
                    /*Stopwatch sw = new Stopwatch();
                    sw.Start();
                    while (sw.ElapsedMilliseconds < XINPUT_UNPLUG_SETTLE_TIME)
                    {
                        // Use SpinWait to keep control of current thread. Using Sleep could potentially
                        // cause other events to get run out of order
                        System.Threading.Thread.SpinWait(500);
                    }
                    sw.Stop();
                    */

                    device.IsRemoved = true;
                    device.Synced = false;
                    DS5Controllers[ind] = null;
                    oscState[ind] = new DS5State();
                    //eventDispatcher.Invoke(() =>
                    //{
                    slotManager.RemoveController(device, ind);
                    if (isUsingOSCSender())
                    {
                        oscSender.Send(new SharpOSC.OscMessage("/ds5control/monitor/" + ind + "/plug", 0));
                    }
                    //});

                    touchPad[ind] = null;
                    lag[ind] = false;
                    inWarnMonitor[ind] = false;
                    useDInputOnly[ind] = true;
                    Global.activeOutDevType[ind] = OutContType.None;
                    /* Leave up to Auto Profile system to change the following flags? */
                    //Global.useTempProfile[ind] = false;
                    //Global.tempprofilename[ind] = string.Empty;
                    //Global.tempprofileDistance[ind] = false;

                    //Thread.Sleep(XINPUT_UNPLUG_SETTLE_TIME);
                }
            }
        }

        public bool[] lag = new bool[MAX_DS5_CONTROLLER_COUNT] { false, false, false, false, false, false, false, false };
        public bool[] inWarnMonitor = new bool[MAX_DS5_CONTROLLER_COUNT] { false, false, false, false, false, false, false, false };
        private byte[] currentBattery = new byte[MAX_DS5_CONTROLLER_COUNT] { 0, 0, 0, 0, 0, 0, 0, 0 };
        private bool[] charging = new bool[MAX_DS5_CONTROLLER_COUNT] { false, false, false, false, false, false, false, false };
        private string[] tempStrings = new string[MAX_DS5_CONTROLLER_COUNT] { string.Empty, string.Empty, string.Empty, string.Empty, string.Empty, string.Empty, string.Empty, string.Empty };
        private DS5Control.Api.ButtonEdgeDetector[] muteButtonEdges = new DS5Control.Api.ButtonEdgeDetector[MAX_DS5_CONTROLLER_COUNT];

        // Called every time a new input report has arrived
        protected void On_Report(DS5Device device, EventArgs e, int ind)
        {
            if (ind != -1)
            {
                string devError = tempStrings[ind] = device.error;
                if (!string.IsNullOrEmpty(devError))
                {
                    LogDebug(devError);
                }

                if (inWarnMonitor[ind])
                {
                    int flashWhenLateAt = getFlashWhenLateAt();
                    if (!lag[ind] && device.Latency >= flashWhenLateAt)
                    {
                        lag[ind] = true;
                        LagFlashWarning(device, ind, true);
                    }
                    else if (lag[ind] && device.Latency < flashWhenLateAt)
                    {
                        lag[ind] = false;
                        LagFlashWarning(device, ind, false);
                    }
                }
                else
                {
                    if (DateTime.UtcNow - device.firstActive > TimeSpan.FromSeconds(5))
                    {
                        inWarnMonitor[ind] = true;
                    }
                }

                DS5State cState, tempControlState;
                if (!device.PerformStateMerge)
                {
                    cState = CurrentState[ind];
                    device.getRawCurrentState(cState);
                    tempControlState = CurrentState[ind];
                }
                else
                {
                    cState = device.JointState;
                    device.MergeStateData(cState);
                    // Need to copy state object info for use in UDP server
                    cState.CopyTo(CurrentState[ind]);
                    tempControlState = CurrentState[ind];
                }

                if (muteButtonEdges[ind].Update(cState.Mute))
                {
                    MuteButtonPressed?.Invoke(this, device, ind);
                }

                DS5State pState = device.getPreviousStateRef();
                //device.getPreviousState(PreviousState[ind]);
                //DS5State pState = PreviousState[ind];


                if (device.firstReport && device.isSynced())
                {
                    // Only send Log message when device is considered a primary device
                    if (device.PrimaryDevice)
                    {
                        if (File.Exists(Path.Combine(appdatapath, "Profiles", $"{ProfilePath[ind]}.xml")))
                        {
                            string prolog = string.Format(DS5ControlApp.Properties.Resources.UsingProfile, (ind + 1).ToString(), ProfilePath[ind], $"{device.Battery}");
                            LogDebug(prolog);
                            AppLogger.LogToTray(prolog);
                        }
                        else
                        {
                            string prolog = string.Format(DS5ControlApp.Properties.Resources.NotUsingProfile, (ind + 1).ToString(), $"{device.Battery}");
                            LogDebug(prolog);
                            AppLogger.LogToTray(prolog);
                        }
                    }

                    device.firstReport = false;
                }

                if (device.PrimaryDevice && Global.UseIconChoice == TrayIconChoice.Battery)
                {
                    InvokeBatteryChanged(cState.Battery);
                }

                if (!device.PrimaryDevice)
                {
                    // Make sure a joined device is still linked
                    int jointInd = device.JointDeviceSlotNumber;
                    if (device.OutputMapGyro &&
                        jointInd != DS5Device.DEFAULT_JOINT_SLOT_NUMBER)
                    {
                        // Output changes from Gyro data early. Seems better to ME... REE
                        GyroOutMode imuOutMode = Global.GetGyroOutMode(device.JointDeviceSlotNumber);
                        if (imuOutMode != GyroOutMode.None)
                        {
                            if (imuOutMode == GyroOutMode.Mouse)
                            {
                                outputKBMHandler.Sync();
                            }
                            else if (imuOutMode == GyroOutMode.MouseJoystick)
                            {
                                // Add new Mapping method and add data to
                                // parent device state
                                DS5State tempMapState = MappedState[jointInd];
                                Mapping.TempMouseJoystick(jointInd, tempMapState);
                                if (!useDInputOnly[jointInd])
                                {
                                    outputDevices[jointInd]?.ConvertandSendReport(tempMapState, jointInd);
                                }
                            }
                        }
                    }
                    else if (!device.OutputMapGyro)
                    {
                        // Copy for use in UDP
                        tempControlState.Motion = device.GetRawCurrentStateRef().Motion;
                    }

                    // Skip mapping routine if part of a joined device
                    return;
                }

                if (getEnableTouchToggle(ind))
                {
                    CheckForTouchToggle(ind, cState, pState);
                }

                cState = device.Debouncer.ProcessInput(cState);

                cState = Mapping.SetCurveAndDeadzone(ind, cState, TempState[ind]);

                if (!recordingMacro && (useTempProfile[ind] ||
                    containsCustomAction(ind) || containsCustomExtras(ind) ||
                    getProfileActionCount(ind) > 0))
                {
                    DS5State tempMapState = MappedState[ind];
                    DS5State oscMapState = oscState[ind];

                    if (isUsingOSCSender())
                    {
                        OSCPreMappingStep(ind, cState, tempMapState, oscMapState);
                    }

                    Mapping.MapCustom(ind, cState, tempMapState, ExposedState[ind], touchPad[ind], this);

                    // Copy current Touchpad and Gyro data
                    // Might change to use new DS5State.CopyExtrasTo method
                    tempMapState.Motion = cState.Motion;
                    tempMapState.ds5Timestamp = cState.ds5Timestamp;
                    tempMapState.FrameCounter = cState.FrameCounter;
                    tempMapState.TouchPacketCounter = cState.TouchPacketCounter;
                    tempMapState.TrackPadTouch0 = cState.TrackPadTouch0;
                    tempMapState.TrackPadTouch1 = cState.TrackPadTouch1;

                    if (isUsingOSCServer())
                    {
                        OSCPostMappingStep(tempMapState, oscMapState);
                    }

                    cState = tempMapState;

                }

                if (!useDInputOnly[ind])
                {
                    // Perform this virtual trigger button check in post
                    if (activeOutDevType[ind] == OutContType.DS4)
                    {
                        DS5TriggerOutputMode trigMode = Global.GetOutputDS5TriggerMode(ind);
                        if (trigMode == DS5TriggerOutputMode.Default)
                        {
                            cState.L2Btn = cState.L2 > 0;
                            cState.R2Btn = cState.R2 > 0;
                        }
                        else if (trigMode == DS5TriggerOutputMode.Buttons)
                        {
                            cState.L2Btn = cState.L2 > 0;
                            cState.R2Btn = cState.R2 > 0;
                            // Disable analog output
                            cState.L2 = 0;
                            cState.R2 = 0;
                        }
                    }

                    outputDevices[ind]?.ConvertandSendReport(cState, ind);
                    //testNewReport(ref x360reports[ind], cState, ind);
                    //x360controls[ind]?.SendReport(x360reports[ind]);

                    //x360Bus.Parse(cState, processingData[ind].Report, ind);
                    // We push the translated Xinput state, and simultaneously we
                    // pull back any possible rumble data coming from Xinput consumers.
                    /*if (x360Bus.Report(processingData[ind].Report, processingData[ind].Rumble))
                    {
                        byte Big = processingData[ind].Rumble[3];
                        byte Small = processingData[ind].Rumble[4];

                        if (processingData[ind].Rumble[1] == 0x08)
                        {
                            SetDevRumble(device, Big, Small, ind);
                        }
                    }
                    */
                }
                else
                {
                    // UseDInputOnly profile may re-map sixaxis gyro sensor values as a VJoy joystick axis (steering wheel emulation mode using VJoy output device). Handle this option because VJoy output works even in USeDInputOnly mode.
                    // If steering wheel emulation uses LS/RS/R2/L2 output axies then the profile should NOT use UseDInputOnly option at all because those require a virtual output device.
                    SASteeringWheelEmulationAxisType steeringWheelMappedAxis = Global.GetSASteeringWheelEmulationAxis(ind);
                    switch (steeringWheelMappedAxis)
                    {
                        case SASteeringWheelEmulationAxisType.None: break;

                        case SASteeringWheelEmulationAxisType.VJoy1X:
                        case SASteeringWheelEmulationAxisType.VJoy2X:
                            VJoyFeeder.vJoyFeeder.FeedAxisValue(cState.SASteeringWheelEmulationUnit, ((((uint)steeringWheelMappedAxis) - ((uint)SASteeringWheelEmulationAxisType.VJoy1X)) / 3) + 1, VJoyFeeder.HID_USAGES.HID_USAGE_X);
                            break;

                        case SASteeringWheelEmulationAxisType.VJoy1Y:
                        case SASteeringWheelEmulationAxisType.VJoy2Y:
                            VJoyFeeder.vJoyFeeder.FeedAxisValue(cState.SASteeringWheelEmulationUnit, ((((uint)steeringWheelMappedAxis) - ((uint)SASteeringWheelEmulationAxisType.VJoy1X)) / 3) + 1, VJoyFeeder.HID_USAGES.HID_USAGE_Y);
                            break;

                        case SASteeringWheelEmulationAxisType.VJoy1Z:
                        case SASteeringWheelEmulationAxisType.VJoy2Z:
                            VJoyFeeder.vJoyFeeder.FeedAxisValue(cState.SASteeringWheelEmulationUnit, ((((uint)steeringWheelMappedAxis) - ((uint)SASteeringWheelEmulationAxisType.VJoy1X)) / 3) + 1, VJoyFeeder.HID_USAGES.HID_USAGE_Z);
                            break;

                        default: break;
                    }
                }

                // Output any synthetic events.
                Mapping.Commit(ind);

                // Update the Lightbar color
                DS5LightBar.updateLightBar(device, ind);

                if (device.PerformStateMerge)
                {
                    device.PreserveMergedStateData();
                }

                if (device.PerformStateMerge && !device.OutputMapGyro)
                {
                    // Copy for use in UDP
                    tempControlState.Motion = device.GetRawCurrentStateRef().Motion;
                }
            }
        }

        private static void OSCPostMappingStep(DS5State tempMapState, DS5State oscMapState)
        {
            tempMapState.Cross |= oscMapState.Cross;
            tempMapState.Square |= oscMapState.Square;
            tempMapState.Circle |= oscMapState.Circle;
            tempMapState.Triangle |= oscMapState.Triangle;
            tempMapState.R1 |= oscMapState.R1;
            tempMapState.R3 |= oscMapState.R3;
            tempMapState.L1 |= oscMapState.L1;
            tempMapState.L3 |= oscMapState.L3;
            tempMapState.DpadUp |= oscMapState.DpadUp;
            tempMapState.DpadLeft |= oscMapState.DpadLeft;
            tempMapState.DpadRight |= oscMapState.DpadRight;
            tempMapState.DpadDown |= oscMapState.DpadDown;
            tempMapState.Options |= oscMapState.Options;
            tempMapState.Share |= oscMapState.Share;

            tempMapState.LX = oscMapState.LX != 128 ? oscMapState.LX : tempMapState.LX;
            tempMapState.LY = oscMapState.LY != 128 ? oscMapState.LY : tempMapState.LY;
            tempMapState.L2 = oscMapState.L2 != 0 ? oscMapState.L2 : tempMapState.L2;
            tempMapState.RX = oscMapState.RX != 128 ? oscMapState.RX : tempMapState.RX;
            tempMapState.RY = oscMapState.RY != 128 ? oscMapState.RY : tempMapState.RY;
            tempMapState.R2 = oscMapState.R2 != 0 ? oscMapState.R2 : tempMapState.R2;
        }

        private void OSCPreMappingStep(int ind, DS5State cState, DS5State tempMapState,
            DS5State oscMapState)
        {
            if (cState.Battery != oscMapState.Battery)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + ind + "/battery", Convert.ToInt32(cState.Battery)));
                oscMapState.Battery = cState.Battery;
            }
            cState.Cross |= oscMapState.Cross;
            cState.Square |= oscMapState.Square;
            cState.Circle |= oscMapState.Circle;
            cState.Triangle |= oscMapState.Triangle;
            cState.R1 |= oscMapState.R1;
            cState.R3 |= oscMapState.R3;
            cState.L1 |= oscMapState.L1;
            cState.L3 |= oscMapState.L3;
            cState.DpadUp |= oscMapState.DpadUp;
            cState.DpadLeft |= oscMapState.DpadLeft;
            cState.DpadRight |= oscMapState.DpadRight;
            cState.DpadDown |= oscMapState.DpadDown;
            cState.Options |= oscMapState.Options;
            cState.Share |= oscMapState.Share;

            cState.LX = oscMapState.LX != 128 ? oscMapState.LX : cState.LX;
            cState.LY = oscMapState.LY != 128 ? oscMapState.LY : cState.LY;
            cState.L2 = oscMapState.L2 != 0 ? oscMapState.L2 : cState.L2;
            cState.RX = oscMapState.RX != 128 ? oscMapState.RX : cState.RX;
            cState.RY = oscMapState.RY != 128 ? oscMapState.RY : cState.RY;
            cState.R2 = oscMapState.R2 != 0 ? oscMapState.R2 : cState.R2;

            CompareAndSendChangesToOSC(ind, tempMapState, cState);
        }

        private void CompareAndSendChangesToOSC(int index, DS5State oldState, DS5State newState)
        {
            // Buttons 
            if (oldState.Square != newState.Square)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/square", newState.Square == true ? 1 : 0));
            }

            if (oldState.Triangle != newState.Triangle)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/triangle", newState.Triangle == true ? 1 : 0));
            }

            if (oldState.Circle != newState.Circle)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/circle", newState.Circle == true ? 1 : 0));
            }

            if (oldState.Cross != newState.Cross)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/cross", newState.Cross == true ? 1 : 0));
            }

            if (oldState.DpadUp != newState.DpadUp)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/dpadup", newState.DpadUp == true ? 1 : 0));
            }

            if (oldState.DpadDown != newState.DpadDown)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/dpaddown", newState.DpadDown == true ? 1 : 0));
            }

            if (oldState.DpadLeft != newState.DpadLeft)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/dpadleft", newState.DpadLeft == true ? 1 : 0));
            }

            if (oldState.DpadRight != newState.DpadRight)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/dpadright", newState.DpadRight == true ? 1 : 0));
            }

            if (oldState.L1 != newState.L1)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/l1", newState.L1 == true ? 1 : 0));
            }

            if (oldState.L3 != newState.L3)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/l3", newState.L3 == true ? 1 : 0));
            }

            if (oldState.R1 != newState.R1)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/r1", newState.R1 == true ? 1 : 0));
            }

            if (oldState.R3 != newState.R3)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/r3", newState.R3 == true ? 1 : 0));
            }

            if (oldState.Options != newState.Options)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/options", newState.Options == true ? 1 : 0));
            }

            if (oldState.Share != newState.Share)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/share", newState.Share == true ? 1 : 0));
            }

            if (oldState.PS != newState.PS)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/ps", newState.PS == true ? 1 : 0));
            }

            // Sticks
            if (oldState.LX != newState.LX)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/lx", Convert.ToInt32(newState.LX)));
            }

            if (oldState.LY != newState.LY)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/ly", Convert.ToInt32(newState.LY)));
            }

            if (oldState.RX != newState.RX)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/rx", Convert.ToInt32(newState.RX)));
            }

            if (oldState.RY != newState.RY)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/ry", Convert.ToInt32(newState.RY)));
            }

            // Triggers
            if (oldState.L2 != newState.L2)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/l2", Convert.ToInt32(newState.L2)));
            }

            if (oldState.R2 != newState.R2)
            {
                oscSender.Send(new OscMessage("/ds5control/monitor/" + index + "/r2", Convert.ToInt32(newState.R2)));
            }

            // if (oldState.Battery != newState.Battery)
            // {
            //     AppLogger.LogToGui("BATTERY " + oldState.Battery + " : " + newState.Battery, false);
            //     oscSender.Send(new SharpOSC.OscMessage("/ds5control/monitor/" + index + "/battery", Convert.ToInt32(newState.Battery)));
            // }
        }

        private void LagFlashWarning(DS5Device device, int ind, bool on)
        {
            if (on)
            {
                lag[ind] = true;
                LogDebug(string.Format(DS5ControlApp.Properties.Resources.LatencyOverTen, (ind + 1), device.Latency), true);
                if (getFlashWhenLate())
                {
                    DS5Color color = new DS5Color { red = 50, green = 0, blue = 0 };
                    DS5LightBar.forcedColor[ind] = color;
                    DS5LightBar.forcedFlash[ind] = 2;
                    DS5LightBar.forcelight[ind] = true;
                }
            }
            else
            {
                lag[ind] = false;
                LogDebug(DS5ControlApp.Properties.Resources.LatencyNotOverTen.Replace("*number*", (ind + 1).ToString()));
                DS5LightBar.forcelight[ind] = false;
                DS5LightBar.forcedFlash[ind] = 0;
                device.LightBarColor = getMainColor(ind);
            }
        }

        public DS5Controls GetActiveInputControl(int ind)
        {
            DS5State cState = CurrentState[ind];
            DS5StateExposed eState = ExposedState[ind];
            Mouse tp = touchPad[ind];
            DS5Controls result = DS5Controls.None;

            if (DS5Controllers[ind] != null)
            {
                if (Mapping.getBoolButtonMapping(cState.Cross))
                    result = DS5Controls.Cross;
                else if (Mapping.getBoolButtonMapping(cState.Circle))
                    result = DS5Controls.Circle;
                else if (Mapping.getBoolButtonMapping(cState.Triangle))
                    result = DS5Controls.Triangle;
                else if (Mapping.getBoolButtonMapping(cState.Square))
                    result = DS5Controls.Square;
                else if (Mapping.getBoolButtonMapping(cState.L1))
                    result = DS5Controls.L1;
                else if (Mapping.getBoolTriggerMapping(cState.L2))
                    result = DS5Controls.L2;
                else if (Mapping.getBoolButtonMapping(cState.L3))
                    result = DS5Controls.L3;
                else if (Mapping.getBoolButtonMapping(cState.R1))
                    result = DS5Controls.R1;
                else if (Mapping.getBoolTriggerMapping(cState.R2))
                    result = DS5Controls.R2;
                else if (Mapping.getBoolButtonMapping(cState.R3))
                    result = DS5Controls.R3;
                else if (Mapping.getBoolButtonMapping(cState.DpadUp))
                    result = DS5Controls.DpadUp;
                else if (Mapping.getBoolButtonMapping(cState.DpadDown))
                    result = DS5Controls.DpadDown;
                else if (Mapping.getBoolButtonMapping(cState.DpadLeft))
                    result = DS5Controls.DpadLeft;
                else if (Mapping.getBoolButtonMapping(cState.DpadRight))
                    result = DS5Controls.DpadRight;
                else if (Mapping.getBoolButtonMapping(cState.Share))
                    result = DS5Controls.Share;
                else if (Mapping.getBoolButtonMapping(cState.Options))
                    result = DS5Controls.Options;
                else if (Mapping.getBoolButtonMapping(cState.PS))
                    result = DS5Controls.PS;
                else if (Mapping.getBoolAxisDirMapping(cState.LX, true))
                    result = DS5Controls.LXPos;
                else if (Mapping.getBoolAxisDirMapping(cState.LX, false))
                    result = DS5Controls.LXNeg;
                else if (Mapping.getBoolAxisDirMapping(cState.LY, true))
                    result = DS5Controls.LYPos;
                else if (Mapping.getBoolAxisDirMapping(cState.LY, false))
                    result = DS5Controls.LYNeg;
                else if (Mapping.getBoolAxisDirMapping(cState.RX, true))
                    result = DS5Controls.RXPos;
                else if (Mapping.getBoolAxisDirMapping(cState.RX, false))
                    result = DS5Controls.RXNeg;
                else if (Mapping.getBoolAxisDirMapping(cState.RY, true))
                    result = DS5Controls.RYPos;
                else if (Mapping.getBoolAxisDirMapping(cState.RY, false))
                    result = DS5Controls.RYNeg;
                else if (Mapping.getBoolTouchMapping(tp.leftDown))
                    result = DS5Controls.TouchLeft;
                else if (Mapping.getBoolTouchMapping(tp.rightDown))
                    result = DS5Controls.TouchRight;
                else if (Mapping.getBoolTouchMapping(tp.multiDown))
                    result = DS5Controls.TouchMulti;
                else if (Mapping.getBoolTouchMapping(tp.upperDown))
                    result = DS5Controls.TouchUpper;
            }

            return result;
        }

        public bool[] touchreleased = new bool[MAX_DS5_CONTROLLER_COUNT] { true, true, true, true, true, true, true, true },
            touchslid = new bool[MAX_DS5_CONTROLLER_COUNT] { false, false, false, false, false, false, false, false };

        public Dispatcher EventDispatcher { get => eventDispatcher; }
        public OutputSlotManager OutputslotMan { get => outputslotMan; }

        protected void CheckForTouchToggle(int deviceID, DS5State cState, DS5State pState)
        {
            if (!IsUsingTouchpadForControls(deviceID) && cState.Touch1 && pState.PS)
            {
                if (GetTouchActive(deviceID) && touchreleased[deviceID])
                {
                    TouchActive[deviceID] = false;
                    LogDebug(DS5ControlApp.Properties.Resources.TouchpadMovementOff);
                    AppLogger.LogToTray(DS5ControlApp.Properties.Resources.TouchpadMovementOff);
                    touchreleased[deviceID] = false;
                }
                else if (touchreleased[deviceID])
                {
                    TouchActive[deviceID] = true;
                    LogDebug(DS5ControlApp.Properties.Resources.TouchpadMovementOn);
                    AppLogger.LogToTray(DS5ControlApp.Properties.Resources.TouchpadMovementOn);
                    touchreleased[deviceID] = false;
                }
            }
            else
                touchreleased[deviceID] = true;
        }

        public void StartTPOff(int deviceID)
        {
            if (deviceID < CURRENT_DS5_CONTROLLER_LIMIT)
            {
                TouchActive[deviceID] = false;
            }
        }

        public string TouchpadSlide(int ind)
        {
            DS5State cState = CurrentState[ind];
            string slidedir = "none";
            if (DS5Controllers[ind] != null && cState.Touch2 &&
               !(touchPad[ind].dragging || touchPad[ind].dragging2))
            {
                if (touchPad[ind].slideright && !touchslid[ind])
                {
                    slidedir = "right";
                    touchslid[ind] = true;
                }
                else if (touchPad[ind].slideleft && !touchslid[ind])
                {
                    slidedir = "left";
                    touchslid[ind] = true;
                }
                else if (!touchPad[ind].slideleft && !touchPad[ind].slideright)
                {
                    slidedir = "";
                    touchslid[ind] = false;
                }
            }

            return slidedir;
        }

        public void LogDebug(String Data, bool warning = false)
        {
            //Console.WriteLine(System.DateTime.Now.ToString("G") + "> " + Data);
            if (Debug != null)
            {
                DebugEventArgs args = new DebugEventArgs(Data, warning);
                OnDebug(this, args);
            }
        }

        public void OnDebug(object sender, DebugEventArgs args)
        {
            if (Debug != null)
                Debug(this, args);
        }

        // sets the rumble adjusted with rumble boost. General use method
        public void setRumble(byte heavyMotor, byte lightMotor, int deviceNum)
        {
            if (deviceNum < CURRENT_DS5_CONTROLLER_LIMIT)
            {
                DS5Device device = DS5Controllers[deviceNum];
                if (device != null)
                    SetDevRumble(device, heavyMotor, lightMotor, deviceNum);
                //device.setRumble((byte)lightBoosted, (byte)heavyBoosted);
            }
        }

        // sets the rumble adjusted with rumble boost. Method more used for
        // report handling. Avoid constant checking for a device.
        public void SetDevRumble(DS5Device device,
            byte heavyMotor, byte lightMotor, int deviceNum)
        {
            byte boost = getRumbleBoost(deviceNum);
            uint lightBoosted = ((uint)lightMotor * (uint)boost) / 100;
            if (lightBoosted > 255)
                lightBoosted = 255;
            uint heavyBoosted = ((uint)heavyMotor * (uint)boost) / 100;
            if (heavyBoosted > 255)
                heavyBoosted = 255;

            if (Global.InverseRumbleMotors[deviceNum])
                device.setRumble((byte)heavyBoosted, (byte)lightBoosted);
            else
                device.setRumble((byte)lightBoosted, (byte)heavyBoosted);
        }

        public DS5State getDS5State(int ind)
        {
            return CurrentState[ind];
        }

        public DS5State getDS5StateMapped(int ind)
        {
            return MappedState[ind];
        }

        public DS5State getDS5StateTemp(int ind)
        {
            return TempState[ind];
        }
    }
}
