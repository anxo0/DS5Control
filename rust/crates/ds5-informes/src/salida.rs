//! Los informes de salida del DualSense: luz, vibración, gatillos adaptativos, luz del Mute y
//! luces de jugador, como `PrepareOutReport` en `DualSenseDevice.cs`. Por Bluetooth es el 0x31
//! con su CRC; por USB, el 0x02. Los dos llevan los mismos datos, por Bluetooth un byte más allá.

use crate::{crc, Conexion};

/// Largo del informe de salida por Bluetooth, con su CRC.
pub const LARGO_SALIDA_BT: usize = 78;
/// Largo del informe de salida por USB.
pub const LARGO_SALIDA_USB: usize = 48;

/// Los 8 bytes de un gatillo adaptativo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EfectoGatillo {
    /// 0 sin resistencia, 1 resistencia continua, 2 resistencia en un tramo.
    pub modo: u8,
    pub inicio: u8,
    pub fuerza: u8,
    pub fuerza_tramo: u8,
    pub cerca_suelto: u8,
    pub cerca_medio: u8,
    pub pulsado: u8,
    pub frecuencia: u8,
}

/// Los efectos que sabe poner la app (`TriggerEffects`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Efecto {
    Ninguno,
    Rigido,
    Pulso,
    /// Un clic al final del recorrido; `maximo` va de 0 a 255.
    ClicCompleto {
        maximo: u8,
    },
}

impl EfectoGatillo {
    /// Como `TriggerEffectData.ChangeData`.
    pub fn de(efecto: Efecto) -> EfectoGatillo {
        match efecto {
            Efecto::Ninguno => EfectoGatillo::default(),
            Efecto::Rigido => EfectoGatillo {
                modo: 0x01,
                ..Default::default()
            },
            Efecto::Pulso => EfectoGatillo {
                modo: 0x02,
                ..Default::default()
            },
            Efecto::ClicCompleto { maximo } => {
                let inicio = (0x94 as f64 * (maximo as f64 / 255.0)) as u8;
                let fuerza =
                    ((0xB4 - inicio as i32) as f64 * (maximo as f64 / 255.0) + inicio as f64) as u8;
                EfectoGatillo {
                    modo: 0x02,
                    inicio,
                    fuerza,
                    fuerza_tramo: 0xFF,
                    ..Default::default()
                }
            }
        }
    }

    fn escribir(&self, b: &mut [u8], i: usize) {
        b[i..i + 7].copy_from_slice(&[
            self.modo,
            self.inicio,
            self.fuerza,
            self.fuerza_tramo,
            self.cerca_suelto,
            self.cerca_medio,
            self.pulsado,
        ]);
        b[i + 9] = self.frecuencia;
    }

    fn leer(b: &[u8], i: usize) -> EfectoGatillo {
        EfectoGatillo {
            modo: b[i],
            inicio: b[i + 1],
            fuerza: b[i + 2],
            fuerza_tramo: b[i + 3],
            cerca_suelto: b[i + 4],
            cerca_medio: b[i + 5],
            pulsado: b[i + 6],
            frecuencia: b[i + 9],
        }
    }
}

/// Luz del botón Mute.
pub mod led_mute {
    pub const APAGADA: u8 = 0x00;
    pub const ENCENDIDA: u8 = 0x01;
    pub const PARPADEO: u8 = 0x02;
}

/// Todo lo que va en un informe de salida, con las posiciones del informe USB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Salida {
    /// Byte 1: qué se cambia. 0x01 y 0x02 los motores, 0x04 el gatillo derecho, 0x08 el izquierdo.
    pub cambios: u8,
    /// Byte 2: 0x01 luz del Mute, 0x04 luz de la barra, 0x10 luces de jugador, 0x40 potencia.
    pub cambios_luces: u8,
    /// Byte 3: motor derecho, el rápido y ligero.
    pub motor_derecho: u8,
    /// Byte 4: motor izquierdo, el lento y fuerte.
    pub motor_izquierdo: u8,
    /// Byte 9.
    pub led_mute: u8,
    /// Bytes 11 a 20.
    pub gatillo_derecho: EfectoGatillo,
    /// Bytes 22 a 31.
    pub gatillo_izquierdo: EfectoGatillo,
    /// Byte 37: cuánto se reduce la fuerza de motores (abajo) y gatillos (arriba), en octavos.
    pub potencia: u8,
    /// Byte 39: 0x02 permite el fundido, 0x04 la vibración mejorada (firmware 2.24 o más).
    pub opciones_luces: u8,
    /// Byte 42.
    pub fundido: u8,
    /// Byte 43: brillo de las luces de jugador, 0 alto, 1 medio, 2 bajo.
    pub brillo_jugador: u8,
    /// Byte 44: las cinco luces de jugador, de izquierda a derecha.
    pub luces_jugador: u8,
    /// Bytes 45 a 47.
    pub rojo: u8,
    pub verde: u8,
    pub azul: u8,
}

