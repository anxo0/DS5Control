using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Reflection;
using System.Text.Json;
using DS5Control;
using DS5ControlApp.Interfaz.ViewModels;

namespace DS5ControlApp.Interfaz.WebUi
{
    /// <summary>
    /// The advanced profile settings, described as data so one generic screen can show all of them.
    /// Values are read and written through <see cref="ProfileSettingsViewModel"/>, which applies
    /// each change to the engine.
    /// </summary>
    internal static class ProfileOptions
    {
        private static readonly string[] Curves =
        {
            "Lineal", "Más precisión", "Cuadrática", "Cúbica", "Suave cuadrática", "Suave cúbica", "Personalizada",
        };

        private static readonly string[] Conditions = { "Todos a la vez", "Cualquiera" };
        private static readonly string[] Smoothing = { "Filtro 1€", "Media ponderada" };
        private static readonly string[] Yaw = { "Girar", "Inclinar" };
        private static readonly string[] OutputStick = { "Ninguno", "Izquierdo", "Derecho" };
        private static readonly string[] OutputAxes = { "Ninguno", "Ambos", "Solo horizontal", "Solo vertical" };

        private static readonly string[] Buttons =
        {
            "Cruz", "Círculo", "Cuadrado", "Triángulo", "L1", "L2", "R1", "R2",
            "Arriba", "Abajo", "Izquierda", "Derecha", "L3", "R3",
            "Dedo en el panel", "Dos dedos en el panel", "Options", "Crear", "PS", "Clic del panel",
            "Mute", "Lateral izquierdo", "Lateral derecho",
            "Función izquierda", "Función derecha", "Palanca izquierda", "Palanca derecha",
        };

        private static readonly Group[] Groups = BuildGroups();
        private static readonly Dictionary<string, Option> ById =
            Groups.SelectMany(g => g.Sections).SelectMany(s => s.Options).ToDictionary(o => o.Id);

        public static object Describe(ProfileSettingsViewModel profile, int slot)
        {
            return Groups.Select(group => new
            {
                id = group.Id,
                nombre = group.Name,
                secciones = group.Sections.Where(s => s.Visible(profile)).Select(section => new
                {
                    nombre = section.Name,
                    ajustes = section.Options.Select(o => new
                    {
                        id = o.Id,
                        nombre = o.Name,
                        tipo = o.Kind,
                        min = o.Min,
                        max = o.Max,
                        paso = o.Step,
                        porcentaje = o.Percent,
                        opciones = o.Choices,
                        valor = o.Read(profile, slot),
                    }).ToArray(),
                }).ToArray(),
            }).ToArray();
        }

        public static bool Apply(ProfileSettingsViewModel profile, int slot, string id, JsonElement value)
        {
            if (!ById.TryGetValue(id, out Option option)) return false;
            option.Write(profile, slot, value);
            return true;
        }

