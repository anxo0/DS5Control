using System;
using System.Collections.Generic;

namespace DS4Windows.Api
{
    /// <summary>Copy of the hardware facts of a connected controller.</summary>
    public sealed record ControllerSnapshot(
        int Slot,
        string Id,
        string Name,
        ControllerKind Kind,
        ControllerConnection Connection,
        int BatteryLevel,
        bool Charging,
        bool BatteryAvailable,
        bool CanOutput);

    /// <summary>
    /// Where <see cref="ControllerService"/> learns about controllers. Keeps the service testable
    /// without hardware. Events may arrive on input threads: handlers must not block.
    /// </summary>
    public interface IControllerSource : IDisposable
    {
        event Action<ControllerSnapshot> Connected;
        event Action<ControllerSnapshot> Disconnected;
        event Action<ControllerSnapshot> BatteryChanged;
        event Action<ControllerSnapshot> MutePressed;

        IReadOnlyList<ControllerSnapshot> GetControllers();

        /// <returns>The controller in the slot, or null.</returns>
        ControllerSnapshot Find(int slot);
    }
}
