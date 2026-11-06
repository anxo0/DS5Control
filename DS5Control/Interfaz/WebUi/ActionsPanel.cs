using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;
using DS5Control;

namespace DS5ControlApp.Interfaz.WebUi
{
    /// <summary>The list of special actions and the part of it enabled in a profile.</summary>
    internal static class ActionsPanel
    {
        private const int ModeMacro = 1;
        private const int ModeProgram = 2;
        private const int ModeProfile = 3;
        private const int ModeKey = 4;
        private const int ModeDisconnect = 5;
        private const int ModeBattery = 6;
        private const int ModeGyroCalibrate = 9;

        public static object[] Describe(int slot)
        {
            List<string> enabled = Global.ProfileActions[slot];
            return Global.GetActions().Select(action => (object)new
            {
                nombre = action.name,
                tipo = TypeName(action),
                controles = action.controls.Split('/', StringSplitOptions.RemoveEmptyEntries),
                detalles = action.details ?? string.Empty,
                extra = action.extra ?? string.Empty,
                retardo = action.delayTime,
                pasos = action.macro?.ToArray() ?? Array.Empty<int>(),
                repetir = action.keyType.HasFlag(DS5KeyType.RepeatMacro),
                activa = enabled.Contains(action.name),
            }).ToArray();
        }

        public static void SetEnabled(int slot, string name, bool enabled)
        {
            List<string> list = Global.ProfileActions[slot];
            if (enabled && !list.Contains(name)) list.Add(name);
            if (!enabled) list.Remove(name);
            Global.CacheExtraProfileInfo(slot);
        }

        public static void Delete(int slot, string name)
        {
            Global.RemoveAction(name);
            Global.ProfileActions[slot].Remove(name);
            Global.CacheExtraProfileInfo(slot);
        }

        public static bool Save(int slot, JsonElement command)
        {
            string name = command.GetProperty("nombre").GetString()?.Trim();
            string previous = command.GetProperty("original").ValueKind == JsonValueKind.String
                ? command.GetProperty("original").GetString() : null;
            string[] controls = command.GetProperty("controles").EnumerateArray().Select(item => item.GetString()).ToArray();

            // The profile stores its actions separated by slashes.
            if (string.IsNullOrEmpty(name) || name.Contains('/') || controls.Length == 0) return false;
            if (name != previous && Global.GetActions().Any(item => item.name == name)) return false;

            bool edit = previous == name;
            if (previous != null && !edit) Delete(slot, previous);

            string joined = string.Join("/", controls);
            double delay = command.GetProperty("retardo").GetDouble();
            string seconds = delay.ToString("#.##", Global.configFileDecimalCulture);
            switch (command.GetProperty("tipoAccion").GetString())
            {
                case "macro":
                    int[] steps = command.GetProperty("pasos").EnumerateArray().Select(item => item.GetInt32()).ToArray();
                    if (steps.Length == 0) return false;
                    Global.SaveAction(name, joined, ModeMacro, string.Join("/", steps), edit,
                        extras: command.GetProperty("repetir").GetBoolean() ? "Repeat" : string.Empty);
                    break;
                case "programa":
                    string path = command.GetProperty("detalles").GetString();
                    if (string.IsNullOrWhiteSpace(path)) return false;
                    Global.SaveAction(name, joined, ModeProgram, $"{path}?{seconds}", edit, delay,
                        command.GetProperty("extra").GetString());
                    break;
                case "perfil":
                    Global.SaveAction(name, joined, ModeProfile, command.GetProperty("detalles").GetString(), edit);
                    break;
                case "tecla":
                    Global.SaveAction(name, joined, ModeKey, command.GetProperty("detalles").GetString(), edit);
                    break;
                case "desconectar":
                    Global.SaveAction(name, joined, ModeDisconnect, seconds, edit, delay);
                    break;
                case "bateria":
                    Global.SaveAction(name, joined, ModeBattery,
                        $"{seconds}|True|False|255|0|0|0|255|0", edit, delay);
                    break;
                case "giroscopio":
                    Global.SaveAction(name, joined, ModeGyroCalibrate, seconds, edit, delay);
                    break;
                default:
                    return false;
            }

            SetEnabled(slot, name, true);
            return true;
        }

        private static string TypeName(SpecialAction action)
        {
            return action.typeID switch
            {
                SpecialAction.ActionTypeId.Macro => "macro",
                SpecialAction.ActionTypeId.Program => "programa",
                SpecialAction.ActionTypeId.Profile => "perfil",
                SpecialAction.ActionTypeId.Key => "tecla",
                SpecialAction.ActionTypeId.DisconnectBT => "desconectar",
                SpecialAction.ActionTypeId.BatteryCheck => "bateria",
                SpecialAction.ActionTypeId.GyroCalibrate => "giroscopio",
                _ => "otra",
            };
        }
    }
}
