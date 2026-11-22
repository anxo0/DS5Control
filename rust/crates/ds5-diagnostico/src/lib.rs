//! Lo que un DualSense guarda de sí mismo, como `DualSenseDiagnostics.cs`: el informe de
//! firmware (0x20), el parche de Bluetooth (0x22) y las órdenes de prueba de fábrica, que salen
//! en el informe de característica 0x80 y se contestan en el 0x81. Solo se usan órdenes que leen.
//! El protocolo lo documenta el proyecto dualsense-tester.
//!
//! La parte pura (`formato`, `firmware`, `telemetria` y las funciones de `protocolo` que miran
//! bytes) no habla con el mando. Las lecturas van por el rasgo [`Canal`], así que los reintentos y
//! las esperas se prueban con un mando simulado; en Windows, [`leer`] lo monta sobre
//! `ds5_hid::Dispositivo`.
//!
//! El resultado es el objeto `datos` del mensaje `diagnostico` de la interfaz (`Diagnostico` en
//! `ui/src/bridge/tipos.ts`).

pub mod firmware;
pub mod formato;
pub mod informe;
pub mod protocolo;
pub mod telemetria;

#[cfg(test)]
mod simulado;
#[cfg(windows)]
mod windows_hid;

pub use firmware::{leer_firmware, Firmware};
pub use informe::diagnostico;
pub use protocolo::Canal;
pub use telemetria::telemetria;
#[cfg(windows)]
pub use windows_hid::leer;
