//! El motor de mapeo: zonas muertas, curvas, botones, giroscopio, ratón, teclado, macros y acciones.
//! Se muda por bloques; cada uno se compara con lo que da C# (`tools/fase5`).

pub mod acciones;
pub mod bezier;
pub mod botones;
pub mod curvas;
pub mod filtro;
pub mod flick;
pub mod giro_raton;
pub mod giroscopio;
pub mod luz;
pub mod macros;
pub mod motor;
pub mod panel;
pub mod panel_stick;
pub mod raton;
pub mod raton_botones;
pub mod sticks;
pub mod volante;

pub use acciones::{AccionesEspeciales, Peticion};
pub use botones::{AjustesBotones, Botones, Contexto, Entradas, LuzForzada, SalidaExtras};
pub use curvas::{AjustesCurvas, Curvas};
pub use giro_raton::{AjustesGiroRaton, GiroRaton};
pub use giroscopio::{Activador, AjustesGiro};
pub use luz::{AjustesLuz, EntradaLuz, EstadoLuz, Luz};
pub use macros::Macros;
pub use motor::{AjustesMotor, Motor, Paso};
pub use panel::{AjustesPanel, Panel, RatonPanel, Suceso};
pub use panel_stick::AjustesStickPanel;
pub use raton::{Clic, EventoRaton, Sintetico};
pub use raton_botones::{AjustesRatonBotones, RatonBotones};
pub use sticks::SticksAccion;
pub use volante::{AjustesVolante, CalibracionVolante, Punto, Volante};
