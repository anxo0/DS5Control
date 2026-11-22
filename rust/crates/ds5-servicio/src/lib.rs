//! El servicio de DS5Control: busca los mandos, arranca un hilo por cada uno con su perfil y su
//! mando virtual, los esconde con HidHide y avisa de lo que pasa. Es la parte de `ControlService`
//! que no es mapeo; no sabe nada de la ventana, que habla con él por `Servicio` y `Aviso`.

pub mod acciones;
pub mod ajustes;
pub mod calibracion;
pub mod generales;
pub mod guardado;
pub mod idioma;
pub mod interfaz;
pub mod mando;
pub mod opciones;
pub mod programa;
pub mod salida;
mod textos;

use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ds5_hid::enumerar;
use ds5_informes::Conexion;
use ds5_mapeo::Peticion;
use ds5_perfiles::Acciones;
use ds5_virtual::hidhide::{self, HidHide, Ocultador};
use ds5_virtual::Bus;

pub use ajustes::Carpeta;
pub use guardado::Guardado;
pub use idioma::Idioma;
pub use mando::{Mando, Orden, Vivo};

/// Cuántos mandos a la vez, como `Global.MAX_DS5_CONTROLLER_COUNT`.
pub const MAX_MANDOS: usize = 8;
/// Cada cuánto se buscan mandos nuevos.
const BUSQUEDA: Duration = Duration::from_secs(1);

/// Lo que cuenta el servicio a quien lo usa.
#[derive(Debug, Clone)]
pub enum Aviso {
    Conectado(usize),
    Desconectado(usize),
    /// Se ha pulsado el botón de silencio del mando de esa ranura.
    Mute(usize),
    /// Una acción especial pide algo que hace la app (cargar un perfil, abrir un programa...).
    Peticion(usize, Peticion),
    /// Una línea para el registro; `true` si es un aviso.
    Registro(String, bool),
}

struct Compartido {
    entorno: Arc<mando::Entorno>,
    mandos: Mutex<Vec<Arc<Mando>>>,
    parar: AtomicBool,
}

pub struct Servicio {
    compartido: Arc<Compartido>,
    vigia: Option<JoinHandle<()>>,
    /// Si ViGEmBus está instalado y funciona.
    pub vigem: bool,
    /// Si HidHide está instalado.
    pub hidhide: bool,
}

/// `ProfileFile`: un nombre de archivo válido en Windows.
pub fn nombre_valido(nombre: &str) -> bool {
    !nombre.trim().is_empty()
        && !nombre
            .chars()
            .any(|c| (c as u32) < 32 || "\"<>|:*?\\/".contains(c))
}

/// El servidor DSU, si está activado en `Profiles.xml` (`ChangeUDPStatus`).
fn arrancar_dsu(carpeta: &Carpeta, avisos: &Sender<Aviso>) -> Option<ds5_dsu::Servidor> {
    let g = carpeta.ajustes().generales;
    if !g.servidor_udp {
        return None;
    }
    let puerto = g.puerto_servidor_udp.clamp(1024, 65535) as u16;
    match ds5_dsu::Servidor::arrancar(&g.direccion_servidor_udp, puerto) {
        Ok(s) => {
            s.fijar_suavizado(
                g.suavizado_udp.activo,
                ds5_dsu::Suavizado {
                    corte_minimo: g.suavizado_udp.corte_minimo,
                    beta: g.suavizado_udp.beta,
                },
            );
            Some(s)
        }
        Err(_) => {
            let idioma = Idioma::de_ajustes(&carpeta.ajustes());
            let _ = avisos.send(Aviso::Registro(
                idioma.formato(
                    "No se ha podido abrir el servidor UDP en {}:{}",
                    &[&g.direccion_servidor_udp, &puerto],
                ),
                true,
            ));
            None
        }
    }
}

/// La primera ranura libre.
fn ranura_libre(ocupadas: &[usize]) -> Option<usize> {
    (0..MAX_MANDOS).find(|r| !ocupadas.contains(r))
}

