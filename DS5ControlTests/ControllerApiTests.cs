using System.Diagnostics;
using System.Text.Json;
using DS5Control;
using DS5Control.Api;
using DS5Control.InputDevices;

namespace DS5ControlTests;

[TestClass]
public class ButtonEdgeDetectorTests
{
    [TestMethod]
    public void ReportsOnlyTheFirstPressedReport()
    {
        var detector = new ButtonEdgeDetector();

        Assert.IsFalse(detector.Update(false));
        Assert.IsTrue(detector.Update(true));
        Assert.IsFalse(detector.Update(true));
        Assert.IsFalse(detector.Update(false));
        Assert.IsTrue(detector.Update(true));
    }

    [TestMethod]
    public void ResetAllowsANewPressWhileHeld()
    {
        var detector = new ButtonEdgeDetector();
        detector.Update(true);

        detector.Reset();

        Assert.IsTrue(detector.Update(true));
    }

    [TestMethod]
    public void WorksInPlaceInsideAnArray()
    {
        // The engine keeps one detector per slot in an array.
        var detectors = new ButtonEdgeDetector[2];

        Assert.IsTrue(detectors[1].Update(true));
        Assert.IsFalse(detectors[1].Update(true));
        Assert.IsTrue(detectors[0].Update(true));
    }
}

[TestClass]
public class LightLevelTests
{
    private static void AssertColor(byte r, byte g, byte b, DS5Color actual)
    {
        Assert.AreEqual(new DS5Color(r, g, b), actual);
    }

    [TestMethod]
    public void SplitsTheStoredColorIntoColorAndBrightness()
    {
        var level = new LightLevel();

        level.Sync(new DS5Color(128, 0, 64));

        Assert.AreEqual(50, level.Brightness);
        AssertColor(255, 0, 128, level.BaseColor);
    }

    [TestMethod]
    public void BrightnessScalesAllChannels()
    {
        var level = new LightLevel();
        level.Sync(new DS5Color(255, 100, 0));

        AssertColor(128, 50, 0, level.SetBrightness(50));
    }

    [TestMethod]
    public void BrightnessCanGoThroughZeroWithoutLosingTheColor()
    {
        var level = new LightLevel();
        level.Sync(new DS5Color(255, 40, 7));

        AssertColor(0, 0, 0, level.SetBrightness(0));
        level.Sync(new DS5Color(0, 0, 0));

        AssertColor(255, 40, 7, level.SetBrightness(100));
    }

    [TestMethod]
    public void LowBrightnessDoesNotShiftTheHue()
    {
        var level = new LightLevel();
        level.Sync(new DS5Color(255, 40, 0));

        DS5Color dim = level.SetBrightness(1);
        level.Sync(dim);

        AssertColor(255, 40, 0, level.SetBrightness(100));
    }

    [TestMethod]
    public void ColorKeepsTheCurrentBrightness()
    {
        var level = new LightLevel();
        level.Sync(new DS5Color(255, 255, 255));
        level.SetBrightness(50);

        AssertColor(0, 128, 0, level.SetColor(new DS5Color(0, 255, 0)));
    }

    [TestMethod]
    public void AColorChangedElsewhereReplacesTheRememberedOne()
    {
        var level = new LightLevel();
        level.Sync(new DS5Color(255, 0, 0));
        level.SetBrightness(50);

        level.Sync(new DS5Color(0, 0, 255));

        Assert.AreEqual(100, level.Brightness);
        AssertColor(0, 0, 255, level.BaseColor);
    }

    [TestMethod]
    public void BlackProfileColorReadsAsZeroBrightness()
    {
        var level = new LightLevel();

        level.Sync(new DS5Color(0, 0, 0));

        Assert.AreEqual(0, level.Brightness);
    }
}

[TestClass]
public class ControllerMappingTests
{
    [TestMethod]
    public void EveryEngineDeviceTypeHasAKind()
    {
        foreach (InputDeviceType type in Enum.GetValues<InputDeviceType>())
        {
            Assert.AreNotEqual(ControllerKind.Unknown, ControllerMapping.ToKind(type), type.ToString());
        }
    }

