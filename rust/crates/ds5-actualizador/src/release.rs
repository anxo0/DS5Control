//! Lee la última release que devuelve la API de GitHub y decide si hay que actualizarse.

use serde_json::Value;

use crate::error::ErrorActualizacion;
use crate::version::Version;

/// La dirección de la API que devuelve la última release publicada.
pub const URL_ULTIMA_RELEASE: &str =
    "https://api.github.com/repos/anxo0/DS5Control/releases/latest";

/// El ejecutable que tiene que traer la release y que se abre al terminar.
pub const PROGRAMA: &str = "DS5Control.exe";

/// El final del nombre del zip de la release que se descarga.
pub const SUFIJO_ZIP: &str = "-win-x64.zip";

/// Solo se descarga de las releases de este repositorio.
pub const DESCARGAS_DE_CONFIANZA: &str = "https://github.com/anxo0/DS5Control/releases/download/";

/// El principio del campo `digest` de cada archivo de la release.
pub const PREFIJO_HUELLA: &str = "sha256:";

/// Una release más nueva a la que puede pasar esta instalación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actualizacion {
    pub version: Version,
    pub url_descarga: String,
    /// La huella SHA-256 del zip en hexadecimal, tal como la da GitHub.
    pub sha256: String,
}

/// Lee una release de la API de GitHub. Devuelve `None` salvo que sea más nueva que `actual`,
/// venga de este repositorio y diga la huella con la que comprobar la descarga.
///
/// Da error si el texto no es JSON o si los campos no tienen la forma esperada (por ejemplo, un
/// `tag_name` que es un número), los mismos casos en que la versión en C# lanza una excepción.
pub fn leer_release(
    json: &str,
    actual: &Version,
) -> Result<Option<Actualizacion>, ErrorActualizacion> {
    let raiz: Value = serde_json::from_str(json)?;
    let raiz = raiz
        .as_object()
        .ok_or(ErrorActualizacion::Formato("la release no es un objeto"))?;

    let Some(etiqueta) = raiz.get("tag_name") else {
        return Ok(None);
    };
    let etiqueta = cadena(etiqueta, "tag_name no es un texto")?;
    let Some(version) = etiqueta.and_then(version_de_etiqueta) else {
        return Ok(None);
    };
    if !version.es_posterior_a(actual) {
        return Ok(None);
    }

    let Some(archivos) = raiz.get("assets") else {
        return Ok(None);
    };
    let archivos = archivos
        .as_array()
        .ok_or(ErrorActualizacion::Formato("assets no es una lista"))?;

    for archivo in archivos {
        let archivo = archivo
            .as_object()
            .ok_or(ErrorActualizacion::Formato("un archivo no es un objeto"))?;
        let nombre = obligatorio(archivo.get("name"), "name")?;
        if !termina_con_sin_mayusculas(nombre, SUFIJO_ZIP) {
            continue;
        }

        let url = obligatorio(archivo.get("browser_download_url"), "browser_download_url")?;
        let huella = match archivo.get("digest") {
            Some(valor) => cadena(valor, "digest no es un texto")?,
            None => None,
        };
        if !url.starts_with(DESCARGAS_DE_CONFIANZA) {
            return Ok(None);
        }
        let Some(sha256) = huella.and_then(|h| h.strip_prefix(PREFIJO_HUELLA)) else {
            return Ok(None);
        };

        return Ok(Some(Actualizacion {
            version,
            url_descarga: url.to_string(),
            sha256: sha256.to_string(),
        }));
    }
    Ok(None)
}

/// La versión de una etiqueta como `v0.2.8` o `v0.3.0-beta`: sin las `v` del principio y sin
/// lo que va desde el primer guion.
pub fn version_de_etiqueta(etiqueta: &str) -> Option<Version> {
    let sin_v = etiqueta.trim_start_matches('v');
    Version::analizar(sin_v.split('-').next().unwrap_or(sin_v))
}

/// Como `JsonElement.GetString()`: un texto, `None` si es `null` y error con cualquier otra cosa.
fn cadena<'a>(
    valor: &'a Value,
    error: &'static str,
) -> Result<Option<&'a str>, ErrorActualizacion> {
    match valor {
        Value::String(texto) => Ok(Some(texto.as_str())),
        Value::Null => Ok(None),
        _ => Err(ErrorActualizacion::Formato(error)),
    }
}

/// Como `GetProperty(nombre).GetString() ?? ""`: el campo tiene que estar, y `null` cuenta como
/// texto vacío.
fn obligatorio<'a>(
    valor: Option<&'a Value>,
    campo: &'static str,
) -> Result<&'a str, ErrorActualizacion> {
    let valor = valor.ok_or(ErrorActualizacion::Formato(campo))?;
    Ok(cadena(valor, campo)?.unwrap_or(""))
}

