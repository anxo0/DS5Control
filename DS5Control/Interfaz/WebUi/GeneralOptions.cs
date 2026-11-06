using System;
using System.Diagnostics;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using DS5Control;
using DS5ControlApp.Interfaz.ViewModels;

namespace DS5ControlApp.Interfaz.WebUi
{
    /// <summary>Application wide settings, in the same shape as <see cref="ProfileOptions"/>.</summary>
    internal sealed class GeneralOptions
    {
        private const string ViGEmPage = "https://github.com/nefarius/ViGEmBus/releases/latest";
        private const string HidHidePage = "https://github.com/nefarius/HidHide/releases/latest";

        private static readonly ProcessPriorityClass[] Priorities =
        {
            ProcessPriorityClass.Normal, ProcessPriorityClass.AboveNormal, ProcessPriorityClass.High,
        };

        private readonly SettingsViewModel settings = new SettingsViewModel();
        private readonly Item[][] sections;
        private readonly string[] names = { "Inicio", "Avisos", "Rendimiento", "Otros programas", "Herramientas" };

        public GeneralOptions(AppHost host)
        {
            sections = new[]
            {
                new[]
                {
                    Toggle("inicioWindows", "Abrir al iniciar Windows", () => settings.RunAtStartup, v => settings.RunAtStartup = v),
                    Toggle("inicioOculto", "Abrir en segundo plano", () => Global.StartMinimized, v => Global.StartMinimized = v),
                    Toggle("actualizaciones", "Buscar versiones nuevas", () => settings.CheckForUpdates, v => settings.CheckForUpdates = v),
                },
                new[]
                {
                    Options("notificaciones", "Notificaciones", new[] { "Ninguna", "Solo problemas", "Todas" },
                        () => settings.ShowNotificationsIndex, v => settings.ShowNotificationsIndex = v),
                },
                new[]
                {
                    Options("prioridad", "Prioridad del programa", new[] { "Normal", "Por encima de lo normal", "Alta" },
                        CurrentPriority, SetPriority),
                },
                new[]
                {
                    Toggle("exclusivo", "Ocultar el mando real a los juegos", () => settings.HideDS5Controller, v =>
                    {
                        settings.HideDS5Controller = v;
                        host.RestartService();
                    }),
                    Toggle("gestos", "Cambiar de perfil deslizando en el panel", () => Global.SwipeProfiles, v =>
                    {
                        Global.SwipeProfiles = v;
                        host.SetHotkeysEnabled(v);
                    }),
                    Toggle("osc", "Servidor OSC", () => settings.UseOSCServer, v =>
                    {
                        settings.UseOSCServer = v;
                        App.rootHub.ChangeOSCListenerStatus(v);
                    }),
                    Toggle("udp", "Servidor de movimiento (emuladores)", () => settings.UseUDPServer, v =>
                    {
                        settings.UseUDPServer = v;
                        ChangeUdp(v);
                    }),
                },
                new[]
                {
                    Button("reiniciar", "Reiniciar el motor", host.RestartService),
                    Button("carpeta", "Abrir carpeta de perfiles", () => Util.StartProcessInExplorer(Global.appdatapath)),
                    Button("buscarVersion", "Buscar versión nueva", () =>
                    {
                        if (!AppHost.CheckForUpdates()) AppLogger.LogToGui("Tienes la última versión.", false);
                        else AppHost.OpenReleases();
                    }),
                    Button("vigem", "Descargar ViGEmBus", () => Util.StartProcessHelper(ViGEmPage)),
                    Button("hidhide", "Descargar HidHide", () => Util.StartProcessHelper(HidHidePage)),
                },
            };
        }

        public object Describe()
        {
            return sections.Select((items, index) => new
            {
                nombre = names[index],
                ajustes = items.Select(item => new
                {
                    id = item.Id,
                    nombre = item.Name,
                    tipo = item.Kind,
                    min = 0,
                    max = 0,
                    paso = 0,
                    porcentaje = false,
                    opciones = item.Choices,
                    valor = item.Read?.Invoke(),
                }).ToArray(),
            }).ToArray();
        }

        public void Apply(string id, JsonElement value)
        {
            sections.SelectMany(items => items).FirstOrDefault(item => item.Id == id)?.Write(value);
        }

        private static int CurrentPriority()
        {
            using Process process = Process.GetCurrentProcess();
            return Math.Max(0, Array.IndexOf(Priorities, process.PriorityClass));
        }

        private static void SetPriority(int index)
        {
            using Process process = Process.GetCurrentProcess();
            process.PriorityClass = Priorities[index];
        }

        private static async void ChangeUdp(bool enabled)
        {
            if (!enabled) App.rootHub.ChangeMotionEventStatus(false);
            else App.rootHub.ChangeUDPStatus(true);

            await Task.Delay(200);

            if (!enabled) App.rootHub.ChangeUDPStatus(false);
            else App.rootHub.ChangeMotionEventStatus(true);
        }

        private static Item Toggle(string id, string name, Func<bool> read, Action<bool> write)
        {
            return new Item(id, name, "interruptor", null, () => read(), value => write(value.GetBoolean()));
        }

        private static Item Options(string id, string name, string[] choices, Func<int> read, Action<int> write)
        {
            return new Item(id, name, "opciones", choices, () => read(), value => write(value.GetInt32()));
        }

        private static Item Button(string id, string name, Action run)
        {
            return new Item(id, name, "accion", null, null, _ => run());
        }

        private sealed record Item(string Id, string Name, string Kind, string[] Choices, Func<object> Read, Action<JsonElement> Write);
    }
}