    [TestMethod]
    public void ConnectionTypes()
    {
        Assert.AreEqual(ControllerConnection.Usb, ControllerMapping.ToConnection(ConnectionType.USB));
        Assert.AreEqual(ControllerConnection.Bluetooth, ControllerMapping.ToConnection(ConnectionType.BT));
        Assert.AreEqual(ControllerConnection.WirelessAdapter, ControllerMapping.ToConnection(ConnectionType.SONYWA));
    }

    [TestMethod]
    public void TriggerEffectsRoundTrip()
    {
        foreach (TriggerEffectKind kind in Enum.GetValues<TriggerEffectKind>())
        {
            Assert.IsTrue(ControllerMapping.TryToEngineEffect(kind, out TriggerEffects engine));
            Assert.AreEqual(kind, ControllerMapping.ToEffectKind(engine));
        }
    }

    [TestMethod]
    public void EveryEngineTriggerEffectIsExposed()
    {
        foreach (TriggerEffects effect in Enum.GetValues<TriggerEffects>())
        {
            TriggerEffectKind kind = ControllerMapping.ToEffectKind(effect);
            ControllerMapping.TryToEngineEffect(kind, out TriggerEffects back);
            Assert.AreEqual(effect, back);
        }
    }

    [TestMethod]
    public void UnknownEffectIsRejected()
    {
        Assert.IsFalse(ControllerMapping.TryToEngineEffect((TriggerEffectKind)99, out _));
    }

    [TestMethod]
    public void FeaturesPerModel()
    {
        Assert.AreEqual(new ControllerFeaturesDto(true, true, true, true),
            ControllerMapping.ToFeatures(ControllerKind.DualSense, canOutput: true));
        Assert.AreEqual(new ControllerFeaturesDto(true, true, false, false),
            ControllerMapping.ToFeatures(ControllerKind.DualShock4, canOutput: true));
        Assert.AreEqual(new ControllerFeaturesDto(false, true, false, false),
            ControllerMapping.ToFeatures(ControllerKind.DualShock3, canOutput: true));
    }

    [TestMethod]
    public void NoOutputMeansNoOutputFeatures()
    {
        Assert.AreEqual(new ControllerFeaturesDto(false, false, false, true),
            ControllerMapping.ToFeatures(ControllerKind.DualSense, canOutput: false));
    }
}

[TestClass]
public class ApiEventQueueTests
{
    private static readonly TimeSpan Timeout = TimeSpan.FromSeconds(5);

    [TestMethod]
    public void RunsInOrderOnAnotherThread()
    {
        using var queue = new ApiEventQueue();
        var seen = new List<int>();
        int threadId = 0;
        var done = new ManualResetEventSlim();

        for (int i = 0; i < 100; i++)
        {
            int value = i;
            queue.Post(() => seen.Add(value));
        }

        queue.Post(() =>
        {
            threadId = Environment.CurrentManagedThreadId;
            done.Set();
        });

        Assert.IsTrue(done.Wait(Timeout));
        CollectionAssert.AreEqual(Enumerable.Range(0, 100).ToList(), seen);
        Assert.AreNotEqual(Environment.CurrentManagedThreadId, threadId);
    }

    [TestMethod]
    public void AFailingActionDoesNotStopTheQueue()
    {
        using var queue = new ApiEventQueue();
        var done = new ManualResetEventSlim();

        queue.Post(() => throw new InvalidOperationException("test"));
        queue.Post(done.Set);

        Assert.IsTrue(done.Wait(Timeout));
    }

    [TestMethod]
    public void PostingAfterDisposeIsIgnored()
    {
        var queue = new ApiEventQueue();
        queue.Dispose();

        queue.Post(() => Assert.Fail("Should not run"));
    }

