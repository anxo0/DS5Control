//! Un mando conectado: su hilo lee los informes, los pasa por el motor y manda el resultado al mando
//! virtual, al teclado y ratón y a la luz, la vibración y los gatillos del propio mando. Es lo que
//! en C# hacen `DS5Device.ReadInput` y `ControlService.On_Report` para cada mando.

use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ds5_hid::{Dispositivo, ErrorHid, InfoMando};
use ds5_informes::salida::{Ajustes, Salida};
use ds5_informes::{boton, crc, Calibracion, Conexion, Decodificador, Estado};
use ds5_mapeo::raton::EventoRaton;
use ds5_mapeo::{AccionesEspeciales, AjustesLuz, AjustesMotor, EntradaLuz, Luz, Motor, Peticion};
use ds5_perfiles::{Acciones, Perfil};
use ds5_virtual::{teclado, Bus, ErrorVirtual, MandoVirtual, Tipo};

use crate::ajustes::Carpeta;
use crate::calibracion;
use crate::guardado::{encabezado, Guardado};
use crate::idioma::Idioma;
use crate::salida::{self, InformeVirtual};
use crate::Aviso;

/// Cuánto se espera tras el último cambio para guardar el perfil.
const GUARDAR_TRAS: Duration = Duration::from_secs(1);
/// `READ_STREAM_TIMEOUT` de C#: sin informes durante este tiempo, el mando se da por desconectado.
const SIN_DATOS: Duration = Duration::from_secs(3);

/// Una orden para el hilo de un mando.
pub enum Orden {
    /// Cambiar el perfil cargado en memoria (los ajustes de la interfaz).
    Ajustar(Box<dyn FnOnce(&mut Perfil) + Send>),
    /// Cargar otro perfil de la carpeta.
    Cargar(String),
    /// Hacer vibrar los dos motores a tope durante un rato.
    ProbarVibracion(Duration),
    /// Han cambiado las acciones especiales.
    RecargarAcciones,
    /// Un perfil temporal (autoperfiles), o `None` para volver al normal del mando.
    Temporal(Option<String>),
}

/// Lo que se sabe del mando en cada momento, para la interfaz.
#[derive(Debug, Clone, Default)]
pub struct Vivo {
    /// El último estado leído, sin mapear.
    pub crudo: Estado,
    pub bateria: Option<u8>,
    pub cargando: bool,
    /// El color que tiene la barra ahora mismo.
    pub luz: (u8, u8, u8),
    pub perfil: String,
    /// El perfil elegido para el mando; es distinto de `perfil` con un perfil temporal.
    pub normal: String,
    /// El perfil cargado, con los cambios de la interfaz.
    pub ajustes: Perfil,
    /// El mando virtual enchufado.
    pub emulando: Option<Tipo>,
}

/// Un mando conectado, visto desde fuera de su hilo.
pub struct Mando {
    pub ranura: usize,
    pub info: InfoMando,
    pub vivo: Mutex<Vivo>,
    ordenes: Mutex<Sender<Orden>>,
    pub(crate) parar: AtomicBool,
    pub(crate) acabado: AtomicBool,
}

impl Mando {
    /// El identificador que ve la interfaz: la ruta del dispositivo.
    pub fn id(&self) -> &str {
        &self.info.ruta
    }

    pub fn ordenar(&self, o: Orden) {
        let _ = self.ordenes.lock().unwrap().send(o);
    }

    pub fn vivo(&self) -> Vivo {
        self.vivo.lock().unwrap().clone()
    }
}

/// Lo que comparte el hilo de un mando con el resto del servicio.
pub(crate) struct Entorno {
    pub carpeta: Carpeta,
    pub bus: Option<Arc<Bus>>,
    pub avisos: Sender<Aviso>,
    pub inicio: Instant,
    pub guardado: Guardado,
    /// Las acciones especiales (`Actions.xml`), compartidas por todos los mandos.
    pub acciones: Mutex<Acciones>,
    /// El servidor DSU para los emuladores, si está activado en los ajustes.
    pub dsu: Option<ds5_dsu::Servidor>,
}

/// La MAC del mando, del informe de característica 0x09 (`getMacAddress`): los bytes 1 a 6 al
/// revés. Ceros si no se puede leer.
fn leer_mac(d: &Dispositivo) -> [u8; 6] {
    let mut b = [0u8; 20];
    b[0] = 0x09;
    if d.leer_caracteristica(&mut b).is_err() {
        return [0; 6];
    }
    [b[6], b[5], b[4], b[3], b[2], b[1]]
}

