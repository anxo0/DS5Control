//! Encuentra los mandos y lee y escribe sus informes HID en Windows, con la misma tabla de
//! fabricantes y modelos que `DS5Devices.cs`.

#[cfg(windows)]
mod windows_hid;
#[cfg(windows)]
pub use windows_hid::{enumerar, Dispositivo, ErrorHid};

/// Sony.
pub const VID_SONY: u16 = 0x054C;

/// Los modelos que se manejan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modelo {
    DualSense,
    DualSenseEdge,
}

impl Modelo {
    pub fn desde(vid: u16, pid: u16) -> Option<Modelo> {
        match (vid, pid) {
            (VID_SONY, 0x0CE6) => Some(Modelo::DualSense),
            (VID_SONY, 0x0DF2) => Some(Modelo::DualSenseEdge),
            _ => None,
        }
    }
}

/// Un mando encontrado, todavía sin abrir.
#[derive(Debug, Clone)]
pub struct InfoMando {
    pub ruta: String,
    pub vid: u16,
    pub pid: u16,
    pub modelo: Modelo,
    pub conexion: ds5_informes::Conexion,
    pub largo_entrada: usize,
    pub largo_salida: usize,
    pub largo_caracteristica: usize,
}
