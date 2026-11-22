//! Decodifica y codifica los informes del DualSense (USB y Bluetooth) y su CRC32. No depende de
//! Windows, así que se prueba con las grabaciones de la fase 0.

pub mod crc;
pub mod dualsense;
pub mod movimiento;
pub mod salida;

pub use dualsense::{boton, Decodificador, Estado, Toque};
pub use movimiento::{Calibracion, Movimiento};
pub use salida::{Ajustes, Efecto, EfectoGatillo, Salida};

/// Cómo está conectado el mando.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conexion {
    Usb,
    Bt,
}

/// Convierte texto hexadecimal en bytes. `None` si no lo es.
pub fn desde_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}
