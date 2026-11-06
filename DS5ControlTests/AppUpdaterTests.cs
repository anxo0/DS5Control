using System;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Net;
using System.Net.Http;
using System.Security.Cryptography;
using System.Threading.Tasks;
using DS5ControlApp.Interfaz;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace DS5ControlTests;

[TestClass]
public class AppUpdaterTests
{
    private static readonly Version Current = new Version(0, 1, 0);
    private const string Url = "https://github.com/anxo0/DS5Control/releases/download/v0.2.0/DS5Control-0.2.0-win-x64.zip";

    private static string Release(string tag = "v0.2.0", string url = Url, string digest = "\"digest\": \"sha256:abc\",")
    {
        return $$"""
            {"tag_name": "{{tag}}", "assets": [
                {"name": "notas.txt", "browser_download_url": "https://github.com/anxo0/DS5Control/releases/download/x/notas.txt"},
                {"name": "DS5Control-0.2.0-win-x64.zip", {{digest}} "browser_download_url": "{{url}}"}
            ]}
            """;
    }

    [TestMethod]
    public void FindsNewerRelease()
    {
        AppUpdate update = AppUpdater.ParseRelease(Release(), Current);

        Assert.AreEqual(new Version(0, 2, 0), update.Version);
        Assert.AreEqual(Url, update.DownloadUrl);
        Assert.AreEqual("abc", update.Sha256);
    }

    [DataTestMethod]
    [DataRow("v0.1.0")]
    [DataRow("0.1")]
    [DataRow("v0.0.9")]
    [DataRow("no-es-version")]
    public void IgnoresReleasesThatAreNotNewer(string tag)
    {
        Assert.IsNull(AppUpdater.ParseRelease(Release(tag), Current));
    }

    [TestMethod]
    public void RefusesDownloadsFromOtherPlaces()
    {
        Assert.IsNull(AppUpdater.ParseRelease(Release(url: "https://example.com/DS5Control-win-x64.zip"), Current));
    }

    [TestMethod]
    public void RefusesReleasesWithoutHash()
    {
        Assert.IsNull(AppUpdater.ParseRelease(Release(digest: string.Empty), Current));
    }

    private static string TempFolder()
    {
        string folder = Path.Combine(Path.GetTempPath(), "ds5test-" + Path.GetRandomFileName());
        Directory.CreateDirectory(folder);
        return folder;
    }

    private static byte[] ZipWith(string entry)
    {
        using var memory = new MemoryStream();
        using (var zip = new ZipArchive(memory, ZipArchiveMode.Create, true))
        {
            using StreamWriter writer = new StreamWriter(zip.CreateEntry(entry).Open());
            writer.Write("nuevo");
        }
        return memory.ToArray();
    }

    private static async Task<string> Download(byte[] content, string sha256)
    {
        int port;
        using (var free = new System.Net.Sockets.TcpListener(IPAddress.Loopback, 0))
        {
            free.Start();
            port = ((IPEndPoint)free.LocalEndpoint).Port;
            free.Stop();
        }

        using var server = new HttpListener();
        server.Prefixes.Add($"http://127.0.0.1:{port}/");
        server.Start();
        Task answer = server.GetContextAsync().ContinueWith(task =>
        {
            task.Result.Response.OutputStream.Write(content);
            task.Result.Response.Close();
        });

        using var client = new HttpClient();
        string folder = await AppUpdater.DownloadAsync(client,
            new AppUpdate(new Version(9, 0), $"http://127.0.0.1:{port}/a.zip", sha256), TempFolder());
        await answer;
        return folder;
    }

    [TestMethod]
    public async Task UnpacksVerifiedDownload()
    {
        byte[] zip = ZipWith("DS5Control-0.2.0/DS5Control.exe");

        string folder = await Download(zip, Convert.ToHexString(SHA256.HashData(zip)));

        Assert.IsTrue(File.Exists(Path.Combine(folder, AppUpdater.ProgramFile)));
    }

    [TestMethod]
    public async Task RejectsCorruptDownload()
    {
        await Assert.ThrowsExceptionAsync<InvalidDataException>(() => Download(ZipWith("DS5Control.exe"), "00"));
    }

    [TestMethod]
    public async Task RejectsReleaseWithoutProgram()
    {
        byte[] zip = ZipWith("otro.exe");

        await Assert.ThrowsExceptionAsync<InvalidDataException>(() => Download(zip, Convert.ToHexString(SHA256.HashData(zip))));
    }

    [TestMethod]
    public async Task InstallWaitsReplacesFilesAndOpensTheProgram()
    {
        string work = TempFolder();
        string newFiles = Directory.CreateDirectory(Path.Combine(work, "nueva")).FullName;
        string target = TempFolder();
        string opened = Path.Combine(target, "abierto.txt");

        File.WriteAllText(Path.Combine(target, "datos.txt"), "viejo");
        File.WriteAllText(Path.Combine(newFiles, "datos.txt"), "nuevo");
        File.WriteAllText(Path.Combine(newFiles, "lanzar.cmd"), $"@echo listo> \"{opened}\"");

        using Process running = Process.Start(new ProcessStartInfo("ping", "-n 3 127.0.0.1") { CreateNoWindow = true, UseShellExecute = false });
        using Process install = AppUpdater.StartInstall(newFiles, target, running.Id, "lanzar.cmd");

        Assert.IsFalse(install.WaitForExit(500), "It must wait for the program to exit");
        Assert.AreEqual("viejo", File.ReadAllText(Path.Combine(target, "datos.txt")));

        Assert.IsTrue(install.WaitForExit(30000));
        Assert.AreEqual("nuevo", File.ReadAllText(Path.Combine(target, "datos.txt")));

        var stopwatch = Stopwatch.StartNew();
        while (!File.Exists(opened) && stopwatch.ElapsedMilliseconds < 5000) await Task.Delay(50);
        Assert.IsTrue(File.Exists(opened), "The new program must be opened");
    }
}
