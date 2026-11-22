//! DS5Control en Rust. La ventana carga la misma interfaz Vue que la versión en C# (`ui/dist`) y
//! habla con ella con los mismos mensajes JSON de `ui/src/bridge/tipos.ts`:
//!
//! - la interfaz llama al comando `mensaje` con lo que en C# mandaba por `postMessage`;
//! - la app emite el evento `mensaje` con lo que en C# mandaba por `PostWebMessageAsJson`.
//!
//! El trabajo de verdad lo hace `ds5-servicio`; aquí está lo que en C# hacen `WebUiWindow.cs` y
//! `AppHost.cs`: el estado que se pinta, los comandos, la bandeja, el botón Mute y manejar la
//! interfaz con el mando.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod archivo_registro;
mod colocacion;
mod instaladores;
mod limpieza;
mod navegador;
mod preferencias;
mod sistema;
mod textos;

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ds5_informes::boton;
use ds5_mapeo::Peticion;
use ds5_servicio::interfaz::{self, NivelLuz, PRUEBA_VIBRACION_MAXIMA_MS};
use ds5_servicio::{programa, Aviso, Orden, Servicio};
use serde_json::{json, Value};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use ds5_autoperfiles::panel as autopanel;
use ds5_autoperfiles::{EstadoApp as AutoEstado, EstadoMando, Peticion as AutoPeticion, Vigilante};
use ds5_perfiles::Autoperfiles;
use ds5_servicio::{acciones, generales, opciones};
use preferencias::Preferencias;

const EVENTO: &str = "mensaje";
const VENTANA: &str = "principal";
/// `MaxLogEntries`.
const MAXIMO_REGISTRO: usize = 150;
/// `ActivePollMs` y `BackgroundPollMs`.
const SONDEO_ACTIVO: Duration = Duration::from_millis(33);
const SONDEO_FONDO: Duration = Duration::from_millis(120);
const SONDEO_OCULTA: Duration = Duration::from_millis(250);
/// `ScrollDeadZone`.
const ZONA_DESPLAZAR: f64 = 0.25;

struct App {
    servicio: Mutex<Servicio>,
    niveles: Mutex<HashMap<String, NivelLuz>>,
    preferencias: Mutex<Preferencias>,
    registro: Mutex<Vec<Value>>,
    /// La página ha mandado `listo`: ya escucha.
    lista: Mutex<bool>,
    /// El último estado mandado, para no repetirlo.
    ultimo: Mutex<String>,
    /// Los botones de navegación que siguen pulsados, para avisar solo al pulsar.
    navegando: Mutex<HashSet<&'static str>>,
    /// Para las notificaciones, que salen del registro.
    app: std::sync::OnceLock<AppHandle>,
    /// Cuándo se movió o cambió de tamaño la ventana por última vez, para guardarlo al pararse.
    colocacion_pendiente: Mutex<Option<std::time::Instant>>,
    /// Un autoperfil con "apagar" ha parado el motor.
    apagado: Mutex<bool>,
    /// Hay un instalador de ViGEmBus o de HidHide abierto.
    instalando_vigem: Mutex<bool>,
    instalando_hidhide: Mutex<bool>,
    /// La sección Datos está abierta: el estado lleva la telemetría.
    telemetria: Mutex<bool>,
    /// El idioma de los textos que se mandan ya escritos (`UseLang` o el de Windows).
    idioma: Mutex<ds5_servicio::Idioma>,
    /// `Logs\ds5control_log.txt`.
    archivo: archivo_registro::ArchivoRegistro,
    /// `Auto Profiles.xml`.
    autoperfiles: Mutex<Autoperfiles>,
    vigilante: Mutex<Vigilante>,
}

impl App {
    /// `ShowNotification`: con la ventana sin el foco, una notificación de Windows si lo piden
    /// los ajustes (2 todas, 1 solo los avisos, 0 ninguna).
    fn notificar(&self, texto: &str, aviso: bool) {
        let Some(app) = self.app.get() else {
            return;
        };
        let activa = ventana(app).is_some_and(|v| v.is_focused().unwrap_or(false));
        if activa {
            return;
        }
        let nivel = match self.servicio.try_lock() {
            Ok(s) => s.carpeta().ajustes().generales.avisos,
            Err(_) => return,
        };
        if nivel == 2 || (nivel == 1 && aviso) {
            use tauri_plugin_notification::NotificationExt;
            let _ = app
                .notification()
                .builder()
                .title("DS5Control")
                .body(texto)
                .show();
        }
    }

    /// Cambia los autoperfiles y, si ha cambiado algo, guarda `Auto Profiles.xml`.
    fn cambiar_autoperfiles(&self, cambio: impl FnOnce(&mut Autoperfiles) -> bool) {
        let texto = {
            let mut l = self.autoperfiles.lock().unwrap();
            if !cambio(&mut l) {
                return;
            }
            self.vigilante.lock().unwrap().cambiar_reglas(&l);
            // Como `AutoProfileHolder.Save`: con la marca UTF-8 y la fecha invariante.
            let comentario = format!(
                "Auto-Profile Configuration Data. {}",
                ds5_servicio::guardado::fecha_invariante()
            );
            format!(
                "\u{feff}{}",
                ds5_servicio::guardado::con_comentario(&l.escribir(), &comentario)
            )
        };
        let r = self
            .servicio
            .lock()
            .unwrap()
            .guardado()
            .escribir("Auto Profiles.xml", &texto);
        if let Err(e) = r {
            self.anotar(format!("No se pudo guardar Auto Profiles.xml: {e}"), true);
        }
    }

    fn anotar(&self, texto: String, aviso: bool) {
        let texto = textos::traducir(*self.idioma.lock().unwrap(), &texto);
        self.notificar(&texto, aviso);
        self.archivo.escribir(&texto, aviso);
        let mut r = self.registro.lock().unwrap();
        r.push(json!({
            "hora": chrono::Local::now().format("%H:%M:%S").to_string(),
            "texto": texto,
            "aviso": aviso,
        }));
        if r.len() > MAXIMO_REGISTRO {
            r.remove(0);
        }
    }

