using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading;

namespace DS5Control.InputDevices
{
    /// <summary>
    /// Reads what a DualSense stores about itself through its factory test commands: a command
    /// goes out in feature report 0x80 and the answer comes back in 0x81. Only commands that
    /// read are used. Protocol documented by the dualsense-tester project.
    /// </summary>
    internal sealed class DualSenseDiagnostics
    {
        private const byte CommandReport = 0x80;
        private const byte ResultReport = 0x81;
        private const byte BluetoothPatchReport = 0x22;
        private const byte BluetoothChecksumSeed = 0x53;
        private const int ReportLength = 64;
        private const int PageSize = 56;
        private const int PayloadOffset = 4;
        private const int TimeoutMs = 1000;

        private const byte StatusComplete = 2;
        private const byte StatusMorePages = 3;

        private const byte DeviceSystem = 1;
        private const byte DeviceAnalog = 4;
        private const byte DeviceTouch = 5;
        private const byte DeviceBluetooth = 9;
        private const byte DeviceTelemetry = 0x70;

        private const int TelemetryPages = 4;
        /// <summary>Older firmware answers with an incomplete block.</summary>
        private const int MinimumTelemetryUpdate = 1106;
        private const int MinimumTraceabilityHardware = 777;
        private const uint MinimumTraceabilityFirmware = 65655;

        private static readonly Dictionary<string, string> Colors = new Dictionary<string, string>
        {
            ["00"] = "Blanco", ["01"] = "Negro medianoche", ["02"] = "Rojo cósmico", ["03"] = "Rosa nova",
            ["04"] = "Púrpura galáctico", ["05"] = "Azul estelar", ["06"] = "Camuflaje gris", ["07"] = "Rojo volcánico",
            ["08"] = "Plata esterlina", ["09"] = "Azul cobalto", ["10"] = "Chroma Teal", ["11"] = "Chroma Indigo",
            ["12"] = "Chroma Pearl", ["13"] = "Rojo techno", ["30"] = "30 aniversario",
            ["Z1"] = "God of War Ragnarök", ["Z2"] = "Spider-Man 2", ["Z3"] = "Astro Bot", ["Z4"] = "Fortnite",
            ["Z5"] = "Monster Hunter Wilds", ["Z6"] = "The Last of Us", ["Z7"] = "Ghost of Yotei",
            ["ZB"] = "Icon Blue", ["ZC"] = "Astro Bot Joyful", ["ZE"] = "Genshin Impact",
        };

        private readonly HidDevice device;
        private readonly bool bluetooth;
        private readonly DualSenseDevice controller;

        public DualSenseDiagnostics(DualSenseDevice controller)
        {
            this.controller = controller;
            device = controller.HidDevice;
            bluetooth = controller.getConnectionType() == ConnectionType.BT;
        }

        public object Read()
        {
            int? millivolts = Read(DeviceAnalog, 3, 4) is { } battery ? BitConverter.ToUInt16(battery, 0) : null;
            return new
            {
                milivoltios = millivolts,
                fabrica = Factory(millivolts),
                uso = Telemetry(),
            };
        }

        private List<object> Factory(int? millivolts)
        {
            byte[] info = controller.FirmwareInfo;
            var rows = new List<object>();
            void Add(string name, string value)
            {
                if (!string.IsNullOrEmpty(value)) rows.Add(new { nombre = name, valor = value });
            }

            Add("Firmware compilado", controller.FirmwareBuild);
            Add("Hardware", $"0x{controller.HardwareInfo:X8}");
            Add("Dispositivo", info == null ? null : "0x" + Hex(info.Skip(32).Take(12).Reverse().ToArray()));
            int firmwareType = info == null ? 0 : BitConverter.ToUInt16(info, 20);
            Add("Tipo de firmware", $"0x{firmwareType:X4}");
            Add("Serie de software", info == null ? null : $"0x{BitConverter.ToUInt16(info, 22):X4}");
            Add("Actualización", $"{controller.UpdateFirmware >> 8:X}.{controller.UpdateFirmware & 0xFF:X2}");
            Add("Arranque (SBL)", ThreePart(controller.BootloaderFirmware));
            Add("Firmware principal", ThreePart(controller.MainFirmware));
            Add("Firmware DSP", $"{controller.AudioFirmware >> 16:X4}_{controller.AudioFirmware & 0xFFFF:X4}");
            Add("Firmware MCU DSP", info == null ? null : ThreePart(BitConverter.ToUInt32(info, 56)));
            Add("Parche de Bluetooth", BluetoothPatch());

            bool traceable = (firmwareType == 2 || firmwareType == 3) &&
                (controller.HardwareInfo & 0xFFFF) >= MinimumTraceabilityHardware &&
                controller.MainFirmware >= MinimumTraceabilityFirmware;
            if (traceable)
            {
                Add("PCBA", Reversed(Text(Read(DeviceSystem, 17, 24))));
                string serial = Text(Read(DeviceSystem, 19, 32));
                Add("Número de serie", serial);
                if (serial?.Length >= 6)
                {
                    Add("Color", Colors.GetValueOrDefault(serial.Substring(4, 2), "Desconocido"));
                    char board = serial[1];
                    Add("Placa", board >= '1' && board <= '5' ? $"BDM-0{board}0" : "Desconocida");
                }
                byte[] parts = Read(DeviceSystem, 21, 32);
                Add("Piezas de montaje", parts == null ? null : "0x" + Hex(parts.Reverse().ToArray()));
                Add("Código de batería", Text(Read(DeviceSystem, 24, 32)));
                Add("Motor izquierdo", Text(Read(DeviceSystem, 26, 32)));
                Add("Motor derecho", Text(Read(DeviceSystem, 28, 32)));
            }
            else
            {
                byte[] pcba = Read(DeviceSystem, 4, 6);
                Add("PCBA", pcba == null ? null : "0x" + Hex(pcba.Reverse().ToArray()));
            }

            byte[] unique = Read(DeviceSystem, 9, 9);
            if (unique != null && unique[0] == 0) Add("ID único", $"0x{BitConverter.ToUInt64(unique, 1):X16}");

            byte[] address = Read(DeviceBluetooth, 2, 6);
            Add("MAC Bluetooth", address == null ? null : string.Join(":", address.Reverse().Select(b => b.ToString("X2"))));

            // The touchpad only answers over USB.
            if (!bluetooth)
            {
                byte[] touch = Read(DeviceTouch, 2, 8);
                Add("Panel táctil", touch == null ? null : "0x" + Hex(touch));
                byte[] touchVersion = Read(DeviceTouch, 4, 8);
                Add("Firmware del panel", touchVersion == null ? null : "0x" + Hex(touchVersion));
            }

            if (millivolts != null) Add("Voltaje de batería", $"{millivolts / 1000.0:0.00} V");
            return rows;
        }