        private static Group[] BuildGroups()
        {
            return new[]
            {
                new Group("sticks", "Sticks", Stick("Stick izquierdo", "LS", "Left").Concat(Stick("Stick derecho", "RS", "Right")).ToArray()),
                new Group("gatillos", "Gatillos", Trigger("L2").Concat(Trigger("R2")).ToArray()),
                new Group("giroscopio", "Giroscopio", Gyro()),
                new Group("panel", "Panel táctil", Touchpad()),
                new Group("luz", "Luz", Light()),
                new Group("vibracion", "Vibración", new Section("Motores",
                    Number("RumbleBoost", "Refuerzo de vibración", 0, 200, 10),
                    Number("RumbleAutostopTime", "Parar tras (milisegundos)", 0, 3600, 10),
                    Toggle("InverseRumbleMotors", "Intercambiar motores"),
                    Choice("DualSenseRumbleEmulationPerIndex", "Modo de vibración del DualSense", "Preciso", "Clásico", "Desactivado"),
                    Choice("DualSenseHapticPowerLevelPerIndex", "Fuerza háptica", "100%", "87%", "75%", "62%", "50%", "37%", "25%", "12%"),
                    Toggle("EnableGenericRumbleStrRescaleForDualSenseDevices", "Reescalar la fuerza"))),
                new Group("raton", "Ratón", new Section("Botones que mueven el ratón",
                    Number("ButtonMouseSensitivity", "Sensibilidad", 0, 255, 1),
                    Number("ButtonMouseVerticalScale", "Escala vertical", 0, 500, 10),
                    Number("ButtonMouseOffset", "Desplazamiento", 0, 100, 1),
                    Toggle("MouseAcceleration", "Aceleración"))),
                new Group("volante", "Volante", new Section("Girar el mando como un volante",
                    Choice("SASteeringWheelEmulationAxisIndex", "Eje que usa", "Ninguno", "Stick izq. X", "Stick izq. Y", "Stick der. X", "Stick der. Y", "L2 y R2"),
                    Choice("SASteeringWheelEmulationRangeIndex", "Giro total (grados)", "90", "180", "270", "360", "450", "720", "900", "1080", "1440"),
                    Number("SASteeringWheelFuzz", "Filtro de temblor", 0, 100, 1),
                    Toggle("SASteeringWheelUseSmoothing", "Suavizado"),
                    Number("SASteeringWheelSmoothMinCutoff", "Corte mínimo", 0.00001, 1000, 0.1),
                    Number("SASteeringWheelSmoothBeta", "Beta", 0, 1, 0.1))),
                new Group("otros", "Otros", new Section("Conexión",
                    Number("IdleDisconnect", "Apagar el mando tras (minutos sin usar)", 0, 60, 1),
                    Number("DebouncingMs", "Antirrebote de botones (milisegundos)", 0, 1000, 1),
                    Toggle("EnableOutputDataToDS5", "Enviar datos al mando"),
                    Toggle("DInputOnly", "Sin mando virtual"),
                    Toggle("VirtualTriggerButtons", "Botones virtuales de gatillo"),
                    Toggle("UseDs3PitchRollSim", "Simular inclinación en DS3"))),
            };
        }

        private static Section[] Stick(string name, string p, string side)
        {
            return new[]
            {
                new Section(name,
                    Number($"{p}DeadZone", "Zona muerta", 0, 1, 0.01, true),
                    Choice($"{p}DeadTypeIndex", "Forma de la zona muerta", "Circular", "Por ejes"),
                    Number($"{p}AntiDeadZone", "Compensar zona muerta del juego", 0, 1, 0.01, true),
                    Number($"{p}MaxZone", "Recorrido máximo", 0, 1, 0.01, true),
                    Number($"{p}MaxOutput", "Salida máxima", 0, 1, 0.01, true),
                    Toggle($"{p}MaxOutputForce", "Forzar salida máxima"),
                    Number($"{p}Sens", "Sensibilidad", 0.1, 5, 0.1),
                    Number($"{p}VerticalScale", "Escala vertical", 0, 4, 0.1),
                    Number($"{p}Rotation", "Rotación (grados)", -180, 180, 1),
                    Number($"{p}Fuzz", "Filtro de temblor", 0, 100, 1),
                    Choice($"{p}OutputCurveIndex", "Curva de respuesta", Curves),
                    Choice($"{p}OutputIndex", "Función", "Ninguna", "Normal", "Flick stick")),
                new Section($"{name}: curva personalizada", Curve($"{p}CustomCurve", "Curva")) { When = ($"{p}CustomCurveSelected", 1) },
                new Section($"{name}: flick stick",
                    Number($"{p}FlickRWC", "Giro completo (centímetros)", 1, 200, 1),
                    Number($"{p}FlickThreshold", "Umbral", 0, 1, 0.05, true),
                    Number($"{p}FlickTime", "Duración (segundos)", 0.01, 10, 0.05),
                    Number($"{p}MinAngleThreshold", "Ángulo mínimo", 0, 180, 0.5)) { When = ($"{p}OutputIndex", 2) },
                new Section($"{name}: precisión",
                    Toggle($"{p}SquareStick", "Recorrido cuadrado"),
                    Number($"{p}SquareRoundness", "Redondeo", 0, 1000, 1),
                    Toggle($"{p}AntiSnapback", "Evitar rebote al soltar"),
                    Number($"{p}AntiSnapbackDelta", "Margen de rebote", 0, 256, 1),
                    Number($"{p}AntiSnapbackTimeout", "Tiempo de rebote (milisegundos)", 0, 1000, 1),
                    Toggle($"{p}OuterBindInvert", "Invertir zona exterior"),
                    Number($"{p}OuterBindDead", "Inicio de la zona exterior", 0, 1, 0.01, true),
                    Toggle($"{p}DeltaAccelEnabled", "Aceleración por velocidad"),
                    Number($"{p}DeltaMultiplier", "Multiplicador", 0, 10, 0.1),
                    Number($"{p}DeltaMinFactor", "Factor mínimo", 1, 10, 0.1),
                    Number($"{p}DeltaMinTravel", "Recorrido mínimo", 0, 1, 0.05, true),
                    Number($"{p}DeltaMaxTravel", "Recorrido máximo", 0, 1, 0.05, true),
                    Number($"{p}DeltaEasingDuration", "Suavizado (segundos)", 0, 600, 0.1)),
                new Section($"{name}: corrección de deriva",
                    Number($"{side}StickDriftXAxis", "Horizontal", -127, 127, 1),
                    Number($"{side}StickDriftYAxis", "Vertical", -127, 127, 1)),
            };
        }