impl Servicio {
    /// Arranca el servicio con los ajustes de `carpeta`. Con `mandos` a `false` no busca mandos
    /// ni toca ViGEmBus ni HidHide: sirve para probar la interfaz sin molestar a otra app.
    pub fn arrancar(carpeta: impl Into<PathBuf>, mandos: bool) -> (Servicio, Receiver<Aviso>) {
        let (avisos, recibe) = std::sync::mpsc::channel();
        let carpeta = Carpeta::new(carpeta);
        let bus = match if mandos {
            Bus::abrir().map(Some)
        } else {
            Ok(None)
        } {
            Ok(b) => b,
            Err(e) => {
                let idioma = Idioma::de_ajustes(&carpeta.ajustes());
                let _ = avisos.send(Aviso::Registro(
                    idioma.formato("ViGEmBus no está disponible: {}", &[&e]),
                    true,
                ));
                None
            }
        };
        // Si está instalado se mira siempre, también sin mandos, para no avisar de que falta.
        let vigem = bus.is_some() || (!mandos && Bus::abrir().is_ok());
        let hidhide = HidHide::abrir().is_some();
        let compartido = Arc::new(Compartido {
            entorno: Arc::new(mando::Entorno {
                dsu: if mandos {
                    arrancar_dsu(&carpeta, &avisos)
                } else {
                    None
                },
                acciones: Mutex::new(carpeta.acciones()),
                guardado: Guardado::new(&carpeta.ruta),
                carpeta,
                bus,
                avisos: avisos.clone(),
                inicio: Instant::now(),
            }),
            mandos: Mutex::new(Vec::new()),
            parar: AtomicBool::new(false),
        });
        let c = compartido.clone();
        let vigia = std::thread::Builder::new()
            .name("busca mandos".into())
            .spawn(move || {
                if mandos {
                    vigilar(&c, HidHide::abrir(), &avisos)
                }
            })
            .expect("no se pudo crear el hilo que busca mandos");
        (
            Servicio {
                compartido,
                vigia: Some(vigia),
                vigem,
                hidhide,
            },
            recibe,
        )
    }

    /// Los mandos conectados, por ranura.
    pub fn mandos(&self) -> Vec<Arc<Mando>> {
        let mut v = self.compartido.mandos.lock().unwrap().clone();
        v.sort_by_key(|m| m.ranura);
        v
    }

    pub fn mando(&self, id: &str) -> Option<Arc<Mando>> {
        self.mandos().into_iter().find(|m| m.id() == id)
    }

    pub fn carpeta(&self) -> &Carpeta {
        &self.compartido.entorno.carpeta
    }

    pub fn guardado(&self) -> &Guardado {
        &self.compartido.entorno.guardado
    }

    /// Las acciones especiales.
    pub fn acciones(&self) -> Acciones {
        self.compartido.entorno.acciones.lock().unwrap().clone()
    }

    /// Cambia las acciones especiales; si `cambio` dice que ha cambiado algo, guarda
    /// `Actions.xml` y avisa a todos los mandos.
    pub fn cambiar_acciones(&self, cambio: impl FnOnce(&mut Acciones) -> bool) -> io::Result<bool> {
        let texto = {
            let mut a = self.compartido.entorno.acciones.lock().unwrap();
            if !cambio(&mut a) {
                return Ok(false);
            }
            guardado::con_comentario(
                &a.escribir(),
                &format!(
                    "Special Actions Configuration Data. {}",
                    guardado::fecha_espanola()
                ),
            )
        };
        for m in self.mandos() {
            m.ordenar(Orden::RecargarAcciones);
        }
        self.guardado().escribir("Actions.xml", &texto)?;
        Ok(true)
    }

    /// `UseProfile`: el perfil pasa a ser el del mando y se apunta en `Profiles.xml`.
    pub fn elegir_perfil(&self, m: &Mando, nombre: &str) -> io::Result<bool> {
        if !nombre_valido(nombre) || !self.carpeta().ruta_perfil(nombre).is_file() {
            return Ok(false);
        }
        m.ordenar(Orden::Cargar(nombre.to_string()));
        let mut ajustes = self.carpeta().ajustes();
        if let Some(r) = ajustes.perfiles.get_mut(m.ranura) {
            *r = nombre.to_string();
        }
        self.guardado().escribir(
            "Profiles.xml",
            &ajustes.escribir(Some(&guardado::encabezado())),
        )?;
        Ok(true)
    }

    /// `CreateProfile`: una copia del perfil actual del mando con otro nombre.
    pub fn nuevo_perfil(&self, m: &Mando, nombre: &str) -> io::Result<bool> {
        let actual = m.vivo().perfil;
        if !nombre_valido(nombre)
            || self.carpeta().ruta_perfil(nombre).exists()
            || !self.carpeta().ruta_perfil(&actual).is_file()
        {
            return Ok(false);
        }
        self.guardado().copiar(
            format!("Profiles/{actual}.xml"),
            format!("Profiles/{nombre}.xml"),
        )?;
        Ok(true)
    }