    /// `Poll`: el estado de toda la app tal como lo pinta la interfaz.
    fn estado(&self, ventana_visible: bool) -> Value {
        let servicio = self.servicio.lock().unwrap();
        let mut niveles = self.niveles.lock().unwrap();
        let mandos: Vec<Value> = servicio
            .mandos()
            .iter()
            .map(|m| {
                let nivel = niveles.entry(m.id().to_string()).or_default();
                interfaz::estado_mando(m, &m.vivo(), nivel, *self.telemetria.lock().unwrap())
            })
            .collect();
        let p = self.preferencias.lock().unwrap();
        json!({
            "tipo": "estado",
            "estado": {
                "mandos": mandos,
                "abrirConMute": p.abrir_con_mute,
                "controlConMando": p.control_con_mando,
                "ventanaVisible": ventana_visible,
                "idioma": self.idioma.lock().unwrap().codigo(),
                "version": env!("CARGO_PKG_VERSION"),
                "perfiles": servicio.carpeta().perfiles(),
                "registro": *self.registro.lock().unwrap(),
                "servicios": {
                    "vigemBus": servicio.vigem,
                    "instalandoVigem": *self.instalando_vigem.lock().unwrap(),
                    "hidHide": servicio.hidhide,
                    "instalandoHidHide": *self.instalando_hidhide.lock().unwrap(),
                    "ofrecerHidHide": !servicio.hidhide && !p.hidhide_ofrecido,
                },
            },
        })
    }
}

fn ventana(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(VENTANA)
}

/// `Reveal`: la ventana delante y con el foco.
fn mostrar(app: &AppHandle) {
    match ventana(app) {
        Some(v) => traer(&v),
        // La ventana se crea la primera vez que se abre, como en C#: oculta en la bandeja, la
        // app no carga la interfaz y gasta mucha menos memoria. Se crea en otro hilo, porque
        // crear una ventana desde el hilo principal puede bloquear la app en Windows.
        None => {
            let app = app.clone();
            std::thread::spawn(move || {
                if let Some(v) = crear_ventana(&app) {
                    traer(&v);
                }
            });
        }
    }
}

fn traer(v: &WebviewWindow) {
    let _ = v.show();
    let _ = v.unminimize();
    let _ = v.set_focus();
}

/// La ventana de `tauri.conf.json`, en el sitio y tamaño en que se dejó.
fn crear_ventana(app: &AppHandle) -> Option<WebviewWindow> {
    // Otro hilo puede haberla creado mientras tanto.
    let _creando = CREANDO_VENTANA.lock().unwrap();
    if let Some(v) = ventana(app) {
        return Some(v);
    }
    let config = app.config().app.windows.first()?.clone();
    let v = tauri::WebviewWindowBuilder::from_config(app, &config)
        .ok()?
        .build()
        .ok()?;
    let carpeta = app
        .state::<Arc<App>>()
        .servicio
        .lock()
        .unwrap()
        .carpeta()
        .ruta
        .clone();
    restaurar_colocacion(&v, &carpeta);
    if let Some(i) = navegador::paleta_guardada(&carpeta)
        .and_then(|p| logo(&p))
        .and_then(|b| tauri::image::Image::from_bytes(b).ok())
    {
        let _ = v.set_icon(i);
    }
    Some(v)
}

static CREANDO_VENTANA: Mutex<()> = Mutex::new(());

fn emitir(app: &AppHandle, mensaje: Value) {
    if *app.state::<Arc<App>>().lista.lock().unwrap() {
        let _ = app.emit(EVENTO, mensaje);
    }
}

#[tauri::command]
fn mensaje(app: AppHandle, mensaje: Value) {
    match mensaje.get("tipo").and_then(Value::as_str) {
        Some("listo") => {
            let estado = app.state::<Arc<App>>();
            *estado.lista.lock().unwrap() = true;
            estado.ultimo.lock().unwrap().clear();
        }
        Some("comando") => comando(&app, &mensaje["comando"]),
        // El tema, la paleta y los colores guardados de la página, para el archivo compartido.
        Some("navegador") => {
            let estado = app.state::<Arc<App>>();
            let texto = mensaje["datos"].to_string();
            let r = estado
                .servicio
                .lock()
                .unwrap()
                .guardado()
                .escribir(navegador::ARCHIVO, &texto);
            if let Err(e) = r {
                estado.anotar(
                    format!("No se pudo guardar {}: {e}", navegador::ARCHIVO),
                    true,
                );
            }
        }
        _ => {}
    }
}

