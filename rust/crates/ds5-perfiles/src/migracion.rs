//! La migración de perfiles antiguos, como `ProfileMigration.cs`.
//!
//! La versión del perfil va en el atributo `config_version`. Sin él, es la 2 si tiene
//! `LSAntiDeadZone` y la 1 si no. Las versiones de la 1 a la 4 se pasan a la 5 paso a paso;
//! cada paso solo mira los elementos de primer nivel.

use crate::numeros;
use crate::xml::{Elemento, ErrorXml, Nodo};

/// La versión actual del formato de los perfiles (`Global.CONFIG_VERSION`).
pub const VERSION_PERFIL: i32 = 5;
pub const VERSION_PERFIL_TEXTO: &str = "5";

/// La zona muerta que pone la migración a la versión 2 si el perfil tenía 0.
const ZONA_MUERTA_POR_DEFECTO: i32 = 10;

/// Lee el texto de un perfil y lo devuelve ya migrado a la versión actual.
pub fn migrar_perfil(texto: &str) -> Result<Elemento, ErrorXml> {
    let mut raiz = Elemento::leer(texto)?;
    let mut version = raiz
        .atributo("config_version")
        .filter(|v| !v.is_empty())
        .and_then(numeros::entero_en::<i32>)
        .unwrap_or(0);
    if version == 0 {
        version = if contiene(&raiz, "LSAntiDeadZone") {
            2
        } else {
            1
        };
    }
    if !(1..VERSION_PERFIL).contains(&version) {
        return Ok(raiz);
    }
    if version == 1 {
        raiz = version_2(&raiz);
    }
    if version <= 3 {
        raiz = version_4(&raiz);
    }
    raiz = version_5(&raiz);
    Ok(raiz)
}

fn contiene(e: &Elemento, nombre: &str) -> bool {
    e.elementos()
        .any(|h| h.nombre == nombre || contiene(h, nombre))
}

/// Un nuevo elemento raíz con los elementos de primer nivel que devuelva `pasar`. Los textos y
/// comentarios del primer nivel se pierden, como al reescribir el XML en C#.
fn reescribir(
    raiz: &Elemento,
    version: &str,
    mut pasar: impl FnMut(&Elemento, &mut Vec<Nodo>),
) -> Elemento {
    let mut nueva = Elemento::nuevo("DS4Windows");
    nueva.atributos = vec![
        ("app_version".into(), String::new()),
        ("config_version".into(), version.into()),
    ];
    for hijo in raiz.elementos() {
        pasar(hijo, &mut nueva.hijos);
    }
    nueva
}

/// Versión 2: una zona muerta de 0 o menos pasa a la de por defecto; una que no es un número
/// desaparece.
fn version_2(raiz: &Elemento) -> Elemento {
    reescribir(raiz, "2", |hijo, salida| match hijo.nombre.as_str() {
        "LSDeadZone" | "RSDeadZone" => {
            if let Some(mut v) = numeros::entero_en::<i32>(&hijo.texto()) {
                if v <= 0 {
                    v = ZONA_MUERTA_POR_DEFECTO;
                }
                salida.push(Nodo::Elemento(Elemento::con_texto(
                    &hijo.nombre,
                    &v.to_string(),
                )));
            }
        }
        _ => salida.push(Nodo::Elemento(hijo.clone())),
    })
}

/// Versión 4: el suavizado del giroscopio pasa a sus grupos. Los números se leen y escriben
/// con la configuración regional, como en C#.
fn version_4(raiz: &Elemento) -> Elemento {
    let mut usar_raton = false;
    let mut peso_raton = 0.5;
    let mut usar_stick = false;
    let mut peso_stick = 0.5;
    for hijo in raiz.elementos() {
        let texto = hijo.texto();
        match hijo.nombre.as_str() {
            "GyroSmoothing" => usar_raton = numeros::bool_net(&texto).unwrap_or(false),
            "GyroSmoothingWeight" => peso_raton = numeros::doble_es(&texto).unwrap_or(0.0),
            "GyroMouseStickSmoothing" => usar_stick = numeros::bool_net(&texto).unwrap_or(false),
            "GyroMouseStickSmoothingWeight" => {
                peso_stick = numeros::doble_es(&texto).unwrap_or(0.0)
            }
            _ => {}
        }
    }
    let grupo = |nombre: &str, usar: bool, peso: f64| {
        let mut g = Elemento::nuevo(nombre);
        g.hijos.push(Nodo::Elemento(Elemento::con_texto(
            "UseSmoothing",
            numeros::formato_bool(usar),
        )));
        if usar {
            g.hijos.push(Nodo::Elemento(Elemento::con_texto(
                "SmoothingMethod",
                "weighted-average",
            )));
        }
        g.hijos.push(Nodo::Elemento(Elemento::con_texto(
            "SmoothingWeight",
            &numeros::formato_doble_es(peso),
        )));
        Nodo::Elemento(g)
    };
    reescribir(raiz, "4", |hijo, salida| match hijo.nombre.as_str() {
        "GyroSmoothing" => salida.push(grupo("GyroMouseSmoothingSettings", usar_raton, peso_raton)),
        "GyroMouseStickSmoothing" => salida.push(grupo(
            "GyroMouseStickSmoothingSettings",
            usar_stick,
            peso_stick,
        )),
        "GyroSmoothingWeight" | "GyroMouseStickSmoothingWeight" => {}
        _ => salida.push(Nodo::Elemento(hijo.clone())),
    })
}

/// Versión 5: `UseTPforControls` activado pasa a `TouchpadOutputMode` con `Controls`.
fn version_5(raiz: &Elemento) -> Elemento {
    reescribir(raiz, "5", |hijo, salida| match hijo.nombre.as_str() {
        "UseTPforControls" => {
            if numeros::bool_net(&hijo.texto()) == Some(true) {
                salida.push(Nodo::Elemento(Elemento::con_texto(
                    "TouchpadOutputMode",
                    "Controls",
                )));
            }
        }
        _ => salida.push(Nodo::Elemento(hijo.clone())),
    })
}
