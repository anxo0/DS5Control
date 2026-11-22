//! Los autoperfiles: cambian el perfil de cada mando según la ventana que está en primer plano,
//! como `AutoProfileChecker.cs` y `AutoProfileHolder.cs`.
//!
//! La lógica es pura y no carga perfiles: [`Vigilante`] mira la ventana (a través de
//! [`Escritorio`]) y devuelve las [`Peticion`]es que la app tiene que hacer, en orden. La parte
//! de Windows está en [`EscritorioWindows`].

mod observador;
pub mod panel;
mod regla;
mod vigilante;
#[cfg(windows)]
mod windows;

pub use observador::{Escritorio, Observador, Ventana};
pub use regla::{normalizar_ruta, normalizar_titulo, Regla, NINGUNO};
pub use vigilante::{EstadoApp, EstadoMando, Peticion, Resultado, Vigilante, MAX_MANDOS};

#[cfg(windows)]
pub use crate::windows::EscritorioWindows;

use std::time::Duration;

/// Cada cuánto se mira la ventana en primer plano. En C# el temporizador no se repite solo: se
/// vuelve a armar al terminar cada comprobación, así que el intervalo cuenta desde el final de la
/// anterior.
pub const INTERVALO_SONDEO: Duration = Duration::from_millis(1000);

/// Lo máximo que C# espera a que el servicio se encienda o se apague antes de seguir.
pub const ESPERA_SERVICIO: Duration = Duration::from_secs(10);