/// `RunCommand`.
fn comando(app: &AppHandle, c: &Value) {
    let estado = app.state::<Arc<App>>();
    let tipo = c["tipo"].as_str().unwrap_or("");
    match tipo {
        "ocultar" => {
            if let Some(v) = ventana(app) {
                let _ = v.hide();
            }
        }
        "ventana" => {
            if let Some(v) = ventana(app) {
                let _ = if c["accion"] == "minimizar" {
                    v.hide()
                } else if v.is_maximized().unwrap_or(false) {
                    v.unmaximize()
                } else {
                    v.maximize()
                };
            }
        }
        "borrarPerfil" => {
            let nombre = c["nombre"].as_str().unwrap_or("");
            if let Err(e) = estado.servicio.lock().unwrap().borrar_perfil(nombre) {
                estado.anotar(format!("No se pudo borrar el perfil: {e}"), true);
            }
        }
        "pedirAuto" => enviar_autoperfiles(app, &estado),
        // `SetAppIcon`: el icono de la ventana, la barra de tareas y la bandeja sigue a la paleta.
        "paleta" => {
            if let Some(p) = c["valor"].as_str() {
                poner_icono(app, p);
            }
        }
        "pedirDiagnostico" => {
            // `SendDiagnostics`: con el primer mando, en otro hilo porque tarda segundos.
            let primero = estado.servicio.lock().unwrap().mandos().into_iter().next();
            let app = app.clone();
            std::thread::spawn(move || {
                let datos = primero
                    .and_then(|m| ds5_hid::Dispositivo::abrir(&m.info).ok())
                    .map_or(Value::Null, |d| ds5_diagnostico::leer(&d));
                emitir(&app, json!({ "tipo": "diagnostico", "datos": datos }));
            });
        }
        "instalarVigem" => instalar(app, false),
        "instalarHidHide" => instalar(app, true),
        "pedirGenerales" => enviar_generales(app, &estado),
        "telemetria" => *estado.telemetria.lock().unwrap() = c["activo"].as_bool().unwrap_or(false),
        "general" => {
            let id = c["id"].as_str().unwrap_or("");
            general(app, &estado, id, &c["valor"]);
            enviar_generales(app, &estado);
        }
        "autoAnadir" => {
            let app = app.clone();
            std::thread::spawn(move || {
                let estado = app.state::<Arc<App>>();
                if let Some(ruta) = elegir_archivo(&app, "Juegos", &["exe"]) {
                    let primero = estado
                        .servicio
                        .lock()
                        .unwrap()
                        .carpeta()
                        .perfiles()
                        .into_iter()
                        .next()
                        .unwrap_or_default();
                    estado.cambiar_autoperfiles(|l| autopanel::anadir(l, &ruta, &primero));
                }
                enviar_autoperfiles(&app, &estado);
            });
        }
        "autoPerfil" | "autoQuitar" => {
            let ruta = c["ruta"].as_str().unwrap_or("");
            if tipo == "autoPerfil" {
                let perfil = c["perfil"].as_str().unwrap_or("");
                estado.cambiar_autoperfiles(|l| autopanel::poner_perfil(l, ruta, perfil));
            } else {
                estado.cambiar_autoperfiles(|l| autopanel::quitar(l, ruta));
            }
            enviar_autoperfiles(app, &estado);
        }
        "abrirConMute" | "controlConMando" | "rechazarHidHide" => {
            let mut p = estado.preferencias.lock().unwrap();
            let activo = c["activo"].as_bool().unwrap_or(false);
            match tipo {
                "abrirConMute" => p.abrir_con_mute = activo,
                "controlConMando" => p.control_con_mando = activo,
                _ => p.hidhide_ofrecido = true,
            }
            let texto = p.texto();
            drop(p);
            let r = estado
                .servicio
                .lock()
                .unwrap()
                .guardado()
                .escribir(preferencias::ARCHIVO, &texto);
            if let Err(e) = r {
                estado.anotar(
                    format!("No se pudo guardar {}: {e}", preferencias::ARCHIVO),
                    true,
                );
            }
        }
        _ => {
            let Some(id) = c["mandoId"].as_str() else {
                return;
            };
            let Some(m) = estado.servicio.lock().unwrap().mando(id) else {
                return;
            };
            match tipo {
                "probarVibracion" => {
                    let ms = c["duracionMs"].as_u64().unwrap_or(0);
                    if ms > 0 {
                        m.ordenar(Orden::ProbarVibracion(Duration::from_millis(
                            ms.min(PRUEBA_VIBRACION_MAXIMA_MS),
                        )));
                    }
                }
                "perfil" | "nuevoPerfil" => {
                    let nombre = c["nombre"].as_str().unwrap_or("");
                    let servicio = estado.servicio.lock().unwrap();
                    let r = if tipo == "perfil" {
                        servicio.elegir_perfil(&m, nombre)
                    } else {
                        servicio.nuevo_perfil(&m, nombre)
                    };
                    if let Err(e) = r {
                        estado.anotar(format!("No se pudo guardar: {e}"), true);
                    }
                }
                "pedirAcciones" => enviar_acciones(app, &estado, &m),
                "pedirOpciones" => enviar_opciones(app, &m.vivo().ajustes),
                "opcion" => {
                    let mut perfil = m.vivo().ajustes;
                    let id = c["id"].as_str().unwrap_or("");
                    if opciones::aplicar(&mut perfil, id, &c["valor"]) {
                        // El perfil se vuelve a montar entero en el hilo del mando, como al
                        // recargarlo en C#.
                        enviar_opciones(app, &perfil);
                        m.ordenar(Orden::Ajustar(Box::new(move |p| *p = perfil)));
                    }
                }
                "guardarAccion" | "borrarAccion" | "activarAccion" => {
                    let mut perfil = m.vivo().ajustes;
                    let antes = perfil.clone();
                    let servicio = estado.servicio.lock().unwrap();
                    let nombre = c["nombre"].as_str().unwrap_or("").to_string();
                    let r = match tipo {
                        "guardarAccion" => {
                            servicio.cambiar_acciones(|a| acciones::guardar(c, a, &mut perfil))
                        }
                        "borrarAccion" => servicio.cambiar_acciones(|a| {
                            acciones::borrar(a, &mut perfil, &nombre);
                            true
                        }),
                        _ => {
                            acciones::activar(
                                &mut perfil,
                                &nombre,
                                c["activa"].as_bool().unwrap_or(false),
                            );
                            Ok(true)
                        }
                    };
                    drop(servicio);
                    if let Err(e) = r {
                        estado.anotar(format!("No se pudo guardar Actions.xml: {e}"), true);
                    }
                    if perfil != antes {
                        m.ordenar(Orden::Ajustar(Box::new(move |p| *p = perfil)));
                    }
                    enviar_acciones(app, &estado, &m);
                }
                "elegirPrograma" => {
                    let app = app.clone();
                    std::thread::spawn(move || {
                        if let Some(ruta) =
                            elegir_archivo(&app, "Programas", &["exe", "bat", "cmd", "lnk"])
                        {
                            emitir(&app, json!({ "tipo": "archivo", "ruta": ruta }));
                        }
                    });
                }
                _ => {
                    let mut perfil = m.vivo().ajustes;
                    let mut niveles = estado.niveles.lock().unwrap();
                    let nivel = niveles.entry(id.to_string()).or_default();
                    if interfaz::aplicar(c, &mut perfil, nivel) {
                        m.ordenar(Orden::Ajustar(Box::new(move |p| *p = perfil)));
                    }
                }
            }
        }
    }
}

/// `SendOptions`.
fn enviar_opciones(app: &AppHandle, perfil: &ds5_perfiles::Perfil) {
    let idioma = *app.state::<Arc<App>>().idioma.lock().unwrap();
    emitir(
        app,
        json!({ "tipo": "opciones", "grupos": opciones::describir(perfil, idioma) }),
    );
}

/// `SendGeneral`.
fn enviar_generales(app: &AppHandle, estado: &App) {
    let ajustes = estado.servicio.lock().unwrap().carpeta().ajustes();
    let s = generales::Sistema {
        inicio_windows: sistema::inicio_windows(),
        prioridad: sistema::prioridad(),
        vigem: ds5_virtual::Bus::abrir().is_ok(),
        hidhide: ds5_virtual::hidhide::instalado(),
    };
    emitir(
        app,
        json!({ "tipo": "generales", "secciones": generales::describir(&ajustes, s, *estado.idioma.lock().unwrap()) }),
    );
}

