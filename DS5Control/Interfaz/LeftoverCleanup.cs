using System;
using System.IO;
using System.Linq;

namespace DS5ControlApp.Interfaz
{
    /// <summary>
    /// An update only adds and replaces files, so it would leave behind what newer versions no
    /// longer ship. Each entry is named, and a folder only goes if it holds nothing but
    /// translations: the portable version may share its folder with the user's own files.
    /// </summary>
    public static class LeftoverCleanup
    {
        private const string ResourceSuffix = ".resources.dll";

        private static readonly string[] OldFiles =
        {
            "DotNetProjects.Wpf.Extended.Toolkit.dll", "HttpProgress.dll", "ICSharpCode.AvalonEdit.dll",
            "MdXaml.Plugins.dll", "MdXaml.dll", "Ookii.Dialogs.Wpf.dll", "WPFLocalizeExtension.dll",
            "XAMLMarkupExtensions.dll",
            @"BezierCurveEditor\build.js", @"BezierCurveEditor\index.html",
        };

        private static readonly string[] OldLanguages =
        {
            "ar", "cs", "de", "el", "fi", "fr", "he", "hu-HU", "it", "ja", "ko", "ms", "nl", "pl", "pt",
            "pt-BR", "ru", "se", "tr", "uk-UA", "vi", "zh-CN", "zh-Hant", "zh-hans",
        };

        /// <summary>Returns how many files and folders were removed.</summary>
        public static int Run(string folder)
        {
            // These names are also files of DS4Windows itself: never clean a folder it lives in.
            if (File.Exists(Path.Combine(folder, "DS4Windows.exe"))) return 0;

            int removed = 0;
            foreach (string name in OldFiles)
            {
                string file = Path.Combine(folder, name);
                if (Delete(() => File.Delete(file), File.Exists(file))) removed++;
            }

            string editor = Path.Combine(folder, "BezierCurveEditor");
            if (Directory.Exists(editor) && !Directory.EnumerateFileSystemEntries(editor).Any()
                && Delete(() => Directory.Delete(editor), true))
            {
                removed++;
            }

            foreach (string name in OldLanguages)
            {
                string language = Path.Combine(folder, name);
                if (!Directory.Exists(language)) continue;
                if (!Directory.EnumerateFileSystemEntries(language).All(IsTranslation)) continue;
                if (Delete(() => Directory.Delete(language, true), true)) removed++;
            }
            return removed;
        }

        private static bool IsTranslation(string path)
        {
            return path.EndsWith(ResourceSuffix, StringComparison.OrdinalIgnoreCase) && File.Exists(path);
        }

        /// <summary>A file in use is not worth failing over: the next start tries again.</summary>
        private static bool Delete(Action delete, bool exists)
        {
            if (!exists) return false;
            try
            {
                delete();
                return true;
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                return false;
            }
        }
    }
}
