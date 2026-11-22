//! Los textos que enseña el programa al actualizarse, los mismos que en `AppHost.cs`.

use std::path::Path;
use std::time::Duration;

use crate::version::Version;

/// La página de releases a la que se manda al usuario si no se puede actualizar solo.
pub const PAGINA_RELEASES: &str = "https://github.com/anxo0/DS5Control/releases/latest";

/// Lo que se espera tras abrir el programa antes de buscar actualizaciones.
pub const ESPERA_ANTES_DE_BUSCAR: Duration = Duration::from_millis(5000);

/// Al abrir la versión nueva, si [`crate::tomar_aviso`] devuelve una versión.
pub fn actualizado(version: &Version) -> String {
    format!(
        "DS5Control se ha actualizado a la versión {}.",
        version.texto_corto()
    )
}

/// En el registro, justo antes de cerrarse para actualizarse.
pub fn actualizando_registro(version: &Version) -> String {
    format!("Actualizando a la versión {}...", version.texto_corto())
}

/// La notificación que avisa de que el programa se va a cerrar y a abrir solo. Se enseña
/// aunque el usuario haya quitado las notificaciones: tiene que saber por qué se cierra.
pub fn actualizando_notificacion(version: &Version) -> String {
    format!(
        "Actualizando a la versión {}. DS5Control se cerrará y volverá a abrirse solo en unos segundos.",
        version.texto_corto()
    )
}

/// Hay versión nueva, pero no se puede escribir en la carpeta del programa. Aquí la versión va
/// entera, como `Version.ToString()` en C#.
pub fn sin_permiso(version: &Version, carpeta_programa: &Path) -> String {
    format!(
        "Hay una versión nueva ({version}), pero no se puede escribir en {}. Descárgala de {PAGINA_RELEASES}",
        carpeta_programa.display()
    )
}

/// Cuando algo falla por el camino.
pub fn fallo(error: &dyn std::fmt::Display) -> String {
    format!("No se pudo actualizar: {error}")
}

/// Las compilaciones de desarrollo no se sustituyen por una release.
pub const DESARROLLO: &str = "Las compilaciones de desarrollo no se actualizan.";

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn los_textos_son_los_de_csharp() {
        let version = Version::analizar("0.2.9").unwrap();
        assert_eq!(
            actualizado(&version),
            "DS5Control se ha actualizado a la versión 0.2.9."
        );
        assert_eq!(
            actualizando_registro(&version),
            "Actualizando a la versión 0.2.9..."
        );
        assert_eq!(
            actualizando_notificacion(&version),
            "Actualizando a la versión 0.2.9. DS5Control se cerrará y volverá a abrirse solo en unos segundos."
        );
        assert_eq!(
            sin_permiso(&Version::analizar("1.0.0.2").unwrap(), Path::new(r"C:\DS5")),
            r"Hay una versión nueva (1.0.0.2), pero no se puede escribir en C:\DS5. Descárgala de https://github.com/anxo0/DS5Control/releases/latest"
        );
        assert_eq!(fallo(&"sin red"), "No se pudo actualizar: sin red");
        assert_eq!(
            actualizado(&Version::analizar("0.3.0.0").unwrap()),
            "DS5Control se ha actualizado a la versión 0.3.0."
        );
    }
}
