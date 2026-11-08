using DS5Control;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace DS5ControlTests;

[TestClass]
public class StickModeTests
{
    [TestMethod]
    public void ReadsTheSavedModes()
    {
        Assert.AreEqual(StickMode.Controls, Global.ParseStickMode("Controls"));
        Assert.AreEqual(StickMode.FlickStick, Global.ParseStickMode("FlickStick"));
        Assert.AreEqual(StickMode.None, Global.ParseStickMode("None"));
    }

    [TestMethod]
    public void UnknownValuesKeepTheStickWorking()
    {
        Assert.AreEqual(StickMode.Controls, Global.ParseStickMode("Flick Stick"));
        Assert.AreEqual(StickMode.Controls, Global.ParseStickMode("7"));
        Assert.AreEqual(StickMode.Controls, Global.ParseStickMode(""));
    }
}
