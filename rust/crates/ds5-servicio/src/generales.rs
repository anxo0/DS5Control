//! Las opciones generales de la app, como `GeneralOptions.cs`: lo que se pinta (`Describe`) y qué
//! hay que hacer al cambiar cada una (`Apply`). Lo que se guarda en `Profiles.xml` se cambia aquí;
//! lo que toca el sistema (arrancar con Windows, la prioridad, los botones) lo hace la app.

use serde_json::{json, Value};

use ds5_perfiles::AjustesApp;

use crate::idioma::{Idioma, IDIOMAS};

/// Las secciones, en el orden de C#.
const SECCIONES: [&str; 5] = [
    "Inicio",
    "Avisos",
    "Rendimiento",
    "Otros programas",
    "Herramientas",
];

pub const PRIORIDADES: [&str; 3] = ["Normal", "Por encima de lo normal", "Alta"];

/// Lo que no está en `Profiles.xml` y la app lee del sistema.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sistema {
    /// Existe el acceso directo en la carpeta Inicio.
    pub inicio_windows: bool,
    /// La prioridad del proceso, como índice de `PRIORIDADES`.
    pub prioridad: usize,
    /// ViGEmBus y HidHide están instalados: el botón ofrece desinstalarlos.
    pub vigem: bool,
    pub hidhide: bool,
}

/// Lo que tiene que hacer la app tras `aplicar`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cambio {
    Nada,
    /// Ha cambiado `Profiles.xml`: hay que guardarlo.
    Guardar,
    /// `exclusivo`: se guarda y se reinicia el servicio.
    GuardarYReiniciar,
    InicioWindows(bool),
    Prioridad(usize),
    /// Un botón: `reiniciar`, `carpeta`, `buscarVersion`, `vigem` o `hidhide`.
    Boton(&'static str),
    /// Ha cambiado el idioma (`None` es automático): se guarda y los textos pasan a ese idioma.
    Idioma(Option<Idioma>),
}

/// Los ajustes se escriben en español y se traducen al describirlos.
fn interruptor(id: &str, nombre: &str, valor: bool, ayuda: Option<&str>) -> Value {
    ajuste(id, nombre, "interruptor", Value::Null, json!(valor), ayuda)
}

fn ajuste(
    id: &str,
    nombre: &str,
    tipo: &str,
    opciones: Value,
    valor: Value,
    ayuda: Option<&str>,
) -> Value {
    json!({
        "id": id, "nombre": nombre, "tipo": tipo, "min": 0, "max": 0, "paso": 0,
        "porcentaje": false, "opciones": opciones, "valor": valor, "ayuda": ayuda,
    })
}

fn boton(id: &str, nombre: &str) -> Value {
    ajuste(id, nombre, "accion", Value::Null, Value::Null, None)
}

/// El índice del idioma guardado en la lista del ajuste: 0 es automático.
fn indice_idioma(guardado: &str) -> usize {
    Idioma::de_codigo(guardado)
        .and_then(|i| IDIOMAS.iter().position(|x| *x == i))
        .map_or(0, |p| p + 1)
}

/// Traduce en el sitio los nombres, las ayudas y las opciones de un ajuste.
fn traducir(ajuste: &mut Value, idioma: Idioma) {
    for campo in ["nombre", "ayuda"] {
        if let Some(t) = ajuste[campo].as_str() {
            ajuste[campo] = json!(idioma.texto(t));
        }
    }
    // La lista de idiomas no se traduce salvo "Automático": cada uno va en su propio idioma.
    if let Some(l) = ajuste["opciones"].as_array_mut() {
        for o in l {
            if let Some(t) = o.as_str() {
                *o = json!(idioma.texto(t));
            }
        }
    }
}

