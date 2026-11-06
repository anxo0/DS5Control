using DS4Windows.InputDevices;

namespace DS4Windows.Api
{
    /// <summary>Translates engine types to the stable names of the API.</summary>
    public static class ControllerMapping
    {
        public static ControllerKind ToKind(InputDeviceType deviceType)
        {
            return deviceType switch
            {
                InputDeviceType.DS4 => ControllerKind.DualShock4,
                InputDeviceType.DualSense => ControllerKind.DualSense,
                InputDeviceType.DS3 => ControllerKind.DualShock3,
                InputDeviceType.SwitchPro => ControllerKind.SwitchPro,
                InputDeviceType.JoyConL => ControllerKind.JoyConLeft,
                InputDeviceType.JoyConR => ControllerKind.JoyConRight,
                InputDeviceType.JoyConGrip => ControllerKind.JoyConGrip,
                _ => ControllerKind.Unknown,
            };
        }

        public static ControllerConnection ToConnection(ConnectionType connectionType)
        {
            return connectionType switch
            {
                ConnectionType.USB => ControllerConnection.Usb,
                ConnectionType.SONYWA => ControllerConnection.WirelessAdapter,
                _ => ControllerConnection.Bluetooth,
            };
        }

        public static TriggerEffectKind ToEffectKind(TriggerEffects effect)
        {
            return effect switch
            {
                TriggerEffects.FullClick => TriggerEffectKind.FullClick,
                TriggerEffects.Rigid => TriggerEffectKind.Rigid,
                TriggerEffects.Pulse => TriggerEffectKind.Pulse,
                _ => TriggerEffectKind.None,
            };
        }

        public static bool TryToEngineEffect(TriggerEffectKind effect, out TriggerEffects engineEffect)
        {
            switch (effect)
            {
                case TriggerEffectKind.None: engineEffect = TriggerEffects.None; return true;
                case TriggerEffectKind.FullClick: engineEffect = TriggerEffects.FullClick; return true;
                case TriggerEffectKind.Rigid: engineEffect = TriggerEffects.Rigid; return true;
                case TriggerEffectKind.Pulse: engineEffect = TriggerEffects.Pulse; return true;
                default: engineEffect = TriggerEffects.None; return false;
            }
        }

        /// <param name="kind">Controller type.</param>
        /// <param name="canOutput">False for models that take no output reports at all.</param>
        public static ControllerFeaturesDto ToFeatures(ControllerKind kind, bool canOutput)
        {
            bool hasLightbar = kind is ControllerKind.DualShock4 or ControllerKind.DualSense;
            bool isDualSense = kind == ControllerKind.DualSense;
            return new ControllerFeaturesDto(
                HasLight: canOutput && hasLightbar,
                HasRumble: canOutput,
                HasTriggerEffects: canOutput && isDualSense,
                HasMuteButton: isDualSense);
        }
    }
}
