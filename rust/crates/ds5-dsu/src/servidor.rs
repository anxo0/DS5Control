//! El servidor por UDP, como `UdpServer.Start`, `Stop` y `NewReportIncoming`.
//!
//! Un hilo propio atiende las peticiones de los clientes (versión, lista de puertos y
//! suscripciones). Los datos los manda quien llame a [`Servidor::enviar_estado`], desde el hilo
//! del mando: solo toma un cerrojo un momento para mirar a quién mandarlos y luego envía por
//! UDP, que no espera a nadie.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use ds5_informes::Estado;

use crate::clientes::Clientes;
use crate::paquetes::{
    leer_peticion, respuesta_datos, respuesta_puerto, respuesta_version, DatosMando, Meta,
    Peticion, RANURAS,
};
use crate::suavizado::{FiltroMovimiento, Suavizado};

/// El puerto de siempre (`DEFAULT_UDP_SERV_PORT`).
pub const PUERTO_POR_DEFECTO: u16 = 26760;

/// Cada cuánto mira el hilo si tiene que parar.
const ESPERA_LECTURA: Duration = Duration::from_millis(100);

/// Lo que comparten el hilo de red y quien manda los datos.
struct Interno {
    clientes: Clientes,
    metas: [Meta; RANURAS],
    suavizar: bool,
    ajustes: Suavizado,
    filtros: [FiltroMovimiento; RANURAS],
}

struct Compartido {
    id: u32,
    corriendo: AtomicBool,
    interno: Mutex<Interno>,
}

impl Compartido {
    fn interno(&self) -> MutexGuard<'_, Interno> {
        // Si un hilo se cayó con el cerrojo tomado, los datos siguen sirviendo.
        self.interno.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Un servidor DSU en marcha. Se puede compartir entre hilos (`Arc<Servidor>`): todos sus
/// métodos toman `&self`. Se para con [`Servidor::parar`] o al soltarlo.
pub struct Servidor {
    compartido: Arc<Compartido>,
    socket: UdpSocket,
    direccion: SocketAddr,
    hilo: Mutex<Option<JoinHandle<()>>>,
}

/// Traduce la dirección de escucha como `UdpServer.Start`: vacía o `127.0.0.1` es solo este
/// equipo, `0.0.0.0` todas las interfaces, y cualquier otra cosa se resuelve como nombre y se
/// usa su primera dirección IPv4.
pub fn resolver_direccion(direccion: &str, puerto: u16) -> io::Result<SocketAddr> {
    let direccion = direccion.trim();
    let ip = match direccion {
        "" | "127.0.0.1" => IpAddr::V4(Ipv4Addr::LOCALHOST),
        "0.0.0.0" => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        nombre => (nombre, puerto)
            .to_socket_addrs()?
            .find(SocketAddr::is_ipv4)
            .map(|d| d.ip())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::AddrNotAvailable,
                    format!("{nombre} no tiene ninguna dirección IPv4"),
                )
            })?,
    };
    Ok(SocketAddr::new(ip, puerto))
}

/// Un número al azar para el id del servidor, sin dependencias: el hasher de la biblioteca
/// estándar se siembra al azar en cada proceso.
fn id_al_azar() -> u32 {
    let mut h = RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    );
    h.finish() as u32
}

impl Servidor {
    /// Abre el puerto y arranca el hilo que atiende a los clientes. `direccion` es la de
    /// `UDPServerListenAddress` y `puerto` el de `UDPServerPort` (0 para que el sistema elija uno
    /// libre). Falla si no se puede resolver la dirección o el puerto está ocupado.
    ///
    /// El suavizado empieza apagado; se enciende con [`Servidor::fijar_suavizado`].
    pub fn arrancar(direccion: &str, puerto: u16) -> io::Result<Servidor> {
        let socket = UdpSocket::bind(resolver_direccion(direccion, puerto)?)?;
        let direccion = socket.local_addr()?;
        let lectura = socket.try_clone()?;
        lectura.set_read_timeout(Some(ESPERA_LECTURA))?;

        let ajustes = Suavizado::default();
        let compartido = Arc::new(Compartido {
            id: id_al_azar(),
            corriendo: AtomicBool::new(true),
            interno: Mutex::new(Interno {
                clientes: Clientes::new(),
                metas: std::array::from_fn(|i| Meta::desconectado(i as u8)),
                suavizar: false,
                ajustes,
                filtros: [FiltroMovimiento::new(ajustes); RANURAS],
            }),
        });
        let para_hilo = Arc::clone(&compartido);
        let hilo = thread::Builder::new()
            .name("servidor-dsu".into())
            .spawn(move || atender(lectura, para_hilo))?;
        Ok(Servidor {
            compartido,
            socket,
            direccion,
            hilo: Mutex::new(Some(hilo)),
        })
    }

    /// La dirección donde escucha, con el puerto que haya dado el sistema.
    pub fn direccion(&self) -> SocketAddr {
        self.direccion
    }

    /// El id que va en cada paquete. Se elige al azar al arrancar.
    pub fn id(&self) -> u32 {
        self.compartido.id
    }

    /// Si sigue en marcha.
    pub fn en_marcha(&self) -> bool {
        self.compartido.corriendo.load(Ordering::Relaxed)
    }

    /// Manda un informe del mando a los clientes suscritos a su ranura (`meta.ranura`, de 0 a
    /// 3) o a su MAC. Se llama desde el hilo del mando con cada informe. Guarda también la
    /// `meta` para responder a las peticiones de puertos.
    ///
    /// Devuelve a cuántos clientes lo ha mandado. No hace nada si la ranura no existe o el
    /// servidor está parado.
    pub fn enviar_estado(&self, meta: &Meta, estado: &Estado) -> usize {
        self.enviar(meta, DatosMando::desde_estado(estado))
    }