fn leer_calibracion(d: &Dispositivo) -> Option<Calibracion> {
    for _ in 0..5 {
        let mut c = [0u8; 41];
        c[0] = 0x05;
        if d.leer_caracteristica(&mut c).is_err() {
            continue;
        }
        if d.info.conexion == Conexion::Usb || crc::valido(crc::CABECERA_CARACTERISTICA, &c) {
            return Calibracion::desde_informe(&c);
        }
    }
    None
}

/// Abre el mando y arranca su hilo.
pub(crate) fn arrancar(
    ranura: usize,
    info: InfoMando,
    entorno: Arc<Entorno>,
) -> Result<Arc<Mando>, ErrorHid> {
    let dispositivo = Dispositivo::abrir(&info)?;
    let (envia, recibe) = std::sync::mpsc::channel();
    let mando = Arc::new(Mando {
        ranura,
        info,
        vivo: Mutex::new(Vivo::default()),
        ordenes: Mutex::new(envia),
        parar: AtomicBool::new(false),
        acabado: AtomicBool::new(false),
    });
    let m = mando.clone();
    std::thread::Builder::new()
        .name(format!("mando {}", ranura + 1))
        .spawn(move || {
            hilo(&m, dispositivo, recibe, &entorno);
            m.acabado.store(true, Ordering::Release);
            let _ = entorno.avisos.send(Aviso::Desconectado(m.ranura));
        })
        .expect("no se pudo crear el hilo del mando");
    Ok(mando)
}

/// El perfil cargado y todo lo que se calcula de él.
struct Cargado {
    nombre: String,
    perfil: Perfil,
    motor: AjustesMotor,
    luz: AjustesLuz,
    volante: Option<ds5_virtual::EjeVolante>,
}

impl Cargado {
    fn new(nombre: String, perfil: Perfil) -> Cargado {
        let motor = AjustesMotor::desde_perfil(&perfil);
        Cargado {
            luz: AjustesLuz::desde_perfil(&perfil.ajustes),
            volante: salida::eje_volante(motor.botones.volante.eje),
            motor,
            nombre,
            perfil,
        }
    }
}

/// La vibración que pide el juego, que llega por otro hilo.
#[derive(Default)]
struct Vibracion {
    grande: AtomicU16,
    pequeno: AtomicU16,
}

fn enviar_eventos(eventos: &[EventoRaton]) {
    for e in eventos {
        match *e {
            EventoRaton::Mover(x, y) => teclado::mover(x, y),
            EventoRaton::Absoluto(x, y) => teclado::mover_absoluto(x, y),
            EventoRaton::Rueda(v, h) => teclado::rueda(v, h),
            EventoRaton::Boton(b) => teclado::boton(b),
            EventoRaton::BotonX(b, cual) => teclado::boton_x(b, cual),
            EventoRaton::Tecla(k) => teclado::pulsar(k),
            EventoRaton::TeclaAlt(k) => teclado::pulsar_exploracion(k),
            EventoRaton::SueltaTecla(k) => teclado::soltar(k),
            EventoRaton::SueltaTeclaAlt(k) => teclado::soltar_exploracion(k),
        }
    }
}

/// `isDS5Idle`, simplificado: hay uso si cambia algún botón o un stick sale del centro.
fn en_uso(e: &Estado, antes: &Estado) -> bool {
    const ZONA: i32 = 10;
    let fuera = |v: u8| (v as i32 - 128).abs() > ZONA;
    e.botones != antes.botones
        || fuera(e.lx)
        || fuera(e.ly)
        || fuera(e.rx)
        || fuera(e.ry)
        || e.l2 > 0
        || e.r2 > 0
}

/// `SaveControllerConfigs` al acabar de calibrar el volante.
fn guardar_calibracion(en: &Entorno, mac: [u8; 6], c: &ds5_mapeo::CalibracionVolante) {
    let ruta = en.carpeta.ruta.join(calibracion::ARCHIVO);
    let anterior = std::fs::read_to_string(&ruta).ok();
    // `InputDeviceType`: el Edge también es "DualSense".
    let texto = calibracion::escribir(anterior.as_deref(), mac, "DualSense", c);
    if let Err(e) = en.guardado.escribir(calibracion::ARCHIVO, &texto) {
        let idioma = Idioma::de_ajustes(&en.carpeta.ajustes());
        let _ = en.avisos.send(Aviso::Registro(
            idioma.formato("No se pudo guardar la calibración del volante: {}", &[&e]),
            true,
        ));
    }
}