/// `GeneralOptions.Apply` y lo que hace cada botón.
fn general(app: &AppHandle, estado: &App, id: &str, valor: &Value) {
    let servicio = estado.servicio.lock().unwrap();
    let mut ajustes = servicio.carpeta().ajustes();
    let cambio = generales::aplicar(&mut ajustes, id, valor);
    let guardar = || {
        let texto = ajustes.escribir(Some(&ds5_servicio::guardado::encabezado()));
        servicio.guardado().escribir("Profiles.xml", &texto)
    };
    match cambio {
        generales::Cambio::Nada => {}
        generales::Cambio::Guardar | generales::Cambio::GuardarYReiniciar => {
            if let Err(e) = guardar() {
                estado.anotar(format!("No se pudo guardar Profiles.xml: {e}"), true);
            }
            if cambio == generales::Cambio::GuardarYReiniciar {
                drop(servicio);
                reiniciar(app, estado);
            }
        }
        generales::Cambio::Idioma(elegido) => {
            if let Err(e) = guardar() {
                estado.anotar(format!("No se pudo guardar Profiles.xml: {e}"), true);
            }
            *estado.idioma.lock().unwrap() =
                elegido.unwrap_or_else(ds5_servicio::Idioma::del_sistema);
            drop(servicio);
            // Los textos que escribe la app salen ya en el idioma nuevo.
            enviar_generales(app, estado);
            if let (Some(bandeja), Ok(menu)) = (app.tray_by_id("bandeja"), menu_bandeja(app)) {
                let _ = bandeja.set_menu(Some(menu));
            }
        }
        generales::Cambio::Prioridad(i) => sistema::poner_prioridad(i),
        generales::Cambio::InicioWindows(activar) => {
            if let Err(e) = sistema::poner_inicio_windows(activar) {
                estado.anotar(
                    format!("No se pudo cambiar el arranque con Windows: {e}"),
                    true,
                );
            }
        }
        generales::Cambio::Boton("reiniciar") => {
            drop(servicio);
            reiniciar(app, estado);
        }
        generales::Cambio::Boton("vigem") => instalar_o_desinstalar(app, false),
        generales::Cambio::Boton("hidhide") => instalar_o_desinstalar(app, true),
        generales::Cambio::Boton("carpeta") => {
            let _ = std::process::Command::new("explorer")
                .arg(&servicio.carpeta().ruta)
                .spawn();
        }
        generales::Cambio::Boton("buscarVersion") => {
            let app = app.clone();
            std::thread::spawn(move || buscar_version(&app, true));
        }
        generales::Cambio::Boton(otro) => estado.anotar(
            format!("Todavía no está en la versión de Rust: {otro}"),
            true,
        ),
    }
}

/// `InstallViGEmAsync` e `InstallHidHideAsync`, en otro hilo: el instalador pide permisos de
/// administrador y se espera a que acabe. Si el controlador queda funcionando, se reinicia el
/// motor para que lo use.
fn instalar(app: &AppHandle, hidhide: bool) {
    let app = app.clone();
    std::thread::spawn(move || {
        let estado = app.state::<Arc<App>>();
        let (nombre, archivo) = if hidhide {
            ("HidHide", instaladores::HIDHIDE)
        } else {
            ("ViGEmBus", instaladores::VIGEM)
        };
        let ruta = instaladores::ruta(archivo);
        if !ruta.is_file() {
            estado.anotar(
                format!("Falta el instalador de {nombre} en {}", ruta.display()),
                true,
            );
            return;
        }
        let bandera = if hidhide {
            &estado.instalando_hidhide
        } else {
            &estado.instalando_vigem
        };
        *bandera.lock().unwrap() = true;
        let argumentos = hidhide.then_some("/passive /norestart");
        let fin = instaladores::ejecutar(&ruta, argumentos);
        *bandera.lock().unwrap() = false;
        let instalado = if hidhide {
            ds5_virtual::hidhide::instalado()
        } else {
            ds5_virtual::Bus::abrir().is_ok()
        };
        let (texto, aviso) = if hidhide {
            instaladores::mensaje_hidhide(fin, instalado)
        } else {
            instaladores::mensaje_vigem(fin, instalado)
        };
        estado.anotar(texto, aviso);
        if instalado {
            reiniciar(&app, &estado);
        }
        enviar_generales(&app, &estado);
    });
}

/// El botón de Herramientas: si el controlador está instalado lo desinstala con el desinstalador
/// que tiene registrado Windows (que pide confirmación); si no, lo instala.
fn instalar_o_desinstalar(app: &AppHandle, hidhide: bool) {
    let instalado = if hidhide {
        ds5_virtual::hidhide::instalado()
    } else {
        ds5_virtual::Bus::abrir().is_ok()
    };
    if !instalado {
        instalar(app, hidhide);
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let estado = app.state::<Arc<App>>();
        let nombre = if hidhide {
            instaladores::NOMBRE_HIDHIDE
        } else {
            instaladores::NOMBRE_VIGEM
        };
        let corto = if hidhide { "HidHide" } else { "ViGEmBus" };
        let Some((programa, argumentos)) =
            instaladores::orden_desinstalar(nombre).and_then(|o| instaladores::partir_orden(&o))
        else {
            estado.anotar(
                format!("No se encuentra el desinstalador de {corto} en Windows."),
                true,
            );
            return;
        };
        // Primero se sueltan los mandos, para que el controlador no esté en uso.
        estado.servicio.lock().unwrap().parar();
        let fin = instaladores::ejecutar(std::path::Path::new(&programa), argumentos.as_deref());
        let sigue = if hidhide {
            ds5_virtual::hidhide::instalado()
        } else {
            ds5_virtual::Bus::abrir().is_ok()
        };
        let (texto, aviso) = instaladores::mensaje_desinstalar(corto, fin, sigue);
        estado.anotar(texto, aviso);
        reiniciar(&app, &estado);
        enviar_generales(&app, &estado);
    });
}

/// `RestartService`: para todos los mandos y los vuelve a buscar con los ajustes del disco.
fn reiniciar(app: &AppHandle, estado: &App) {
    let mut servicio = estado.servicio.lock().unwrap();
    let carpeta = servicio.carpeta().ruta.clone();
    servicio.parar();
    let (nuevo, avisos) = Servicio::arrancar(&carpeta, true);
    *servicio = nuevo;
    drop(servicio);
    let h = app.clone();
    std::thread::spawn(move || atender_avisos(h, avisos));
    estado.anotar("Motor reiniciado".into(), false);
}