    [TestMethod]
    public void PostingDoesNotWaitForASlowAction()
    {
        using var queue = new ApiEventQueue();
        var release = new ManualResetEventSlim();

        queue.Post(release.Wait);
        var watch = Stopwatch.StartNew();
        queue.Post(() => { });
        watch.Stop();
        release.Set();

        Assert.IsTrue(watch.Elapsed < TimeSpan.FromSeconds(1));
    }
}

[TestClass]
public class ControllerDtoJsonTests
{
    private static ControllerInfoDto Sample() => new(
        0, "AA:BB", "DualSense", ControllerKind.DualSense, ControllerConnection.Bluetooth,
        new ControllerBatteryDto(0, "AA:BB", 80, true, true),
        new ControllerFeaturesDto(true, true, true, true),
        new ControllerSettingsDto(new RgbColorDto(1, 2, 3), 50, 100,
            TriggerEffectKind.FullClick, TriggerEffectKind.None));

    [TestMethod]
    public void UsesCamelCaseAndTextEnums()
    {
        string json = JsonSerializer.Serialize(Sample(), ControllerApiJson.Options);

        using JsonDocument document = JsonDocument.Parse(json);
        JsonElement root = document.RootElement;
        Assert.AreEqual("dualSense", root.GetProperty("kind").GetString());
        Assert.AreEqual("bluetooth", root.GetProperty("connection").GetString());
        Assert.AreEqual(80, root.GetProperty("battery").GetProperty("level").GetInt32());
        Assert.AreEqual("fullClick", root.GetProperty("settings").GetProperty("l2Effect").GetString());
        Assert.AreEqual(3, root.GetProperty("settings").GetProperty("lightColor").GetProperty("b").GetInt32());
    }

    [TestMethod]
    public void RoundTrips()
    {
        ControllerInfoDto original = Sample();

        string json = JsonSerializer.Serialize(original, ControllerApiJson.Options);
        var parsed = JsonSerializer.Deserialize<ControllerInfoDto>(json, ControllerApiJson.Options);

        Assert.AreEqual(original.Settings, parsed.Settings);
        Assert.AreEqual(original.Battery, parsed.Battery);
        Assert.AreEqual(original.Features, parsed.Features);
        Assert.AreEqual(original.Kind, parsed.Kind);
    }
}

[TestClass]
public class ControllerServiceTests
{
    private const int Slot = 3;
    private static readonly TimeSpan Timeout = TimeSpan.FromSeconds(5);

    private sealed class FakeSource : IControllerSource
    {
        public readonly List<ControllerSnapshot> Controllers = [];
        public bool Disposed;

        public event Action<ControllerSnapshot> Connected;
        public event Action<ControllerSnapshot> Disconnected;
        public event Action<ControllerSnapshot> BatteryChanged;
        public event Action<ControllerSnapshot> MutePressed;

        public IReadOnlyList<ControllerSnapshot> GetControllers() => Controllers;
        public ControllerSnapshot Find(int slot) => Controllers.FirstOrDefault(c => c.Slot == slot);

        public void RaiseConnected(ControllerSnapshot c) => Connected?.Invoke(c);
        public void RaiseDisconnected(ControllerSnapshot c) => Disconnected?.Invoke(c);
        public void RaiseBattery(ControllerSnapshot c) => BatteryChanged?.Invoke(c);
        public void RaiseMute(ControllerSnapshot c) => MutePressed?.Invoke(c);
        public void Dispose() => Disposed = true;
    }

    private static ControllerSnapshot Controller(ControllerKind kind = ControllerKind.DualSense,
        bool canOutput = true, int battery = 60) =>
        new(Slot, "AA:BB", "Pad", kind, ControllerConnection.Bluetooth, battery, false, true, canOutput);

    private FakeSource source;
    private ControllerService service;
    private DS5Color savedColor;
    private byte savedRumble;