        private static Section[] Trigger(string t)
        {
            return new[]
            {
                new Section(t,
                    Number($"{t}DeadZone", "Zona muerta", 0, 1, 0.01, true),
                    Number($"{t}AntiDeadZone", "Compensar zona muerta del juego", 0, 1, 0.01, true),
                    Number($"{t}MaxZone", "Recorrido máximo", 0, 1, 0.01, true),
                    Number($"{t}MaxOutput", "Salida máxima", 0, 1, 0.01, true),
                    Number($"{t}Sens", "Sensibilidad", 0.1, 10, 0.1),
                    Choice($"{t}OutputCurveIndex", "Curva de respuesta", Curves),
                    Choice($"{t}TriggerMode", "Dos fases", "Desactivado", "Normal", "Solo botones", "Gatillo rápido", "Disparo de cadera", "Cadera y botones"),
                    Number($"{t}HipFireTime", "Tiempo de cadera (milisegundos)", 0, 5000, 10)),
                new Section($"{t}: curva personalizada", Curve($"{t}CustomCurve", "Curva")) { When = ($"{t}CustomCurveSelected", 1) },
            };
        }

        private static Section[] Gyro()
        {
            return new[]
            {
                new Section("Uso del giroscopio",
                    Choice("GyroOutModeIndex", "Función", "Sensores al juego", "Ratón", "Stick", "Gestos", "Directo")),
                new Section("Sensores al juego",
                    Triggers("gyroControles", "Se activa con", slot => Global.GyroControlsInf[slot].triggers, (slot, v) => Global.GyroControlsInf[slot].triggers = v),
                    Choice("GyroControlsEvalCondIndex", "Botones necesarios", Conditions),
                    Toggle("GyroControlsTurns", "Encender o apagar al pulsar"),
                    Toggle("GyroControlsToggle", "Alternar")) { When = ("GyroOutModeIndex", 0) },
                new Section("Ratón con giroscopio",
                    Triggers("gyroRaton", "Se activa con", slot => Global.SATriggers[slot], (slot, v) => Global.SATriggers[slot] = v),
                    Choice("GyroMouseEvalCondIndex", "Botones necesarios", Conditions),
                    Toggle("GyroMouseTurns", "Encender o apagar al pulsar"),
                    Toggle("GyroMouseToggle", "Alternar"),
                    Number("GyroSensitivity", "Sensibilidad", 0, 500, 10),
                    Number("GyroVertScale", "Escala vertical", 0, 400, 10),
                    Number("GyroMouseDeadZone", "Zona muerta", 0, 1000, 1),
                    Number("GyroMouseMinThreshold", "Umbral mínimo", 1, 40, 1),
                    Choice("GyroMouseXAxis", "Eje horizontal", Yaw),
                    Toggle("GyroMouseInvertX", "Invertir horizontal"),
                    Toggle("GyroMouseInvertY", "Invertir vertical"),
                    Toggle("GyroMouseJitterCompensation", "Compensar temblor"),
                    Toggle("GyroMouseSmooth", "Suavizado"),
                    Choice("GyroMouseSmoothMethodIndex", "Método de suavizado", Smoothing),
                    Number("GyroMouseSmoothWeight", "Peso", 0, 1, 0.1),
                    Number("GyroMouseOneEuroMinCutoff", "Corte mínimo", 0.00001, 100, 0.1),
                    Number("GyroMouseOneEuroBeta", "Beta", 0, 1, 0.1)) { When = ("GyroOutModeIndex", 1) },
                new Section("Stick con giroscopio",
                    Triggers("gyroStick", "Se activa con", slot => Global.SAMousestickTriggers[slot], (slot, v) => Global.SAMousestickTriggers[slot] = v),
                    Choice("GyroMouseStickEvalCondIndex", "Botones necesarios", Conditions),
                    Toggle("GyroMouseStickTurns", "Encender o apagar al pulsar"),
                    Toggle("GyroMouseStickToggle", "Alternar"),
                    Choice("GyroMouseStickOutputStick", "Stick de salida", OutputStick),
                    Choice("GyroMouseStickOutputAxes", "Ejes", OutputAxes),
                    Number("GyroMouseStickDeadZone", "Zona muerta", 0, 1000, 1),
                    Number("GyroMouseStickMaxZone", "Recorrido máximo", 1, 1000, 1),
                    Number("GyroMouseStickAntiDeadX", "Compensar zona muerta horizontal", 0, 100, 1),
                    Number("GyroMouseStickAntiDeadY", "Compensar zona muerta vertical", 0, 100, 1),
                    Number("GyroMouseStickVertScale", "Escala vertical", 0, 400, 10),
                    Toggle("GyroMouseStickMaxOutputEnabled", "Limitar salida"),
                    Number("GyroMouseStickMaxOutput", "Salida máxima", 0, 100, 1),
                    Choice("GyroMouseStickXAxis", "Eje horizontal", Yaw),
                    Toggle("GyroMouseStickInvertX", "Invertir horizontal"),
                    Toggle("GyroMouseStickInvertY", "Invertir vertical"),
                    Toggle("GyroMouseStickJitterCompensation", "Compensar temblor"),
                    Toggle("GyroMouseStickSmooth", "Suavizado"),
                    Choice("GyroMouseStickSmoothMethodIndex", "Método de suavizado", Smoothing),
                    Number("GyroMouseStickOneEuroMinCutoff", "Corte mínimo", 0.00001, 100, 0.1),
                    Number("GyroMouseStickOneEuroBeta", "Beta", 0, 1, 0.1)) { When = ("GyroOutModeIndex", 2) },
                new Section("Gestos con giroscopio",
                    Triggers("gyroGestos", "Se activa con", slot => Global.GyroSwipeInf[slot].triggers, (slot, v) => Global.GyroSwipeInf[slot].triggers = v),
                    Choice("GyroSwipeEvalCondIndex", "Botones necesarios", Conditions),
                    Toggle("GyroSwipeTurns", "Encender o apagar al pulsar"),
                    Choice("GyroSwipeXAxis", "Eje horizontal", Yaw),
                    Number("GyroSwipeDeadZoneX", "Zona muerta horizontal", 0, 2000, 10),
                    Number("GyroSwipeDeadZoneY", "Zona muerta vertical", 0, 2000, 10),
                    Number("GyroSwipeDelayTime", "Retardo (milisegundos)", 0, 5000, 10)) { When = ("GyroOutModeIndex", 3) },
                new Section("Sensibilidad de sensores",
                    Number("SXDeadZone", "Zona muerta lateral", 0, 1, 0.01, true),
                    Number("SZDeadZone", "Zona muerta frontal", 0, 1, 0.01, true),
                    Number("SXMaxZone", "Recorrido lateral", 0, 1, 0.01, true),
                    Number("SZMaxZone", "Recorrido frontal", 0, 1, 0.01, true),
                    Number("SXAntiDeadZone", "Compensación lateral", 0, 1, 0.01, true),
                    Number("SZAntiDeadZone", "Compensación frontal", 0, 1, 0.01, true),
                    Number("SXSens", "Sensibilidad lateral", 0, 5, 0.1),
                    Number("SZSens", "Sensibilidad frontal", 0, 5, 0.1),
                    Choice("SXOutputCurveIndex", "Curva lateral", Curves),
                    Choice("SZOutputCurveIndex", "Curva frontal", Curves)),
                new Section("Curva lateral personalizada", Curve("SXCustomCurve", "Curva")) { When = ("SXCustomCurveSelected", 1) },
                new Section("Curva frontal personalizada", Curve("SZCustomCurve", "Curva")) { When = ("SZCustomCurveSelected", 1) },
            };
        }

