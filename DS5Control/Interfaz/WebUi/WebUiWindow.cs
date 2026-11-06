using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Shell;
using System.Windows.Threading;
using DS5Control;
using DS5Control.Api;
using DS5ControlApp.Interfaz.ViewModels;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.Wpf;

namespace DS5ControlApp.Interfaz.WebUi
{
    /// <summary>
    /// Hosts the web interface. It polls the engine from the UI thread only while it is visible,
    /// so hidden in the tray it costs nothing and the input threads never wait for it.
    /// </summary>
    public sealed class WebUiWindow : Window
    {
        private const string VirtualHost = "app.interfaz";
        private const string SettingsFile = "InterfazWeb.txt";
        private const string PlacementFile = "Ventana.txt";
        private const double RainbowSeconds = 4.0;
        private const byte TestRumble = 200;
        private const double FrameThickness = 5;
        private const int MaxLogEntries = 150;

        /// <summary>Remappable controls, by their name in <see cref="DS5Controls"/>.</summary>
        private static readonly DS5Controls[] MappableControls =
        {
            DS5Controls.Cross, DS5Controls.Circle, DS5Controls.Square, DS5Controls.Triangle,
            DS5Controls.L1, DS5Controls.R1, DS5Controls.L2, DS5Controls.R2, DS5Controls.L3, DS5Controls.R3,
            DS5Controls.DpadUp, DS5Controls.DpadDown, DS5Controls.DpadLeft, DS5Controls.DpadRight,
            DS5Controls.Share, DS5Controls.Options, DS5Controls.PS,
        };