/// Lo que se manda en cada informe normal, como `PrepareOutReport`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ajustes {
    pub vibracion: bool,
    pub vibracion_precisa: bool,
    pub motor_derecho: u8,
    pub motor_izquierdo: u8,
    pub led_mute: u8,
    pub gatillo_derecho: EfectoGatillo,
    pub gatillo_izquierdo: EfectoGatillo,
    pub potencia: u8,
    pub luces_jugador: u8,
    pub color: (u8, u8, u8),
}

impl Default for Ajustes {
    fn default() -> Self {
        Ajustes {
            vibracion: true,
            vibracion_precisa: true,
            motor_derecho: 0,
            motor_izquierdo: 0,
            led_mute: led_mute::APAGADA,
            gatillo_derecho: EfectoGatillo::default(),
            gatillo_izquierdo: EfectoGatillo::default(),
            potencia: 0,
            luces_jugador: 0,
            color: (0, 0, 0),
        }
    }
}

/// Las luces de jugador según la batería, como `DeviceBatteryLinearMask`.
pub fn luces_por_bateria(bateria: u8) -> u8 {
    match bateria {
        95.. => 0x01 | 0x02 | 0x08 | 0x10,
        70.. => 0x01 | 0x02 | 0x08,
        50.. => 0x01 | 0x02,
        20.. => 0x01,
        5.. => 0x01 | 0x02 | 0x04,
        _ => 0x00,
    }
}

/// Las luces de jugador del mando número `ranura` (desde 0), como `CalculateDeviceSlotMask`.
pub fn luces_por_ranura(ranura: usize) -> u8 {
    match ranura {
        0 => 0x04,
        1 => 0x02 | 0x08,
        2 => 0x01 | 0x04 | 0x10,
        3 => 0x01 | 0x02 | 0x08 | 0x10,
        4 => 0x01 | 0x10,
        5 => 0x02 | 0x04 | 0x08,
        6 => 0x1F,
        _ => 0x00,
    }
}

impl Salida {
    /// El informe de cada momento.
    pub fn normal(a: &Ajustes) -> Salida {
        let motores = a.vibracion || a.vibracion_precisa;
        Salida {
            cambios: if a.vibracion { 0x0F } else { 0x0C },
            cambios_luces: 0x55,
            motor_derecho: if motores { a.motor_derecho } else { 0 },
            motor_izquierdo: if motores { a.motor_izquierdo } else { 0 },
            led_mute: a.led_mute,
            gatillo_derecho: a.gatillo_derecho,
            gatillo_izquierdo: a.gatillo_izquierdo,
            potencia: a.potencia,
            opciones_luces: if a.vibracion_precisa { 0x06 } else { 0x02 },
            fundido: 0x02,
            brillo_jugador: 0x02,
            luces_jugador: a.luces_jugador,
            rojo: a.color.0,
            verde: a.color.1,
            azul: a.color.2,
        }
    }

    /// El que se manda al soltar el mando: todo parado y la barra en blanco, como
    /// `SendEmptyOutputReport`.
    pub fn vacia(vibracion: bool) -> Salida {
        Salida {
            cambios: if vibracion { 0x0F } else { 0x0C },
            cambios_luces: 0x15,
            rojo: 0xFF,
            verde: 0xFF,
            azul: 0xFF,
            ..Default::default()
        }
    }