        private static Section[] Touchpad()
        {
            return new[]
            {
                new Section("Uso del panel",
                    Choice("TouchpadOutputIndex", "Función", "Ratón", "Controles", "Stick", "Ratón absoluto", "Directo"),
                    Toggle("EnableTouchpadToggle", "Encender o apagar con PS + panel"),
                    Toggle("StartTouchpadOff", "Empezar apagado"),
                    Toggle("TouchpadClickPassthru", "Pasar el clic al juego"),
                    Toggle("LowerRightTouchRMB", "Esquina inferior derecha: clic derecho")),
                new Section("Ratón con el panel",
                    Number("TouchSens", "Sensibilidad", 0, 500, 1),
                    Number("TouchScroll", "Desplazamiento con dos dedos", 0, 255, 1),
                    Number("TouchTap", "Toque para clic", 0, 500, 1),
                    Toggle("TouchDoubleTap", "Doble toque"),
                    Toggle("TouchJitter", "Compensar temblor"),
                    Choice("TouchInvertIndex", "Invertir", "No", "Horizontal", "Vertical", "Ambos"),
                    Number("TouchRelMouseRotation", "Rotación (grados)", -180, 180, 1),
                    Number("TouchRelMouseMinThreshold", "Umbral mínimo", 1, 40, 1),
                    Toggle("TouchTrackball", "Inercia"),
                    Number("TouchTrackballFriction", "Fricción de la inercia", 0.1, 20, 0.1)) { When = ("TouchpadOutputIndex", 0) },
                new Section("Stick con el panel",
                    Choice("TouchMouseStickOutputStick", "Stick de salida", OutputStick),
                    Choice("TouchMouseStickOutputAxes", "Ejes", OutputAxes),
                    Number("TouchMouseStickDeadZone", "Zona muerta", 0, 1000, 1),
                    Number("TouchMouseStickMaxZone", "Recorrido máximo", 1, 1000, 1),
                    Number("TouchMouseStickAntiDeadX", "Compensar zona muerta horizontal", 0, 100, 1),
                    Number("TouchMouseStickAntiDeadY", "Compensar zona muerta vertical", 0, 100, 1),
                    Number("TouchMouseStickVertScale", "Escala vertical", 0, 400, 10),
                    Toggle("TouchMouseStickMaxOutputEnabled", "Limitar salida"),
                    Number("TouchMouseStickMaxOutput", "Salida máxima", 0, 100, 1),
                    Toggle("TouchMouseStickInvertX", "Invertir horizontal"),
                    Toggle("TouchMouseStickInvertY", "Invertir vertical"),
                    Number("TouchMouseStickRotation", "Rotación (grados)", -180, 180, 1),
                    Toggle("TouchMouseStickSmooth", "Suavizado"),
                    Toggle("TouchMouseStickTrackball", "Inercia"),
                    Number("TouchMouseStickTrackballFriction", "Fricción de la inercia", 0.1, 20, 0.1)) { When = ("TouchpadOutputIndex", 2) },
                new Section("Ratón absoluto",
                    Number("TouchAbsMouseMaxZoneX", "Recorrido horizontal", 0, 100, 1),
                    Number("TouchAbsMouseMaxZoneY", "Recorrido vertical", 0, 100, 1),
                    Toggle("TouchAbsMouseSnapCenter", "Volver al centro")) { When = ("TouchpadOutputIndex", 3) },
            };
        }