/// `EndsWith(sufijo, StringComparison.OrdinalIgnoreCase)` para un sufijo ASCII.
pub(crate) fn termina_con_sin_mayusculas(texto: &str, sufijo: &str) -> bool {
    texto.len() >= sufijo.len()
        && texto.is_char_boundary(texto.len() - sufijo.len())
        && texto[texto.len() - sufijo.len()..].eq_ignore_ascii_case(sufijo)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    const URL: &str =
        "https://github.com/anxo0/DS5Control/releases/download/v0.2.0/DS5Control-0.2.0-win-x64.zip";

    fn actual() -> Version {
        Version::nueva(0, 1, 0)
    }

    /// La misma release de ejemplo que `AppUpdaterTests.Release`.
    fn release(etiqueta: &str, url: &str, huella: &str) -> String {
        format!(
            r#"{{"tag_name": "{etiqueta}", "assets": [
    {{"name": "notas.txt", "browser_download_url": "https://github.com/anxo0/DS5Control/releases/download/x/notas.txt"}},
    {{"name": "DS5Control-0.2.0-win-x64.zip", {huella} "browser_download_url": "{url}"}}
]}}"#
        )
    }

    fn por_defecto() -> String {
        release("v0.2.0", URL, r#""digest": "sha256:abc","#)
    }

    #[test]
    fn encuentra_una_release_mas_nueva() {
        let actualizacion = leer_release(&por_defecto(), &actual()).unwrap().unwrap();

        assert_eq!(actualizacion.version, Version::nueva(0, 2, 0));
        assert_eq!(actualizacion.url_descarga, URL);
        assert_eq!(actualizacion.sha256, "abc");
    }

    #[test]
    fn ignora_las_releases_que_no_son_mas_nuevas() {
        for etiqueta in ["v0.1.0", "0.1", "v0.0.9", "no-es-version"] {
            let json = release(etiqueta, URL, r#""digest": "sha256:abc","#);
            assert_eq!(leer_release(&json, &actual()).unwrap(), None, "{etiqueta}");
        }
    }

    #[test]
    fn rechaza_descargas_de_otros_sitios() {
        let json = release(
            "v0.2.0",
            "https://example.com/DS5Control-win-x64.zip",
            r#""digest": "sha256:abc","#,
        );
        assert_eq!(leer_release(&json, &actual()).unwrap(), None);
    }

    #[test]
    fn rechaza_releases_sin_huella() {
        let json = release("v0.2.0", URL, "");
        assert_eq!(leer_release(&json, &actual()).unwrap(), None);
    }

    #[test]
    fn rechaza_huellas_que_no_son_sha256() {
        let json = release("v0.2.0", URL, r#""digest": "md5:abc","#);
        assert_eq!(leer_release(&json, &actual()).unwrap(), None);
        let json = release("v0.2.0", URL, r#""digest": null,"#);
        assert_eq!(leer_release(&json, &actual()).unwrap(), None);
    }

    #[test]
    fn la_etiqueta_pierde_las_v_y_lo_que_va_tras_el_guion() {
        assert_eq!(
            version_de_etiqueta("vv0.3.1-beta-2"),
            Some(Version::nueva(0, 3, 1))
        );
        assert_eq!(version_de_etiqueta("V0.3.1"), None);
        let json = release("v0.3.0-beta", URL, r#""digest": "sha256:abc","#);
        let actualizacion = leer_release(&json, &actual()).unwrap().unwrap();
        assert_eq!(actualizacion.version, Version::nueva(0, 3, 0));
    }

    #[test]
    fn el_nombre_del_zip_no_distingue_mayusculas() {
        let json = r#"{"tag_name": "v0.2.0", "assets": [
            {"name": "DS5CONTROL-WIN-X64.ZIP", "digest": "sha256:abc", "browser_download_url": "https://github.com/anxo0/DS5Control/releases/download/v0.2.0/a.zip"}
        ]}"#;
        assert!(leer_release(json, &actual()).unwrap().is_some());
    }

    #[test]
    fn el_primer_zip_decide_aunque_no_valga() {
        let json = r#"{"tag_name": "v0.2.0", "assets": [
            {"name": "malo-win-x64.zip", "browser_download_url": "https://example.com/a.zip", "digest": "sha256:abc"},
            {"name": "bueno-win-x64.zip", "browser_download_url": "https://github.com/anxo0/DS5Control/releases/download/v0.2.0/b.zip", "digest": "sha256:abc"}
        ]}"#;
        assert_eq!(leer_release(json, &actual()).unwrap(), None);
    }

    #[test]
    fn sin_etiqueta_o_sin_archivos_no_hay_actualizacion() {
        assert_eq!(leer_release(r#"{"assets": []}"#, &actual()).unwrap(), None);
        assert_eq!(
            leer_release(r#"{"tag_name": null}"#, &actual()).unwrap(),
            None
        );
        assert_eq!(
            leer_release(r#"{"tag_name": "v0.2.0"}"#, &actual()).unwrap(),
            None
        );
        let vacia = r#"{"tag_name": "v0.2.0", "assets": []}"#;
        assert_eq!(leer_release(vacia, &actual()).unwrap(), None);
    }

    #[test]
    fn la_forma_equivocada_es_un_error_como_en_csharp() {
        for json in [
            "no es json",
            "[]",
            r#"{"tag_name": 5}"#,
            r#"{"tag_name": "v0.2.0", "assets": {}}"#,
            r#"{"tag_name": "v0.2.0", "assets": [5]}"#,
            r#"{"tag_name": "v0.2.0", "assets": [{"browser_download_url": "x"}]}"#,
            r#"{"tag_name": "v0.2.0", "assets": [{"name": "a-win-x64.zip"}]}"#,
            r#"{"tag_name": "v0.2.0", "assets": [{"name": "a-win-x64.zip", "browser_download_url": "x", "digest": 1}]}"#,
        ] {
            assert!(leer_release(json, &actual()).is_err(), "{json}");
        }
    }

    #[test]
    fn una_release_antigua_no_mira_los_archivos() {
        // C# sale antes de leer los archivos, así que su forma no importa.
        let json = r#"{"tag_name": "v0.0.1", "assets": 5}"#;
        assert_eq!(leer_release(json, &actual()).unwrap(), None);
    }

    #[test]
    fn el_ultimo_campo_repetido_es_el_que_vale() {
        let json = r#"{"tag_name": "v9.0.0", "tag_name": "v0.0.1", "assets": []}"#;
        assert_eq!(leer_release(json, &actual()).unwrap(), None);
    }
}
