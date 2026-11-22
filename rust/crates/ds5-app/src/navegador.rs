//! Lo que la página guarda en el almacenamiento del navegador (el tema, la paleta del logo y los
//! colores guardados) vive en `InterfazNavegador.json`, en la carpeta de ajustes, igual que en la
//! versión en C# (`WebUiWindow.BrowserStorageScript`). Así se hereda al pasar de una versión a la
//! otra, aunque cada una tenga su propio navegador.
//!
//! Antes de la página corre un script: al abrir la ventana, los valores del archivo mandan sobre
//! los del navegador, y cada cambio de una de esas claves se manda a la app para escribirlo. Al
//! recargar la página (Ctrl+Shift+R) ya no se importan: el archivo que leyó la app al arrancar
//! está viejo y deshacería los cambios hechos desde entonces.

use std::path::Path;

pub const ARCHIVO: &str = "InterfazNavegador.json";
const CLAVES: [&str; 3] = ["tema", "paleta", "colores"];

/// La paleta del logo guardada, para poner el icono antes de que exista la página.
pub fn paleta_guardada(carpeta: &Path) -> Option<String> {
    let t = std::fs::read_to_string(carpeta.join(ARCHIVO)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&t).ok()?;
    v.get("paleta")?.as_str().map(str::to_string)
}

/// El tema guardado (`claro` u `oscuro`), para el menú de la bandeja.
pub fn tema_guardado(carpeta: &Path) -> Option<String> {
    let t = std::fs::read_to_string(carpeta.join(ARCHIVO)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&t).ok()?;
    v.get("tema")?.as_str().map(str::to_string)
}

/// El script, con lo que haya guardado en `carpeta`.
pub fn script(carpeta: &Path) -> String {
    let guardado = std::fs::read_to_string(carpeta.join(ARCHIVO))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .filter(|v| v.is_object())
        .map_or("null".to_string(), |v| v.to_string());
    let claves = serde_json::to_string(&CLAVES).unwrap_or_default();
    format!(
        "(function (guardado, claves) {{\
         try {{\
           if (guardado && !sessionStorage.getItem('ds5-importado')) for (const k of claves) if (typeof guardado[k] === 'string') localStorage.setItem(k, guardado[k]);\
           sessionStorage.setItem('ds5-importado', '1');\
           const enviar = () => {{ const datos = {{}}; for (const k of claves) {{ const v = localStorage.getItem(k); if (v !== null) datos[k] = v; }}\
             const t = window.__TAURI__; if (t) t.core.invoke('mensaje', {{ mensaje: {{ tipo: 'navegador', datos }} }}); }};\
           const poner = Storage.prototype.setItem;\
           Storage.prototype.setItem = function (k, v) {{ poner.call(this, k, v); if (this === localStorage && claves.includes(k)) enviar(); }};\
           document.addEventListener('DOMContentLoaded', enviar);\
         }} catch (e) {{}}\
         }})({guardado}, {claves});"
    )
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn sin_archivo_no_importa_nada() {
        let s = script(Path::new("C:\\no-existe-ds5"));
        assert!(s.ends_with("})(null, [\"tema\",\"paleta\",\"colores\"]);"));
    }

    #[test]
    fn con_archivo_lo_mete_en_el_script() {
        let d = std::env::temp_dir().join(format!("ds5-navegador-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(ARCHIVO), r#"{"tema":"oscuro","paleta":"fuego"}"#).unwrap();
        let s = script(&d);
        assert!(s.contains(r#"({"paleta":"fuego","tema":"oscuro"}, "#));
        let _ = std::fs::remove_dir_all(&d);
    }
}