/// `UpdateAsync`, sin instalar: la versión de pruebas en Rust no se sustituye por la de C#.
fn buscar_version(app: &AppHandle, a_mano: bool) {
    use ds5_actualizador::avisos;
    let estado = app.state::<Arc<App>>();
    // Una compilación de desarrollo vive en `target`: nunca se cambia por una release.
    let Some(programa) = carpeta_del_programa().filter(|c| !es_desarrollo(c)) else {
        if a_mano {
            estado.anotar(avisos::DESARROLLO.into(), false);
        }
        return;
    };
    let agente = ds5_actualizador::red::nuevo_agente();
    let a = match ds5_actualizador::red::buscar(&agente, &version_actual()) {
        Ok(Some(a)) => a,
        Ok(None) => {
            if a_mano {
                estado.anotar("Tienes la última versión.".into(), false);
            }
            return;
        }
        Err(e) => return estado.anotar(avisos::fallo(&e), true),
    };
    if !ds5_actualizador::puede_escribir_en(&programa) {
        return estado.anotar(avisos::sin_permiso(&a.version, &programa), true);
    }
    let nuevos = match ds5_actualizador::red::descargar(&agente, &a, None) {
        Ok(n) => n,
        Err(e) => return estado.anotar(avisos::fallo(&e), true),
    };

    // Cerrarse y abrirse solo parece un cuelgue si no se dice antes.
    estado.anotar(avisos::actualizando_registro(&a.version), false);
    let visible = ventana(app).is_some_and(|v| v.is_visible().unwrap_or(false));
    notificar_siempre(app, &avisos::actualizando_notificacion(&a.version));
    if visible {
        emitir(
            app,
            json!({ "tipo": "actualizando", "version": a.version.texto_corto() }),
        );
        std::thread::sleep(AVISO_ACTUALIZANDO);
    }

    let ajustes = estado.servicio.lock().unwrap().carpeta().ruta.clone();
    ds5_actualizador::guardar_aviso(&ajustes, &a.version);
    if let Err(e) = ds5_actualizador::iniciar_instalacion(
        &nuevos,
        &programa,
        std::process::id(),
        ds5_actualizador::PROGRAMA,
        !visible,
    ) {
        return estado.anotar(avisos::fallo(&e), true);
    }
    estado.servicio.lock().unwrap().parar();
    app.exit(0);
}

/// Los logos de cada paleta, dentro del programa.
fn logo(paleta: &str) -> Option<&'static [u8]> {
    Some(match paleta {
        "fuego" => include_bytes!("../../../../assets/logo/final/ds5control-fuego-128.png"),
        "azul" => include_bytes!("../../../../assets/logo/final/ds5control-azul-128.png"),
        "menta" => include_bytes!("../../../../assets/logo/final/ds5control-menta-128.png"),
        "clasico" => include_bytes!("../../../../assets/logo/final/ds5control-clasico-128.png"),
        _ => return None,
    })
}

/// Los iconos de cada paleta en formato `.ico`, para el acceso directo del menú Inicio.
fn icono_ico(paleta: &str) -> Option<&'static [u8]> {
    Some(match paleta {
        "fuego" => include_bytes!("../../../../assets/logo/final/ds5control-fuego.ico"),
        "azul" => include_bytes!("../../../../assets/logo/final/ds5control-azul.ico"),
        "menta" => include_bytes!("../../../../assets/logo/final/ds5control-menta.ico"),
        "clasico" => include_bytes!("../../../../assets/logo/final/ds5control-clasico.ico"),
        _ => return None,
    })
}

/// Deja el `.ico` de la paleta en `iconos`, junto al programa como en C# (o en la carpeta de
/// ajustes si ahí no se puede escribir), y pone ese icono al acceso directo del menú Inicio.
fn icono_del_acceso(app: &AppHandle, paleta: &str) {
    let Some(bytes) = icono_ico(paleta) else {
        return;
    };
    let Some(programa) = carpeta_del_programa().filter(|c| !es_desarrollo(c)) else {
        return;
    };
    let ajustes = app
        .state::<Arc<App>>()
        .servicio
        .lock()
        .unwrap()
        .carpeta()
        .ruta
        .clone();
    let nombre = format!("ds5control-{paleta}.ico");
    let guardado = [programa.join("iconos"), ajustes.join("iconos")]
        .into_iter()
        .map(|c| c.join(&nombre))
        .find(|r| {
            r.is_file()
                || (r
                    .parent()
                    .is_some_and(|c| std::fs::create_dir_all(c).is_ok())
                    && std::fs::write(r, bytes).is_ok())
        });
    if let Some(ruta) = guardado {
        if let Err(e) = sistema::icono_menu_inicio(&ruta) {
            app.state::<Arc<App>>().anotar(
                format!("No se pudo cambiar el icono del menú Inicio: {e}"),
                true,
            );
        }
    }
}

/// Cambia al momento el icono de la ventana (y con él el de la barra de tareas) y el de la
/// bandeja.
fn poner_icono(app: &AppHandle, paleta: &str) {
    let Some(imagen) = logo(paleta).and_then(|b| tauri::image::Image::from_bytes(b).ok()) else {
        return;
    };
    if let Some(v) = ventana(app) {
        let _ = v.set_icon(imagen.clone());
        #[cfg(windows)]
        if let (Ok(h), Some(ico)) = (v.hwnd(), icono_ico(paleta)) {
            sistema::icono_grande(h.0 as isize, ico);
        }
    }
    if let Some(b) = app.tray_by_id("bandeja") {
        let _ = b.set_icon(Some(imagen));
    }
    // El acceso directo, en otro hilo: tarda un poco y no hace falta esperarlo. Los hilos van de
    // uno en uno y solo escribe la última paleta pedida, por si se cambia varias veces seguidas.
    static ULTIMA: Mutex<String> = Mutex::new(String::new());
    static ESCRIBIENDO: Mutex<()> = Mutex::new(());
    *ULTIMA.lock().unwrap() = paleta.to_string();
    let app = app.clone();
    std::thread::spawn(move || {
        let _turno = ESCRIBIENDO.lock().unwrap();
        let paleta = ULTIMA.lock().unwrap().clone();
        icono_del_acceso(&app, &paleta);
    });
}

