//! Lo que sobra de la versión en C# cuando la de Rust se instala encima. El actualizador de C#
//! copia los archivos nuevos sobre los viejos sin borrar nada, así que en la carpeta del programa
//! se quedan las librerías de .NET (unos 150 MB) que ya no usa nadie.
//!
//! Solo se borra si la carpeta es de verdad la de la versión en C# (tiene `DS5Control.dll` y
//! `DS5Control.runtimeconfig.json`), y solo lo que .NET publica: librerías, sus descripciones y
//! las carpetas de idiomas y de `runtimes`. Los ajustes no están aquí, sino en `%APPDATA%`.

use std::fs;
use std::path::Path;

/// Las marcas de la versión en C#: si falta alguna, no se toca nada.
const MARCAS: [&str; 2] = ["DS5Control.dll", "DS5Control.runtimeconfig.json"];

/// Archivos sueltos de la publicación de .NET que no acaban en `.dll`.
const SUELTOS: [&str; 5] = [
    "DS5Control.deps.json",
    "DS5Control.runtimeconfig.json",
    "DS5Control.pdb",
    "NLog.config",
    "createdump.exe",
];

/// Borra lo que sobra de C# en `carpeta` y dice cuántos archivos y carpetas se fueron.
pub fn limpiar(carpeta: &Path) -> usize {
    if !MARCAS.iter().all(|m| carpeta.join(m).is_file()) {
        return 0;
    }
    let mut borrados = 0;
    let Ok(entradas) = fs::read_dir(carpeta) else {
        return 0;
    };
    for e in entradas.flatten() {
        let ruta = e.path();
        let nombre = e.file_name().to_string_lossy().to_string();
        let tipo = match e.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let sobra = if tipo.is_file() {
            nombre.to_ascii_lowercase().ends_with(".dll") || SUELTOS.contains(&nombre.as_str())
        } else if tipo.is_dir() {
            nombre.eq_ignore_ascii_case("runtimes") || solo_recursos(&ruta)
        } else {
            false
        };
        if !sobra {
            continue;
        }
        let r = if tipo.is_dir() {
            fs::remove_dir_all(&ruta)
        } else {
            fs::remove_file(&ruta)
        };
        if r.is_ok() {
            borrados += 1;
        }
    }
    borrados
}

/// Una carpeta de idioma de .NET (`de`, `es`, `zh-Hans`...): solo tiene `*.resources.dll`.
fn solo_recursos(carpeta: &Path) -> bool {
    let Ok(entradas) = fs::read_dir(carpeta) else {
        return false;
    };
    let mut alguno = false;
    for e in entradas.flatten() {
        let nombre = e.file_name().to_string_lossy().to_ascii_lowercase();
        if !e.file_type().is_ok_and(|t| t.is_file()) || !nombre.ends_with(".resources.dll") {
            return false;
        }
        alguno = true;
    }
    alguno
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn carpeta(nombre: &str) -> std::path::PathBuf {
        let c = std::env::temp_dir().join(format!("ds5-limpieza-{nombre}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&c);
        fs::create_dir_all(&c).unwrap();
        c
    }

    fn poner(c: &Path, relativo: &str) {
        let r = c.join(relativo);
        fs::create_dir_all(r.parent().unwrap()).unwrap();
        fs::write(r, "x").unwrap();
    }

    #[test]
    fn borra_lo_de_csharp_y_deja_lo_demas() {
        let c = carpeta("csharp");
        for a in [
            "DS5Control.exe",
            "DS5Control.dll",
            "DS5Control.runtimeconfig.json",
            "DS5Control.deps.json",
            "NLog.config",
            "WebView2Loader.dll",
            "runtimes/win-x64/native/a.dll",
            "es/DS5Control.resources.dll",
            "drivers/ViGEmBus.exe",
            "COPYING",
            "mis notas.txt",
            "fotos/una.dll.txt",
        ] {
            poner(&c, a);
        }
        assert_eq!(limpiar(&c), 7);
        for queda in [
            "DS5Control.exe",
            "drivers/ViGEmBus.exe",
            "COPYING",
            "mis notas.txt",
            "fotos/una.dll.txt",
        ] {
            assert!(c.join(queda).exists(), "{queda}");
        }
        for va in ["DS5Control.dll", "runtimes", "es", "NLog.config"] {
            assert!(!c.join(va).exists(), "{va}");
        }
        let _ = fs::remove_dir_all(c);
    }

    #[test]
    fn sin_las_marcas_no_toca_nada() {
        let c = carpeta("otra");
        poner(&c, "DS5Control.exe");
        poner(&c, "algo.dll");
        poner(&c, "DS5Control.dll");
        assert_eq!(limpiar(&c), 0);
        assert!(c.join("algo.dll").exists());
        let _ = fs::remove_dir_all(c);
    }
}