        private object Telemetry()
        {
            if (controller.UpdateFirmware <= MinimumTelemetryUpdate) return null;

            byte[] data = ReadPages(DeviceTelemetry, 1, TelemetryPages);
            if (data == null || data.Length < 190) return null;

            // The first byte is a status; the block starts after it.
            data = data.Skip(1).ToArray();

            return new
            {
                registros = BitConverter.ToUInt32(data, 17),
                activo = Low(data, 21),
                carga = Low(data, 25),
                haptico = Low(data, 29),
                gatilloIzq = Count(data, 33),
                gatilloDer = Count(data, 37),
                ciclosCarga = ShortCount(data, 41),
                cargasCompletas = BitConverter.ToUInt16(data, 60),
                conexion = new object[]
                {
                    Row("Conexiones USB", BitConverter.ToUInt16(data, 66)),
                    Row("Conexiones Bluetooth", BitConverter.ToUInt16(data, 68)),
                    Row("Tiempos de espera Bluetooth", BitConverter.ToUInt16(data, 70)),
                    Row("USB SDP (puerto de PC)", BitConverter.ToUInt16(data, 43)),
                    Row("USB CDP (puerto de carga)", ShortCount(data, 45)),
                    Row("USB DCP (cargador)", BitConverter.ToUInt16(data, 47)),
                    Row("USB-C 1,5 A", BitConverter.ToUInt16(data, 49)),
                    Row("USB-C 3 A", BitConverter.ToUInt16(data, 51)),
                    Row("USB desconocido", BitConverter.ToUInt16(data, 53)),
                    Row("Conexiones del cargador", BitConverter.ToUInt16(data, 55)),
                    Row("Cascos detectados", BitConverter.ToUInt16(data, 62)),
                    Row("Auriculares detectados", BitConverter.ToUInt16(data, 64)),
                    Row("Intentos de autenticación", Count(data, 74)),
                    Row("Autenticaciones correctas", Count(data, 78)),
                },
                errores = new object[]
                {
                    Row("Voltaje", data[57]),
                    Row("Temperatura", data[58]),
                    Row("Circuito de carga (PMIC)", data[59]),
                    Row("Procesador secundario", BitConverter.ToUInt16(data, 72)),
                    Row("Autenticación", Count(data, 82)),
                },
                pulsaciones = new (string Name, int Offset)[]
                {
                    ("Cruz", 130), ("Círculo", 138), ("Cuadrado", 134), ("Triángulo", 126),
                    ("L1", 142), ("R1", 154), ("L2", 146), ("R2", 158), ("L3", 150), ("R3", 162),
                    ("Arriba", 110), ("Abajo", 114), ("Izquierda", 118), ("Derecha", 122),
                    ("Options", 174), ("Crear", 178), ("PS", 182), ("Mute", 186),
                    ("Toques del panel", 166), ("Clic del panel", 170),
                }.Select(item => Row(item.Name, Count(data, item.Offset))).ToArray(),
                sticks = new object[]
                {
                    Row("Izquierdo, ciclos X", BitConverter.ToUInt32(data, 86)),
                    Row("Izquierdo, rango X", BitConverter.ToUInt16(data, 90)),
                    Row("Izquierdo, ciclos Y", BitConverter.ToUInt32(data, 92)),
                    Row("Izquierdo, rango Y", BitConverter.ToUInt16(data, 96)),
                    Row("Derecho, ciclos X", BitConverter.ToUInt32(data, 98)),
                    Row("Derecho, rango X", BitConverter.ToUInt16(data, 102)),
                    Row("Derecho, ciclos Y", BitConverter.ToUInt32(data, 104)),
                    Row("Derecho, rango Y", BitConverter.ToUInt16(data, 108)),
                },
            };
        }