    [TestInitialize]
    public void Setup()
    {
        source = new FakeSource();
        service = new ControllerService(source);

        LightbarSettingInfo light = Global.LightbarSettingsInfo[Slot];
        savedColor = light.ds5winSettings.m_Led;
        savedRumble = Global.RumbleBoost[Slot];
        light.ds5winSettings.m_Led = new DS5Color(255, 0, 0);
    }

    [TestCleanup]
    public void Cleanup()
    {
        service.Dispose();

        LightbarSettingInfo light = Global.LightbarSettingsInfo[Slot];
        light.ds5winSettings.m_Led = savedColor;
        light.ds5winSettings.rainbow = 0;
        light.ds5winSettings.useCustomLed = false;
        light.Mode = LightbarSettingInfo.DEFAULT_MODE;
        Global.RumbleBoost[Slot] = savedRumble;
        Global.L2OutputSettings[Slot].TriggerEffect = TriggerEffects.None;
        Global.R2OutputSettings[Slot].TriggerEffect = TriggerEffects.None;
    }

    private static DS5Color StoredColor => Global.LightbarSettingsInfo[Slot].ds5winSettings.m_Led;

    [TestMethod]
    public void CommandsWithoutAControllerAreRefused()
    {
        Assert.AreEqual(CommandResult.NoController, service.SetLightColor(Slot, new RgbColorDto(1, 2, 3)));
        Assert.AreEqual(CommandResult.NoController, service.SetLightBrightness(Slot, 50));
        Assert.AreEqual(CommandResult.NoController, service.SetRumbleStrength(Slot, 50));
        Assert.AreEqual(CommandResult.NoController,
            service.SetTriggerEffect(Slot, ControllerTrigger.L2, TriggerEffectKind.Rigid));
        Assert.AreEqual(CommandResult.NoController, service.SaveProfile(Slot));
    }

    [TestMethod]
    public void ValuesOutOfRangeAreRefused()
    {
        source.Controllers.Add(Controller());

        Assert.AreEqual(CommandResult.InvalidValue, service.SetLightColor(Slot, null));
        Assert.AreEqual(CommandResult.InvalidValue, service.SetLightBrightness(Slot, -1));
        Assert.AreEqual(CommandResult.InvalidValue, service.SetLightBrightness(Slot, 101));
        Assert.AreEqual(CommandResult.InvalidValue, service.SetRumbleStrength(Slot, -1));
        Assert.AreEqual(CommandResult.InvalidValue, service.SetRumbleStrength(Slot, 201));
        Assert.AreEqual(CommandResult.InvalidValue,
            service.SetTriggerEffect(Slot, ControllerTrigger.L2, (TriggerEffectKind)99));
        Assert.AreEqual(CommandResult.InvalidValue,
            service.SetTriggerEffect(Slot, (ControllerTrigger)99, TriggerEffectKind.Rigid));
    }

    [TestMethod]
    public void LightColorReachesTheProfileAndOverridesOtherLightModes()
    {
        source.Controllers.Add(Controller());
        LightbarSettingInfo light = Global.LightbarSettingsInfo[Slot];
        light.Mode = LightbarMode.Passthru;
        light.ds5winSettings.rainbow = 5;
        light.ds5winSettings.useCustomLed = true;

        Assert.AreEqual(CommandResult.Ok, service.SetLightColor(Slot, new RgbColorDto(10, 20, 30)));

        Assert.AreEqual(new DS5Color(10, 20, 30), StoredColor);
        Assert.AreEqual(LightbarMode.DS5Win, light.Mode);
        Assert.AreEqual(0, light.ds5winSettings.rainbow);
        Assert.IsFalse(light.ds5winSettings.useCustomLed);
    }

    [TestMethod]
    public void BrightnessIsAppliedAndReadBack()
    {
        source.Controllers.Add(Controller());

        Assert.AreEqual(CommandResult.Ok, service.SetLightBrightness(Slot, 50));

        Assert.AreEqual(new DS5Color(128, 0, 0), StoredColor);
        ControllerSettingsDto settings = service.GetControllers().Single().Settings;
        Assert.AreEqual(50, settings.LightBrightness);
        Assert.AreEqual(new RgbColorDto(255, 0, 0), settings.LightColor);
    }

