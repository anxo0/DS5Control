using DS5ControlApp.Interfaz;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace DS5ControlTests;

[TestClass]
public class HidHideControlTests
{
    [TestMethod]
    public void TurnsTheDevicePathIntoItsInstance()
    {
        Assert.AreEqual(@"HID\VID_054C&PID_0CE6&MI_03\8&E01D75D&0&0000",
            HidHideControl.InstanceFromPath(@"\\?\hid#vid_054c&pid_0ce6&mi_03#8&e01d75d&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}"));
    }

    [TestMethod]
    public void IgnoresEmptyPaths()
    {
        Assert.IsNull(HidHideControl.InstanceFromPath(""));
        Assert.IsNull(HidHideControl.InstanceFromPath(null));
    }
}