    /// `DeleteProfile`: no se borra un perfil que use algún mando.
    pub fn borrar_perfil(&self, nombre: &str) -> io::Result<bool> {
        if !nombre_valido(nombre)
            || !self.carpeta().ruta_perfil(nombre).is_file()
            || self.mandos().iter().any(|m| m.vivo().perfil == nombre)
        {
            return Ok(false);
        }
        self.guardado().borrar(format!("Profiles/{nombre}.xml"))?;
        Ok(true)
    }

    /// Para todos los mandos y deja HidHide como estaba.
    pub fn parar(&mut self) {
        self.compartido.parar.store(true, Ordering::Relaxed);
        if let Some(h) = self.vigia.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Servicio {
    fn drop(&mut self) {
        self.parar();
    }
}

fn vigilar(c: &Compartido, hh: Option<HidHide>, avisos: &Sender<Aviso>) {
    // El propio programa en la lista blanca de HidHide, para seguir viendo los mandos escondidos.
    let exe = std::env::current_exe().ok();
    if let (Some(h), Some(exe)) = (&hh, &exe) {
        h.permitir_programa(exe);
    }
    let mut ocultador = Ocultador::new(std::env::temp_dir().join("ds5control-rust-hidhide.txt"));
    let mut escondidos: Vec<String> = Vec::new();

    while !c.parar.load(Ordering::Relaxed) {
        {
            let mut mandos = c.mandos.lock().unwrap();
            mandos.retain(|m| !m.acabado.load(Ordering::Acquire));
            let mut nuevos = enumerar();
            // Si el mismo mando está por cable y por Bluetooth, primero el cable.
            nuevos.sort_by_key(|m| m.conexion != Conexion::Usb);
            for info in nuevos {
                if mandos.iter().any(|m| m.info.ruta == info.ruta) {
                    continue;
                }
                let ocupadas: Vec<usize> = mandos.iter().map(|m| m.ranura).collect();
                let Some(ranura) = ranura_libre(&ocupadas) else {
                    break;
                };
                match mando::arrancar(ranura, info, c.entorno.clone()) {
                    Ok(m) => mandos.push(m),
                    Err(e) => {
                        let idioma = Idioma::de_ajustes(&c.entorno.carpeta.ajustes());
                        let _ = avisos.send(Aviso::Registro(
                            idioma.formato("No se pudo abrir un mando: {}", &[&e]),
                            true,
                        ));
                    }
                }
            }
            if hh.is_some() {
                let mut esconder: Vec<String> = mandos
                    .iter()
                    .filter_map(|m| hidhide::instancia_de(&m.info.ruta))
                    .collect();
                esconder.sort();
                if esconder != escondidos {
                    ocultador.aplicar(&esconder);
                    escondidos = esconder;
                }
            }
        }
        std::thread::sleep(BUSQUEDA);
    }

    let mandos: Vec<Arc<Mando>> = c.mandos.lock().unwrap().drain(..).collect();
    for m in &mandos {
        m.parar.store(true, Ordering::Relaxed);
    }
    let espera = Instant::now();
    while mandos.iter().any(|m| !m.acabado.load(Ordering::Acquire))
        && espera.elapsed() < Duration::from_secs(2)
    {
        std::thread::sleep(Duration::from_millis(20));
    }
    if hh.is_some() {
        ocultador.aplicar(&[]);
    }
    if let (Some(h), Some(exe)) = (&hh, &exe) {
        h.retirar_programa(exe);
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_ranura_es_la_primera_libre() {
        assert_eq!(ranura_libre(&[]), Some(0));
        assert_eq!(ranura_libre(&[0, 1, 3]), Some(2));
        assert_eq!(ranura_libre(&[0, 1, 2, 3, 4, 5, 6, 7]), None);
    }

    #[test]
    fn nombres_de_perfil() {
        assert!(nombre_valido("Carreras 2"));
        for malo in [
            "", "  ", "a/b", "a\\b", "a:b", "a?b", "a*b", "a\"b", "a<b", "a|b",
        ] {
            assert!(!nombre_valido(malo), "{malo}");
        }
    }
}
