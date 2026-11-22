//! El menú de clic derecho de la bandeja, igual que el de C# (`TrayIconViewModel.cs` con
//! `MenuBandeja.xaml`): "Perfil del mando N" con sus perfiles, "Desconectar" con los mandos por
//! Bluetooth, "Abrir DS5Control" en negrita, "Detener"/"Iniciar" y "Salir", con los submenús
//! desplegados al lado al pasar el ratón.
//!
//! El menú nativo de Windows no se puede dibujar así, de modo que es una ventanita sin marco
//! (`ui/public/menu-bandeja.html`). Se crea la primera vez y después solo se oculta y se vuelve
//! a enseñar: cerrarla y crearla otra vez con la misma etiqueta fallaba y no se reabría.

use std::sync::{Arc, Mutex};

use ds5_informes::Conexion;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use crate::{mostrar, navegador, reiniciar, sistema, textos, App};

pub const ETIQUETA: &str = "bandeja";

/// Dónde se hizo el clic, en píxeles de pantalla: el menú se abre encima de ese punto.
static PUNTERO: Mutex<(f64, f64)> = Mutex::new((0.0, 0.0));
static CREANDO: Mutex<()> = Mutex::new(());

/// Abre el menú junto al puntero, con los mandos y perfiles de ahora.
pub fn abrir(app: &AppHandle, x: f64, y: f64) {
    *PUNTERO.lock().unwrap() = (x, y);
    let datos = datos(app).to_string();
    if let Some(v) = app.get_webview_window(ETIQUETA) {
        let _ = v.eval(format!("window.pintar({datos})"));
        return;
    }
    let app = app.clone();
    // En otro hilo, como la ventana principal: crearla desde el hilo de la bandeja puede
    // bloquear la app en Windows.
    std::thread::spawn(move || {
        let _creando = CREANDO.lock().unwrap();
        if let Some(v) = app.get_webview_window(ETIQUETA) {
            let _ = v.eval(format!("window.pintar({datos})"));
            return;
        }
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
                .inner_size(220.0, 160.0)
                .initialization_script(format!("window.__MENU__ = {datos};"))
                .build()
        else {
            return;
        };
        let a = app.clone();
        v.on_window_event(move |e| {
            if let tauri::WindowEvent::Focused(false) = e {
                ocultar(&a);
            }
        });
    });
}

/// Los textos, el tema, los mandos y los perfiles, como `PopulateContextMenu`.
fn datos(app: &AppHandle) -> Value {
    let estado = app.state::<Arc<App>>();
    let idioma = *estado.idioma.lock().unwrap();
    let t = |s: &str| textos::traducir(idioma, s);
    let servicio = estado.servicio.lock().unwrap();
    let mandos: Vec<Value> = servicio
        .mandos()
        .iter()
        .map(|m| {
            let v = m.vivo();
            json!({
                "ranura": m.ranura,
                "perfil": v.perfil,
                // `Synced && !Charging`: los que se pueden desconectar por Bluetooth.
                "desconectable": m.info.conexion == Conexion::Bt && !v.cargando,
            })
        })
        .collect();
    json!({
        "oscuro": navegador::tema_guardado(&servicio.carpeta().ruta).as_deref() == Some("oscuro"),
        "enMarcha": servicio.en_marcha(),
        "textos": {
            "abrir": t("Abrir DS5Control"),
            "perfil": t("Perfil del mando {}"),
            "desconectar": t("Desconectar"),
            "mando": t("Mando {}"),
            "detener": t("Detener"),
            "iniciar": t("Iniciar"),
            "salir": t("Salir"),
        },
        "mandos": mandos,
        "perfiles": servicio.carpeta().perfiles(),
    })
}

pub fn ocultar(app: &AppHandle) {
    if let Some(v) = app.get_webview_window(ETIQUETA) {
        let _ = v.hide();
        let _ = v.eval("window.cerrarSubmenu && window.cerrarSubmenu()");
    }
}

