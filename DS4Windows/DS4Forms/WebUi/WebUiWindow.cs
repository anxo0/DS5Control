using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Shell;
using System.Windows.Threading;
using DS4Windows;
using DS4Windows.Api;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.Wpf;

namespace DS4WinWPF.DS4Forms.WebUi
{
    /// <summary>
    /// Hosts the web interface. It polls the engine from the UI thread only while it is visible,
    /// so hidden in the tray it costs nothing and the input threads never wait for it.
    /// </summary>
    public sealed class WebUiWindow : Window
    {
        private const string VirtualHost = "app.interfaz";
        private const string SettingsFile = "InterfazWeb.txt";
        private const double RainbowSeconds = 4.0;
        private const byte TestRumble = 200;
        private const double FrameThickness = 5;

        private readonly ControlService control;
        private readonly ControllerService service;
        private readonly WebView2 web = new WebView2();
        private readonly Border frame = new Border();
        private readonly DispatcherTimer poll = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(33) };
        private readonly DispatcherTimer save = new DispatcherTimer { Interval = TimeSpan.FromSeconds(1) };
        private readonly HashSet<int> unsavedSlots = new HashSet<int>();
        private readonly HashSet<string> heldButtons = new HashSet<string>();
        private bool pageReady;
        private bool openWithMute;
        private string lastState = string.Empty;

        /// <summary>Raised when WebView2 cannot start, so the old window can take over.</summary>
        public event EventHandler StartFailed;

        public WebUiWindow(ControlService control)
        {
            this.control = control;
            service = new ControllerService(control);
            openWithMute = ReadOpenWithMute();

            Title = "DS5Control";
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
                return;
            }

            if (kind == "paleta")
            {
                ApplyPalette(command.GetProperty("valor").GetString());
                return;
            }

            if (kind == "abrirConMute")
            {
                openWithMute = command.GetProperty("activo").GetBoolean();
                WriteOpenWithMute();
                return;
            }

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

        private void ApplyPalette(string palette)
        {
            string path = Path.Combine(AppContext.BaseDirectory, "iconos", $"ds5control-{palette}.ico");
            if (File.Exists(path)) Icon = BitmapFrame.Create(new Uri(path));
        }

        private void ChangeOutput(int slot, string output)
        {
            OutContType wanted = output == "ds4" ? OutContType.DS4 : OutContType.X360;
            if (Global.OutContType[slot] == wanted) return;

            Global.OutContType[slot] = wanted;
            string profile = Global.ProfilePath[slot];
            if (string.IsNullOrEmpty(profile)) return;

            // Reloading is how the old editor applies it: it replaces the virtual controller.
            SavePending();
            Global.SaveProfile(slot, profile);
            Global.LoadProfile(slot, false, control);
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
            DS4Device device = control.DS4Controllers[slot];
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
            WindowState = WindowState.Maximized;
        }

        /// <summary>Brings the window to the front with the input focus on the page.</summary>
        public void Reveal()
        {
            Show();
            if (WindowState == WindowState.Minimized) WindowState = WindowState.Normal;
            Activate();
            web.Focus();
            PostMessage(new { tipo = "mute" });
        }

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
            DS4State input = control.getDS4State(slot);
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
            var light = Global.LightbarSettingsInfo[slot].ds4winSettings;
            DS4Device device = control.DS4Controllers[slot];
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
                emulando = Global.activeOutDevType[slot] == OutContType.DS4 ? "DualShock 4" : "Xbox 360",
                luzActual = device != null
                    ? ToHex(new RgbColorDto(device.LightBarColor.red, device.LightBarColor.green, device.LightBarColor.blue))
                    : color,
                pulsados = PressedControls(slot),
                sticks = StickPositions(slot),
                gatillos = TriggerTravel(slot),
                ajustes = new
                {
                    luz = color,
                    brillo = settings.LightBrightness,
                    luzSegunBateria = light.ledAsBattery,
                    emulacion = Global.OutContType[slot] == OutContType.DS4 ? "ds4" : "xbox",
                    arcoiris = light.rainbow > 0,
                    vibracion = Math.Min(settings.RumbleStrength, 100),
                    gatilloL2 = ToEffectName(settings.L2Effect),
                    gatilloR2 = ToEffectName(settings.R2Effect),
                },
            };
        }

        private object StickPositions(int slot)
        {
            DS4State input = control.getDS4State(slot);
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
            DS4State input = control.getDS4State(slot);
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
            DS4State input = control.getDS4State(slot);
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
