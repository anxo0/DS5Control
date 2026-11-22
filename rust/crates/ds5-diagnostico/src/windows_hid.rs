//! El canal sobre un mando de verdad. `ds5_hid::Dispositivo` lee los informes de característica
//! pero no los escribe, así que para mandar las órdenes se abre otro handle a la misma ruta y se
//! usa `HidD_SetFeature`, como `WriteFeatureReport` en C#.

use std::time::{Duration, Instant};

use ds5_hid::Dispositivo;
use ds5_informes::Conexion;
use serde_json::Value;
use windows::core::HSTRING;
use windows::Win32::Devices::HumanInterfaceDevice::HidD_SetFeature;
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};

use crate::firmware::leer_firmware;
use crate::informe::diagnostico;
use crate::protocolo::Canal;

/// Un handle que se cierra solo.
struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

fn abrir_escritura(ruta: &str) -> Option<Handle> {
    let ruta = HSTRING::from(ruta);
    let h = unsafe {
        CreateFileW(
            &ruta,
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    }
    .ok()?;
    Some(Handle(h))
}

struct CanalHid<'a> {
    dispositivo: &'a Dispositivo,
    /// `None` si no se pudo abrir: entonces ninguna orden sale y todas se dan por no contestadas.
    escritura: Option<Handle>,
}

impl Canal for CanalHid<'_> {
    fn leer_caracteristica(&mut self, bufer: &mut [u8]) -> bool {
        self.dispositivo.leer_caracteristica(bufer).is_ok()
    }

    fn escribir_caracteristica(&mut self, informe: &[u8]) -> bool {
        let Some(h) = &self.escritura else {
            return false;
        };
        unsafe { HidD_SetFeature(h.0, informe.as_ptr() as *const _, informe.len() as u32) }
    }

    fn ahora(&self) -> Instant {
        Instant::now()
    }

    fn esperar(&mut self, tiempo: Duration) {
        std::thread::sleep(tiempo);
    }
}

/// Lee el diagnóstico de un DualSense abierto: el informe de firmware, la batería, los datos de
/// fábrica y el bloque de uso. Tarda: cada orden sin respuesta espera un segundo, así que hay que
/// llamarlo fuera del hilo de la ventana, como `SendDiagnostics` en C#.
///
/// Siempre devuelve el objeto, como `DualSenseDiagnostics.Read`; el `null` del mensaje
/// `diagnostico` es cosa de quien llama cuando no hay ningún DualSense.
pub fn leer(d: &Dispositivo) -> Value {
    let bluetooth = d.info.conexion == Conexion::Bt;
    let mut canal = CanalHid {
        dispositivo: d,
        escritura: abrir_escritura(&d.info.ruta),
    };
    let fw = leer_firmware(&mut canal, bluetooth);
    diagnostico(&mut canal, &fw, bluetooth)
}