        private string BluetoothPatch()
        {
            var report = new byte[ReportLength];
            report[0] = BluetoothPatchReport;
            return device.readFeatureData(report) && report[0] == BluetoothPatchReport
                ? $"0x{BitConverter.ToUInt32(report, 31):X8}" : null;
        }

        private static object Row(string name, long value) => new { nombre = name, valor = value };

        /// <summary>Unwritten memory reads as high values, which must not look like real counts.</summary>
        private static long Count(byte[] data, int offset)
        {
            uint value = BitConverter.ToUInt32(data, offset);
            uint low = value & 0xFFFF;
            uint high = value >> 16;
            if (high == 0) return low;
            return high >= 0xFF00 && low >= 0xFF00 ? 0 : low;
        }

        private static long Low(byte[] data, int offset)
        {
            uint value = BitConverter.ToUInt32(data, offset) & 0xFFFFFF;
            return value < 0xFFFF00 ? value : 0;
        }

        private static long ShortCount(byte[] data, int offset)
        {
            return data[offset + 1] < 0xE0 ? BitConverter.ToUInt16(data, offset) : data[offset];
        }

        private static string Reversed(string text) => text == null ? null : new string(text.Reverse().ToArray());

        private static string Hex(byte[] data) => string.Concat(data.Select(b => b.ToString("X2")));

        private static string ThreePart(uint version) =>
            $"{version >> 24}.{(version >> 16) & 0xFF}.{version & 0xFFFF}";

        private static string Text(byte[] data)
        {
            return data == null ? null : Encoding.ASCII.GetString(data).TrimEnd('\0', ' ');
        }

        private bool Send(byte deviceId, byte actionId)
        {
            var report = new byte[ReportLength];
            report[0] = CommandReport;
            report[1] = deviceId;
            report[2] = actionId;

            if (bluetooth)
            {
                int position = ReportLength - 4;
                uint hash = ~Crc32Algorithm.Compute(new[] { BluetoothChecksumSeed });
                hash = ~Crc32Algorithm.CalculateBasicHash(ref hash, ref report, 0, position);
                BitConverter.GetBytes(hash).CopyTo(report, position);
            }
            return device.WriteFeatureReport(report);
        }

        /// <summary>Returns null when the controller does not answer this command.</summary>
        private byte[] Read(byte deviceId, byte actionId, int length)
        {
            if (!Send(deviceId, actionId)) return null;

            var result = new byte[length];
            int page = 0;
            DateTime limit = DateTime.UtcNow.AddMilliseconds(TimeoutMs);
            while (DateTime.UtcNow < limit)
            {
                byte? status = ReadResult(deviceId, actionId, out byte[] report);
                if (status == StatusComplete || status == StatusMorePages)
                {
                    int offset = PageSize * page;
                    int count = Math.Min(PageSize, length - offset);
                    if (count > 0) Array.Copy(report, PayloadOffset, result, offset, count);
                    page++;
                    if (status == StatusComplete) return result;
                    limit = DateTime.UtcNow.AddMilliseconds(TimeoutMs);
                    continue;
                }
                Thread.Sleep(10);
            }
            return null;
        }

        /// <summary>A paged block never reports completion: its header just clears when it ends.</summary>
        private byte[] ReadPages(byte deviceId, byte actionId, int pages)
        {
            if (!Send(deviceId, actionId)) return null;

            var result = new byte[pages * PageSize];
            int page = 0;
            for (int attempt = 0; attempt < pages * 4 + 8 && page < pages; attempt++)
            {
                byte? status = ReadResult(deviceId, actionId, out byte[] report);
                if (status == StatusComplete || status == StatusMorePages)
                {
                    Array.Copy(report, PayloadOffset, result, page * PageSize, PageSize);
                    page++;
                    if (status == StatusComplete) break;
                }
                else if (page > 0 && status == null)
                {
                    break;
                }
                Thread.Sleep(10);
            }
            return page > 0 ? result : null;
        }

        /// <summary>The status of the answer, or null if the report belongs to another command.</summary>
        private byte? ReadResult(byte deviceId, byte actionId, out byte[] report)
        {
            report = new byte[ReportLength];
            report[0] = ResultReport;
            if (!device.readFeatureData(report)) return null;

            bool matches = report[0] == ResultReport && report[1] == deviceId && report[2] == actionId;
            return matches ? report[3] : null;
        }
    }
}
