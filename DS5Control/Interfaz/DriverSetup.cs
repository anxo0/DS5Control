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
        private const string HidHideInstaller = "HidHide_1.5.230_x64.exe";
        private const int RebootRequired = 3010;
        private const int UserCancelledElevation = 1223;

        public static string ViGEmInstallerPath => Path.Combine(AppContext.BaseDirectory, "drivers", ViGEmInstaller);
        public static string HidHideInstallerPath => Path.Combine(AppContext.BaseDirectory, "drivers", HidHideInstaller);

        /// <summary>
        /// Runs the HidHide installer that ships with the program, which hides the real controller
        /// from the games so they do not get every press twice. Returns true once it works.
        /// </summary>
        public static async Task<bool> InstallHidHideAsync()
        {
            if (!File.Exists(HidHideInstallerPath))
            {
                AppLogger.LogToGui($"Falta el instalador de HidHide en {HidHideInstallerPath}", true);
                return false;
            }

            int exitCode;
            try
            {
                using Process installer = Process.Start(new ProcessStartInfo(HidHideInstallerPath, "/passive /norestart")
                {
                    UseShellExecute = true,
                    Verb = "runas",
                });
                await installer.WaitForExitAsync();
                exitCode = installer.ExitCode;
            }
            catch (Win32Exception exception) when (exception.NativeErrorCode == UserCancelledElevation)
            {
                AppLogger.LogToGui("Instalación de HidHide cancelada.", true);
                return false;
            }

            Global.RefreshHidHideInfo();
            if (Global.hidHideInstalled) App.rootHub?.CheckHidHidePresence();
            if (exitCode == RebootRequired)
            {
                AppLogger.LogToGui("HidHide instalado. Reinicia el PC para que empiece a funcionar.", true);
            }
            else
            {
                AppLogger.LogToGui(Global.hidHideInstalled
                    ? "HidHide instalado. Apaga y enciende el mando para que los juegos dejen de verlo."
                    : "HidHide no se ha instalado.", !Global.hidHideInstalled);
            }
            return Global.hidHideInstalled;
        }

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