/// `Describe`, con los textos en `idioma`.
pub fn describir(a: &AjustesApp, s: Sistema, idioma: Idioma) -> Value {
    let g = &a.generales;
    let mut idiomas = vec!["Automático"];
    idiomas.extend(IDIOMAS.iter().map(|i| i.nombre()));
    let secciones = [
        vec![
            interruptor(
                "inicioWindows",
                "Abrir al iniciar Windows",
                s.inicio_windows,
                None,
            ),
            interruptor(
                "inicioOculto",
                "Abrir en segundo plano",
                a.ventana.empezar_minimizada,
                None,
            ),
            interruptor(
                "actualizaciones",
                "Actualizar solo al abrir",
                a.comprobacion.comprobar_cada > 0,
                None,
            ),
            ajuste(
                "idioma",
                "Idioma",
                "opciones",
                json!(idiomas),
                json!(indice_idioma(&g.idioma)),
                None,
            ),
        ],
        vec![ajuste(
            "notificaciones",
            "Notificaciones",
            "opciones",
            json!(["Ninguna", "Solo problemas", "Todas"]),
            json!(g.avisos),
            None,
        )],
        vec![ajuste(
            "prioridad",
            "Prioridad del programa",
            "opciones",
            json!(PRIORIDADES),
            json!(s.prioridad),
            None,
        )],
        vec![
            interruptor(
                "exclusivo",
                "Modo exclusivo (solo con cable)",
                a.ventana.modo_exclusivo,
                Some("DS5Control se queda el mando para él solo. Con HidHide no hace falta."),
            ),
            interruptor(
                "gestos",
                "Cambiar de perfil deslizando en el panel",
                g.deslizar_perfiles,
                Some("Desliza el dedo en el panel táctil para pasar al perfil anterior o al siguiente."),
            ),
            interruptor(
                "osc",
                "Servidor OSC",
                g.servidor_osc,
                Some("Deja que otros programas de tu red controlen DS5Control, como TouchOSC o apps de realidad virtual."),
            ),
            interruptor(
                "udp",
                "Servidor de movimiento (emuladores)",
                g.servidor_udp,
                Some("Envía el giroscopio a emuladores como Cemu, Ryujinx o Dolphin para los controles por movimiento."),
            ),
        ],
        vec![
            boton("reiniciar", "Reiniciar el motor"),
            boton("carpeta", "Abrir carpeta de perfiles"),
            boton("buscarVersion", "Buscar versión nueva"),
            boton(
                "vigem",
                if s.vigem {
                    "Desinstalar ViGEmBus"
                } else {
                    "Instalar ViGEmBus"
                },
            ),
            boton(
                "hidhide",
                if s.hidhide {
                    "Desinstalar HidHide"
                } else {
                    "Instalar HidHide"
                },
            ),
        ],
    ];
    Value::Array(
        SECCIONES
            .iter()
            .zip(secciones)
            .map(|(nombre, mut ajustes)| {
                for a in &mut ajustes {
                    traducir(a, idioma);
                }
                // `id` es el nombre en español de siempre, para que la interfaz las reconozca.
                json!({ "id": nombre, "nombre": idioma.texto(nombre), "ajustes": ajustes })
            })
            .collect(),
    )
}

