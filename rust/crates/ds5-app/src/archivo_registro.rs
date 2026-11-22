//! El registro en archivo, como `LoggerHolder` con la configuración de `NLog.config`:
//! `Logs\ds5control_log.txt` en la carpeta de ajustes, una línea `fecha|NIVEL|texto` por
//! mensaje, y al arrancar el archivo anterior pasa a `ds5control_log_<día>.<n>.txt`, guardando
//! los siete últimos.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const NOMBRE: &str = "ds5control_log";
/// `maxArchiveFiles`.
const ARCHIVADOS: usize = 7;

pub struct ArchivoRegistro {
    archivo: Mutex<Option<File>>,
}

impl ArchivoRegistro {
    /// Archiva el registro anterior y abre uno nuevo. Si no se puede, el registro solo queda en
    /// la ventana.
    pub fn abrir(carpeta_ajustes: &Path) -> ArchivoRegistro {
        let carpeta = carpeta_ajustes.join("Logs");
        let archivo = fs::create_dir_all(&carpeta).ok().and_then(|_| {
            archivar(&carpeta);
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(carpeta.join(format!("{NOMBRE}.txt")))
                .ok()
        });
        ArchivoRegistro {
            archivo: Mutex::new(archivo),
        }
    }

    /// `logger.Info` o `logger.Warn`.
    pub fn escribir(&self, texto: &str, aviso: bool) {
        if let Some(f) = self.archivo.lock().unwrap().as_mut() {
            let _ = f.write_all(linea(&chrono::Local::now(), texto, aviso).as_bytes());
        }
    }
}

/// `${longdate}|${level:uppercase=true}|${message}`.
fn linea<Z: chrono::TimeZone>(fecha: &chrono::DateTime<Z>, texto: &str, aviso: bool) -> String
where
    Z::Offset: std::fmt::Display,
{
    let nivel = if aviso { "WARN" } else { "INFO" };
    // Cuatro decimales de segundo, que chrono no tiene como formato.
    let diezmilesimas = fecha.timestamp_subsec_nanos() / 100_000;
    format!(
        "{}.{diezmilesimas:04}|{nivel}|{texto}\r\n",
        fecha.format("%Y-%m-%d %H:%M:%S")
    )
}

/// `archiveOldFileOnStartup` con `archiveNumbering="DateAndSequence"`: el archivo anterior se
/// renombra con el día en que se escribió y el siguiente número libre de ese día, y se borran los
/// archivados más viejos.
fn archivar(carpeta: &Path) {
    let actual = carpeta.join(format!("{NOMBRE}.txt"));
    let Ok(meta) = fs::metadata(&actual) else {
        return;
    };
    if meta.len() > 0 {
        let dia = meta
            .modified()
            .map(chrono::DateTime::<chrono::Local>::from)
            .unwrap_or_else(|_| chrono::Local::now())
            .format("%Y%m%d")
            .to_string();
        let destino = (0..)
            .map(|n| carpeta.join(format!("{NOMBRE}_{dia}.{n}.txt")))
            .find(|r| !r.exists())
            .expect("siempre hay un número libre");
        let _ = fs::rename(&actual, destino);
    }
    for viejo in sobrantes(carpeta) {
        let _ = fs::remove_file(viejo);
    }
}

/// Los archivados que pasan de los siete más nuevos. Los nombres ordenan por fecha y número.
fn sobrantes(carpeta: &Path) -> Vec<PathBuf> {
    let prefijo = format!("{NOMBRE}_");
    let mut archivados: Vec<(u64, u32, PathBuf)> = fs::read_dir(carpeta)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let nombre = e.file_name().into_string().ok()?;
            let resto = nombre.strip_prefix(&prefijo)?.strip_suffix(".txt")?;
            let (dia, n) = resto.split_once('.')?;
            Some((dia.parse().ok()?, n.parse().ok()?, e.path()))
        })
        .collect();
    archivados.sort();
    let sobran = archivados.len().saturating_sub(ARCHIVADOS);
    archivados
        .into_iter()
        .take(sobran)
        .map(|(_, _, r)| r)
        .collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn la_linea_como_nlog() {
        let f = chrono::Utc.with_ymd_and_hms(2026, 11, 22, 9, 5, 7).unwrap();
        assert_eq!(
            linea(&f, "Mando 1 conectado", false),
            "2026-11-22 09:05:07.0000|INFO|Mando 1 conectado\r\n"
        );
        assert_eq!(
            linea(&f, "Falta ViGEmBus", true),
            "2026-11-22 09:05:07.0000|WARN|Falta ViGEmBus\r\n"
        );
    }

    #[test]
    fn archiva_y_guarda_siete() {
        let carpeta =
            std::env::temp_dir().join(format!("ds5-prueba-registro-{}", std::process::id()));
        let _ = fs::remove_dir_all(&carpeta);
        fs::create_dir_all(&carpeta).unwrap();
        for dia in 1..=9 {
            fs::write(carpeta.join(format!("{NOMBRE}_202611{dia:02}.0.txt")), "x").unwrap();
        }
        fs::write(carpeta.join("ds5windows_log.txt"), "de DS4Windows").unwrap();
        fs::write(carpeta.join(format!("{NOMBRE}.txt")), "anterior").unwrap();
        archivar(&carpeta);
        let mut nombres: Vec<String> = fs::read_dir(&carpeta)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        nombres.sort();
        assert!(!nombres.contains(&format!("{NOMBRE}.txt")));
        assert!(nombres.contains(&"ds5windows_log.txt".to_string()));
        assert_eq!(
            nombres
                .iter()
                .filter(|n| n.starts_with(&format!("{NOMBRE}_")))
                .count(),
            ARCHIVADOS
        );
        assert!(!nombres.contains(&format!("{NOMBRE}_20261101.0.txt")));
        let _ = fs::remove_dir_all(carpeta);
    }
}
