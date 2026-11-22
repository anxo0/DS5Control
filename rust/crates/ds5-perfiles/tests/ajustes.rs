//! La colección de ajustes de C#: `Profiles.xml`, `Actions.xml`, `Auto Profiles.xml` y
//! `LinkedProfiles.xml`. El prefijo de cada archivo dice de qué tipo es.

use std::path::PathBuf;

use ds5_perfiles::{Acciones, AjustesApp, Autoperfiles, Enlazados};

fn carpeta() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../DS5ControlTests/Ajustes")
}

fn ida_y_vuelta(nombre: &str, texto: &str) -> Result<String, String> {
    if nombre.starts_with("ajustes-") {
        AjustesApp::leer(texto).map(|a| a.escribir(None))
    } else if nombre.starts_with("acciones-") {
        Acciones::leer(texto).map(|a| a.escribir())
    } else if nombre.starts_with("autoperfiles-") {
        Autoperfiles::leer(texto).map(|a| a.escribir())
    } else if nombre.starts_with("enlazados-") {
        Enlazados::leer(texto).map(|a| a.escribir())
    } else {
        Err(format!("no sé qué tipo de archivo es {nombre}"))
    }
}

#[test]
fn la_coleccion_se_guarda_igual_que_en_csharp() {
    // La colección sale de ajustes reales y no está en git: sin ella no se comprueba nada.
    let Ok(lista) = std::fs::read_dir(carpeta().join("entrada")) else {
        eprintln!("No está la colección de ajustes: no se comprueba nada.");
        return;
    };
    let mut fallos = Vec::new();
    let mut total = 0;
    for entrada in lista {
        let ruta = entrada.unwrap().path();
        let nombre = ruta.file_name().unwrap().to_string_lossy().to_string();
        total += 1;
        let original = std::fs::read_to_string(&ruta).unwrap();
        let esperado = std::fs::read_to_string(carpeta().join("esperado").join(&nombre)).unwrap();
        match ida_y_vuelta(&nombre, &original) {
            Ok(guardado) if guardado == esperado => {
                let otra_vez = ida_y_vuelta(&nombre, &guardado).unwrap();
                if otra_vez != guardado {
                    fallos.push(format!("{nombre} cambia cada vez que se guarda"));
                }
            }
            Ok(guardado) => {
                let linea = guardado
                    .lines()
                    .zip(esperado.lines())
                    .position(|(a, b)| a != b)
                    .unwrap_or_else(|| guardado.lines().count().min(esperado.lines().count()));
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
    assert_eq!(total, 8, "la colección debería tener ocho archivos");
    assert!(fallos.is_empty(), "{}", fallos.join("\n"));
}
