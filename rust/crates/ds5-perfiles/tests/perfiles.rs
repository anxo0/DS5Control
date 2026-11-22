//! La colección de perfiles y los casos campo a campo que genera la versión en C#.

use std::collections::BTreeMap;
use std::path::PathBuf;

use ds5_perfiles::xml::{Elemento, Nodo};
use ds5_perfiles::Perfil;

/// La colección sale de perfiles reales y no está en git: sin ella no se comprueba nada.
fn coleccion() -> Option<std::fs::ReadDir> {
    let lista = std::fs::read_dir(carpeta().join("entrada")).ok();
    if lista.is_none() {
        eprintln!("No está la colección de perfiles: no se comprueba nada.");
    }
    lista
}

fn carpeta() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../DS5ControlTests/Perfiles")
}

fn ida_y_vuelta(texto: &str) -> Result<String, String> {
    Perfil::leer(texto)
        .map(|p| p.escribir(None))
        .map_err(|e| e.0)
}

#[test]
fn la_coleccion_se_guarda_igual_que_en_csharp() {
    let Some(lista) = coleccion() else {
        return;
    };
    let mut fallos = Vec::new();
    for entrada in lista {
        let ruta = entrada.unwrap().path();
        let nombre = ruta.file_name().unwrap().to_string_lossy().to_string();
        let original = std::fs::read_to_string(&ruta).unwrap();
        let esperado = std::fs::read_to_string(carpeta().join("esperado").join(&nombre)).unwrap();
        match ida_y_vuelta(&original) {
            Ok(guardado) if guardado == esperado => {}
            Ok(guardado) => {
                let linea = guardado
                    .lines()
                    .zip(esperado.lines())
                    .position(|(a, b)| a != b)
                    .unwrap_or(0);
                fallos.push(format!(
                    "{nombre}, línea {}: {:?} en vez de {:?}",
                    linea + 1,
                    guardado.lines().nth(linea),
                    esperado.lines().nth(linea)
                ));
            }
            Err(e) => fallos.push(format!("{nombre}: {e}")),
        }
    }
    assert!(fallos.is_empty(), "{}", fallos.join("\n"));
}

/// Los elementos simples fuera de las asignaciones y las acciones, por su ruta.
fn aplanar(e: &Elemento, ruta: &str, salida: &mut BTreeMap<String, String>) {
    for hijo in e.elementos() {
        if ruta.is_empty()
            && matches!(
                hijo.nombre.as_str(),
                "Control" | "ShiftControl" | "ProfileActions"
            )
        {
            continue;
        }
        let r = if ruta.is_empty() {
            hijo.nombre.clone()
        } else {
            format!("{ruta}/{}", hijo.nombre)
        };
        if hijo.elementos().next().is_none() {
            salida.insert(r, hijo.texto());
        } else {
            aplanar(hijo, &r, salida);
        }
    }
}

fn plano(texto: &str) -> BTreeMap<String, String> {
    let mut salida = BTreeMap::new();
    aplanar(&Elemento::leer(texto).unwrap(), "", &mut salida);
    salida
}

/// Cambia o quita un elemento simple del perfil base.
fn modificar(raiz: &mut Elemento, ruta: &[&str], valor: Option<&str>) {
    let (primero, resto) = ruta.split_first().unwrap();
    let posicion = raiz
        .hijos
        .iter()
        .position(|n| matches!(n, Nodo::Elemento(e) if e.nombre == *primero))
        .unwrap();
    if resto.is_empty() {
        match valor {
            None => {
                raiz.hijos.remove(posicion);
            }
            Some(v) => {
                if let Nodo::Elemento(e) = &mut raiz.hijos[posicion] {
                    e.hijos = vec![Nodo::Texto(v.to_string())];
                }
            }
        }
    } else if let Nodo::Elemento(e) = &mut raiz.hijos[posicion] {
        modificar(e, resto, valor);
    }
}

/// Vuelve a escribir un árbol como texto, solo para dárselo al lector.
fn texto_de(e: &Elemento) -> String {
    fn esc(t: &str) -> String {
        t.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }
    let mut s = format!("<{}", e.nombre);
    for (n, v) in &e.atributos {
        s.push_str(&format!(" {n}=\"{}\"", esc(v)));
    }
    s.push('>');
    for h in &e.hijos {
        match h {
            Nodo::Texto(t) => s.push_str(&esc(t)),
            Nodo::Elemento(h) => s.push_str(&texto_de(h)),
        }
    }
    s.push_str(&format!("</{}>", e.nombre));
    s
}

#[test]
fn los_casos_campo_a_campo_dan_lo_mismo_que_en_csharp() {
    if coleccion().is_none() {
        return;
    }
    let base_texto =
        std::fs::read_to_string(carpeta().join("entrada/02-por-defecto-ds4.xml")).unwrap();
    let base = Elemento::leer(&base_texto).unwrap();
    let salida_base = plano(&ida_y_vuelta(&base_texto).unwrap());
    let casos = std::fs::read_to_string(carpeta().join("casos.jsonl")).unwrap();

    let mut fallos = Vec::new();
    let mut total = 0;
    for linea in casos.lines().filter(|l| !l.is_empty()) {
        total += 1;
        let caso: serde_json::Value = serde_json::from_str(linea).unwrap();
        let ruta = caso["ruta"].as_str().unwrap();
        let valor = caso["valor"].as_str();
        let mut copia = base.clone();
        modificar(&mut copia, &ruta.split('/').collect::<Vec<_>>(), valor);
        let resultado = ida_y_vuelta(&texto_de(&copia));

        let esperado_error = caso.get("error").is_some();
        let obtenido = match resultado {
            Err(_) if esperado_error => continue,
            Err(e) => format!("error: {e}"),
            Ok(_) if esperado_error => "se carga, pero C# lo rechaza".to_string(),
            Ok(texto) => {
                let salida = plano(&texto);
                let mut cambios = serde_json::Map::new();
                for (k, v) in &salida {
                    if salida_base.get(k) != Some(v) {
                        cambios.insert(k.clone(), serde_json::Value::String(v.clone()));
                    }
                }
                for k in salida_base.keys().filter(|k| !salida.contains_key(*k)) {
                    cambios.insert(k.clone(), serde_json::Value::Null);
                }
                let cambios = serde_json::Value::Object(cambios);
                if cambios == caso["cambios"] {
                    continue;
                }
                format!("{cambios} en vez de {}", caso["cambios"])
            }
        };
        fallos.push(format!("{ruta} = {valor:?}: {obtenido}"));
    }
    let muestra: Vec<_> = fallos.iter().take(60).cloned().collect();
    assert!(
        fallos.is_empty(),
        "{} de {total} casos fallan:\n{}",
        fallos.len(),
        muestra.join("\n")
    );
}
