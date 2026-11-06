using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Threading.Tasks;
using DS5Control;

namespace DS5ControlApp.Interfaz
{
    /// <summary>
    /// Runs the ViGEmBus installer that ships with the program. Without that driver there is no
    /// virtual controller, so the games see nothing.
    /// </summary>
    public static class DriverSetup
    {
        private const string ViGEmInstaller = "ViGEmBus_1.22.0_x64_x86_arm64.exe";
        private const int UserCancelledElevation = 1223;

        public static string ViGEmInstallerPath => Path.Combine(AppContext.BaseDirectory, "drivers", ViGEmInstaller);

        /// <summary>Returns true once the driver is working.</summary>
        public static async Task<bool> InstallViGEmAsync()
        {
            if (!File.Exists(ViGEmInstallerPath))
            {
                AppLogger.LogToGui($"Falta el instalador de ViGEmBus en {ViGEmInstallerPath}", true);
                return false;
            }

            try
            {
                using Process installer = Process.Start(new ProcessStartInfo(ViGEmInstallerPath)
                {
                    UseShellExecute = true,
                    Verb = "runas",
                });
                await installer.WaitForExitAsync();
            }
            catch (Win32Exception exception) when (exception.NativeErrorCode == UserCancelledElevation)
            {
                AppLogger.LogToGui("Instalación de ViGEmBus cancelada.", true);
                return false;
            }

            Global.RefreshViGEmBusInfo();
            bool installed = Global.IsRunningSupportedViGEmBus();
            AppLogger.LogToGui(installed
                ? "ViGEmBus instalado."
                : "ViGEmBus no se ha instalado. Si lo acabas de instalar, reinicia el PC.", !installed);
            return installed;
        }
    }
}
