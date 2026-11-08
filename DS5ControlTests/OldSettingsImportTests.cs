using System.IO;
using DS5ControlApp.Interfaz;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace DS5ControlTests;

[TestClass]
public class OldSettingsImportTests
{
    private static string Folder()
    {
        string folder = Path.Combine(Path.GetTempPath(), "ds5test-" + Path.GetRandomFileName());
        Directory.CreateDirectory(folder);
        return folder;
    }

    [TestMethod]
    public void CopiesWithoutTouchingDS4Windows()
    {
        string root = Folder();
        string ds4 = Path.Combine(root, "DS4Windows");
        string ds5 = Path.Combine(root, "DS5Control");
        Directory.CreateDirectory(Path.Combine(ds4, "Profiles"));
        File.WriteAllText(Path.Combine(ds4, "Profiles.xml"), "ajustes");
        File.WriteAllText(Path.Combine(ds4, "Profiles", "Default.xml"), "perfil");

        Assert.IsTrue(OldSettingsImport.Run(ds4, ds5));

        Assert.AreEqual("perfil", File.ReadAllText(Path.Combine(ds5, "Profiles", "Default.xml")));
        Assert.AreEqual("ajustes", File.ReadAllText(Path.Combine(ds4, "Profiles.xml")));
        Assert.AreEqual("perfil", File.ReadAllText(Path.Combine(ds4, "Profiles", "Default.xml")));
        Assert.IsFalse(Directory.Exists(ds5 + ".copiando"));
    }

    [TestMethod]
    public void KeepsExistingSettings()
    {
        string root = Folder();
        string ds4 = Path.Combine(root, "DS4Windows");
        string ds5 = Path.Combine(root, "DS5Control");
        Directory.CreateDirectory(ds4);
        Directory.CreateDirectory(ds5);
        File.WriteAllText(Path.Combine(ds4, "Profiles.xml"), "de DS4Windows");
        File.WriteAllText(Path.Combine(ds5, "Profiles.xml"), "propios");

        Assert.IsFalse(OldSettingsImport.Run(ds4, ds5));
        Assert.AreEqual("propios", File.ReadAllText(Path.Combine(ds5, "Profiles.xml")));
    }
}
