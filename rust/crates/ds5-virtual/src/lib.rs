//! El mando virtual (ViGEmBus: Xbox 360 y DS4) y HidHide.

pub mod informes;

#[cfg(windows)]
pub mod bus;
#[cfg(windows)]
pub mod hidhide;
pub mod teclado;

pub use informes::{EjeVolante, InformeXbox, Respuesta};

#[cfg(windows)]
pub use bus::{Bus, ErrorVirtual, MandoVirtual, Tipo};
