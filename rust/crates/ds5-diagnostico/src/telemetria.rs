//! El bloque de uso que guarda el mando (dispositivo 0x70): horas, cargas, pulsaciones,
//! conexiones y errores. Es `UsoMando` en la interfaz.

use serde_json::{json, Value};

use crate::formato::{u16_en, u32_en};

/// El bloque tal y como llega, con el byte de estado delante. C# pide al menos 190 bytes, pero lee
/// hasta el 190 incluido; con menos de 191 no se puede interpretar.
pub const LARGO_MINIMO: usize = 191;

/// La memoria sin escribir se lee con valores altos, que no deben parecer cuentas de verdad.
pub fn cuenta(datos: &[u8], posicion: usize) -> i64 {
    let valor = u32_en(datos, posicion);
    let bajo = valor & 0xFFFF;
    let alto = valor >> 16;
    if alto == 0 {
        i64::from(bajo)
    } else if alto >= 0xFF00 && bajo >= 0xFF00 {
        0
    } else {
        i64::from(bajo)
    }
}

/// Los tres bytes bajos, salvo que parezcan memoria sin escribir.
pub fn bajo(datos: &[u8], posicion: usize) -> i64 {
    let valor = u32_en(datos, posicion) & 0xFF_FFFF;
    if valor < 0xFF_FF00 {
        i64::from(valor)
    } else {
        0
    }
}

/// Dos bytes, o solo el primero si el segundo parece memoria sin escribir.
pub fn cuenta_corta(datos: &[u8], posicion: usize) -> i64 {
    if datos[posicion + 1] < 0xE0 {
        i64::from(u16_en(datos, posicion))
    } else {
        i64::from(datos[posicion])
    }
}

fn fila(nombre: &str, valor: i64) -> Value {
    json!({ "nombre": nombre, "valor": valor })
}

/// Las pulsaciones de cada control, en el orden de la interfaz, con su posición en el bloque.
const PULSACIONES: &[(&str, usize)] = &[
    ("Cruz", 130),
    ("Círculo", 138),
    ("Cuadrado", 134),
    ("Triángulo", 126),
    ("L1", 142),
    ("R1", 154),
    ("L2", 146),
    ("R2", 158),
    ("L3", 150),
    ("R3", 162),
    ("Arriba", 110),
    ("Abajo", 114),
    ("Izquierda", 118),
    ("Derecha", 122),
    ("Options", 174),
    ("Crear", 178),
    ("PS", 182),
    ("Mute", 186),
    ("Toques del panel", 166),
    ("Clic del panel", 170),
];

