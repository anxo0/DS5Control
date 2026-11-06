using System.IO;
using DS5ControlApp.Interfaz;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace DS5ControlTests;

[TestClass]
public class LeftoverCleanupTests
{
    private static string Folder()
    {
        string folder = Path.Combine(Path.GetTempPath(), "ds5test-" + Path.GetRandomFileName());
        Directory.CreateDirectory(folder);
        return folder;
    }

    private static void Write(string folder, params string[] names)
    {
        foreach (string name in names)
        {
            Directory.CreateDirectory(Path.GetDirectoryName(Path.Combine(folder, name))!);
            File.WriteAllText(Path.Combine(folder, name), "x");
        }
    }

    [TestMethod]
    public void RemovesOldLibrariesAndLanguages()
    {
        string folder = Folder();
        Write(folder, "MdXaml.dll", @"fr\DS5Control.resources.dll");

        Assert.AreEqual(2, LeftoverCleanup.Run(folder));
        Assert.IsFalse(File.Exists(Path.Combine(folder, "MdXaml.dll")));
        Assert.IsFalse(Directory.Exists(Path.Combine(folder, "fr")));
    }

    [TestMethod]
    public void KeepsWhatTheProgramStillUses()
    {
        string folder = Folder();
        Write(folder, "DS5Control.exe", "NLog.dll", @"es\DS5Control.resources.dll", @"ui\index.html");

        Assert.AreEqual(0, LeftoverCleanup.Run(folder));
        Assert.IsTrue(File.Exists(Path.Combine(folder, "NLog.dll")));
        Assert.IsTrue(File.Exists(Path.Combine(folder, @"es\DS5Control.resources.dll")));
    }

    [TestMethod]
    public void KeepsFoldersWithFilesOfTheUser()
    {
        string folder = Folder();
        Write(folder, @"fr\DS5Control.resources.dll", @"fr\mis notas.txt");

        Assert.AreEqual(0, LeftoverCleanup.Run(folder));
        Assert.IsTrue(File.Exists(Path.Combine(folder, @"fr\mis notas.txt")));
    }
}
