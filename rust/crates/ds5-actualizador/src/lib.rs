//! La actualización automática: lo mismo que `AppUpdater.cs` en la versión en C#.
//!
//! El camino completo, como `AppHost.UpdateAsync`:
//!
//! 1. [`buscar`] pide a GitHub la última release y la devuelve si es más nueva que la versión
//!    en marcha, viene de este repositorio y trae la huella SHA-256 del zip.
//! 2. Si [`puede_escribir_en`] dice que no en la carpeta del programa, se avisa con
//!    [`avisos::sin_permiso`] y no se sigue.
//! 3. [`descargar`] baja el zip, comprueba la huella y lo descomprime.
//! 4. Se avisa con [`avisos::actualizando_notificacion`], se deja el aviso con
//!    [`guardar_aviso`] y se lanza [`iniciar_instalacion`]; después el programa se cierra.
//! 5. El script espera a que se cierre, copia los archivos nuevos y lo vuelve a abrir, y la
//!    versión nueva enseña [`avisos::actualizado`] si [`tomar_aviso`] devuelve su versión.
//!
//! La lógica pura (leer la release, comparar versiones, elegir el zip y el script) está
//! separada de la red ([`red`]) y del disco ([`archivos`]).

pub mod archivos;
pub mod avisos;
mod error;
pub mod red;
pub mod release;
pub mod script;
pub mod version;

pub use archivos::{
    guardar_aviso, iniciar_instalacion, puede_escribir_en, tomar_aviso, verificar_y_descomprimir,
};
pub use error::ErrorActualizacion;
pub use red::{buscar, descargar, nuevo_agente};
pub use release::{leer_release, Actualizacion, PROGRAMA, URL_ULTIMA_RELEASE};
pub use script::SCRIPT_INSTALACION;
pub use version::Version;