/// Lo que manda la página del menú.
pub fn mensaje(app: &AppHandle, m: &Value) {
    let estado = app.state::<Arc<App>>();
    let ranura = m["ranura"].as_u64();
    let mando = || {
        estado
            .servicio
            .lock()
            .unwrap()
            .mandos()
            .into_iter()
            .find(|x| Some(x.ranura as u64) == ranura)
    };
    match m["accion"].as_str().unwrap_or("") {
        "tamano" => colocar(app, m),
        "abrir" => {
            ocultar(app);
            mostrar(app);
        }
        "perfil" => {
            ocultar(app);
            if let Some(x) = mando() {
                let nombre = m["nombre"].as_str().unwrap_or("");
                if let Err(e) = estado.servicio.lock().unwrap().elegir_perfil(&x, nombre) {
                    estado.anotar(format!("No se pudo guardar: {e}"), true);
                }
            }
        }
        "desconectar" => {
            ocultar(app);
            if let Some(mac) = mando().map(|x| x.vivo().mac) {
                if mac.iter().any(|b| *b != 0) {
                    sistema::desconectar_bt(mac);
                }
            }
        }
        // `ChangeControlServiceItem_Click`: para el motor o lo vuelve a arrancar.
        "servicio" => {
            ocultar(app);
            let en_marcha = estado.servicio.lock().unwrap().en_marcha();
            if en_marcha {
                estado.servicio.lock().unwrap().parar();
            } else {
                reiniciar(app, &estado);
            }
        }
        "salir" => {
            ocultar(app);
            estado.servicio.lock().unwrap().parar();
            app.exit(0);
        }
        "ocultar" => ocultar(app),
        _ => {}
    }
}

/// La página ya sabe lo que ocupan el menú y su submenú más grande: el menú se pone encima del
/// puntero, y los submenús a su derecha o, si no caben, a su izquierda, como un menú de Windows.
fn colocar(app: &AppHandle, m: &Value) {
    let Some(v) = app.get_webview_window(ETIQUETA) else {
        return;
    };
    let escala = v.scale_factor().unwrap_or(1.0);
    let px = |clave: &str| (m[clave].as_f64().unwrap_or(0.0) * escala).ceil();
    let (mw, mh, sw, sh) = (
        px("anchoMenu"),
        px("altoMenu"),
        px("anchoSub"),
        px("altoSub"),
    );
    let hueco = if sw > 0.0 {
        (8.0 * escala).round()
    } else {
        0.0
    };
    let (x, y) = *PUNTERO.lock().unwrap();
    let (mut zx, mut zy, mut zw, mut zh) = (
        f64::MIN / 4.0,
        f64::MIN / 4.0,
        f64::MAX / 2.0,
        f64::MAX / 2.0,
    );
    if let Ok(Some(pantalla)) = app.monitor_from_point(x, y) {
        let zona = pantalla.work_area();
        (zx, zy) = (zona.position.x as f64, zona.position.y as f64);
        (zw, zh) = (zona.size.width as f64, zona.size.height as f64);
    }
    // El menú principal: su esquina de abajo a la derecha en el puntero, dentro de la pantalla.
    let mx = (x - mw).clamp(zx, (zx + zw - mw).max(zx));
    let mut my = y - mh;
    if my < zy {
        my = y;
    }
    let my = my.clamp(zy, (zy + zh - mh).max(zy));
    let derecha = mx + mw + hueco + sw <= zx + zw;
    let ancho = mw + hueco + sw;
    let alto = mh.max(sh);
    let wx = if derecha { mx } else { mx - hueco - sw };
    let wy = (my + mh - alto).max(zy);
    let _ = v.set_size(PhysicalSize::new(ancho as u32, alto as u32));
    let _ = v.set_position(PhysicalPosition::new(wx as i32, wy as i32));
    let _ = v.eval(format!(
        "window.colocar({})",
        json!({
            "lado": if derecha { "derecha" } else { "izquierda" },
            "izquierda": (mx - wx) / escala,
            "arriba": (my - wy) / escala,
        })
    ));
    let _ = v.show();
    let _ = v.set_focus();
}