        private readonly ControlService control;
        private readonly AppHost host;
        private readonly AutoProfilesPanel autoProfiles;
        private readonly GeneralOptions general;
        private readonly ControllerService service;
        private readonly WebView2 web = new WebView2();
        private readonly Border frame = new Border();
        private readonly DispatcherTimer poll = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(33) };
        private readonly DispatcherTimer save = new DispatcherTimer { Interval = TimeSpan.FromSeconds(1) };
        private readonly DispatcherTimer placement = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(400) };
        private readonly HashSet<int> unsavedSlots = new HashSet<int>();
        private readonly HashSet<string> heldButtons = new HashSet<string>();
        private readonly List<object> logEntries = new List<object>();
        private readonly Dictionary<int, ProfileSettingsViewModel> profileViews = new Dictionary<int, ProfileSettingsViewModel>();
        private string[] profiles = Array.Empty<string>();
        private bool pageReady;
        private bool telemetry;
        private bool openWithMute;
        private string lastState = string.Empty;

        /// <summary>Raised when WebView2 cannot start.</summary>
        public event EventHandler StartFailed;

        /// <summary>Raised with the id of the logo palette the page uses.</summary>
        public event Action<string> PaletteChanged;

        /// <summary>Raised when the page changes between light and dark.</summary>
        public event Action<bool> ThemeChanged;

        public WebUiWindow(ControlService control, AppHost host)
        {
            this.control = control;
            this.host = host;
            autoProfiles = new AutoProfilesPanel(host.AutoProfiles);
            general = new GeneralOptions(host);
            host.OpenInterface = Reveal;
            host.Closed += (_, _) => SavePending();
            host.IsInterfaceActive = () => IsActive;
            PaletteChanged += host.SetAppIcon;
            ThemeChanged += host.SetTrayMenuTheme;
            service = new ControllerService(control);
            openWithMute = ReadOpenWithMute();
            RefreshProfiles();
            AppLogger.GuiLog += OnLog;

            Title = "DS5Control";
            WindowStartupLocation = WindowStartupLocation.CenterScreen;
            Width = 1320;
            Height = 820;
            MinWidth = 960;
            MinHeight = 640;
            frame.Child = web;
            frame.Background = new SolidColorBrush(Color.FromRgb(0xF2, 0xF4, 0xF7));
            Content = frame;

            // The page draws the title bar. The thin frame around it is what gives the mouse
            // the window edges to resize from, because WebView2 covers everything else.
            WindowChrome.SetWindowChrome(this, new WindowChrome
            {
                CaptionHeight = 0,
                ResizeBorderThickness = new Thickness(FrameThickness),
                GlassFrameThickness = new Thickness(0),
                UseAeroCaptionButtons = false,
            });
            StateChanged += (_, _) => UpdateFrame();
            UpdateFrame();
            RestorePlacement();
            placement.Tick += (_, _) => SavePlacement();
            LocationChanged += (_, _) => QueuePlacement();
            SizeChanged += (_, _) => QueuePlacement();

            poll.Tick += (_, _) => Poll();
            save.Tick += (_, _) => SavePending();
            service.MuteButtonPressed += (_, _) => Dispatcher.BeginInvoke(OnMutePressed);
            Loaded += async (_, _) => await StartAsync();
            IsVisibleChanged += (_, _) => UpdatePolling();
        }

        protected override void OnClosing(CancelEventArgs e)
        {
            // Closing sends the app to the background. It stays in the tray.
            e.Cancel = true;
            SavePending();
            Hide();
        }

        protected override void OnClosed(EventArgs e)
        {
            poll.Stop();
            save.Stop();
            AppLogger.GuiLog -= OnLog;
            service.Dispose();
            base.OnClosed(e);
        }

        private async System.Threading.Tasks.Task StartAsync()
        {
            try
            {
                string userData = Path.Combine(Global.appdatapath, "WebView2");
                CoreWebView2Environment environment =
                    await CoreWebView2Environment.CreateAsync(null, userData);
                await web.EnsureCoreWebView2Async(environment);

                CoreWebView2 core = web.CoreWebView2;
                core.Settings.AreDefaultContextMenusEnabled = false;
                core.Settings.IsStatusBarEnabled = false;
                core.Settings.IsNonClientRegionSupportEnabled = true;
                core.SetVirtualHostNameToFolderMapping(VirtualHost,
                    Path.Combine(AppContext.BaseDirectory, "ui"),
                    CoreWebView2HostResourceAccessKind.Deny);
                core.WebMessageReceived += OnWebMessage;
                core.Navigate($"https://{VirtualHost}/index.html");
            }
            catch (Exception exception)
            {
                AppLogger.LogToGui($"Web interface could not start: {exception.Message}", true);
                StartFailed?.Invoke(this, EventArgs.Empty);
            }
        }

        private void OnWebMessage(object sender, CoreWebView2WebMessageReceivedEventArgs e)
        {
            try
            {
                using JsonDocument document = JsonDocument.Parse(e.WebMessageAsJson);
                JsonElement root = document.RootElement;
                string kind = root.GetProperty("tipo").GetString();
                if (kind == "listo")
                {
                    pageReady = true;
                    lastState = string.Empty;
                    Poll();
                }
                else if (kind == "comando")
                {
                    RunCommand(root.GetProperty("comando"));
                    Poll();
                }
            }
            catch (Exception exception) when (exception is JsonException or KeyNotFoundException or InvalidOperationException)
            {
                AppLogger.LogToGui($"Invalid message from the web interface: {exception.Message}", true);
            }
        }

        private void RunCommand(JsonElement command)
        {
            string kind = command.GetProperty("tipo").GetString();
            if (kind == "ocultar")
            {
                Hide();
                return;
            }

            if (kind == "ventana")
            {
                if (command.GetProperty("accion").GetString() == "minimizar")
                {
                    WindowState = WindowState.Minimized;
                }
                else
                {
                    WindowState = WindowState == WindowState.Maximized
                        ? WindowState.Normal : WindowState.Maximized;
                }
                return;
            }

            if (kind == "tema")
            {
                bool dark = command.GetProperty("valor").GetString() == "oscuro";
                frame.Background = new SolidColorBrush(dark
                    ? Color.FromRgb(0x10, 0x11, 0x14) : Color.FromRgb(0xF2, 0xF4, 0xF7));
                ThemeChanged?.Invoke(dark);
                return;
            }

            if (kind == "paleta")
            {
                ApplyPalette(command.GetProperty("valor").GetString());
                return;
            }

            if (kind == "borrarPerfil")
            {
                DeleteProfile(command.GetProperty("nombre").GetString());
                return;
            }

            if (kind == "abrirConMute")
            {
                openWithMute = command.GetProperty("activo").GetBoolean();
                WriteOpenWithMute();
                return;
            }

            if (RunAppCommand(kind, command)) return;

            int slot = FindSlot(command.GetProperty("mandoId").GetString());
            if (slot < 0) return;

            switch (kind)
            {
                case "luz":
                    service.SetLightColor(slot, ParseColor(command.GetProperty("color").GetString()));
                    break;
                case "brillo":
                    service.SetLightBrightness(slot, command.GetProperty("valor").GetInt32());
                    break;
                case "luzSegunBateria":
                    service.SetLightByBattery(slot, command.GetProperty("activo").GetBoolean());
                    break;
                case "arcoiris":
                    service.SetLightRainbow(slot, command.GetProperty("activo").GetBoolean() ? RainbowSeconds : 0);
                    break;
                case "vibracion":
                    service.SetRumbleStrength(slot, command.GetProperty("fuerza").GetInt32());
                    break;
                case "emulacion":
                    ChangeOutput(slot, command.GetProperty("valor").GetString());
                    return;
                case "perfil":
                    UseProfile(slot, command.GetProperty("nombre").GetString());
                    return;
                case "nuevoPerfil":
                    CreateProfile(slot, command.GetProperty("nombre").GetString());
                    return;
                case "mapear":
                    JsonElement output = command.GetProperty("salida");
                    MapButton(slot, command.GetProperty("control").GetString(),
                        output.ValueKind == JsonValueKind.String ? output.GetString() : null);
                    break;
                case "pedirAcciones":
                    SendActions(slot);
                    return;
                case "guardarAccion":
                    ActionsPanel.Save(slot, command);
                    SendActions(slot);
                    break;
                case "borrarAccion":
                    ActionsPanel.Delete(slot, command.GetProperty("nombre").GetString());
                    SendActions(slot);
                    break;
                case "activarAccion":
                    ActionsPanel.SetEnabled(slot, command.GetProperty("nombre").GetString(),
                        command.GetProperty("activa").GetBoolean());
                    SendActions(slot);
                    break;
                case "elegirPrograma":
                    PickProgram();
                    return;
                case "mapearTecla":
                    MapKey(slot, command.GetProperty("control").GetString(), command.GetProperty("tecla").GetInt32());
                    break;
                case "mapearMacro":
                    MapMacro(slot, command.GetProperty("control").GetString(),
                        command.GetProperty("pasos").EnumerateArray().Select(item => item.GetInt32()).ToArray());
                    break;
                case "pedirOpciones":
                    SendOptions(slot);
                    return;
                case "opcion":
                    ProfileOptions.Apply(ProfileView(slot), slot, command.GetProperty("id").GetString(),
                        command.GetProperty("valor"));
                    Global.CacheProfileCustomsFlags(slot);
                    SendOptions(slot);
                    break;
                case "probarVibracion":
                    TestRumbleMotors(slot);
                    return;
                case "gatillo":
                    ControllerTrigger trigger = command.GetProperty("gatillo").GetString() == "l2"
                        ? ControllerTrigger.L2 : ControllerTrigger.R2;
                    service.SetTriggerEffect(slot, trigger, ToEffect(command.GetProperty("efecto").GetString()));
                    break;
                default:
                    return;
            }

            unsavedSlots.Add(slot);
            save.Stop();
            save.Start();
        }

        /// <summary>Commands that do not depend on any controller.</summary>
        private bool RunAppCommand(string kind, JsonElement command)
        {
            switch (kind)
            {
                case "telemetria":
                    telemetry = command.GetProperty("activo").GetBoolean();
                    return true;
                case "pedirDiagnostico":
                    SendDiagnostics();
                    return true;
                case "pedirGenerales":
                    SendGeneral();
                    return true;
                case "general":
                    general.Apply(command.GetProperty("id").GetString(), command.GetProperty("valor"));
                    Global.Save();
                    SendGeneral();
                    return true;
                case "pedirAuto":
                    SendAutoProfiles();
                    return true;
                case "autoAnadir":
                    string program = PickFile("Juegos|*.exe|Todos los archivos|*.*");
                    if (program != null) autoProfiles.Add(program, profiles.FirstOrDefault() ?? string.Empty);
                    SendAutoProfiles();
                    return true;
                case "autoPerfil":
                    autoProfiles.SetProfile(command.GetProperty("ruta").GetString(), command.GetProperty("perfil").GetString());
                    SendAutoProfiles();
                    return true;
                case "autoQuitar":
                    autoProfiles.Remove(command.GetProperty("ruta").GetString());
                    SendAutoProfiles();
                    return true;
                default:
                    return false;
            }
        }

        /// <summary>Live detail of a DualSense. It changes constantly, so only an open screen asks for it.</summary>
        private static object Telemetry(DS5Device device)
        {
            if (device is not DS5Control.InputDevices.DualSenseDevice sense) return null;

            DS5State state = device.getCurrentStateRef();
            SixAxis motion = state.Motion;
            var touches = new[] { state.TrackPadTouch0, state.TrackPadTouch1 }
                .Where(touch => touch.IsActive)
                .Select(touch => new { x = (int)touch.X, y = (int)touch.Y }).ToArray();

            return new
            {
                bateria = new { nivel = sense.BatteryStatus & 0x0F, estado = sense.BatteryStatus >> 4 },
                audio = new { cascos = (sense.AudioStatus & 1) != 0, micro = (sense.AudioStatus & 2) != 0 },
                giro = new { x = motion?.gyroYawFull ?? 0, y = motion?.gyroPitchFull ?? 0, z = motion?.gyroRollFull ?? 0 },
                acel = new { x = motion?.accelXG ?? 0, y = motion?.accelYG ?? 0, z = motion?.accelZG ?? 0 },
                toques = touches,
            };
        }

        /// <summary>The controller answers slowly, so this cannot hold up the window.</summary>
        private void SendDiagnostics()
        {
            DS5Device device = control.DS5Controllers.FirstOrDefault(item => item != null);
            if (device is not DS5Control.InputDevices.DualSenseDevice sense)
            {
                PostMessage(new { tipo = "diagnostico", datos = (object)null });
                return;
            }

            Task.Run(() =>
            {
                object result = new DS5Control.InputDevices.DualSenseDiagnostics(sense).Read();
                Dispatcher.BeginInvoke(() => PostMessage(new { tipo = "diagnostico", datos = result }));
            });
        }

        private void SendGeneral() => PostMessage(new { tipo = "generales", secciones = general.Describe() });

        private void SendAutoProfiles() => PostMessage(new { tipo = "autoPerfiles", lista = autoProfiles.Describe() });

        private string PickFile(string filter)
        {
            var dialog = new Microsoft.Win32.OpenFileDialog { Filter = filter };
            return dialog.ShowDialog(this) == true ? dialog.FileName : null;
        }

        private void ApplyPalette(string palette)
        {
            string path = Path.Combine(AppContext.BaseDirectory, "iconos", $"ds5control-{palette}.ico");
            if (!File.Exists(path)) return;

            Icon = BitmapFrame.Create(new Uri(path));
            PaletteChanged?.Invoke(palette);
            UpdateStartMenuIcon(path);
        }

        /// <summary>
        /// Windows shows the icon of the shortcut a program was opened from, so the shortcut
        /// has to follow the palette too.
        /// </summary>
        private static void UpdateStartMenuIcon(string icon)
        {
            string link = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.Programs), "DS5Control.lnk");
            if (!File.Exists(link)) return;

            try
            {
                dynamic shell = Activator.CreateInstance(Type.GetTypeFromProgID("WScript.Shell"));
                dynamic shortcut = shell.CreateShortcut(link);
                shortcut.IconLocation = $"{icon},0";
                shortcut.Save();
            }
            catch (Exception exception) when (exception is IOException or System.Runtime.InteropServices.COMException)
            {
                AppLogger.LogToGui($"Could not update the Start menu icon: {exception.Message}", true);
            }
        }

        private void ChangeOutput(int slot, string output)
        {
            // With no virtual controller and nothing written to the real one, a game that
            // supports the DualSense drives its triggers and rumble without interference.
            bool direct = output == "nativo";
            OutContType wanted = output == "ds4" ? OutContType.DS4 : OutContType.X360;
            if (direct == Global.DinputOnly[slot] && (direct || Global.OutContType[slot] == wanted)) return;

            Global.DinputOnly[slot] = direct;
            Global.EnableOutputDataToDS5[slot] = !direct;
            if (!direct) Global.OutContType[slot] = wanted;
            string profile = Global.ProfilePath[slot];
            if (string.IsNullOrEmpty(profile)) return;

            // Reloading is how the old editor applies it: it replaces the virtual controller.
            SavePending();
            Global.SaveProfile(slot, profile);
            Global.LoadProfile(slot, false, control);
        }

        private void SendActions(int slot)
        {
            PostMessage(new { tipo = "acciones", acciones = ActionsPanel.Describe(slot) });
        }

        private void PickProgram()
        {
            string path = PickFile("Programas|*.exe;*.bat;*.cmd;*.lnk|Todos los archivos|*.*");
            if (path != null) PostMessage(new { tipo = "archivo", ruta = path });
        }

        private static void MapKey(int slot, string controlName, int key)
        {
            if (!Enum.TryParse(controlName, out DS5Controls target) || !MappableControls.Contains(target)) return;

            Global.UpdateDS5CSetting(slot, controlName, false, key, "", DS5KeyType.None, 0);
            Global.CacheProfileCustomsFlags(slot);
        }

        private static void MapMacro(int slot, string controlName, int[] steps)
        {
            if (steps.Length == 0) return;
            if (!Enum.TryParse(controlName, out DS5Controls target) || !MappableControls.Contains(target)) return;

            Global.UpdateDS5CSetting(slot, controlName, false, steps, "", DS5KeyType.Macro, 0);
            Global.CacheProfileCustomsFlags(slot);
        }

        private ProfileSettingsViewModel ProfileView(int slot)
        {
            if (!profileViews.TryGetValue(slot, out ProfileSettingsViewModel view))
            {
                // Its constructor resets this value, which is not what a running controller wants.
                OutContType output = Global.outDevTypeTemp[slot];
                view = new ProfileSettingsViewModel(slot);
                Global.outDevTypeTemp[slot] = output;
                profileViews[slot] = view;
            }
            return view;
        }

        private void SendOptions(int slot)
        {
            PostMessage(new { tipo = "opciones", grupos = ProfileOptions.Describe(ProfileView(slot), slot) });
        }

        private void OnLog(object sender, DebugEventArgs e)
        {
            Dispatcher.BeginInvoke(() =>
            {
                logEntries.Add(new { hora = e.Time.ToString("HH:mm:ss"), texto = e.Data, aviso = e.Warning });
                if (logEntries.Count > MaxLogEntries) logEntries.RemoveAt(0);
            });
        }

        private static string ProfilesFolder => Path.Combine(Global.appdatapath, "Profiles");

        private static string ProfileFile(string name)
        {
            if (string.IsNullOrWhiteSpace(name) || name.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0)
            {
                return null;
            }
            return Path.Combine(ProfilesFolder, $"{name}.xml");
        }

        private void RefreshProfiles()
        {
            profiles = Directory.Exists(ProfilesFolder)
                ? Directory.GetFiles(ProfilesFolder, "*.xml").Select(Path.GetFileNameWithoutExtension)
                    .OrderBy(name => name, StringComparer.OrdinalIgnoreCase).ToArray()
                : Array.Empty<string>();
            host.Profiles.Refresh();
        }

        private void UseProfile(int slot, string name)
        {
            string file = ProfileFile(name);
            if (file == null || !File.Exists(file)) return;

            SavePending();
            Global.ProfilePath[slot] = name;
            Global.OlderProfilePath[slot] = name;
            Global.LoadProfile(slot, true, control);
            Global.Save();
        }

        private void CreateProfile(int slot, string name)
        {
            string file = ProfileFile(name);
            if (file == null || File.Exists(file)) return;

            SavePending();
            string current = ProfileFile(Global.ProfilePath[slot]);
            if (current == null || !File.Exists(current)) return;

            File.Copy(current, file);
            RefreshProfiles();
        }

        private void DeleteProfile(string name)
        {
            string file = ProfileFile(name);
            if (file == null || !File.Exists(file)) return;

            // A profile in use by any controller stays.
            if (service.GetControllers().Any(item => Global.ProfilePath[item.Slot] == name)) return;

            File.Delete(file);
            RefreshProfiles();
        }

        private static void MapButton(int slot, string controlName, string output)
        {
            if (!Enum.TryParse(controlName, out DS5Controls target) || !MappableControls.Contains(target)) return;

            if (output == null)
            {
                Global.GetDS5CSetting(slot, target).Reset();
            }
            else if (Enum.TryParse(output, out X360Controls action))
            {
                Global.UpdateDS5CSetting(slot, controlName, false, action, "", DS5KeyType.None, 0);
            }
            Global.CacheProfileCustomsFlags(slot);
        }

        private static object[] MappedButtons(int slot)
        {
            return MappableControls.Select(item =>
            {
                DS5ControlSettings setting = Global.GetDS5CSetting(slot, item);
                string output = setting.actionType switch
                {
                    DS5ControlSettings.ActionType.Button => setting.action.actionBtn.ToString(),
                    DS5ControlSettings.ActionType.Default => null,
                    DS5ControlSettings.ActionType.Key => $"Key:{setting.action.actionKey}",
                    DS5ControlSettings.ActionType.Macro => "Macro",
                    _ => "Personalizado",
                };
                return (object)new { control = item.ToString(), salida = output };
            }).ToArray();
        }

        private void UpdateFrame()
        {
            // A maximized window hangs past the screen edges by the size of its border.
            frame.Padding = WindowState == WindowState.Maximized
                ? SystemParameters.WindowResizeBorderThickness
                : new Thickness(FrameThickness);
        }

        private void TestRumbleMotors(int slot)
        {
            DS5Device device = control.DS5Controllers[slot];
            if (device == null) return;

            byte force = (byte)Math.Min(255, TestRumble * Global.RumbleBoost[slot] / 100);
            device.setRumble(force, force);
            var stop = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(600) };
            stop.Tick += (_, _) =>
            {
                stop.Stop();
                device.setRumble(0, 0);
            };
            stop.Start();
        }

        private void SavePending()
        {
            save.Stop();
            foreach (int slot in unsavedSlots)
            {
                service.SaveProfile(slot);
            }
            unsavedSlots.Clear();
        }

        private void UpdatePolling()
        {
            if (IsVisible) poll.Start();
            else poll.Stop();
            lastState = string.Empty;
        }

        private void OnMutePressed()
        {
            if (!openWithMute) return;

            if (IsVisible && IsActive)
            {
                Hide();
                return;
            }

            Reveal();
        }

        /// <summary>Brings the window to the front with the input focus on the page.</summary>
        public void Reveal()
        {
            Show();
            if (WindowState == WindowState.Minimized) WindowState = WindowState.Normal;
            BringToForeground();
            Activate();
            web.Focus();
            PostMessage(new { tipo = "mute" });
        }

        /// <summary>
        /// Windows stops a background program from taking the foreground and only flashes its
        /// taskbar button. Sharing input with the current foreground thread lifts that limit.
        /// </summary>
        private void BringToForeground()
        {
            IntPtr window = new WindowInteropHelper(this).EnsureHandle();
            IntPtr current = GetForegroundWindow();
            if (current == window) return;

            uint foregroundThread = GetWindowThreadProcessId(current, IntPtr.Zero);
            uint thisThread = GetCurrentThreadId();
            bool attached = foregroundThread != thisThread && AttachThreadInput(thisThread, foregroundThread, true);

            BringWindowToTop(window);
            SetForegroundWindow(window);

            if (attached) AttachThreadInput(thisThread, foregroundThread, false);
        }

        [DllImport("user32.dll")]
        private static extern IntPtr GetForegroundWindow();

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool SetForegroundWindow(IntPtr window);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool BringWindowToTop(IntPtr window);

        [DllImport("user32.dll")]
        private static extern uint GetWindowThreadProcessId(IntPtr window, IntPtr processId);

        [DllImport("kernel32.dll")]
        private static extern uint GetCurrentThreadId();

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool AttachThreadInput(uint attach, uint attachTo, [MarshalAs(UnmanagedType.Bool)] bool enable);

        private void Poll()
        {
            if (!pageReady) return;

            IReadOnlyList<ControllerInfoDto> controllers = service.GetControllers();
            string state = JsonSerializer.Serialize(new
            {
                tipo = "estado",
                estado = new
                {
                    mandos = controllers.Select(ToState).ToArray(),
                    abrirConMute = openWithMute,
                    ventanaVisible = IsVisible,
                    perfiles = profiles,
                    registro = logEntries,
                    servicios = new
                    {
                        vigemBus = Global.IsViGEmBusInstalled(),
                        hidHide = Global.hidHideInstalled,
                    },
                },
            });

            if (state != lastState)
            {
                lastState = state;
                web.CoreWebView2?.PostWebMessageAsJson(state);
            }

            if (IsActive && controllers.Count > 0)
            {
                Navigate(controllers[0].Slot);
            }
        }

        private void Navigate(int slot)
        {
            DS5State input = control.getDS5State(slot);
            if (input == null) return;

            NavigateOnPress("arriba", input.DpadUp);
            NavigateOnPress("abajo", input.DpadDown);
            NavigateOnPress("izquierda", input.DpadLeft);
            NavigateOnPress("derecha", input.DpadRight);
            NavigateOnPress("elegir", input.Cross);
            NavigateOnPress("atras", input.Circle);
            NavigateOnPress("pestanaAnterior", input.L1);
            NavigateOnPress("pestanaSiguiente", input.R1);
        }

        private void NavigateOnPress(string action, bool pressed)
        {
            if (!pressed)
            {
                heldButtons.Remove(action);
            }
            else if (heldButtons.Add(action))
            {
                PostMessage(new { tipo = "navegacion", accion = action });
            }
        }

        private void PostMessage(object message)
        {
            if (pageReady) web.CoreWebView2?.PostWebMessageAsJson(JsonSerializer.Serialize(message));
        }

        private object ToState(ControllerInfoDto controller)
        {
            int slot = controller.Slot;
            ControllerSettingsDto settings = controller.Settings;
            var light = Global.LightbarSettingsInfo[slot].ds5winSettings;
            DS5Device device = control.DS5Controllers[slot];
            string color = ToHex(settings.LightColor);

            return new
            {
                id = controller.Id,
                nombre = controller.Name,
                tipo = controller.Kind == ControllerKind.DualShock4 ? "dualshock4" : "dualsense",
                conexion = controller.Connection == ControllerConnection.Bluetooth ? "bluetooth" : "usb",
                conectado = true,
                bateria = controller.Battery.Available ? controller.Battery.Level : (int?)null,
                cargando = controller.Battery.Charging,
                emulando = Global.activeOutDevType[slot] switch
                {
                    OutContType.DS4 => "DualShock 4",
                    OutContType.X360 => "Xbox 360",
                    _ => null,
                },
                luzActual = device != null
                    ? ToHex(new RgbColorDto(device.LightBarColor.red, device.LightBarColor.green, device.LightBarColor.blue))
                    : color,
                pulsados = PressedControls(slot),
                datos = telemetry ? Telemetry(device) : null,
                perfil = Global.ProfilePath[slot],
                mapeo = MappedButtons(slot),
                sticks = StickPositions(slot),
                gatillos = TriggerTravel(slot),
                ajustes = new
                {
                    luz = color,
                    brillo = settings.LightBrightness,
                    luzSegunBateria = light.ledAsBattery,
                    emulacion = Global.DinputOnly[slot] ? "nativo" : Global.OutContType[slot] == OutContType.DS4 ? "ds4" : "xbox",
                    arcoiris = light.rainbow > 0,
                    vibracion = Math.Min(settings.RumbleStrength, 100),
                    gatilloL2 = ToEffectName(settings.L2Effect),
                    gatilloR2 = ToEffectName(settings.R2Effect),
                },
            };
        }

        private object StickPositions(int slot)
        {
            DS5State input = control.getDS5State(slot);
            if (input == null)
            {
                return new { izq = new { x = 0.0, y = 0.0 }, der = new { x = 0.0, y = 0.0 } };
            }

            return new
            {
                izq = new { x = ToAxis(input.LX), y = ToAxis(input.LY) },
                der = new { x = ToAxis(input.RX), y = ToAxis(input.RY) },
            };
        }

        private object TriggerTravel(int slot)
        {
            DS5State input = control.getDS5State(slot);
            return new
            {
                l2 = input == null ? 0.0 : Math.Round(input.L2 / 255.0, 2),
                r2 = input == null ? 0.0 : Math.Round(input.R2 / 255.0, 2),
            };
        }

        private static double ToAxis(byte value)
        {
            return Math.Round(Math.Clamp((value - 128) / 127.0, -1.0, 1.0), 2);
        }

        private string[] PressedControls(int slot)
        {
            DS5State input = control.getDS5State(slot);
            if (input == null) return Array.Empty<string>();

            var pressed = new List<string>();
            void Add(string id, bool value)
            {
                if (value) pressed.Add(id);
            }

            Add("cross", input.Cross);
            Add("circle", input.Circle);
            Add("rect", input.Square);
            Add("triangle", input.Triangle);
            Add("dpad-up", input.DpadUp);
            Add("dpad-down", input.DpadDown);
            Add("dpad-left", input.DpadLeft);
            Add("dpad-right", input.DpadRight);
            Add("l1", input.L1);
            Add("r1", input.R1);
            Add("l2", input.L2Btn);
            Add("r2", input.R2Btn);
            Add("l3", input.L3);
            Add("r3", input.R3);
            Add("create", input.Share);
            Add("options", input.Options);
            Add("ps", input.PS);
            Add("touchpad", input.TouchButton);
            Add("mute", input.Mute);
            return pressed.ToArray();
        }

        private int FindSlot(string id)
        {
            ControllerInfoDto match = service.GetControllers().FirstOrDefault(item => item.Id == id);
            return match?.Slot ?? -1;
        }

        private static RgbColorDto ParseColor(string hex)
        {
            int value = Convert.ToInt32(hex.TrimStart('#'), 16);
            return new RgbColorDto((byte)(value >> 16), (byte)(value >> 8), (byte)value);
        }

        private static string ToHex(RgbColorDto color)
        {
            return $"#{color.R:X2}{color.G:X2}{color.B:X2}";
        }

        private static TriggerEffectKind ToEffect(string name)
        {
            return name switch
            {
                "clicCompleto" => TriggerEffectKind.FullClick,
                "rigido" => TriggerEffectKind.Rigid,
                "pulso" => TriggerEffectKind.Pulse,
                _ => TriggerEffectKind.None,
            };
        }

        private static string ToEffectName(TriggerEffectKind effect)
        {
            return effect switch
            {
                TriggerEffectKind.FullClick => "clicCompleto",
                TriggerEffectKind.Rigid => "rigido",
                TriggerEffectKind.Pulse => "pulso",
                _ => "ninguno",
            };
        }

        private void QueuePlacement()
        {
            placement.Stop();
            placement.Start();
        }

        private void SavePlacement()
        {
            placement.Stop();
            if (!IsVisible || WindowState == WindowState.Minimized) return;

            Rect bounds = WindowState == WindowState.Maximized ? RestoreBounds : new Rect(Left, Top, Width, Height);
            string text = string.Join(";", new[] { bounds.X, bounds.Y, bounds.Width, bounds.Height }
                .Select(value => value.ToString(CultureInfo.InvariantCulture)));
            File.WriteAllText(Path.Combine(Global.appdatapath, PlacementFile),
                $"{text};{WindowState == WindowState.Maximized}");
        }

        private void RestorePlacement()
        {
            string path = Path.Combine(Global.appdatapath, PlacementFile);
            if (!File.Exists(path)) return;

            string[] parts = File.ReadAllText(path).Trim().Split(';');
            if (parts.Length != 5) return;

            double[] values = parts.Take(4)
                .Select(item => double.TryParse(item, NumberStyles.Float, CultureInfo.InvariantCulture, out double value) ? value : double.NaN)
                .ToArray();
            if (values.Any(double.IsNaN)) return;

            // A monitor that is no longer connected must not leave the window out of sight.
            var saved = new Rect(values[0], values[1], values[2], values[3]);
            var screen = new Rect(SystemParameters.VirtualScreenLeft, SystemParameters.VirtualScreenTop,
                SystemParameters.VirtualScreenWidth, SystemParameters.VirtualScreenHeight);
            if (!saved.IntersectsWith(screen)) return;

            WindowStartupLocation = WindowStartupLocation.Manual;
            Left = saved.X;
            Top = saved.Y;
            Width = Math.Max(MinWidth, saved.Width);
            Height = Math.Max(MinHeight, saved.Height);
            if (parts[4] == bool.TrueString) WindowState = WindowState.Maximized;
        }

        private static bool ReadOpenWithMute()
        {
            string path = Path.Combine(Global.appdatapath, SettingsFile);
            return !File.Exists(path) || File.ReadAllText(path).Trim() != "abrirConMute=false";
        }

        private void WriteOpenWithMute()
        {
            File.WriteAllText(Path.Combine(Global.appdatapath, SettingsFile),
                $"abrirConMute={(openWithMute ? "true" : "false")}");
        }
    }
}