    [TestMethod]
    public void LightOnAControllerWithoutLightbarIsNotSupported()
    {
        source.Controllers.Add(Controller(ControllerKind.DualShock3));

        Assert.AreEqual(CommandResult.NotSupported, service.SetLightColor(Slot, new RgbColorDto(1, 2, 3)));
        Assert.AreEqual(CommandResult.NotSupported, service.SetLightBrightness(Slot, 50));
        Assert.AreEqual(new DS5Color(255, 0, 0), StoredColor);
    }

    [TestMethod]
    public void RumbleReachesTheProfile()
    {
        source.Controllers.Add(Controller());

        Assert.AreEqual(CommandResult.Ok, service.SetRumbleStrength(Slot, 150));

        Assert.AreEqual((byte)150, Global.RumbleBoost[Slot]);
        Assert.AreEqual(150, service.GetControllers().Single().Settings.RumbleStrength);
    }

    [TestMethod]
    public void TriggerEffectReachesTheRightTrigger()
    {
        source.Controllers.Add(Controller());

        Assert.AreEqual(CommandResult.Ok, service.SetTriggerEffect(Slot, ControllerTrigger.L2, TriggerEffectKind.Pulse));
        Assert.AreEqual(CommandResult.Ok, service.SetTriggerEffect(Slot, ControllerTrigger.R2, TriggerEffectKind.FullClick));

        Assert.AreEqual(TriggerEffects.Pulse, Global.L2OutputSettings[Slot].TriggerEffect);
        Assert.AreEqual(TriggerEffects.FullClick, Global.R2OutputSettings[Slot].TriggerEffect);
        ControllerSettingsDto settings = service.GetControllers().Single().Settings;
        Assert.AreEqual(TriggerEffectKind.Pulse, settings.L2Effect);
        Assert.AreEqual(TriggerEffectKind.FullClick, settings.R2Effect);
    }

    [TestMethod]
    public void TriggerEffectChangeNotifiesTheEngine()
    {
        source.Controllers.Add(Controller());
        int notified = 0;
        Global.L2OutputSettings[Slot].TriggerEffectChanged += (_, _) => notified++;

        service.SetTriggerEffect(Slot, ControllerTrigger.L2, TriggerEffectKind.Rigid);

        Assert.AreEqual(1, notified);
    }

    [TestMethod]
    public void TriggerEffectOnDualShock4IsNotSupported()
    {
        source.Controllers.Add(Controller(ControllerKind.DualShock4));

        Assert.AreEqual(CommandResult.NotSupported,
            service.SetTriggerEffect(Slot, ControllerTrigger.L2, TriggerEffectKind.Rigid));
        Assert.AreEqual(TriggerEffects.None, Global.L2OutputSettings[Slot].TriggerEffect);
    }

    [TestMethod]
    public void TemporaryProfileIsNotOverwritten()
    {
        source.Controllers.Add(Controller());
        Global.useTempProfile[Slot] = true;

        try
        {
            Assert.AreEqual(CommandResult.NoProfile, service.SaveProfile(Slot));
        }
        finally
        {
            Global.useTempProfile[Slot] = false;
        }
    }

    [TestMethod]
    public void SlotWithoutProfileCannotBeSaved()
    {
        source.Controllers.Add(Controller());
        string saved = Global.ProfilePath[Slot];
        Global.ProfilePath[Slot] = string.Empty;

        try
        {
            Assert.AreEqual(CommandResult.NoProfile, service.SaveProfile(Slot));
        }
        finally
        {
            Global.ProfilePath[Slot] = saved;
        }
    }

    [TestMethod]
    public void ListIncludesHardwareFactsAndFeatures()
    {
        source.Controllers.Add(Controller(battery: 42));

        ControllerInfoDto info = service.GetControllers().Single();

        Assert.AreEqual(Slot, info.Slot);
        Assert.AreEqual("AA:BB", info.Id);
        Assert.AreEqual(ControllerConnection.Bluetooth, info.Connection);
        Assert.AreEqual(42, info.Battery.Level);
        Assert.IsTrue(info.Features.HasMuteButton);
    }

