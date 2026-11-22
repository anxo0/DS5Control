//! Lee y guarda los perfiles y los demás ajustes en el mismo formato XML que la versión en C#.
//!
//! La referencia es la colección de pruebas de C# (`DS5ControlTests/Perfiles` y
//! `DS5ControlTests/Ajustes`): leer y guardar cada archivo tiene que dar exactamente el mismo
//! resultado que da la versión en C#, rarezas incluidas.

pub mod ajustes;
mod campos;
pub mod carpeta;
mod formato;
pub mod mapeos;
pub mod migracion;
pub mod numeros;
pub mod otros;
pub mod perfil;
pub mod xml;

pub use ajustes::AjustesApp;
pub use campos::{Curva, MetodoSuavizado, Rgb};
pub use otros::{Acciones, Autoperfiles, Enlazados};
pub use perfil::{Encabezado, ErrorPerfil, Perfil};
