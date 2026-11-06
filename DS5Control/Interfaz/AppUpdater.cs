using System;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net.Http;
using System.Security.Cryptography;
using System.Text.Json;
using System.Threading.Tasks;

namespace DS5ControlApp.Interfaz
{
    /// <summary>A newer release that this installation can move to.</summary>
    public sealed record AppUpdate(Version Version, string DownloadUrl, string Sha256);

    /// <summary>
    /// Replaces the program with the latest GitHub release. The running program cannot overwrite
    /// its own files, so a small script waits for it to exit, copies the new files and opens it.
    /// </summary>
    public static class AppUpdater
    {
        public const string LatestReleaseUrl = "https://api.github.com/repos/anxo0/DS5Control/releases/latest";
        public const string ProgramFile = "DS5Control.exe";

        private const string AssetSuffix = "-win-x64.zip";
        private const string TrustedDownloads = "https://github.com/anxo0/DS5Control/releases/download/";
        private const string DigestPrefix = "sha256:";
        private const string WorkFolderName = "DS5Control-actualizacion";

        private const string InstallScript = """
            param([int]$Proceso, [string]$Origen, [string]$Destino, [string]$Programa)
            Wait-Process -Id $Proceso -Timeout 60 -ErrorAction SilentlyContinue
            for ($intento = 0; $intento -lt 10; $intento++) {
                robocopy $Origen $Destino /E /R:2 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null
                if ($LASTEXITCODE -lt 8) { break }
                Start-Sleep -Seconds 1
            }
            Start-Process -FilePath (Join-Path $Destino $Programa) -WorkingDirectory $Destino
            """;

        /// <summary>
        /// Reads a release from the GitHub API. Returns null unless it is newer, comes from this
        /// repository and states the hash to verify the download with.
        /// </summary>
        public static AppUpdate ParseRelease(string json, Version current)
        {
            using JsonDocument document = JsonDocument.Parse(json);
            JsonElement root = document.RootElement;
            if (!root.TryGetProperty("tag_name", out JsonElement tag)) return null;
            if (!Version.TryParse(tag.GetString()?.TrimStart('v').Split('-')[0], out Version version)) return null;
            if (Normalize(version) <= Normalize(current)) return null;
            if (!root.TryGetProperty("assets", out JsonElement assets)) return null;

            foreach (JsonElement asset in assets.EnumerateArray())
            {
                string name = asset.GetProperty("name").GetString() ?? string.Empty;
                if (!name.EndsWith(AssetSuffix, StringComparison.OrdinalIgnoreCase)) continue;

                string url = asset.GetProperty("browser_download_url").GetString() ?? string.Empty;
                string digest = asset.TryGetProperty("digest", out JsonElement value) ? value.GetString() : null;
                if (!url.StartsWith(TrustedDownloads, StringComparison.Ordinal)) return null;
                if (digest == null || !digest.StartsWith(DigestPrefix, StringComparison.Ordinal)) return null;

                return new AppUpdate(version, url, digest.Substring(DigestPrefix.Length));
            }
            return null;
        }

        public static async Task<AppUpdate> FindAsync(HttpClient client, Version current)
        {
            return ParseRelease(await client.GetStringAsync(LatestReleaseUrl), current);
        }

        /// <summary>Downloads and unpacks a release. Returns the folder that holds the new files.</summary>
        public static async Task<string> DownloadAsync(HttpClient client, AppUpdate update, string workFolder = null)
        {
            workFolder ??= Path.Combine(Path.GetTempPath(), WorkFolderName);
            if (Directory.Exists(workFolder)) Directory.Delete(workFolder, true);
            Directory.CreateDirectory(workFolder);

            string zip = Path.Combine(workFolder, "actualizacion.zip");
            await using (FileStream file = File.Create(zip))
            await using (Stream download = await client.GetStreamAsync(update.DownloadUrl))
            {
                await download.CopyToAsync(file);
            }

            await using (FileStream file = File.OpenRead(zip))
            {
                string hash = Convert.ToHexString(await SHA256.HashDataAsync(file));
                if (!hash.Equals(update.Sha256, StringComparison.OrdinalIgnoreCase))
                {
                    throw new InvalidDataException("La descarga no coincide con la huella de la release.");
                }
            }

            string files = Path.Combine(workFolder, "nueva");
            ZipFile.ExtractToDirectory(zip, files);
            return Directory.GetFiles(files, ProgramFile, SearchOption.AllDirectories)
                .Select(Path.GetDirectoryName).FirstOrDefault()
                ?? throw new InvalidDataException($"La release no contiene {ProgramFile}.");
        }

        public static bool CanWriteTo(string folder)
        {
            try
            {
                string probe = Path.Combine(folder, Path.GetRandomFileName());
                File.WriteAllBytes(probe, Array.Empty<byte>());
                File.Delete(probe);
                return true;
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                return false;
            }
        }

        /// <summary>Starts the script. The caller must exit for it to go on.</summary>
        public static Process StartInstall(string newFiles, string target, int processId, string program = ProgramFile)
        {
            string script = Path.Combine(Directory.GetParent(newFiles)!.FullName, "instalar.ps1");
            File.WriteAllText(script, InstallScript);

            var start = new ProcessStartInfo("powershell.exe")
            {
                CreateNoWindow = true,
                UseShellExecute = false,
            };
            foreach (string argument in new[]
            {
                "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", script,
                "-Proceso", processId.ToString(), "-Origen", newFiles, "-Destino", target, "-Programa", program,
            })
            {
                start.ArgumentList.Add(argument);
            }
            return Process.Start(start);
        }

        /// <summary>1.2 and 1.2.0 are the same version, but <see cref="Version"/> orders them apart.</summary>
        private static Version Normalize(Version version)
        {
            return new Version(version.Major, Math.Max(version.Minor, 0), Math.Max(version.Build, 0));
        }
    }
}