/// El menú de la bandeja, en el idioma de la app.
fn menu_bandeja(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let idioma = *app.state::<Arc<App>>().idioma.lock().unwrap();
    let t = |s| textos::traducir(idioma, s);
    let abrir = MenuItem::with_id(app, "abrir", t("Abrir DS5Control"), true, None::<&str>)?;
    let salir = MenuItem::with_id(app, "salir", t("Salir"), true, None::<&str>)?;
    Menu::with_items(app, &[&abrir, &salir])
}

/// `UpdateNoticeMs`: lo que se ve el aviso de la ventana antes de cerrarse para actualizar.
const AVISO_ACTUALIZANDO: std::time::Duration = std::time::Duration::from_millis(2500);

fn version_actual() -> ds5_actualizador::Version {
    ds5_actualizador::Version::analizar(env!("CARGO_PKG_VERSION"))
        .unwrap_or(ds5_actualizador::Version::nueva(0, 0, 0))
}

fn carpeta_del_programa() -> Option<std::path::PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
}

/// La carpeta está dentro de `target`, la de las compilaciones de cargo.
fn es_desarrollo(carpeta: &std::path::Path) -> bool {
    carpeta
        .components()
        .any(|c| c.as_os_str().eq_ignore_ascii_case("target"))
}

/// `ShowUpdateNotification`: sale aunque se hayan quitado las notificaciones, porque hay que
/// saber por qué se cierra o se ha reabierto el programa.
fn notificar_siempre(app: &AppHandle, texto: &str) {
    use tauri_plugin_notification::NotificationExt;
    let texto = &textos::traducir(*app.state::<Arc<App>>().idioma.lock().unwrap(), texto);
    let _ = app
        .notification()
        .builder()
        .title("DS5Control")
        .body(texto)
        .show();
}

/// `SendActions`.
fn enviar_acciones(app: &AppHandle, estado: &App, m: &ds5_servicio::Mando) {
    let todas = estado.servicio.lock().unwrap().acciones();
    let lista = acciones::describir(&todas, &m.vivo().ajustes);
    emitir(app, json!({ "tipo": "acciones", "acciones": lista }));
}

/// `SendAutoProfiles`.
fn enviar_autoperfiles(app: &AppHandle, estado: &App) {
    let lista: Vec<Value> = autopanel::describir(&estado.autoperfiles.lock().unwrap())
        .into_iter()
        .map(|j| json!({ "ruta": j.ruta, "nombre": j.nombre, "perfil": j.perfil }))
        .collect();
    emitir(app, json!({ "tipo": "autoPerfiles", "lista": lista }));
}