    /// Como [`Servidor::enviar_estado`], con los datos ya convertidos.
    pub fn enviar(&self, meta: &Meta, mut datos: DatosMando) -> usize {
        let ranura = meta.ranura as usize;
        if ranura >= RANURAS || !self.en_marcha() {
            return 0;
        }
        let destinos = {
            let mut i = self.compartido.interno();
            i.metas[ranura] = *meta;
            if i.suavizar {
                if datos.transcurrido == 0.0 {
                    // Sin marca de tiempo no se puede suavizar: C# se salta el informe.
                    return 0;
                }
                if let Some(m) = datos.movimiento.as_mut() {
                    i.filtros[ranura].aplicar(m, datos.transcurrido);
                }
            }
            i.clientes
                .destinatarios(meta.ranura, meta.mac, Instant::now())
        };
        if destinos.is_empty() {
            return 0;
        }
        let paquete = respuesta_datos(self.compartido.id, meta, &datos);
        destinos
            .iter()
            .filter(|d| self.socket.send_to(&paquete, d).is_ok())
            .count()
    }

    /// Cambia lo que se responde a las peticiones de puertos sin mandar datos: por ejemplo
    /// cuando se conecta un mando antes de su primer informe.
    pub fn actualizar_meta(&self, meta: Meta) {
        if let Some(m) = self
            .compartido
            .interno()
            .metas
            .get_mut(meta.ranura as usize)
        {
            *m = meta;
        }
    }

    /// Marca la ranura como vacía, para cuando se desconecta el mando.
    pub fn desconectar(&self, ranura: u8) {
        self.actualizar_meta(Meta::desconectado(ranura));
    }

    /// Enciende o apaga el suavizado y cambia sus ajustes sin perder lo filtrado, como
    /// `ChangeUdpSmoothingAttrs`. Los ajustes se acotan como en C#.
    pub fn fijar_suavizado(&self, activo: bool, ajustes: Suavizado) {
        let ajustes = ajustes.acotado();
        let mut i = self.compartido.interno();
        i.suavizar = activo;
        i.ajustes = ajustes;
        for f in &mut i.filtros {
            f.fijar(ajustes);
        }
    }

    /// Empieza de cero los filtros de una ranura, como `ResetUdpSmoothingFilters`: hay que
    /// llamarlo cuando entra un mando nuevo en ella.
    pub fn reiniciar_suavizado(&self, ranura: u8) {
        let mut i = self.compartido.interno();
        let ajustes = i.ajustes;
        if let Some(f) = i.filtros.get_mut(ranura as usize) {
            *f = FiltroMovimiento::new(ajustes);
        }
    }

    /// Cuántos clientes hay apuntados ahora mismo.
    pub fn clientes(&self) -> usize {
        self.compartido.interno().clientes.len()
    }

    /// Para el hilo y deja de mandar datos. Espera a que el hilo acabe (como mucho una décima
    /// de segundo). Se puede llamar más de una vez.
    pub fn parar(&self) {
        self.compartido.corriendo.store(false, Ordering::Relaxed);
        let hilo = self.hilo.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(h) = hilo {
            let _ = h.join();
        }
    }
}

impl Drop for Servidor {
    fn drop(&mut self) {
        self.parar();
    }
}

/// El bucle del hilo: lee peticiones hasta que se pare el servidor.
fn atender(socket: UdpSocket, c: Arc<Compartido>) {
    let mut bufer = [0u8; 1024];
    while c.corriendo.load(Ordering::Relaxed) {
        let (n, origen) = match socket.recv_from(&mut bufer) {
            Ok(r) => r,
            // Se acabó la espera, o Windows avisa de que un cliente cerró su puerto
            // (`WSAECONNRESET`, lo que C# apaga con `SIO_UDP_CONNRESET`): se sigue.
            Err(_) => continue,
        };
        let Some(p) = leer_peticion(&bufer[..n]) else {
            continue;
        };
        match p.peticion {
            Peticion::Version => {
                let _ = socket.send_to(&respuesta_version(c.id), origen);
            }
            Peticion::Puertos(ranuras) => {
                let metas = c.interno().metas;
                for r in ranuras {
                    let _ = socket.send_to(&respuesta_puerto(c.id, &metas[r as usize]), origen);
                }
            }
            Peticion::Datos {
                banderas,
                ranura,
                mac,
            } => {
                c.interno()
                    .clientes
                    .registrar(origen, banderas, ranura, mac, Instant::now());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resuelve_las_direcciones_como_csharp() {
        assert_eq!(
            resolver_direccion("", 26760).unwrap(),
            SocketAddr::from(([127, 0, 0, 1], 26760))
        );
        assert_eq!(
            resolver_direccion("127.0.0.1", 1).unwrap(),
            SocketAddr::from(([127, 0, 0, 1], 1))
        );
        assert_eq!(
            resolver_direccion(" 0.0.0.0 ", 2).unwrap(),
            SocketAddr::from(([0, 0, 0, 0], 2))
        );
        assert_eq!(
            resolver_direccion("192.168.1.20", 3).unwrap(),
            SocketAddr::from(([192, 168, 1, 20], 3))
        );
        assert!(resolver_direccion("localhost", 4).unwrap().is_ipv4());
        assert!(resolver_direccion("::1", 4).is_err());
    }
}
