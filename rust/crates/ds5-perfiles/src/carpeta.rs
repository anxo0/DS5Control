//! Dónde están los ajustes, con las mismas reglas que `FindConfigLocation`:
//!
//! - si la carpeta del programa tiene ajustes, es una instalación portable y se usan ahí;
//! - si no, se usan los de `%APPDATA%\DS5Control`;
//! - si no hay en ningún sitio, o hay en los dos (y no son la misma carpeta), es la primera vez
//!   y se usa `%APPDATA%\DS5Control`.

use std::path::{Path, PathBuf};

/// Una carpeta tiene ajustes si tiene `Auto Profiles.xml` o `Profiles.xml`.
pub fn tiene_ajustes(carpeta: &Path) -> bool {
    carpeta.join("Auto Profiles.xml").is_file() || carpeta.join("Profiles.xml").is_file()
}

/// `%APPDATA%\DS5Control`.
pub fn carpeta_appdata() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("DS5Control"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarpetaAjustes {
    pub ruta: PathBuf,
    /// No hay ajustes y hay que crearlos.
    pub primera_vez: bool,
    /// Hay ajustes en la carpeta del programa y en AppData a la vez.
    pub dos_sitios: bool,
}

pub fn buscar(carpeta_programa: &Path, appdata: &Path) -> CarpetaAjustes {
    let en_programa = tiene_ajustes(carpeta_programa);
    let en_appdata = tiene_ajustes(appdata);
    let misma = en_appdata && carpeta_programa == appdata;
    let (ruta, primera_vez, dos_sitios) = if en_programa && en_appdata && !misma {
        (appdata, true, true)
    } else if en_programa {
        (carpeta_programa, false, false)
    } else if en_appdata {
        (appdata, false, false)
    } else {
        (appdata, true, false)
    };
    CarpetaAjustes {
        ruta: ruta.to_path_buf(),
        primera_vez,
        dos_sitios,
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// Una carpeta temporal propia de la prueba, que nunca es la de los ajustes reales.
    fn temporal(nombre: &str) -> PathBuf {
        let ruta =
            std::env::temp_dir().join(format!("ds5-perfiles-{nombre}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ruta);
        std::fs::create_dir_all(&ruta).unwrap();
        ruta
    }

    #[test]
    fn reglas_de_carpeta() {
        let programa = temporal("programa");
        let appdata = temporal("appdata");

        let r = buscar(&programa, &appdata);
        assert_eq!(
            (r.ruta.as_path(), r.primera_vez, r.dos_sitios),
            (appdata.as_path(), true, false)
        );

        std::fs::write(appdata.join("Profiles.xml"), "").unwrap();
        let r = buscar(&programa, &appdata);
        assert_eq!(
            (r.ruta.as_path(), r.primera_vez),
            (appdata.as_path(), false)
        );

        std::fs::write(programa.join("Auto Profiles.xml"), "").unwrap();
        let r = buscar(&programa, &appdata);
        assert!(r.primera_vez && r.dos_sitios);

        std::fs::remove_file(appdata.join("Profiles.xml")).unwrap();
        let r = buscar(&programa, &appdata);
        assert_eq!(
            (r.ruta.as_path(), r.primera_vez),
            (programa.as_path(), false)
        );

        let _ = std::fs::remove_dir_all(&programa);
        let _ = std::fs::remove_dir_all(&appdata);
    }
}
