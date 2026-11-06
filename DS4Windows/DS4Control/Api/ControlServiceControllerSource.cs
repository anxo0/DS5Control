using System;
using System.Collections.Generic;
using System.Linq;

namespace DS4Windows.Api
{
    /// <summary>
    /// <see cref="IControllerSource"/> over <see cref="ControlService"/>. It uses the same events
    /// as the current controller list: ServiceStarted, HotplugController, PreServiceStop and
    /// the Removal event of each device.
    /// </summary>
    public sealed class ControlServiceControllerSource : IControllerSource
    {
        private sealed record Subscription(int Slot, EventHandler<EventArgs> Removal, EventHandler Battery);

        private readonly ControlService control;
        private readonly object subscriptionsLock = new object();
        private readonly Dictionary<DS4Device, Subscription> subscriptions = new Dictionary<DS4Device, Subscription>();

        public event Action<ControllerSnapshot> Connected;
        public event Action<ControllerSnapshot> Disconnected;
        public event Action<ControllerSnapshot> BatteryChanged;
        public event Action<ControllerSnapshot> MutePressed;

        public ControlServiceControllerSource(ControlService control)
        {
            this.control = control;
            control.ServiceStarted += OnServiceStarted;
            control.PreServiceStop += OnPreServiceStop;
            control.HotplugController += OnHotplugController;
            control.MuteButtonPressed += OnMuteButtonPressed;
            OnServiceStarted(control, EventArgs.Empty);
        }

        public IReadOnlyList<ControllerSnapshot> GetControllers()
        {
            using (new ReadLocker(control.slotManager.CollectionLocker))
            {
                return control.slotManager.ControllerDict
                    .OrderBy(pair => pair.Key)
                    .Select(pair => CreateSnapshot(pair.Value, pair.Key))
                    .ToList();
            }
        }

        public ControllerSnapshot Find(int slot)
        {
            using (new ReadLocker(control.slotManager.CollectionLocker))
            {
                return control.slotManager.ControllerDict.TryGetValue(slot, out DS4Device device)
                    ? CreateSnapshot(device, slot)
                    : null;
            }
        }

        private static ControllerSnapshot CreateSnapshot(DS4Device device, int slot)
        {
            VidPidFeatureSet features = device.FeatureSet;
            return new ControllerSnapshot(
                slot,
                device.getMacAddress(),
                device.DisplayName,
                ControllerMapping.ToKind(device.DeviceType),
                ControllerMapping.ToConnection(device.getConnectionType()),
                device.Battery,
                device.Charging,
                BatteryAvailable: (features & VidPidFeatureSet.NoBatteryReading) == 0,
                CanOutput: (features & VidPidFeatureSet.NoOutputData) == 0);
        }

        private void OnServiceStarted(object sender, EventArgs e)
        {
            List<KeyValuePair<int, DS4Device>> devices;
            using (new ReadLocker(control.slotManager.CollectionLocker))
            {
                devices = control.slotManager.ControllerDict.ToList();
            }

            foreach (KeyValuePair<int, DS4Device> pair in devices)
            {
                Track(pair.Value, pair.Key);
            }
        }

        private void OnHotplugController(ControlService sender, DS4Device device, int index)
        {
            Track(device, index);
        }

        private void OnPreServiceStop(object sender, EventArgs e)
        {
            List<DS4Device> devices;
            lock (subscriptionsLock)
            {
                devices = subscriptions.Keys.ToList();
            }

            foreach (DS4Device device in devices)
            {
                Untrack(device);
            }
        }

        private void OnMuteButtonPressed(ControlService sender, DS4Device device, int index)
        {
            MutePressed?.Invoke(CreateSnapshot(device, index));
        }

        private void Track(DS4Device device, int slot)
        {
            if (device.IsRemoving) return;

            var subscription = new Subscription(slot,
                (sender, e) => Untrack(device),
                (sender, e) => BatteryChanged?.Invoke(CreateSnapshot(device, slot)));

            lock (subscriptionsLock)
            {
                if (!subscriptions.TryAdd(device, subscription)) return;
            }

            device.Removal += subscription.Removal;
            device.BatteryChanged += subscription.Battery;
            device.ChargingChanged += subscription.Battery;
            Connected?.Invoke(CreateSnapshot(device, slot));
        }

        private void Untrack(DS4Device device)
        {
            Subscription subscription;
            lock (subscriptionsLock)
            {
                if (!subscriptions.Remove(device, out subscription)) return;
            }

            Unsubscribe(device, subscription);
            Disconnected?.Invoke(CreateSnapshot(device, subscription.Slot));
        }

        private static void Unsubscribe(DS4Device device, Subscription subscription)
        {
            device.Removal -= subscription.Removal;
            device.BatteryChanged -= subscription.Battery;
            device.ChargingChanged -= subscription.Battery;
        }

        public void Dispose()
        {
            control.ServiceStarted -= OnServiceStarted;
            control.PreServiceStop -= OnPreServiceStop;
            control.HotplugController -= OnHotplugController;
            control.MuteButtonPressed -= OnMuteButtonPressed;

            lock (subscriptionsLock)
            {
                foreach (KeyValuePair<DS4Device, Subscription> pair in subscriptions)
                {
                    Unsubscribe(pair.Key, pair.Value);
                }

                subscriptions.Clear();
            }
        }
    }
}