    /// El primero por Bluetooth, que activa el modo completo, como `SendInitialBTOutputReport`.
    pub fn inicial_bt() -> Salida {
        Salida {
            cambios_luces: 0x15,
            ..Default::default()
        }
    }

    /// El informe listo para mandar, con su CRC por Bluetooth.
    pub fn codificar(&self, conexion: Conexion) -> Vec<u8> {
        let (largo, o) = match conexion {
            Conexion::Usb => (LARGO_SALIDA_USB, 0),
            Conexion::Bt => (LARGO_SALIDA_BT, 1),
        };
        let mut b = vec![0u8; largo];
        match conexion {
            Conexion::Usb => b[0] = 0x02,
            Conexion::Bt => {
                b[0] = 0x31;
                b[1] = 0x02;
            }
        }
        b[1 + o] = self.cambios;
        b[2 + o] = self.cambios_luces;
        b[3 + o] = self.motor_derecho;
        b[4 + o] = self.motor_izquierdo;
        b[9 + o] = self.led_mute;
        self.gatillo_derecho.escribir(&mut b, 11 + o);
        self.gatillo_izquierdo.escribir(&mut b, 22 + o);
        b[37 + o] = self.potencia;
        b[39 + o] = self.opciones_luces;
        b[42 + o] = self.fundido;
        b[43 + o] = self.brillo_jugador;
        b[44 + o] = self.luces_jugador;
        b[45 + o] = self.rojo;
        b[46 + o] = self.verde;
        b[47 + o] = self.azul;
        if conexion == Conexion::Bt {
            crc::firmar(crc::CABECERA_SALIDA, &mut b);
        }
        b
    }

    /// Lee un informe ya codificado. Sirve para comprobar los grabados.
    pub fn leer(conexion: Conexion, b: &[u8]) -> Option<Salida> {
        let o = match conexion {
            Conexion::Usb if b.len() == LARGO_SALIDA_USB && b[0] == 0x02 => 0,
            Conexion::Bt if b.len() == LARGO_SALIDA_BT && b[0] == 0x31 && b[1] == 0x02 => 1,
            _ => return None,
        };
        Some(Salida {
            cambios: b[1 + o],
            cambios_luces: b[2 + o],
            motor_derecho: b[3 + o],
            motor_izquierdo: b[4 + o],
            led_mute: b[9 + o],
            gatillo_derecho: EfectoGatillo::leer(b, 11 + o),
            gatillo_izquierdo: EfectoGatillo::leer(b, 22 + o),
            potencia: b[37 + o],
            opciones_luces: b[39 + o],
            fundido: b[42 + o],
            brillo_jugador: b[43 + o],
            luces_jugador: b[44 + o],
            rojo: b[45 + o],
            verde: b[46 + o],
            azul: b[47 + o],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desde_hex;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02X}")).collect()
    }

    #[test]
    fn inicial_por_bluetooth() {
        let b = Salida::inicial_bt().codificar(Conexion::Bt);
        assert!(hex(&b).starts_with("3102001500"));
        assert!(hex(&b).ends_with("E6D82C3D"));
    }

    #[test]
    fn vacia_por_usb() {
        let esperado = desde_hex(
            "020F15000000000000000000000000000000000000000000000000000000000000000000000000000000000000FFFFFF",
        )
        .unwrap();
        assert_eq!(Salida::vacia(true).codificar(Conexion::Usb), esperado);
    }

    #[test]
    fn clic_completo_como_en_c() {
        let e = EfectoGatillo::de(Efecto::ClicCompleto { maximo: 255 });
        assert_eq!(
            (e.modo, e.inicio, e.fuerza, e.fuerza_tramo),
            (2, 0x94, 0xB4, 0xFF)
        );
        let e = EfectoGatillo::de(Efecto::ClicCompleto { maximo: 128 });
        assert_eq!((e.inicio, e.fuerza), (74, 127));
    }

    #[test]
    fn normal_con_luz() {
        let a = Ajustes {
            color: (0x00, 0xFF, 0xE5),
            ..Default::default()
        };
        let b = Salida::normal(&a).codificar(Conexion::Bt);
        assert_eq!(&hex(&b)[..8], "31020F55");
        assert_eq!(&hex(&b)[80..98], "06000002020000FFE5");
    }
}