/// Interpreta el bloque leído (con su byte de estado delante). `None` si es demasiado corto.
pub fn telemetria(bloque: &[u8]) -> Option<Value> {
    if bloque.len() < LARGO_MINIMO {
        return None;
    }
    // El primer byte es un estado; el bloque empieza detrás.
    let d = &bloque[1..];
    let u16_fila = |nombre: &str, posicion: usize| fila(nombre, i64::from(u16_en(d, posicion)));
    let u32_fila = |nombre: &str, posicion: usize| fila(nombre, i64::from(u32_en(d, posicion)));

    let pulsaciones: Vec<Value> = PULSACIONES
        .iter()
        .map(|&(nombre, posicion)| fila(nombre, cuenta(d, posicion)))
        .collect();

    Some(json!({
        "registros": u32_en(d, 17),
        "activo": bajo(d, 21),
        "carga": bajo(d, 25),
        "haptico": bajo(d, 29),
        "gatilloIzq": cuenta(d, 33),
        "gatilloDer": cuenta(d, 37),
        "ciclosCarga": cuenta_corta(d, 41),
        "cargasCompletas": u16_en(d, 60),
        "conexion": [
            u16_fila("Conexiones USB", 66),
            u16_fila("Conexiones Bluetooth", 68),
            u16_fila("Tiempos de espera Bluetooth", 70),
            u16_fila("USB SDP (puerto de PC)", 43),
            fila("USB CDP (puerto de carga)", cuenta_corta(d, 45)),
            u16_fila("USB DCP (cargador)", 47),
            u16_fila("USB-C 1,5 A", 49),
            u16_fila("USB-C 3 A", 51),
            u16_fila("USB desconocido", 53),
            u16_fila("Conexiones del cargador", 55),
            u16_fila("Cascos detectados", 62),
            u16_fila("Auriculares detectados", 64),
            fila("Intentos de autenticación", cuenta(d, 74)),
            fila("Autenticaciones correctas", cuenta(d, 78)),
        ],
        "errores": [
            fila("Voltaje", i64::from(d[57])),
            fila("Temperatura", i64::from(d[58])),
            fila("Circuito de carga (PMIC)", i64::from(d[59])),
            u16_fila("Procesador secundario", 72),
            fila("Autenticación", cuenta(d, 82)),
        ],
        "pulsaciones": pulsaciones,
        "sticks": [
            u32_fila("Izquierdo, ciclos X", 86),
            u16_fila("Izquierdo, rango X", 90),
            u32_fila("Izquierdo, ciclos Y", 92),
            u16_fila("Izquierdo, rango Y", 96),
            u32_fila("Derecho, ciclos X", 98),
            u16_fila("Derecho, rango X", 102),
            u32_fila("Derecho, ciclos Y", 104),
            u16_fila("Derecho, rango Y", 108),
        ],
    }))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Escribe en el bloque contando desde detrás del byte de estado, como las posiciones de C#.
    pub struct Bloque(pub Vec<u8>);

    impl Bloque {
        pub fn nuevo() -> Bloque {
            let mut b = vec![0u8; 224];
            b[0] = 0x55;
            Bloque(b)
        }
        pub fn u8(&mut self, posicion: usize, valor: u8) -> &mut Self {
            self.0[posicion + 1] = valor;
            self
        }
        pub fn u16(&mut self, posicion: usize, valor: u16) -> &mut Self {
            self.0[posicion + 1..posicion + 3].copy_from_slice(&valor.to_le_bytes());
            self
        }
        pub fn u32(&mut self, posicion: usize, valor: u32) -> &mut Self {
            self.0[posicion + 1..posicion + 5].copy_from_slice(&valor.to_le_bytes());
            self
        }
    }

    fn filas(v: &Value) -> Vec<(String, i64)> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    f["nombre"].as_str().unwrap().to_string(),
                    f["valor"].as_i64().unwrap(),
                )
            })
            .collect()
    }

    fn valor(v: &Value, nombre: &str) -> i64 {
        filas(v).into_iter().find(|(n, _)| n == nombre).unwrap().1
    }

    #[test]
    fn cuentas() {
        let c = |v: u32| cuenta(&v.to_le_bytes(), 0);
        assert_eq!(c(0x0000_1234), 0x1234);
        assert_eq!(c(0x0000_FFFF), 0xFFFF);
        assert_eq!(c(0xFFFF_FFFF), 0);
        assert_eq!(c(0xFF00_FF00), 0);
        assert_eq!(c(0xFF00_FEFF), 0xFEFF);
        assert_eq!(c(0xFEFF_FF00), 0xFF00);
        assert_eq!(c(0x0001_0005), 5);
    }

    #[test]
    fn tres_bytes_bajos() {
        let b = |v: u32| bajo(&v.to_le_bytes(), 0);
        assert_eq!(b(0x0012_3456), 0x12_3456);
        assert_eq!(b(0xAB12_3456), 0x12_3456);
        assert_eq!(b(0x00FF_FEFF), 0xFF_FEFF);
        assert_eq!(b(0x00FF_FF00), 0);
        assert_eq!(b(0xFFFF_FFFF), 0);
    }

    #[test]
    fn cuentas_cortas() {
        assert_eq!(cuenta_corta(&[0x34, 0x12], 0), 0x1234);
        assert_eq!(cuenta_corta(&[0x34, 0xDF], 0), 0xDF34);
        assert_eq!(cuenta_corta(&[0x34, 0xE0], 0), 0x34);
        assert_eq!(cuenta_corta(&[0xFF, 0xFF], 0), 0xFF);
    }

    #[test]
    fn bloque_corto() {
        assert_eq!(telemetria(&[0u8; 190]), None);
        assert_eq!(telemetria(&[]), None);
        assert!(telemetria(&[0u8; 191]).is_some());
    }

    #[test]
    fn bloque_vacio_todo_a_cero() {
        let t = telemetria(&Bloque::nuevo().0).unwrap();
        assert_eq!(t["registros"], 0);
        assert_eq!(t["conexion"].as_array().unwrap().len(), 14);
        assert_eq!(t["errores"].as_array().unwrap().len(), 5);
        assert_eq!(t["pulsaciones"].as_array().unwrap().len(), 20);
        assert_eq!(t["sticks"].as_array().unwrap().len(), 8);
    }

    #[test]
    fn interpreta_cada_campo() {
        let mut b = Bloque::nuevo();
        b.u32(17, 0x0102_0304)
            .u32(21, 0x00AB_CDEF)
            .u32(25, 0xFFFF_FFFF)
            .u32(29, 1000)
            .u32(33, 0xFFFF_FFFF)
            .u32(37, 0x0002_0007)
            .u16(41, 0xE0_05)
            .u16(43, 11)
            .u16(45, 0x0102)
            .u16(47, 13)
            .u16(49, 14)
            .u16(51, 15)
            .u16(53, 16)
            .u16(55, 17)
            .u8(57, 1)
            .u8(58, 2)
            .u8(59, 3)
            .u16(60, 450)
            .u16(62, 18)
            .u16(64, 19)
            .u16(66, 20)
            .u16(68, 21)
            .u16(70, 22)
            .u16(72, 23)
            .u32(74, 24)
            .u32(78, 25)
            .u32(82, 0xFF00_FF00)
            .u32(86, 0x0001_0000)
            .u16(90, 31)
            .u32(92, 32)
            .u16(96, 33)
            .u32(98, 34)
            .u16(102, 35)
            .u32(104, 36)
            .u16(108, 37);
        for (i, &(_, posicion)) in PULSACIONES.iter().enumerate() {
            b.u32(posicion, 100 + i as u32);
        }
        let t = telemetria(&b.0).unwrap();

        assert_eq!(t["registros"], 0x0102_0304);
        assert_eq!(t["activo"], 0xAB_CDEF);
        assert_eq!(t["carga"], 0);
        assert_eq!(t["haptico"], 1000);
        assert_eq!(t["gatilloIzq"], 0);
        assert_eq!(t["gatilloDer"], 7);
        assert_eq!(t["ciclosCarga"], 5);
        assert_eq!(t["cargasCompletas"], 450);

        assert_eq!(
            filas(&t["conexion"]),
            [
                ("Conexiones USB", 20),
                ("Conexiones Bluetooth", 21),
                ("Tiempos de espera Bluetooth", 22),
                ("USB SDP (puerto de PC)", 11),
                ("USB CDP (puerto de carga)", 0x0102),
                ("USB DCP (cargador)", 13),
                ("USB-C 1,5 A", 14),
                ("USB-C 3 A", 15),
                ("USB desconocido", 16),
                ("Conexiones del cargador", 17),
                ("Cascos detectados", 18),
                ("Auriculares detectados", 19),
                ("Intentos de autenticación", 24),
                ("Autenticaciones correctas", 25),
            ]
            .map(|(n, v)| (n.to_string(), v))
        );
        assert_eq!(
            filas(&t["errores"]),
            [
                ("Voltaje", 1),
                ("Temperatura", 2),
                ("Circuito de carga (PMIC)", 3),
                ("Procesador secundario", 23),
                ("Autenticación", 0),
            ]
            .map(|(n, v)| (n.to_string(), v))
        );
        assert_eq!(
            filas(&t["sticks"]),
            [
                ("Izquierdo, ciclos X", 0x0001_0000),
                ("Izquierdo, rango X", 31),
                ("Izquierdo, ciclos Y", 32),
                ("Izquierdo, rango Y", 33),
                ("Derecho, ciclos X", 34),
                ("Derecho, rango X", 35),
                ("Derecho, ciclos Y", 36),
                ("Derecho, rango Y", 37),
            ]
            .map(|(n, v)| (n.to_string(), v))
        );
        let p = filas(&t["pulsaciones"]);
        let nombres: Vec<&str> = p.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            nombres,
            [
                "Cruz",
                "Círculo",
                "Cuadrado",
                "Triángulo",
                "L1",
                "R1",
                "L2",
                "R2",
                "L3",
                "R3",
                "Arriba",
                "Abajo",
                "Izquierda",
                "Derecha",
                "Options",
                "Crear",
                "PS",
                "Mute",
                "Toques del panel",
                "Clic del panel"
            ]
        );
        assert_eq!(valor(&t["pulsaciones"], "Cruz"), 100);
        assert_eq!(valor(&t["pulsaciones"], "Clic del panel"), 119);
    }

    #[test]
    fn pulsaciones_en_su_sitio() {
        let mut b = Bloque::nuevo();
        b.u32(130, 42).u32(186, 0xFFFF_FFFF).u32(110, 0x0003_0009);
        let t = telemetria(&b.0).unwrap();
        assert_eq!(valor(&t["pulsaciones"], "Cruz"), 42);
        assert_eq!(valor(&t["pulsaciones"], "Mute"), 0);
        assert_eq!(valor(&t["pulsaciones"], "Arriba"), 9);
        assert_eq!(valor(&t["pulsaciones"], "Círculo"), 0);
    }

    #[test]
    fn registros_es_u32_entero() {
        let mut b = Bloque::nuevo();
        b.u32(17, u32::MAX);
        let t = telemetria(&b.0).unwrap();
        assert_eq!(t["registros"], u64::from(u32::MAX));
    }
}