    [TestMethod]
    public void EventsArriveInOrderOnAnotherThread()
    {
        var log = new List<string>();
        int threadId = 0;
        var done = new ManualResetEventSlim();
        service.ControllerConnected += (_, e) => log.Add($"connected {e.Slot} {e.Settings.LightBrightness}");
        service.BatteryChanged += (_, e) => log.Add($"battery {e.Level}");
        service.MuteButtonPressed += (_, e) => log.Add($"mute {e.Id}");
        service.ControllerDisconnected += (_, e) =>
        {
            log.Add($"disconnected {e.Slot}");
            threadId = Environment.CurrentManagedThreadId;
            done.Set();
        };

        ControllerSnapshot controller = Controller(battery: 70);
        source.RaiseConnected(controller);
        source.RaiseBattery(controller);
        source.RaiseMute(controller);
        source.RaiseDisconnected(controller);

        Assert.IsTrue(done.Wait(Timeout));
        CollectionAssert.AreEqual(
            new[] { $"connected {Slot} 100", "battery 70", "mute AA:BB", $"disconnected {Slot}" }, log);
        Assert.AreNotEqual(Environment.CurrentManagedThreadId, threadId);
    }

    [TestMethod]
    public void ASlowSubscriberDoesNotHoldTheInputThread()
    {
        var release = new ManualResetEventSlim();
        service.MuteButtonPressed += (_, _) => release.Wait();

        var watch = Stopwatch.StartNew();
        source.RaiseMute(Controller());
        watch.Stop();
        release.Set();

        Assert.IsTrue(watch.Elapsed < TimeSpan.FromSeconds(1));
    }

    [TestMethod]
    public void DisposeReleasesTheSource()
    {
        var localSource = new FakeSource();
        var local = new ControllerService(localSource);

        local.Dispose();

        Assert.IsTrue(localSource.Disposed);
    }
}

[TestClass]
public class ProcessPriorityOptionsTests
{
    [TestMethod]
    public void OrderMatchesTheSavedIndex()
    {
        CollectionAssert.AreEqual(
            new[]
            {
                ProcessPriorityClass.Normal, ProcessPriorityClass.AboveNormal,
                ProcessPriorityClass.High, ProcessPriorityClass.RealTime,
            },
            ProcessPriorityOptions.Classes);
    }
}

/// <summary>Keeps the engine folders free of references to the interface.</summary>
[TestClass]
public class EngineLayeringTests
{
    private static readonly string[] EngineFolders = ["Motor", "Mandos"];

    private static readonly string[] ForbiddenText =
    [
        "Interfaz", "MessageBox", "Application.Current", "App.rootHub", "App.requestClient",
        "System.Windows.Controls", "System.Windows.Data", "System.Windows.Interop", "System.Windows.Media",
    ];

    private static string FindRepositoryRoot()
    {
        DirectoryInfo directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, "DS5Control.sln")))
        {
            directory = directory.Parent;
        }

        return directory?.FullName;
    }

    [TestMethod]
    public void EngineSourceDoesNotReferenceTheInterface()
    {
        string root = FindRepositoryRoot();
        if (root is null) Assert.Inconclusive("Source tree not found.");

        var offenders = new List<string>();
        foreach (string folder in EngineFolders)
        {
            foreach (string file in Directory.EnumerateFiles(Path.Combine(root, "DS5Control", folder), "*.cs", SearchOption.AllDirectories))
            {
                string text = File.ReadAllText(file);
                offenders.AddRange(ForbiddenText.Where(text.Contains).Select(word => $"{Path.GetFileName(file)}: {word}"));
            }
        }

        Assert.AreEqual(0, offenders.Count, string.Join(Environment.NewLine, offenders));
    }
}
