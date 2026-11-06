using System.Reflection;
using System.Runtime.CompilerServices;
using DS4Windows;
using DS4Windows.InputDevices;

namespace DS4WindowsTests
{
    [TestClass]
    public class InputThreadContainmentTests
    {
        private static DS4Device CreateDevice(Type deviceType)
        {
            // No hardware here: the device is never initialized and only needs a HidDevice reference
            HidDevice hidDevice = (HidDevice)RuntimeHelpers.GetUninitializedObject(typeof(HidDevice));
            return (DS4Device)Activator.CreateInstance(deviceType, hidDevice, "Test device",
                VidPidFeatureSet.DefaultDS4);
        }

        private static void RunInputThread(DS4Device device, Action inputLoop)
        {
            MethodInfo method = typeof(DS4Device).GetMethod("RunInputThread",
                BindingFlags.Instance | BindingFlags.NonPublic);
            method.Invoke(device, new object[] { inputLoop });
        }

        private static List<string> CaptureLog(Action action)
        {
            List<string> messages = new List<string>();
            EventHandler<DebugEventArgs> handler = (_, e) =>
            {
                lock (messages)
                {
                    messages.Add(e.Data);
                }
            };

            AppLogger.GuiLog += handler;
            try
            {
                action();
            }
            finally
            {
                AppLogger.GuiLog -= handler;
            }

            return messages;
        }

        [TestMethod]
        [DataRow(typeof(DS4Device))]
        [DataRow(typeof(DualSenseDevice))]
        [DataRow(typeof(DS3Device))]
        [DataRow(typeof(SwitchProDevice))]
        [DataRow(typeof(JoyConDevice))]
        public void ExceptionInInputLoopIsLoggedAndRemovesOnlyThatDevice(Type deviceType)
        {
            DS4Device device = CreateDevice(deviceType);
            DS4Device otherDevice = CreateDevice(deviceType);
            int removals = 0;
            int otherRemovals = 0;
            device.Removal += (_, _) => removals++;
            otherDevice.Removal += (_, _) => otherRemovals++;
            string errorText = $"Report failure {Guid.NewGuid()}";

            List<string> log = CaptureLog(() =>
                RunInputThread(device, () => throw new NullReferenceException(errorText)));

            Assert.AreEqual(1, removals);
            Assert.AreEqual(0, otherRemovals);
            Assert.IsTrue(device.IsDisconnecting);
            Assert.IsFalse(otherDevice.IsDisconnecting);
            Assert.IsTrue(log.Any(message => message.Contains(errorText)));
        }

        [TestMethod]
        public void InputLoopThatEndsNormallyDoesNotRemoveTheDevice()
        {
            DS4Device device = CreateDevice(typeof(DS4Device));
            int removals = 0;
            device.Removal += (_, _) => removals++;

            RunInputThread(device, () => { });

            Assert.AreEqual(0, removals);
            Assert.IsFalse(device.IsDisconnecting);
        }

        [TestMethod]
        public void FailingRemovalHandlerIsLoggedAndContained()
        {
            DS4Device device = CreateDevice(typeof(DS4Device));
            string removalError = $"Removal failure {Guid.NewGuid()}";
            device.Removal += (_, _) => throw new InvalidOperationException(removalError);

            List<string> log = CaptureLog(() =>
                RunInputThread(device, () => throw new InvalidOperationException("Report failure")));

            Assert.IsTrue(log.Any(message => message.Contains(removalError)));
        }
    }
}