        private static Section[] Light()
        {
            return new[]
            {
                new Section("Avisos",
                    Choice("LightbarModeIndex", "Quién controla la luz", "DS5Control", "El juego"),
                    Number("FlashAt", "Parpadear con batería por debajo de (%)", 0, 100, 5),
                    Choice("FlashTypeIndex", "Forma de avisar", "Parpadeo", "Pulso"),
                    Choice("ChargingType", "Luz mientras carga", "Normal", "Pulso", "Arcoíris", "Color")),
                new Section("Color con batería baja",
                    Number("LowColorR", "Rojo", 0, 255, 1),
                    Number("LowColorG", "Verde", 0, 255, 1),
                    Number("LowColorB", "Azul", 0, 255, 1)),
                new Section("Arcoíris",
                    Number("MaxSatRainbow", "Saturación máxima", 0, 100, 1)),
            };
        }

        private static Option Toggle(string id, string name) => new Option(id, name, "interruptor");

        private static Option Number(string id, string name, double min, double max, double step, bool percent = false)
        {
            return new Option(id, name, "numero") { Min = min, Max = max, Step = step, Percent = percent };
        }

        private static Option Choice(string id, string name, params string[] choices)
        {
            return new Option(id, name, "opciones") { Choices = choices };
        }

        private static Option Curve(string id, string name) => new Option(id, name, "curva");

