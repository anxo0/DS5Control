//! Lo que puede salir mal al buscar, descargar o instalar una actualización.

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum ErrorActualizacion {
    /// No se pudo hablar con GitHub o respondió con un error (`HttpRequestException` en C#).
    Red(Box<ureq::Error>),
    /// No se pudo leer o escribir en el disco (`IOException` y `UnauthorizedAccessException`).
    Disco(io::Error),
    /// La respuesta no es JSON (`JsonException`).
    Json(serde_json::Error),
    /// El JSON no tiene la forma de una release: dice qué campo falla.
    Formato(&'static str),
    /// El zip descargado no tiene la huella que dice la release (`InvalidDataException`).
    HuellaDistinta,
    /// El zip no trae el programa (`InvalidDataException`).
    SinPrograma,
    /// El zip está dañado o usa algo que no se sabe leer (`InvalidDataException`).
    ZipDanado(String),
}

impl fmt::Display for ErrorActualizacion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorActualizacion::Red(error) => write!(f, "{error}"),
            ErrorActualizacion::Disco(error) => write!(f, "{error}"),
            ErrorActualizacion::Json(error) => write!(f, "{error}"),
            ErrorActualizacion::Formato(campo) => {
                write!(f, "La release no tiene la forma esperada: {campo}.")
            }
            ErrorActualizacion::HuellaDistinta => {
                write!(f, "La descarga no coincide con la huella de la release.")
            }
            ErrorActualizacion::SinPrograma => {
                write!(f, "La release no contiene {}.", crate::PROGRAMA)
            }
            ErrorActualizacion::ZipDanado(motivo) => write!(f, "El zip está dañado: {motivo}"),
        }
    }
}

impl std::error::Error for ErrorActualizacion {}

impl From<io::Error> for ErrorActualizacion {
    fn from(error: io::Error) -> Self {
        ErrorActualizacion::Disco(error)
    }
}

impl From<serde_json::Error> for ErrorActualizacion {
    fn from(error: serde_json::Error) -> Self {
        ErrorActualizacion::Json(error)
    }
}

impl From<ureq::Error> for ErrorActualizacion {
    fn from(error: ureq::Error) -> Self {
        // ureq envuelve los fallos de lectura del cuerpo; se dejan como fallos de red.
        ErrorActualizacion::Red(Box::new(error))
    }
}
