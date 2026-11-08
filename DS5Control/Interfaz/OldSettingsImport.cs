using System;
using System.IO;

namespace DS5ControlApp.Interfaz
{
    /// <summary>
    /// Brings the settings of a DS4Windows folder on the first start. It only copies: DS4Windows
    /// may still be installed and in use, so its folder is never moved or changed.
    /// </summary>
    public static class OldSettingsImport
    {
        /// <summary>Returns whether the settings were copied.</summary>
        public static bool Run(string oldFolder, string newFolder)
        {
            if (!Directory.Exists(oldFolder) || Directory.Exists(newFolder)) return false;

            // Copied aside first, so a failure halfway never leaves a partial folder that
            // would stop the next start from trying again.
            string partial = newFolder + ".copiando";
            try
            {
                if (Directory.Exists(partial)) Directory.Delete(partial, true);
                CopyFolder(oldFolder, partial);
                Directory.Move(partial, newFolder);
                return true;
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                try
                {
                    if (Directory.Exists(partial)) Directory.Delete(partial, true);
                }
                catch (Exception cleanup) when (cleanup is IOException or UnauthorizedAccessException)
                {
                }
                return false;
            }
        }

        private static void CopyFolder(string source, string destination)
        {
            Directory.CreateDirectory(destination);
            foreach (string file in Directory.GetFiles(source))
                File.Copy(file, Path.Combine(destination, Path.GetFileName(file)));
            foreach (string folder in Directory.GetDirectories(source))
                CopyFolder(folder, Path.Combine(destination, Path.GetFileName(folder)));
        }
    }
}
