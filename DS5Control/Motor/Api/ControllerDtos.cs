using System.Text.Json;
using System.Text.Json.Serialization;

namespace DS5Control.Api
{
    public enum ControllerKind
    {
        Unknown,
        DualShock4,
        DualSense,
        DualShock3,
        SwitchPro,
        JoyConLeft,
        JoyConRight,
        JoyConGrip,
    }

    public enum ControllerConnection
    {
        Usb,
        Bluetooth,
        /// <summary>Sony wireless USB adapter.</summary>
        WirelessAdapter,
    }

    public enum ControllerTrigger
    {
        L2,
        R2,
    }

    public enum TriggerEffectKind
    {
        None,
        FullClick,
        Rigid,
        Pulse,
    }

    public enum CommandResult
    {
        Ok,
        /// <summary>No controller is connected in that slot.</summary>
        NoController,
        /// <summary>The controller in that slot lacks the feature.</summary>
        NotSupported,
        /// <summary>The value is out of range.</summary>
        InvalidValue,
        /// <summary>The slot has no profile of its own to save into.</summary>
        NoProfile,
    }

    public sealed record RgbColorDto(byte R, byte G, byte B);

    public sealed record ControllerFeaturesDto(
        bool HasLight,
        bool HasRumble,
        bool HasTriggerEffects,
        bool HasMuteButton);

    public sealed record ControllerSettingsDto(
        RgbColorDto LightColor,
        int LightBrightness,
        int RumbleStrength,
        TriggerEffectKind L2Effect,
        TriggerEffectKind R2Effect);

    public sealed record ControllerBatteryDto(
        int Slot,
        string Id,
        int Level,
        bool Charging,
        bool Available);

    public sealed record ControllerInfoDto(
        int Slot,
        string Id,
        string Name,
        ControllerKind Kind,
        ControllerConnection Connection,
        ControllerBatteryDto Battery,
        ControllerFeaturesDto Features,
        ControllerSettingsDto Settings);

    public sealed record ControllerRefDto(int Slot, string Id);

    /// <summary>JSON settings for the bridge: camelCase members and enums as text.</summary>
    public static class ControllerApiJson
    {
        public static JsonSerializerOptions Options { get; } = new JsonSerializerOptions(JsonSerializerDefaults.Web)
        {
            Converters = { new JsonStringEnumConverter(JsonNamingPolicy.CamelCase) },
        };
    }
}