        /// <summary>A set of buttons stored as comma separated indexes, where -1 means always on.</summary>
        private static Option Triggers(string id, string name, Func<int, string> read, Action<int, string> write)
        {
            return new Option(id, name, "botones")
            {
                Choices = Buttons,
                Getter = (_, slot) => (read(slot) ?? "-1").Split(',', StringSplitOptions.RemoveEmptyEntries)
                    .Select(item => int.TryParse(item, out int index) ? index : -1)
                    .Where(index => index >= 0).ToArray(),
                Setter = (_, slot, value) =>
                {
                    int[] indexes = value.EnumerateArray().Select(item => item.GetInt32())
                        .Where(index => index >= 0 && index < Buttons.Length).OrderBy(index => index).ToArray();
                    write(slot, indexes.Length == 0 ? "-1" : string.Join(",", indexes));
                },
            };
        }

        private static PropertyInfo Property(string name)
        {
            return typeof(ProfileSettingsViewModel).GetProperty(name)
                ?? throw new InvalidOperationException($"Unknown profile setting {name}");
        }

        private sealed class Group
        {
            public Group(string id, string name, params Section[] sections)
            {
                Id = id;
                Name = name;
                Sections = sections;
            }

            public string Id { get; }
            public string Name { get; }
            public Section[] Sections { get; }
        }

        private sealed class Section
        {
            public Section(string name, params Option[] options)
            {
                Name = name;
                Options = options;
            }

            public string Name { get; }
            public Option[] Options { get; }

            /// <summary>The section only shows while this property has this value.</summary>
            public (string Property, int Value)? When { get; init; }

            public bool Visible(ProfileSettingsViewModel profile)
            {
                if (When == null) return true;

                object value = Property(When.Value.Property).GetValue(profile);
                int current = value is bool flag ? (flag ? 1 : 0) : Convert.ToInt32(value, CultureInfo.InvariantCulture);
                return current == When.Value.Value;
            }
        }

        private sealed class Option
        {
            public Option(string id, string name, string kind)
            {
                Id = id;
                Name = name;
                Kind = kind;
            }

            public string Id { get; }
            public string Name { get; }
            public string Kind { get; }
            public double Min { get; init; }
            public double Max { get; init; }
            public double Step { get; init; }
            public bool Percent { get; init; }
            public string[] Choices { get; init; }
            public Func<ProfileSettingsViewModel, int, object> Getter { get; init; }
            public Action<ProfileSettingsViewModel, int, JsonElement> Setter { get; init; }

            public object Read(ProfileSettingsViewModel profile, int slot)
            {
                if (Getter != null) return Getter(profile, slot);

                object value = Property(Id).GetValue(profile);
                return value is Enum ? Convert.ToInt32(value, CultureInfo.InvariantCulture) : value;
            }

            public void Write(ProfileSettingsViewModel profile, int slot, JsonElement value)
            {
                if (Setter != null)
                {
                    Setter(profile, slot, value);
                    return;
                }

                PropertyInfo property = Property(Id);
                Type type = property.PropertyType;
                object converted = value.ValueKind switch
                {
                    JsonValueKind.True or JsonValueKind.False => value.GetBoolean(),
                    JsonValueKind.String => value.GetString(),
                    _ when type.IsEnum => Enum.ToObject(type, value.GetInt32()),
                    _ => Convert.ChangeType(value.GetDouble(), type, CultureInfo.InvariantCulture),
                };
                property.SetValue(profile, converted);
            }
        }
    }
}