/// `PickFile`: el selector de archivos de Windows. Bloquea: se llama desde otro hilo.
fn elegir_archivo(app: &AppHandle, nombre: &str, extensiones: &[&str]) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let idioma = *app.state::<Arc<App>>().idioma.lock().unwrap();
    let mut d = app
        .dialog()
        .file()
        .add_filter(textos::traducir(idioma, nombre), extensiones)
        .add_filter(textos::traducir(idioma, "Todos los archivos"), &["*"]);
    if let Some(v) = ventana(app) {
        d = d.set_parent(&v);
    }
    d.blocking_pick_file()
        .and_then(|r| r.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

/// Lo que avisa el servicio: el botón Mute, las conexiones y el registro.
fn atender_avisos(app: AppHandle, avisos: Receiver<Aviso>) {
    for aviso in avisos {
        let estado = app.state::<Arc<App>>();
        match aviso {
            Aviso::Mute(_) => {
                emitir(&app, json!({ "tipo": "mute" }));
                // `OnMutePressed`: oculta si está delante, si no la trae.
                if estado.preferencias.lock().unwrap().abrir_con_mute {
                    let delante = ventana(&app).is_some_and(|v| {
                        v.is_visible().unwrap_or(false) && v.is_focused().unwrap_or(false)
                    });
                    if delante {
                        if let Some(v) = ventana(&app) {
                            let _ = v.hide();
                        }
                    } else {
                        mostrar(&app);
                    }
                }
            }
            Aviso::Conectado(r) => estado.anotar(format!("Mando {} conectado", r + 1), false),
            Aviso::Desconectado(r) => estado.anotar(format!("Mando {} desconectado", r + 1), false),
            Aviso::Registro(texto, aviso) => estado.anotar(texto, aviso),
            Aviso::Peticion(r, Peticion::Programa { ruta, argumentos }) => {
                let comspec = std::env::var("COMSPEC").ok();
                let l = programa::preparar(&ruta, argumentos.as_deref(), comspec.as_deref());
                if !programa::abrir(&l) {
                    estado.anotar(format!("Mando {}: no se pudo abrir {ruta}", r + 1), true);
                }
            }
            Aviso::Peticion(r, p) => estado.anotar(
                format!("Mando {}: acción sin hacer todavía: {p:?}", r + 1),
                true,
            ),
        }
    }
}

/// `Navigate`: la cruceta, X, O, L1 y R1 del primer mando mueven la interfaz, y el stick derecho
/// la desplaza.
fn navegar(app: &AppHandle, estado: &App) {
    let Some(m) = estado.servicio.lock().unwrap().mandos().into_iter().next() else {
        return;
    };
    let e = m.vivo().crudo;
    let mut pulsados = estado.navegando.lock().unwrap();
    for (accion, b) in [
        ("arriba", boton::ARRIBA),
        ("abajo", boton::ABAJO),
        ("izquierda", boton::IZQUIERDA),
        ("derecha", boton::DERECHA),
        ("elegir", boton::EQUIS),
        ("atras", boton::CIRCULO),
        ("pestanaAnterior", boton::L1),
        ("pestanaSiguiente", boton::R1),
    ] {
        if !e.pulsado(b) {
            pulsados.remove(accion);
        } else if pulsados.insert(accion) {
            emitir(app, json!({ "tipo": "navegacion", "accion": accion }));
        }
    }
    let empuje = (e.ry as f64 - 128.0) / 127.0;
    if empuje.abs() > ZONA_DESPLAZAR {
        emitir(app, json!({ "tipo": "desplazar", "dy": empuje }));
    }
}

/// `SavePlacement`: cuando la ventana lleva un rato quieta, guarda dónde está.
fn guardar_colocacion(app: &AppHandle, estado: &App) {
    let listo = {
        let mut p = estado.colocacion_pendiente.lock().unwrap();
        match *p {
            Some(t) if t.elapsed() >= colocacion::ESPERA => {
                *p = None;
                true
            }
            _ => false,
        }
    };
    let Some(v) = ventana(app).filter(|_| listo) else {
        return;
    };
    if !v.is_visible().unwrap_or(false) || v.is_minimized().unwrap_or(false) {
        return;
    }
    let maximizada = v.is_maximized().unwrap_or(false);
    // Maximizada se guarda el tamaño de antes (`RestoreBounds`): se deja el que había.
    let anterior = std::fs::read_to_string(
        estado
            .servicio
            .lock()
            .unwrap()
            .carpeta()
            .ruta
            .join(colocacion::ARCHIVO),
    )
    .ok()
    .and_then(|t| colocacion::Colocacion::leer(&t));
    let c = if maximizada {
        match anterior {
            Some(a) => colocacion::Colocacion {
                maximizada: true,
                ..a
            },
            None => return,
        }
    } else {
        let escala = v.scale_factor().unwrap_or(1.0);
        let (Ok(pos), Ok(tam)) = (v.outer_position(), v.outer_size()) else {
            return;
        };
        let pos = pos.to_logical::<f64>(escala);
        let tam = tam.to_logical::<f64>(escala);
        colocacion::Colocacion {
            x: pos.x.round(),
            y: pos.y.round(),
            ancho: tam.width.round(),
            alto: tam.height.round(),
            maximizada: false,
        }
    };
    let r = estado
        .servicio
        .lock()
        .unwrap()
        .guardado()
        .escribir(colocacion::ARCHIVO, &c.texto());
    if let Err(e) = r {
        estado.anotar(
            format!("No se pudo guardar {}: {e}", colocacion::ARCHIVO),
            true,
        );
    }
}

/// `RestorePlacement`.
fn restaurar_colocacion(v: &WebviewWindow, carpeta: &std::path::Path) {
    let Some(c) = std::fs::read_to_string(carpeta.join(colocacion::ARCHIVO))
        .ok()
        .and_then(|t| colocacion::Colocacion::leer(&t))
    else {
        return;
    };
    // La pantalla virtual: la unión de todos los monitores, en píxeles lógicos.
    let monitores = v.available_monitors().unwrap_or_default();
    let mut pantalla: Option<(f64, f64, f64, f64)> = None;
    for m in &monitores {
        let e = m.scale_factor();
        let p = m.position().to_logical::<f64>(e);
        let t = m.size().to_logical::<f64>(e);
        pantalla = Some(match pantalla {
            None => (p.x, p.y, p.x + t.width, p.y + t.height),
            Some((a, b, c, d)) => (
                a.min(p.x),
                b.min(p.y),
                c.max(p.x + t.width),
                d.max(p.y + t.height),
            ),
        });
    }
    let Some((x0, y0, x1, y1)) = pantalla else {
        return;
    };
    if !c.se_ve_en((x0, y0, x1 - x0, y1 - y0)) {
        return;
    }
    let _ = v.set_size(tauri::LogicalSize::new(c.ancho, c.alto));
    let _ = v.set_position(tauri::LogicalPosition::new(c.x, c.y));
    if c.maximizada {
        let _ = v.maximize();
    }
}

/// Manda el estado a la interfaz cuando cambia, más a menudo con la ventana delante.
fn sondear(app: AppHandle) {
    loop {
        let estado = app.state::<Arc<App>>();
        guardar_colocacion(&app, &estado);
        let (visible, activa) = ventana(&app).map_or((false, false), |v| {
            (
                v.is_visible().unwrap_or(false),
                v.is_focused().unwrap_or(false),
            )
        });
        if visible && *estado.lista.lock().unwrap() {
            let texto = estado.estado(visible).to_string();
            let mut ultimo = estado.ultimo.lock().unwrap();
            if *ultimo != texto {
                if let Ok(v) = serde_json::from_str::<Value>(&texto) {
                    let _ = app.emit(EVENTO, v);
                }
                *ultimo = texto;
            }
            drop(ultimo);
            if activa && estado.preferencias.lock().unwrap().control_con_mando {
                navegar(&app, &estado);
            }
        }
        std::thread::sleep(match (visible, activa) {
            (true, true) => SONDEO_ACTIVO,
            (true, false) => SONDEO_FONDO,
            _ => SONDEO_OCULTA,
        });
    }
}

/// Los autoperfiles: cada segundo mira la ventana de delante y carga o quita perfiles temporales.
#[cfg(windows)]
fn vigilar_ventanas(app: AppHandle) {
    let mut escritorio = ds5_autoperfiles::EscritorioWindows;
    loop {
        std::thread::sleep(ds5_autoperfiles::INTERVALO_SONDEO);
        let estado = app.state::<Arc<App>>();
        if !estado.vigilante.lock().unwrap().vigilando() {
            continue;
        }
        let servicio = estado.servicio.lock().unwrap();
        let ajustes = servicio.carpeta().ajustes();
        let conectados = servicio.mandos();
        drop(servicio);
        let mandos: Vec<EstadoMando> = (0..ds5_servicio::MAX_MANDOS)
            .map(|r| match conectados.iter().find(|m| m.ranura == r) {
                Some(m) => {
                    let v = m.vivo();
                    EstadoMando {
                        perfil_temporal: (v.perfil != v.normal).then(|| v.perfil.clone()),
                        perfil_defecto: v.normal,
                    }
                }
                None => EstadoMando {
                    perfil_defecto: ajustes.perfiles[r].clone(),
                    perfil_temporal: None,
                },
            })
            .collect();
        let app_estado = AutoEstado {
            servicio_activo: !*estado.apagado.lock().unwrap(),
            volver_al_defecto: ajustes.generales.autoperfil_vuelve_a_defecto,
            mandos: &mandos,
        };
        let resultado = estado
            .vigilante
            .lock()
            .unwrap()
            .sondear(&mut escritorio, &app_estado);
        for p in resultado.peticiones {
            let (mando, orden) = match p {
                AutoPeticion::CargarTemporal { mando, perfil } => {
                    (mando, Orden::Temporal(Some(perfil)))
                }
                AutoPeticion::VolverAlDefecto { mando } => (mando, Orden::Temporal(None)),
                // Una regla "apagar": el juego tiene el mando para él solo mientras está delante.
                AutoPeticion::ApagarServicio => {
                    estado.servicio.lock().unwrap().parar();
                    *estado.apagado.lock().unwrap() = true;
                    estado.anotar("Motor parado por un autoperfil".into(), false);
                    continue;
                }
                AutoPeticion::EncenderServicio => {
                    *estado.apagado.lock().unwrap() = false;
                    reiniciar(&app, &estado);
                    continue;
                }
            };
            if let Some(m) = conectados.iter().find(|m| m.ranura == mando) {
                m.ordenar(orden);
            }
        }
    }
}

#[cfg(not(windows))]
fn vigilar_ventanas(_app: AppHandle) {}

fn carpeta_ajustes() -> std::path::PathBuf {
    let programa = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();
    let appdata = ds5_perfiles::carpeta::carpeta_appdata().unwrap_or_else(|| programa.clone());
    ds5_perfiles::carpeta::buscar(&programa, &appdata).ruta
}

fn main() {
    let minimizada = std::env::args().any(|a| a == "-m");
    // Antes de crear ninguna ventana, para que la barra de tareas use el icono de la ventana.
    sistema::identidad("com.anxo0.ds5control");
    sistema::reparar_inicio_windows();
    let carpeta = carpeta_ajustes();
    let sin_mandos = std::env::args().any(|a| a == "--sin-mandos");
    let autoperfiles = std::fs::read_to_string(carpeta.join("Auto Profiles.xml"))
        .ok()
        .and_then(|t| Autoperfiles::leer(&t).ok())
        .unwrap_or_default();
    let (servicio, avisos) = Servicio::arrancar(&carpeta, !sin_mandos);
    let idioma = ds5_servicio::Idioma::de_ajustes(&servicio.carpeta().ajustes());
    let app_estado = Arc::new(App {
        servicio: Mutex::new(servicio),
        niveles: Mutex::new(HashMap::new()),
        preferencias: Mutex::new(Preferencias::leer(&carpeta)),
        registro: Mutex::new(Vec::new()),
        lista: Mutex::new(false),
        ultimo: Mutex::new(String::new()),
        navegando: Mutex::new(HashSet::new()),
        app: std::sync::OnceLock::new(),
        telemetria: Mutex::new(false),
        idioma: Mutex::new(idioma),
        archivo: archivo_registro::ArchivoRegistro::abrir(&carpeta),
        apagado: Mutex::new(false),
        colocacion_pendiente: Mutex::new(None),
        instalando_vigem: Mutex::new(false),
        instalando_hidhide: Mutex::new(false),
        vigilante: Mutex::new(Vigilante::new(&autoperfiles)),
        autoperfiles: Mutex::new(autoperfiles),
    });
    app_estado.anotar(format!("Ajustes en {}", carpeta.display()), false);

    tauri::Builder::default()
        // La instancia única: abrir el programa otra vez solo trae la ventana que ya hay.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            mostrar(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri::plugin::Builder::<tauri::Wry>::new("navegador")
                .js_init_script(navegador::script(&carpeta))
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .manage(app_estado)
        .invoke_handler(tauri::generate_handler![mensaje])
        .setup(move |app| {
            let menu = menu_bandeja(app.handle())?;
            let mut bandeja = TrayIconBuilder::with_id("bandeja")
                .tooltip("DS5Control")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, evento| match evento.id().as_ref() {
                    "abrir" => mostrar(app),
                    "salir" => {
                        app.state::<Arc<App>>().servicio.lock().unwrap().parar();
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|bandeja, evento| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = evento
                    {
                        mostrar(bandeja.app_handle());
                    }
                });
            if let Some(icono) = app.default_window_icon() {
                bandeja = bandeja.icon(icono.clone());
            }
            bandeja.build(app)?;

            let _ = app.state::<Arc<App>>().app.set(app.handle().clone());
            if let Some(p) = navegador::paleta_guardada(&carpeta) {
                poner_icono(app.handle(), &p);
            }
            if !minimizada {
                mostrar(app.handle());
            }
            let h = app.handle().clone();
            std::thread::spawn(move || atender_avisos(h, avisos));
            let h = app.handle().clone();
            std::thread::spawn(move || sondear(h));
            let h = app.handle().clone();
            std::thread::spawn(move || vigilar_ventanas(h));
            let h = app.handle().clone();
            // `TakeNotice`: si se acaba de actualizar, se dice.
            let carpeta_ajustes = app
                .state::<Arc<App>>()
                .servicio
                .lock()
                .unwrap()
                .carpeta()
                .ruta
                .clone();
            if let Some(v) = ds5_actualizador::tomar_aviso(&carpeta_ajustes, &version_actual()) {
                let texto = ds5_actualizador::avisos::actualizado(&v);
                app.state::<Arc<App>>().anotar(texto.clone(), false);
                notificar_siempre(app.handle(), &texto);
            }
            std::thread::spawn(move || {
                // Lo que dejó la versión en C# si esta se instaló encima.
                if let Some(c) = carpeta_del_programa().filter(|c| !es_desarrollo(c)) {
                    let n = limpieza::limpiar(&c);
                    if n > 0 {
                        h.state::<Arc<App>>().anotar(
                            format!("Borrados {n} archivos que sobraban de la versión en C#"),
                            false,
                        );
                    }
                }
                std::thread::sleep(ds5_actualizador::avisos::ESPERA_ANTES_DE_BUSCAR);
                let buscar = h
                    .state::<Arc<App>>()
                    .servicio
                    .lock()
                    .unwrap()
                    .carpeta()
                    .ajustes()
                    .comprobacion
                    .comprobar_cada
                    > 0;
                if buscar {
                    buscar_version(&h, false);
                }
            });
            Ok(())
        })
        .on_window_event(|ventana, evento| match evento {
            // La X de Windows (Alt+F4) también va a la bandeja, como la de la interfaz.
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = ventana.hide();
            }
            tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                // Minimizar desde Windows también la manda a la bandeja, como en C#.
                if ventana.is_minimized().unwrap_or(false) {
                    let _ = ventana.unminimize();
                    let _ = ventana.hide();
                    return;
                }
                let estado = ventana.state::<Arc<App>>();
                *estado.colocacion_pendiente.lock().unwrap() = Some(std::time::Instant::now());
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("no se pudo abrir la ventana de DS5Control");
}

#[cfg(test)]
mod pruebas_iconos {
    #[test]
    fn los_logos_de_las_cuatro_paletas_se_leen() {
        for p in ["fuego", "azul", "menta", "clasico"] {
            let b = super::logo(p).unwrap();
            let i = tauri::image::Image::from_bytes(b).unwrap();
            assert_eq!((i.width(), i.height()), (128, 128), "{p}");
        }
        assert!(super::logo("otra").is_none());
    }
}
