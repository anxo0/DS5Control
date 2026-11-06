using System;
using System.Collections.Generic;

namespace DS4Windows.Api
{
    /// <summary>
    /// What the main screen needs from the engine. Events are raised one at a time, in order, on
    /// a background thread of their own, never on an input thread, so a slow subscriber delays
    /// other subscribers but not the controllers. Commands apply to the settings of the slot and
    /// take effect at once. They stay in memory until <see cref="SaveProfile"/> is called.
    /// </summary>
    public interface IControllerService : IDisposable
    {
        event EventHandler<ControllerInfoDto> ControllerConnected;
        event EventHandler<ControllerRefDto> ControllerDisconnected;
        event EventHandler<ControllerBatteryDto> BatteryChanged;
        event EventHandler<ControllerRefDto> MuteButtonPressed;

        IReadOnlyList<ControllerInfoDto> GetControllers();

        CommandResult SetLightColor(int slot, RgbColorDto color);

        /// <param name="slot">Controller slot.</param>
        /// <param name="percent">From 0 to 100.</param>
        CommandResult SetLightBrightness(int slot, int percent);

        /// <param name="slot">Controller slot.</param>
        /// <param name="percent">From 0 to <see cref="ControllerService.MaxRumbleStrength"/>, 100 being unchanged.</param>
        CommandResult SetRumbleStrength(int slot, int percent);

        CommandResult SetTriggerEffect(int slot, ControllerTrigger trigger, TriggerEffectKind effect);

        /// <summary>Writes the current settings of the slot to its active profile.</summary>
        CommandResult SaveProfile(int slot);
    }
}
