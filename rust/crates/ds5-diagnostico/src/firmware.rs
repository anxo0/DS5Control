//! El informe de firmware (0x20), que en C# lee `DualSenseDevice` al abrir el mando y deja en
//! `FirmwareInfo`, `FirmwareBuild`, `HardwareInfo` y compañía.

use ds5_informes::crc;

use crate::formato::{ascii, u16_en, u32_en};
use crate::protocolo::{Canal, LARGO};

/// Número del informe de firmware.
pub const INFORME_FIRMWARE: u8 = 0x20;
/// Intentos por Bluetooth hasta que el CRC cuadre, como `ReadBTFeatureReport`.
pub const INTENTOS_BT: usize = 5;

/// Las versiones del mando. Si no se pudo leer el informe, todo queda a cero y sin informe,
/// como las propiedades de C# sin rellenar.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Firmware {
    /// El informe entero (`FirmwareInfo`).
    pub informe: Option<[u8; LARGO]>,
    /// La fecha y la hora de compilación (`FirmwareBuild`).
    pub compilado: String,
    pub hardware: u32,
    pub principal: u32,
    pub actualizacion: u32,
    pub arranque: u32,
    pub audio: u32,
}

impl Firmware {
    /// `None` si el informe es más corto de 64 bytes.
    pub fn desde_informe(informe: &[u8]) -> Option<Firmware> {
        let informe: [u8; LARGO] = informe.get(..LARGO)?.try_into().ok()?;
        Some(Firmware {
            compilado: format!("{} {}", ascii(&informe[1..12]), ascii(&informe[12..20])),
            hardware: u32_en(&informe, 24),
            principal: u32_en(&informe, 28),
            actualizacion: u32::from(u16_en(&informe, 44)),
            arranque: u32_en(&informe, 48),
            audio: u32_en(&informe, 52),
            informe: Some(informe),
        })
    }

    /// El tipo de firmware (bytes 20 y 21), o 0 sin informe.
    pub fn tipo(&self) -> u16 {
        self.informe.map_or(0, |i| u16_en(&i, 20))
    }
}

/// Si el CRC de un informe de característica leído por Bluetooth es bueno.
pub fn valido_bt(informe: &[u8]) -> bool {
    crc::valido(crc::CABECERA_CARACTERISTICA, informe)
}

