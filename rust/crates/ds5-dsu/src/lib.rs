//! El servidor DSU (cemuhook), como `UdpServer.cs`: manda los botones, los sticks, el panel
//! táctil y sobre todo el giroscopio a emuladores como Cemu, Ryujinx o Dolphin.
//!
//! - [`paquetes`]: los paquetes byte a byte y la lectura de las peticiones, sin red.
//! - [`clientes`]: quién está suscrito y cuándo caduca.
//! - [`suavizado`]: el filtro One Euro opcional del movimiento.
//! - [`servidor`]: el socket y su hilo.
//!
//! Uso desde el servicio:
//!
//! ```no_run
//! use ds5_dsu::{Meta, Servidor, Suavizado};
//! # let estado = ds5_informes::Estado::default();
//! let servidor = Servidor::arrancar("127.0.0.1", 26760)?;
//! servidor.fijar_suavizado(false, Suavizado::default());
//! // Con cada informe del mando de la ranura 0:
//! let meta = Meta::de_mando(0, [1, 2, 3, 4, 5, 6], ds5_informes::Conexion::Usb, true, &estado);
//! servidor.enviar_estado(&meta, &estado);
//! // Al acabar:
//! servidor.parar();
//! # Ok::<(), std::io::Error>(())
//! ```

pub mod clientes;
pub mod crc;
pub mod paquetes;
pub mod servidor;
pub mod suavizado;

pub use paquetes::{
    inactivo, leer_mac, DatosMando, EstadoRanura, Meta, Modelo, MovimientoDsu, NivelBateria,
    TipoConexion, ToqueDsu, RANURAS,
};
pub use servidor::{Servidor, PUERTO_POR_DEFECTO};
pub use suavizado::Suavizado;
