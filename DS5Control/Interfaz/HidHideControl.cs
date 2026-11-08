using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using DS5Control;
using DS5ControlApp.Motor;

namespace DS5ControlApp.Interfaz
{
    /// <summary>
    /// Hides the real controllers from the games while DS5Control feeds them a virtual one, so
    /// every press arrives once. Only touches what it added itself: the user's own HidHide list
    /// and its on/off state are left as they were when DS5Control closes.
    /// </summary>
    public static class HidHideControl
    {
        private const string StateFile = "HidHide.txt";
        private const string TurnedOnLine = "encendido=true";
        private const string HiddenPrefix = "oculto=";

        private static readonly object Gate = new object();
        private static readonly HashSet<string> Added = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        private static HashSet<string> lastWanted;
        private static bool turnedOn;
        private static bool loaded;

        /// <summary>Hides exactly these device instances; an empty list undoes everything.</summary>
        public static void Apply(IReadOnlyCollection<string> hide)
        {
            if (!Global.hidHideInstalled) return;

            lock (Gate)
            {
                // What a previous run left behind if it ended without closing properly.
                if (!loaded) Load();

                var wanted = new HashSet<string>(hide, StringComparer.OrdinalIgnoreCase);
                if (lastWanted != null && wanted.SetEquals(lastWanted)) return;

                using var device = new HidHideAPIDevice();
                if (!device.IsOpen()) return;

                List<string> list = device.GetBlacklist();
                bool changed = false;
                foreach (string instance in Added.Where(i => !wanted.Contains(i)).ToList())
                {
                    changed |= list.RemoveAll(i => string.Equals(i, instance, StringComparison.OrdinalIgnoreCase)) > 0;
                    Added.Remove(instance);
                }
                foreach (string instance in wanted)
                {
                    if (list.Any(i => string.Equals(i, instance, StringComparison.OrdinalIgnoreCase))) continue;
                    list.Add(instance);
                    Added.Add(instance);
                    changed = true;
                }
                if (changed && !device.SetBlacklist(list)) return;

                if (wanted.Count > 0 && !device.GetActiveState())
                {
                    device.SetActiveState(true);
                    turnedOn = device.GetActiveState();
                }
                else if (wanted.Count == 0 && turnedOn)
                {
                    device.SetActiveState(false);
                    turnedOn = false;
                }

                lastWanted = wanted;
                Save();
            }
        }

        private static string StatePath => System.IO.Path.Combine(Global.appdatapath ?? Global.appDataPpath, StateFile);

        private static void Load()
        {
            loaded = true;
            try
            {
                if (!System.IO.File.Exists(StatePath)) return;
                foreach (string line in System.IO.File.ReadAllLines(StatePath))
                {
                    if (line == TurnedOnLine) turnedOn = true;
                    else if (line.StartsWith(HiddenPrefix)) Added.Add(line.Substring(HiddenPrefix.Length));
                }
            }
            catch (Exception exception) when (exception is System.IO.IOException or UnauthorizedAccessException)
            {
            }
        }

        private static void Save()
        {
            try
            {
                var lines = Added.Select(i => HiddenPrefix + i).ToList();
                if (turnedOn) lines.Insert(0, TurnedOnLine);
                if (lines.Count == 0)
                {
                    if (System.IO.File.Exists(StatePath)) System.IO.File.Delete(StatePath);
                }
                else
                {
                    System.IO.File.WriteAllLines(StatePath, lines);
                }
            }
            catch (Exception exception) when (exception is System.IO.IOException or UnauthorizedAccessException)
            {
            }
        }

        /// <summary>Instances of the controllers DS5Control is emulating right now.</summary>
        public static List<string> EmulatedControllers()
        {
            var result = new List<string>();
            if (App.rootHub == null || !App.rootHub.running) return result;

            for (int slot = 0; slot < ControlService.CURRENT_DS5_CONTROLLER_LIMIT; slot++)
            {
                DS5Device device = App.rootHub.DS5Controllers[slot];
                // The real DualSense mode needs the game to see the controller itself.
                if (device?.HidDevice == null || Global.DinputOnly[slot]) continue;
                string instance = InstanceOf(device.HidDevice.DevicePath);
                if (instance != null) result.Add(instance);
            }
            return result;
        }

        /// <summary>Asks Windows for the exact instance id; the path itself is the fallback.</summary>
        public static string InstanceOf(string devicePath)
        {
            if (string.IsNullOrEmpty(devicePath)) return null;
            try
            {
                DevPropKey key = InstanceIdKey;
                uint size = 0;
                int result = CM_Get_Device_Interface_PropertyW(devicePath, ref key, out _, null, ref size, 0);
                if (result == CrBufferSmall && size > 0)
                {
                    var buffer = new byte[size];
                    if (CM_Get_Device_Interface_PropertyW(devicePath, ref key, out _, buffer, ref size, 0) == 0)
                    {
                        return System.Text.Encoding.Unicode.GetString(buffer).TrimEnd('\0');
                    }
                }
            }
            catch (Exception exception) when (exception is DllNotFoundException or EntryPointNotFoundException)
            {
            }
            return InstanceFromPath(devicePath);
        }

        /// <summary>
        /// "\\?\hid#vid_054c&amp;pid_0ce6&amp;mi_03#8&amp;e01d75d&amp;0&amp;0000#{guid}" becomes
        /// "HID\VID_054C&amp;PID_0CE6&amp;MI_03\8&amp;E01D75D&amp;0&amp;0000".
        /// </summary>
        public static string InstanceFromPath(string devicePath)
        {
            if (string.IsNullOrEmpty(devicePath)) return null;
            string path = devicePath.StartsWith(@"\\?\") ? devicePath.Substring(4) : devicePath;
            int guid = path.LastIndexOf("#{", StringComparison.Ordinal);
            if (guid > 0) path = path.Substring(0, guid);
            return path.Replace('#', '\\').ToUpperInvariant();
        }

        private const int CrBufferSmall = 0x1A;

        [StructLayout(LayoutKind.Sequential)]
        private struct DevPropKey
        {
            public Guid fmtid;
            public uint pid;
        }

        private static readonly DevPropKey InstanceIdKey = new DevPropKey
        {
            fmtid = new Guid("78c34fc8-104a-4aca-9ea4-524d52996e57"),
            pid = 256,
        };

        [DllImport("cfgmgr32.dll", CharSet = CharSet.Unicode)]
        private static extern int CM_Get_Device_Interface_PropertyW(string deviceInterface, ref DevPropKey key,
            out uint type, byte[] buffer, ref uint size, uint flags);
    }
}