/// Lee el informe 0x20. Por USB vale la primera lectura que Windows dé por buena; por Bluetooth
/// se repite hasta cinco veces hasta que el CRC cuadre, sin mirar si la lectura falló.
pub fn leer_firmware<C: Canal>(canal: &mut C, bluetooth: bool) -> Firmware {
    let mut bufer = [0u8; LARGO];
    bufer[0] = INFORME_FIRMWARE;
    let leido = if bluetooth {
        (0..INTENTOS_BT).any(|_| {
            canal.leer_caracteristica(&mut bufer);
            valido_bt(&bufer)
        })
    } else {
        canal.leer_caracteristica(&mut bufer)
    };
    if leido {
        Firmware::desde_informe(&bufer).unwrap_or_default()
    } else {
        Firmware::default()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::simulado::Simulado;

    /// Un informe 0x20 de un mando con trazabilidad.
    pub fn informe_firmware() -> [u8; LARGO] {
        let mut f = [0u8; LARGO];
        f[0] = INFORME_FIRMWARE;
        f[1..12].copy_from_slice(b"Jun 10 2024");
        f[12..20].copy_from_slice(b"12:34:56");
        f[20..22].copy_from_slice(&3u16.to_le_bytes());
        f[22..24].copy_from_slice(&0x0102u16.to_le_bytes());
        f[24..28].copy_from_slice(&0x0000_0310u32.to_le_bytes());
        f[28..32].copy_from_slice(&0x0104_0010u32.to_le_bytes());
        for (i, b) in f[32..44].iter_mut().enumerate() {
            *b = i as u8 + 1;
        }
        f[44..46].copy_from_slice(&0x0630u16.to_le_bytes());
        f[48..52].copy_from_slice(&0x0001_0002u32.to_le_bytes());
        f[52..56].copy_from_slice(&0x0004_0105u32.to_le_bytes());
        f[56..60].copy_from_slice(&0x0203_0004u32.to_le_bytes());
        f
    }

    #[test]
    fn interpreta_el_informe() {
        let fw = Firmware::desde_informe(&informe_firmware()).unwrap();
        assert_eq!(fw.compilado, "Jun 10 2024 12:34:56");
        assert_eq!(fw.hardware, 0x310);
        assert_eq!(fw.principal, 0x0104_0010);
        assert_eq!(fw.actualizacion, 0x0630);
        assert_eq!(fw.arranque, 0x0001_0002);
        assert_eq!(fw.audio, 0x0004_0105);
        assert_eq!(fw.tipo(), 3);
    }

    #[test]
    fn la_compilacion_no_se_recorta() {
        let mut f = informe_firmware();
        f[1..12].copy_from_slice(&[0; 11]);
        f[12..20].copy_from_slice(&[0xFF, b'1', b' ', 0, 0, 0, 0, 0]);
        let fw = Firmware::desde_informe(&f).unwrap();
        assert_eq!(
            fw.compilado,
            format!("{} ?1 {}", "\0".repeat(11), "\0".repeat(5))
        );
    }

    #[test]
    fn informe_corto() {
        assert_eq!(Firmware::desde_informe(&informe_firmware()[..63]), None);
        assert_eq!(Firmware::desde_informe(&[]), None);
        // Uno más largo vale: se usan los 64 primeros.
        let mut largo = informe_firmware().to_vec();
        largo.push(0xAA);
        assert!(Firmware::desde_informe(&largo).is_some());
    }

    #[test]
    fn sin_informe_todo_a_cero() {
        let fw = Firmware::default();
        assert_eq!(fw.tipo(), 0);
        assert_eq!(fw.compilado, "");
        assert_eq!(fw.informe, None);
    }

    #[test]
    fn por_usb_basta_una_lectura() {
        let mut m = Simulado::nuevo();
        m.firmware = vec![informe_firmware()];
        let fw = leer_firmware(&mut m, false);
        assert_eq!(fw.hardware, 0x310);
        assert_eq!(m.lecturas_firmware, 1);
    }

    #[test]
    fn por_usb_sin_lectura() {
        let mut m = Simulado::nuevo();
        assert_eq!(leer_firmware(&mut m, false), Firmware::default());
    }

    #[test]
    fn por_bluetooth_reintenta_hasta_que_el_crc_cuadra() {
        let mut bueno = informe_firmware();
        crc::firmar(crc::CABECERA_CARACTERISTICA, &mut bueno);
        let mut malo = bueno;
        malo[63] ^= 0xFF;
        let mut m = Simulado::nuevo();
        m.firmware = vec![malo, malo, bueno];
        let fw = leer_firmware(&mut m, true);
        assert_eq!(fw.principal, 0x0104_0010);
        assert_eq!(m.lecturas_firmware, 3);
    }

    #[test]
    fn por_bluetooth_se_rinde_a_los_cinco() {
        let mut malo = informe_firmware();
        crc::firmar(crc::CABECERA_CARACTERISTICA, &mut malo);
        malo[10] ^= 1;
        let mut m = Simulado::nuevo();
        m.firmware = vec![malo];
        assert_eq!(leer_firmware(&mut m, true), Firmware::default());
        assert_eq!(m.lecturas_firmware, 5);
    }

    #[test]
    fn crc_de_bluetooth() {
        let mut f = informe_firmware();
        assert!(!valido_bt(&f));
        crc::firmar(crc::CABECERA_CARACTERISTICA, &mut f);
        assert!(valido_bt(&f));
        assert!(!valido_bt(&f[..3]));
    }
}