/// `Apply`.
pub fn aplicar(a: &mut AjustesApp, id: &str, valor: &Value) -> Cambio {
    let si = || valor.as_bool().unwrap_or(false);
    let numero = || valor.as_i64().unwrap_or(0);
    match id {
        "inicioWindows" => Cambio::InicioWindows(si()),
        "inicioOculto" => {
            a.ventana.empezar_minimizada = si();
            Cambio::Guardar
        }
        "actualizaciones" => {
            a.comprobacion.comprobar_cada = if si() { 24 } else { 0 };
            Cambio::Guardar
        }
        "idioma" => match usize::try_from(numero()) {
            Ok(0) => {
                a.generales.idioma = String::new();
                Cambio::Idioma(None)
            }
            Ok(i) if i <= IDIOMAS.len() => {
                let elegido = IDIOMAS[i - 1];
                a.generales.idioma = elegido.codigo().to_string();
                Cambio::Idioma(Some(elegido))
            }
            _ => Cambio::Nada,
        },
        "notificaciones" => {
            a.generales.avisos = numero() as i32;
            Cambio::Guardar
        }
        "prioridad" => match usize::try_from(numero()) {
            Ok(i) if i < PRIORIDADES.len() => Cambio::Prioridad(i),
            _ => Cambio::Nada,
        },
        "exclusivo" => {
            a.ventana.modo_exclusivo = si();
            Cambio::GuardarYReiniciar
        }
        "gestos" => {
            a.generales.deslizar_perfiles = si();
            Cambio::Guardar
        }
        "osc" => {
            a.generales.servidor_osc = si();
            Cambio::Guardar
        }
        "udp" => {
            a.generales.servidor_udp = si();
            Cambio::Guardar
        }
        "reiniciar" => Cambio::Boton("reiniciar"),
        "carpeta" => Cambio::Boton("carpeta"),
        "buscarVersion" => Cambio::Boton("buscarVersion"),
        "vigem" => Cambio::Boton("vigem"),
        "hidhide" => Cambio::Boton("hidhide"),
        _ => Cambio::Nada,
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn describe_las_cinco_secciones() {
        let d = describir(&AjustesApp::default(), Sistema::default(), Idioma::Es);
        let d = d.as_array().unwrap();
        assert_eq!(d.len(), 5);
        assert_eq!(d[0]["nombre"], "Inicio");
        assert_eq!(d[0]["ajustes"][2]["valor"], true); // comprobar cada 24 h
        assert_eq!(d[1]["ajustes"][0]["valor"], 2); // todas las notificaciones
        assert_eq!(d[3]["ajustes"][1]["valor"], true); // deslizar perfiles
        assert_eq!(d[4]["ajustes"][0]["tipo"], "accion");
        assert_eq!(d[4]["ajustes"][0]["valor"], Value::Null);
        assert_eq!(d[4]["ajustes"][3]["nombre"], "Instalar ViGEmBus");
        let instalado = Sistema {
            vigem: true,
            hidhide: true,
            ..Sistema::default()
        };
        let d = describir(&AjustesApp::default(), instalado, Idioma::Es);
        assert_eq!(d[4]["ajustes"][3]["nombre"], "Desinstalar ViGEmBus");
        assert_eq!(d[4]["ajustes"][4]["nombre"], "Desinstalar HidHide");
    }

    #[test]
    fn aplicar_cambia_profiles_xml() {
        let mut a = AjustesApp::default();
        assert_eq!(
            aplicar(&mut a, "actualizaciones", &json!(false)),
            Cambio::Guardar
        );
        assert_eq!(a.comprobacion.comprobar_cada, 0);
        assert_eq!(
            aplicar(&mut a, "notificaciones", &json!(1)),
            Cambio::Guardar
        );
        assert_eq!(a.generales.avisos, 1);
        assert_eq!(
            aplicar(&mut a, "exclusivo", &json!(true)),
            Cambio::GuardarYReiniciar
        );
        assert!(a.ventana.modo_exclusivo);
        assert_eq!(
            aplicar(&mut a, "prioridad", &json!(2)),
            Cambio::Prioridad(2)
        );
        assert_eq!(aplicar(&mut a, "prioridad", &json!(7)), Cambio::Nada);
        assert_eq!(
            aplicar(&mut a, "carpeta", &Value::Null),
            Cambio::Boton("carpeta")
        );
    }

    #[test]
    fn el_idioma_se_guarda_en_use_lang() {
        let mut a = AjustesApp::default();
        let idioma =
            |a: &AjustesApp| describir(a, Sistema::default(), Idioma::Es)[0]["ajustes"][3].clone();
        let d = idioma(&a);
        assert_eq!(d["nombre"], "Idioma");
        assert_eq!(
            d["opciones"],
            json!(["Automático", "Español", "English", "Português"])
        );
        assert_eq!(d["valor"], 0);
        assert_eq!(
            aplicar(&mut a, "idioma", &json!(3)),
            Cambio::Idioma(Some(Idioma::Pt))
        );
        assert_eq!(a.generales.idioma, "pt");
        assert_eq!(idioma(&a)["valor"], 3);
        assert_eq!(
            aplicar(&mut a, "idioma", &json!(2)),
            Cambio::Idioma(Some(Idioma::En))
        );
        assert_eq!(a.generales.idioma, "en");
        assert_eq!(aplicar(&mut a, "idioma", &json!(0)), Cambio::Idioma(None));
        assert_eq!(a.generales.idioma, "");
        assert_eq!(aplicar(&mut a, "idioma", &json!(4)), Cambio::Nada);
        // Lo que dejó la versión en C# también vale.
        a.generales.idioma = "es-ES".into();
        assert_eq!(idioma(&a)["valor"], 1);
    }

    #[test]
    fn en_ingles_y_en_portugues() {
        let instalado = Sistema {
            vigem: true,
            ..Sistema::default()
        };
        let en = describir(&AjustesApp::default(), instalado, Idioma::En);
        assert_eq!(en[0]["nombre"], "Startup");
        assert_eq!(en[0]["id"], "Inicio");
        assert_eq!(en[0]["ajustes"][0]["nombre"], "Launch at Windows startup");
        assert_eq!(en[0]["ajustes"][3]["nombre"], "Language");
        assert_eq!(
            en[0]["ajustes"][3]["opciones"],
            json!(["Automatic", "Español", "English", "Português"])
        );
        assert_eq!(en[1]["ajustes"][0]["opciones"][1], "Problems only");
        assert_eq!(en[4]["ajustes"][3]["nombre"], "Uninstall ViGEmBus");
        assert_eq!(en[4]["ajustes"][4]["nombre"], "Install HidHide");
        assert!(en[3]["ajustes"][0]["ayuda"]
            .as_str()
            .unwrap()
            .contains("HidHide"));

        let pt = describir(&AjustesApp::default(), Sistema::default(), Idioma::Pt);
        assert_eq!(pt[2]["nombre"], "Desempenho");
        assert_eq!(pt[2]["ajustes"][0]["opciones"][1], "Acima do normal");
        assert_eq!(pt[0]["ajustes"][3]["opciones"][0], "Automático");
        assert_eq!(
            pt[3]["ajustes"][1]["nombre"],
            "Trocar de perfil deslizando no touchpad"
        );
        assert_eq!(pt[4]["ajustes"][2]["nombre"], "Verificar atualizações");
    }
}
