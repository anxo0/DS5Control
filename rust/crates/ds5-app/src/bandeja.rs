//! El menú de clic derecho de la bandeja con el aspecto de la app, como el de C#
//! (`TrayIconViewModel.cs` con `MenuBandeja.xaml`): fondo del tema, bordes redondeados, "Abrir
//! DS5Control", el perfil de cada mando, "Desconectar" para los mandos por Bluetooth y "Salir".
//!
//! El menú nativo de Windows no se puede dibujar así, de modo que es una ventanita sin marco
//! (`ui/public/menu-bandeja.html`) que se abre junto al puntero y se cierra al perder el foco.

use std::sync::{Arc, Mutex};

use ds5_informes::Conexion;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use crate::{mostrar, navegador, sistema, textos, App};

pub const ETIQUETA: &str = "bandeja";

/// Dónde se hizo el clic, en píxeles de pantalla: el menú se coloca junto a ese punto.
static PUNTERO: Mutex<(f64, f64)> = Mutex::new((0.0, 0.0));

/// Abre el menú junto al puntero, con los mandos y perfiles de ahora.
pub fn abrir(app: &AppHandle, x: f64, y: f64) {
    cerrar(app);
    *PUNTERO.lock().unwrap() = (x, y);
    let estado = app.state::<Arc<App>>();
    let idioma = *estado.idioma.lock().unwrap();
    let t = |s: &str| textos::traducir(idioma, s);
    let (carpeta, perfiles, mandos) = {
        let servicio = estado.servicio.lock().unwrap();
        let mandos: Vec<Value> = servicio
            .mandos()
            .iter()
            .map(|m| {
                json!({
                    "ranura": m.ranura,
                    "perfil": m.vivo().perfil,
                    "bluetooth": m.info.conexion == Conexion::Bt,
                })
            })
            .collect();
        (
            servicio.carpeta().ruta.clone(),
            servicio.carpeta().perfiles(),
            mandos,
        )
    };
    let datos = json!({
        "oscuro": navegador::tema_guardado(&carpeta).as_deref() == Some("oscuro"),
        "textos": {
            "abrir": t("Abrir DS5Control"),
            "perfil": t("Perfil del mando {}"),
            "desconectar": t("Desconectar"),
            "mando": t("Mando {}"),
            "atras": t("Atrás"),
            "salir": t("Salir"),
        },
        "mandos": mandos,
        "perfiles": perfiles,
    });
    let script = format!("window.__MENU__ = {datos};");
    let app = app.clone();
    // En otro hilo, como la ventana principal: crearla desde el hilo de la bandeja puede
    // bloquear la app en Windows.
    std::thread::spawn(move || {
        let Ok(v) =
            WebviewWindowBuilder::new(&app, ETIQUETA, WebviewUrl::App("menu-bandeja.html".into()))
                .title("DS5Control")
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .resizable(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .inner_size(220.0, 120.0)
                .initialization_script(&script)
                .build()
        else {
            return;
        };
        let a = app.clone();
        v.on_window_event(move |e| {
            if let tauri::WindowEvent::Focused(false) = e {
                cerrar(&a);
            }
        });
    });
}

pub fn cerrar(app: &AppHandle) {
    if let Some(v) = app.get_webview_window(ETIQUETA) {
        let _ = v.close();
    }
}

/// Lo que manda la página del menú.
pub fn mensaje(app: &AppHandle, m: &Value) {
    let estado = app.state::<Arc<App>>();
    match m["accion"].as_str().unwrap_or("") {
        // La página ya sabe cuánto ocupa: se coloca encima del puntero, dentro de la pantalla.
        "tamano" => {
            let Some(v) = app.get_webview_window(ETIQUETA) else {
                return;
            };
            let escala = v.scale_factor().unwrap_or(1.0);
            let ancho = (m["ancho"].as_f64().unwrap_or(220.0) * escala).ceil();
            let alto = (m["alto"].as_f64().unwrap_or(120.0) * escala).ceil();
            let (x, y) = *PUNTERO.lock().unwrap();
            let (mut px, mut py) = (x - ancho, y - alto);
            if let Ok(Some(pantalla)) = app.monitor_from_point(x, y) {
                let zona = pantalla.work_area();
                let (zx, zy) = (zona.position.x as f64, zona.position.y as f64);
                let (zw, zh) = (zona.size.width as f64, zona.size.height as f64);
                px = px.clamp(zx, (zx + zw - ancho).max(zx));
                if py < zy {
                    py = y;
                }
                py = py.clamp(zy, (zy + zh - alto).max(zy));
            }
            let _ = v.set_size(PhysicalSize::new(ancho as u32, alto as u32));
            let _ = v.set_position(PhysicalPosition::new(px as i32, py as i32));
            let _ = v.show();
            let _ = v.set_focus();
        }
        "abrir" => {
            cerrar(app);
            mostrar(app);
        }
        "perfil" => {
            cerrar(app);
            let nombre = m["nombre"].as_str().unwrap_or("");
            let servicio = estado.servicio.lock().unwrap();
            if let Some(mando) = servicio
                .mandos()
                .into_iter()
                .find(|x| Some(x.ranura as u64) == m["ranura"].as_u64())
            {
                if let Err(e) = servicio.elegir_perfil(&mando, nombre) {
                    estado.anotar(format!("No se pudo guardar: {e}"), true);
                }
            }
        }
        "desconectar" => {
            cerrar(app);
            let mac = estado
                .servicio
                .lock()
                .unwrap()
                .mandos()
                .into_iter()
                .find(|x| Some(x.ranura as u64) == m["ranura"].as_u64())
                .map(|x| x.vivo().mac);
            if let Some(mac) = mac.filter(|mac| mac.iter().any(|b| *b != 0)) {
                sistema::desconectar_bt(mac);
            }
        }
        "salir" => {
            cerrar(app);
            estado.servicio.lock().unwrap().parar();
            app.exit(0);
        }
        _ => {}
    }
}