fn hilo(mando: &Arc<Mando>, d: Dispositivo, ordenes: Receiver<Orden>, en: &Entorno) {
    let us = || en.inicio.elapsed().as_micros() as i64;
    let conexion = d.info.conexion;
    let cal = leer_calibracion(&d);
    let mandar = |s: &Salida| d.escribir(&s.codificar(conexion), Duration::from_millis(100));
    if conexion == Conexion::Bt && mandar(&Salida::inicial_bt()).is_err() {
        return;
    }

    let (nombre, perfil) = en.carpeta.perfil_de(mando.ranura);
    // El perfil normal del mando, al que se vuelve después de un perfil temporal.
    let mut normal = nombre.clone();
    let mut c = Cargado::new(nombre, perfil);
    let acciones = || en.acciones.lock().unwrap().clone();
    let mac = leer_mac(&d);
    let mut calibracion = std::fs::read_to_string(en.carpeta.ruta.join(calibracion::ARCHIVO))
        .ok()
        .and_then(|t| calibracion::leer(&t, mac));
    let mut motor = Motor::nuevo(&c.motor);
    motor.botones.volante = ds5_mapeo::Volante::nuevo(calibracion);
    motor.botones.acciones = AccionesEspeciales::nuevas(&acciones());
    let mut luz = Luz::nueva(us());

    // El mando virtual y el hilo que recoge la vibración que pide el juego.
    let vibracion = Arc::new(Vibracion::default());
    let mut virtual_: Option<(Tipo, Arc<MandoVirtual>)> = None;
    let enchufar = |tipo: Option<Tipo>, actual: &mut Option<(Tipo, Arc<MandoVirtual>)>| {
        if actual.as_ref().map(|(t, _)| *t) == tipo {
            return;
        }
        *actual = None;
        let (Some(tipo), Some(bus)) = (tipo, en.bus.as_ref()) else {
            return;
        };
        match MandoVirtual::enchufar(bus, tipo) {
            Ok(v) => {
                let v = Arc::new(v);
                let (v2, vib, m) = (v.clone(), vibracion.clone(), mando.clone());
                std::thread::spawn(move || loop {
                    if m.parar.load(Ordering::Relaxed) || Arc::strong_count(&v2) == 1 {
                        break;
                    }
                    match v2.esperar_respuesta(Duration::from_millis(200)) {
                        Ok(r) if r.con_vibracion => {
                            vib.grande.store(r.motor_grande as u16, Ordering::Relaxed);
                            vib.pequeno.store(r.motor_pequeno as u16, Ordering::Relaxed);
                        }
                        Ok(_) | Err(ErrorVirtual::TiempoAgotado) => {}
                        Err(_) => std::thread::sleep(Duration::from_millis(50)),
                    }
                });
                *actual = Some((tipo, v));
            }
            Err(e) => {
                let idioma = Idioma::de_ajustes(&en.carpeta.ajustes());
                let _ = en.avisos.send(Aviso::Registro(
                    idioma.formato("No se pudo enchufar el mando virtual: {}", &[&e]),
                    true,
                ));
            }
        }
    };
    enchufar(salida::virtual_del_perfil(&c.perfil), &mut virtual_);
    let _ = en.avisos.send(Aviso::Conectado(mando.ranura));
    // DSU solo tiene 4 ranuras.
    let dsu = en.dsu.as_ref().filter(|_| mando.ranura < 4);
    if let Some(s) = dsu {
        s.reiniciar_suavizado(mando.ranura as u8);
    }

    let mut dec = Decodificador::new(conexion, cal, us());
    let mut bufer = vec![0u8; d.info.largo_entrada.max(78)];
    let mut ultima_salida: Option<Ajustes> = None;
    let mut antes = Estado::default();
    let mut ultimo_uso = us();
    let mut prueba_hasta: Option<Instant> = None;
    let mut errores = 0u32;
    let mut ultimo_dato = Instant::now();
    let mut guardar_en: Option<Instant> = None;

    while !mando.parar.load(Ordering::Relaxed) {
        // Las órdenes de la interfaz.
        loop {
            match ordenes.try_recv() {
                Ok(Orden::Ajustar(f)) => {
                    let mut p = c.perfil.clone();
                    f(&mut p);
                    c = Cargado::new(c.nombre.clone(), p);
                    enchufar(salida::virtual_del_perfil(&c.perfil), &mut virtual_);
                    // Como el temporizador `save` de C#: se guarda un segundo después del último
                    // cambio.
                    guardar_en = Some(Instant::now() + GUARDAR_TRAS);
                }
                Ok(Orden::Temporal(n)) => {
                    let n = n.unwrap_or_else(|| normal.clone());
                    let p = en.carpeta.cargar(&n).unwrap_or_default();
                    c = Cargado::new(n, p);
                    motor.botones.acciones.perfil_cargado(&c.perfil.acciones);
                    enchufar(salida::virtual_del_perfil(&c.perfil), &mut virtual_);
                }
                Ok(Orden::RecargarAcciones) => {
                    motor.botones.acciones = AccionesEspeciales::nuevas(&acciones());
                }
                Ok(Orden::Cargar(nombre)) => {
                    normal = nombre.clone();
                    let p = en.carpeta.cargar(&nombre).unwrap_or_default();
                    c = Cargado::new(nombre, p);
                    motor = Motor::nuevo(&c.motor);
                    motor.botones.volante = ds5_mapeo::Volante::nuevo(calibracion);
                    motor.botones.acciones = AccionesEspeciales::nuevas(&acciones());
                    motor.botones.acciones.perfil_cargado(&c.perfil.acciones);
                    enchufar(salida::virtual_del_perfil(&c.perfil), &mut virtual_);
                }
                Ok(Orden::ProbarVibracion(t)) => prueba_hasta = Some(Instant::now() + t),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        // `SaveProfile`: un perfil temporal no es el elegido para el mando y no se guarda.
        if guardar_en.is_some_and(|t| Instant::now() >= t) {
            guardar_en = None;
            if c.nombre == normal {
                let texto = c.perfil.escribir(Some(&encabezado()));
                if let Err(e) = en
                    .guardado
                    .escribir(format!("Profiles/{}.xml", c.nombre), &texto)
                {
                    let idioma = Idioma::de_ajustes(&en.carpeta.ajustes());
                    let _ = en.avisos.send(Aviso::Registro(
                        idioma.formato("No se pudo guardar el perfil {}: {}", &[&c.nombre, &e]),
                        true,
                    ));
                }
            }
        }

        let n = match d.leer(&mut bufer, Duration::from_millis(500)) {
            Ok(n) => n,
            Err(ErrorHid::TiempoAgotado) => {
                // Como `READ_STREAM_TIMEOUT` y `TimeoutTestThread` de C#: el mando manda informes
                // sin parar, así que 3 s sin nada es que se ha apagado. Si no se diera por
                // desconectado, se quedaría con la última lectura (el botón PS que se mantiene
                // para apagarlo) y al encenderlo de nuevo no se volvería a coger. Antes se pide un
                // informe de característica para que Windows note que el dispositivo ya no está.
                if ultimo_dato.elapsed() >= SIN_DATOS {
                    let mut serie = [0u8; 64];
                    serie[0] = 0x09;
                    let _ = d.leer_caracteristica(&mut serie);
                    return;
                }
                continue;
            }
            Err(_) => {
                errores += 1;
                if errores > 3 {
                    return;
                }
                continue;
            }
        };
        errores = 0;
        ultimo_dato = Instant::now();
        let informe = &bufer[..n];
        if !dec.valido(informe) {
            if conexion == Conexion::Bt && informe.first() == Some(&0x31) {
                dec.perdido();
            }
            continue;
        }
        let ahora = us();
        let crudo = dec.decodificar(informe, ahora);
        if let Some(s) = dsu {
            let meta = ds5_dsu::Meta::de_mando(mando.ranura as u8, mac, conexion, true, &crudo);
            s.enviar_estado(&meta, &crudo);
        }
        if crudo.pulsado(boton::MUTE) && !antes.pulsado(boton::MUTE) {
            let _ = en.avisos.send(Aviso::Mute(mando.ranura));
        }
        if en_uso(&crudo, &antes) {
            ultimo_uso = ahora;
        }

        let paso = motor.procesar(&crudo, &c.motor, ahora);
        enviar_eventos(&paso.eventos);
        if let Some(nueva) = motor.botones.volante.salida.guardar.take() {
            calibracion = Some(nueva);
            guardar_calibracion(en, mac, &nueva);
        }
        let peticiones = std::mem::take(&mut motor.botones.acciones.peticiones);
        for p in peticiones {
            // Los perfiles temporales se cargan aquí mismo, sin tocar el estado de las acciones
            // (`LoadTempProfile`); lo demás lo hace la app.
            let temporal = match &p {
                Peticion::CargarPerfil(n) => Some(n.clone()),
                Peticion::VolverPerfil(n) => Some(n.clone().unwrap_or_else(|| normal.clone())),
                _ => None,
            };
            match temporal {
                Some(n) => {
                    let perfil = en.carpeta.cargar(&n).unwrap_or_default();
                    c = Cargado::new(n, perfil);
                    motor.botones.acciones.perfil_cargado(&c.perfil.acciones);
                    enchufar(salida::virtual_del_perfil(&c.perfil), &mut virtual_);
                }
                None => {
                    let _ = en.avisos.send(Aviso::Peticion(mando.ranura, p));
                }
            }
        }
        if let Some((tipo, v)) = &virtual_ {
            let r = match salida::informe(*tipo, &paso.salida, c.volante) {
                InformeVirtual::Xbox(x) => v.enviar_xbox(&x),
                InformeVirtual::Ds4(r) => v.enviar_ds4(&r),
            };
            if r.is_err() {
                virtual_ = None;
            }
        }

        // La luz, la vibración y los gatillos del mando.
        let extras = &motor.botones.salida_extras;
        let (grande, pequeno) = if prueba_hasta.is_some_and(|t| Instant::now() < t) {
            (255, 255)
        } else {
            prueba_hasta = None;
            match extras.vibracion.or(motor.botones.macros.vibracion) {
                Some((g, p)) => (g, p),
                None => (
                    vibracion.grande.load(Ordering::Relaxed) as u8,
                    vibracion.pequeno.load(Ordering::Relaxed) as u8,
                ),
            }
        };
        let a = &c.perfil.ajustes;
        let (derecho, izquierdo) =
            salida::vibracion_reforzada(grande, pequeno, a.vibracion, a.invertir_motores);
        luz.actualizar(
            &c.luz,
            &EntradaLuz {
                bateria: crudo.bateria as i32,
                cargando: crudo.cargando,
                bluetooth: conexion == Conexion::Bt,
                personal: None,
                por_defecto: false,
                apagando: false,
                distancia: false,
                vibracion_fuerte: grande,
                ultimo_uso_us: ultimo_uso,
                ahora_us: ahora,
            },
            &extras.luz,
        );
        let (l2, r2) = salida::gatillos(&c.perfil);
        let deseada = Ajustes {
            motor_derecho: derecho,
            motor_izquierdo: izquierdo,
            gatillo_izquierdo: l2,
            gatillo_derecho: r2,
            // `apagada` es `LightBarExplicitlyOff` de C#: dice que el color está puesto a
            // propósito, no que la luz vaya apagada. El color es siempre el que manda.
            color: luz.estado.color,
            ..Default::default()
        };
        // Sin `outputDataToDS4` (modo nativo) no se escribe nada en el mando: el juego lo maneja.
        if c.perfil.ajustes.enviar_al_mando
            && ultima_salida != Some(deseada)
            && mandar(&Salida::normal(&deseada)).is_ok()
        {
            ultima_salida = Some(deseada);
        }

        {
            let mut v = mando.vivo.lock().unwrap();
            v.crudo = crudo.clone();
            v.bateria = Some(crudo.bateria);
            v.cargando = crudo.cargando;
            v.luz = deseada.color;
            if v.normal != normal {
                v.normal = normal.clone();
            }
            if v.perfil != c.nombre || v.ajustes != c.perfil {
                v.perfil = c.nombre.clone();
                v.ajustes = c.perfil.clone();
            }
            v.emulando = virtual_.as_ref().map(|(t, _)| *t);
        }
        antes = crudo;
    }

    if let Some(s) = dsu {
        s.desconectar(mando.ranura as u8);
    }
    // Al parar, la luz y los motores como si no hubiera nadie.
    let _ = mandar(&Salida::normal(&Ajustes::default()));
}
