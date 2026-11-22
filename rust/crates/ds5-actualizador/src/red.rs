//! La parte en red: pedir la última release a GitHub y descargar el zip.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::archivos::{
    carpeta_trabajo_por_defecto, preparar_carpeta_trabajo, verificar_y_descomprimir,
};
use crate::error::ErrorActualizacion;
use crate::release::{leer_release, Actualizacion, URL_ULTIMA_RELEASE};
use crate::version::Version;

/// El agente con el que se pide a GitHub, que no responde sin `User-Agent`.
pub const AGENTE_USUARIO: &str = "DS5Control";

/// El tiempo máximo de cada petición, el de `HttpClient` por defecto.
pub const ESPERA_MAXIMA: Duration = Duration::from_secs(100);

/// Un agente HTTP como el `HttpClient` de C#: con `User-Agent: DS5Control`, siguiendo las
/// redirecciones (las descargas de GitHub redirigen) y dando error con las respuestas 4xx y 5xx.
pub fn nuevo_agente() -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent(AGENTE_USUARIO)
        .build()
        .into()
}

/// Pide a GitHub la última release y la devuelve si es más nueva que `actual`.
pub fn buscar(
    agente: &ureq::Agent,
    actual: &Version,
) -> Result<Option<Actualizacion>, ErrorActualizacion> {
    buscar_en(agente, URL_ULTIMA_RELEASE, actual)
}

/// Como [`buscar`], pero pidiendo a otra dirección (para las pruebas).
pub fn buscar_en(
    agente: &ureq::Agent,
    url: &str,
    actual: &Version,
) -> Result<Option<Actualizacion>, ErrorActualizacion> {
    let mut respuesta = agente
        .get(url)
        .config()
        .timeout_global(Some(ESPERA_MAXIMA))
        .build()
        .call()?;
    let texto = respuesta
        .body_mut()
        .with_config()
        .limit(u64::MAX)
        .lossy_utf8(true)
        .read_to_string()?;
    // `GetStringAsync` quita la marca de UTF-8 del principio.
    leer_release(texto.strip_prefix('\u{feff}').unwrap_or(&texto), actual)
}

/// Descarga y descomprime una release. Devuelve la carpeta que tiene los archivos nuevos.
/// Sin `carpeta_trabajo`, usa `%TEMP%\DS5Control-actualizacion`, que se vacía antes.
pub fn descargar(
    agente: &ureq::Agent,
    actualizacion: &Actualizacion,
    carpeta_trabajo: Option<&Path>,
) -> Result<PathBuf, ErrorActualizacion> {
    let carpeta = carpeta_trabajo
        .map(Path::to_path_buf)
        .unwrap_or_else(carpeta_trabajo_por_defecto);
    let zip = preparar_carpeta_trabajo(&carpeta)?;

    {
        let mut archivo = fs::File::create(&zip)?;
        // Como `GetStreamAsync`, el límite de tiempo es hasta tener la respuesta, no para
        // todo el cuerpo, que puede tardar.
        let respuesta = agente
            .get(&actualizacion.url_descarga)
            .config()
            .timeout_connect(Some(ESPERA_MAXIMA))
            .timeout_recv_response(Some(ESPERA_MAXIMA))
            .build()
            .call()?;
        let mut cuerpo = respuesta.into_body().into_reader();
        io::copy(&mut cuerpo, &mut archivo)?;
    }

    verificar_y_descomprimir(&carpeta, &actualizacion.sha256)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::archivos::pruebas::{huella, zip_con, Temporal};
    use crate::release::PROGRAMA;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread::JoinHandle;

    /// Un servidor en 127.0.0.1 que responde una sola vez con `cuerpo`, como el `HttpListener`
    /// de las pruebas en C#. Devuelve la dirección y lo que pidió el cliente.
    fn servidor(estado: u16, cuerpo: Vec<u8>) -> (String, JoinHandle<String>) {
        let escucha = TcpListener::bind("127.0.0.1:0").unwrap();
        let direccion = format!("http://{}/a.zip", escucha.local_addr().unwrap());
        let hilo = std::thread::spawn(move || {
            let (conexion, _) = escucha.accept().unwrap();
            let mut lector = BufReader::new(conexion.try_clone().unwrap());
            let mut peticion = String::new();
            loop {
                let mut linea = String::new();
                if lector.read_line(&mut linea).unwrap() == 0 || linea == "\r\n" {
                    break;
                }
                peticion.push_str(&linea);
            }
            let mut conexion = conexion;
            write!(
                conexion,
                "HTTP/1.1 {estado} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                cuerpo.len()
            )
            .unwrap();
            conexion.write_all(&cuerpo).unwrap();
            peticion
        });
        (direccion, hilo)
    }

    fn descargar_de(contenido: Vec<u8>, sha256: &str) -> Result<PathBuf, ErrorActualizacion> {
        let (url, hilo) = servidor(200, contenido);
        let trabajo = Temporal::nueva();
        let actualizacion = Actualizacion {
            version: Version::analizar("9.0").unwrap(),
            url_descarga: url,
            sha256: sha256.to_string(),
        };
        let resultado = descargar(&nuevo_agente(), &actualizacion, Some(&trabajo.0));
        hilo.join().unwrap();
        // Se comprueba aquí, antes de que se borre la carpeta temporal.
        resultado.map(|carpeta| {
            assert!(carpeta.join(PROGRAMA).is_file());
            carpeta.strip_prefix(&trabajo.0).unwrap().to_path_buf()
        })
    }

    #[test]
    fn descomprime_la_descarga_comprobada() {
        let zip = zip_con("DS5Control-0.2.0/DS5Control.exe");
        let sha = huella(&zip);

        let carpeta = descargar_de(zip, &sha).unwrap();

        assert_eq!(carpeta, Path::new("nueva").join("DS5Control-0.2.0"));
    }

    #[test]
    fn rechaza_una_descarga_corrupta() {
        let error = descargar_de(zip_con("DS5Control.exe"), "00").unwrap_err();
        assert!(matches!(error, ErrorActualizacion::HuellaDistinta));
    }

    #[test]
    fn rechaza_una_release_sin_programa() {
        let zip = zip_con("otro.exe");
        let sha = huella(&zip);
        let error = descargar_de(zip, &sha).unwrap_err();
        assert!(matches!(error, ErrorActualizacion::SinPrograma));
    }

    #[test]
    fn una_respuesta_de_error_es_un_fallo_de_red() {
        let (url, hilo) = servidor(404, Vec::new());
        let error = buscar_en(&nuevo_agente(), &url, &Version::nueva(0, 1, 0)).unwrap_err();
        hilo.join().unwrap();
        assert!(matches!(error, ErrorActualizacion::Red(_)));
    }

    #[test]
    fn busca_la_release_y_se_presenta_como_ds5control() {
        let json = "\u{feff}{\"tag_name\": \"v0.2.0\", \"assets\": [{\"name\": \"DS5Control-0.2.0-win-x64.zip\", \"digest\": \"sha256:abc\", \"browser_download_url\": \"https://github.com/anxo0/DS5Control/releases/download/v0.2.0/DS5Control-0.2.0-win-x64.zip\"}]}";
        let (url, hilo) = servidor(200, json.as_bytes().to_vec());

        let actualizacion = buscar_en(&nuevo_agente(), &url, &Version::nueva(0, 1, 0))
            .unwrap()
            .unwrap();

        assert_eq!(actualizacion.version, Version::nueva(0, 2, 0));
        let peticion = hilo.join().unwrap().to_ascii_lowercase();
        assert!(
            peticion.contains("user-agent: ds5control\r\n"),
            "{peticion}"
        );
    }
}
