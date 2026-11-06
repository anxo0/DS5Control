using System;
using System.Collections.Generic;
using System.Linq;
using DS4Windows.InputDevices;

namespace DS4Windows.Api
{
    /// <summary>
    /// Implementation of <see cref="IControllerService"/> over the per-slot profile settings in
    /// <see cref="Global"/>, the same values the profile editor changes, so the engine picks them
    /// up without further wiring.
    /// </summary>
    public sealed class ControllerService : IControllerService
    {
        public const int MaxLightBrightness = 100;

        /// <summary>Same upper limit as the rumble boost field of the profile editor.</summary>
        public const int MaxRumbleStrength = 200;

        private readonly IControllerSource source;
        private readonly ApiEventQueue queue = new ApiEventQueue();
        private readonly object settingsLock = new object();
        private readonly LightLevel[] lightLevels;

        public event EventHandler<ControllerInfoDto> ControllerConnected;
        public event EventHandler<ControllerRefDto> ControllerDisconnected;
        public event EventHandler<ControllerBatteryDto> BatteryChanged;
        public event EventHandler<ControllerRefDto> MuteButtonPressed;

        public ControllerService(ControlService control)
            : this(new ControlServiceControllerSource(control))
        {
        }

        public ControllerService(IControllerSource source)
        {
            this.source = source;
            lightLevels = Enumerable.Range(0, Global.MAX_DS4_CONTROLLER_COUNT)
                .Select(_ => new LightLevel())
                .ToArray();

            source.Connected += OnConnected;
            source.Disconnected += OnDisconnected;
            source.BatteryChanged += OnBatteryChanged;
            source.MutePressed += OnMutePressed;
        }

        public IReadOnlyList<ControllerInfoDto> GetControllers()
        {
            return source.GetControllers().Select(ToInfo).ToList();
        }

        public CommandResult SetLightColor(int slot, RgbColorDto color)
        {
            if (color is null) return CommandResult.InvalidValue;

            return ChangeLight(slot, level => level.SetColor(new DS4Color(color.R, color.G, color.B)));
        }

        public CommandResult SetLightBrightness(int slot, int percent)
        {
            if (percent is < 0 or > MaxLightBrightness) return CommandResult.InvalidValue;

            return ChangeLight(slot, level => level.SetBrightness(percent));
        }

        public CommandResult SetRumbleStrength(int slot, int percent)
        {
            if (percent is < 0 or > MaxRumbleStrength) return CommandResult.InvalidValue;

            ControllerSnapshot controller = source.Find(slot);
            if (controller is null) return CommandResult.NoController;
            if (!ToFeatures(controller).HasRumble) return CommandResult.NotSupported;

            Global.RumbleBoost[slot] = (byte)percent;
            return CommandResult.Ok;
        }

        public CommandResult SetTriggerEffect(int slot, ControllerTrigger trigger, TriggerEffectKind effect)
        {
            if (!ControllerMapping.TryToEngineEffect(effect, out TriggerEffects engineEffect))
            {
                return CommandResult.InvalidValue;
            }

            ControllerSnapshot controller = source.Find(slot);
            if (controller is null) return CommandResult.NoController;
            if (!ToFeatures(controller).HasTriggerEffects) return CommandResult.NotSupported;

            switch (trigger)
            {
                case ControllerTrigger.L2:
                    Global.L2OutputSettings[slot].TriggerEffect = engineEffect;
                    return CommandResult.Ok;
                case ControllerTrigger.R2:
                    Global.R2OutputSettings[slot].TriggerEffect = engineEffect;
                    return CommandResult.Ok;
                default:
                    return CommandResult.InvalidValue;
            }
        }

        public CommandResult SaveProfile(int slot)
        {
            if (source.Find(slot) is null) return CommandResult.NoController;

            // A temporary auto profile is not the profile chosen for the slot.
            string profile = Global.ProfilePath[slot];
            if (Global.useTempProfile[slot] || string.IsNullOrEmpty(profile)) return CommandResult.NoProfile;

            Global.SaveProfile(slot, profile);
            return CommandResult.Ok;
        }

        private CommandResult ChangeLight(int slot, Func<LightLevel, DS4Color> change)
        {
            ControllerSnapshot controller = source.Find(slot);
            if (controller is null) return CommandResult.NoController;
            if (!ToFeatures(controller).HasLight) return CommandResult.NotSupported;

            LightbarSettingInfo light = Global.LightbarSettingsInfo[slot];
            lock (settingsLock)
            {
                LightLevel level = lightLevels[slot];
                level.Sync(light.ds4winSettings.m_Led);
                light.ds4winSettings.m_Led = change(level);

                // Anything else driving the light would hide the chosen colour.
                light.ds4winSettings.useCustomLed = false;
                light.ds4winSettings.rainbow = 0;
                light.Mode = LightbarMode.DS4Win;
            }

            return CommandResult.Ok;
        }

        private ControllerInfoDto ToInfo(ControllerSnapshot controller)
        {
            return new ControllerInfoDto(
                controller.Slot,
                controller.Id,
                controller.Name,
                controller.Kind,
                controller.Connection,
                ToBattery(controller),
                ToFeatures(controller),
                ReadSettings(controller.Slot));
        }

        private ControllerSettingsDto ReadSettings(int slot)
        {
            RgbColorDto color;
            int brightness;
            lock (settingsLock)
            {
                LightLevel level = lightLevels[slot];
                level.Sync(Global.LightbarSettingsInfo[slot].ds4winSettings.m_Led);
                DS4Color baseColor = level.BaseColor;
                color = new RgbColorDto(baseColor.red, baseColor.green, baseColor.blue);
                brightness = level.Brightness;
            }

            return new ControllerSettingsDto(
                color,
                brightness,
                Global.RumbleBoost[slot],
                ControllerMapping.ToEffectKind(Global.L2OutputSettings[slot].TriggerEffect),
                ControllerMapping.ToEffectKind(Global.R2OutputSettings[slot].TriggerEffect));
        }

        private static ControllerFeaturesDto ToFeatures(ControllerSnapshot controller)
        {
            return ControllerMapping.ToFeatures(controller.Kind, controller.CanOutput);
        }

        private static ControllerBatteryDto ToBattery(ControllerSnapshot controller)
        {
            return new ControllerBatteryDto(controller.Slot, controller.Id, controller.BatteryLevel,
                controller.Charging, controller.BatteryAvailable);
        }

        private void OnConnected(ControllerSnapshot controller)
        {
            queue.Post(() => ControllerConnected?.Invoke(this, ToInfo(controller)));
        }

        private void OnDisconnected(ControllerSnapshot controller)
        {
            var reference = new ControllerRefDto(controller.Slot, controller.Id);
            queue.Post(() => ControllerDisconnected?.Invoke(this, reference));
        }

        private void OnBatteryChanged(ControllerSnapshot controller)
        {
            queue.Post(() => BatteryChanged?.Invoke(this, ToBattery(controller)));
        }

        private void OnMutePressed(ControllerSnapshot controller)
        {
            var reference = new ControllerRefDto(controller.Slot, controller.Id);
            queue.Post(() => MuteButtonPressed?.Invoke(this, reference));
        }

        public void Dispose()
        {
            source.Connected -= OnConnected;
            source.Disconnected -= OnDisconnected;
            source.BatteryChanged -= OnBatteryChanged;
            source.MutePressed -= OnMutePressed;
            source.Dispose();
            queue.Dispose();
        }
    }
}
