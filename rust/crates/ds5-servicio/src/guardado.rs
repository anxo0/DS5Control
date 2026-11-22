//! Escribir en la carpeta de ajustes sin riesgo: antes de la primera escritura de cada sesión se
//! hace una copia de seguridad de la carpeta entera al lado (`<carpeta>-copia-rust-<fecha>`), y cada
//! archivo se escribe primero aparte y luego se cambia por el bueno, para que un corte no lo deje a
//! medias.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ds5_perfiles::Encabezado;

/// Las subcarpetas que no se copian: se pueden volver a generar y pueden pesar mucho.
const SIN_COPIA: [&str; 2] = ["Portadas", "Logs"];

#[derive(Debug)]
pub struct Guardado {
    carpeta: PathBuf,
    /// Dónde quedó la copia de esta sesión, si ya se ha hecho.
    copia: Mutex<Option<PathBuf>>,
}

fn copiar_carpeta(origen: &Path, destino: &Path) -> io::Result<()> {
    fs::create_dir_all(destino)?;
    for e in fs::read_dir(origen)? {
        let e = e?;
        let ruta = e.path();
        let nombre = e.file_name();
        if e.file_type()?.is_dir() {
            if SIN_COPIA.iter().any(|s| nombre.eq_ignore_ascii_case(s)) {
                continue;
            }
            copiar_carpeta(&ruta, &destino.join(&nombre))?;
        } else {
            fs::copy(&ruta, destino.join(&nombre))?;
        }
    }
    Ok(())
}

/// El encabezado de los archivos que guarda la app, con la fecha como la escribe C#: mes/día/año.
pub fn encabezado() -> Encabezado {
    Encabezado {
        fecha: chrono::Local::now().format("%m/%d/%Y %H:%M:%S").to_string(),
        version_app: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// La fecha como `DateTime.Now.ToString()` en la cultura española: día/mes/año
/// (`Actions.xml`).
pub fn fecha_espanola() -> String {
    chrono::Local::now().format("%d/%m/%Y %H:%M:%S").to_string()
}

/// La fecha en la cultura invariante de .NET: mes/día/año (`Auto Profiles.xml`, perfiles).
pub fn fecha_invariante() -> String {
    chrono::Local::now().format("%m/%d/%Y %H:%M:%S").to_string()
}

/// Mete el comentario de C# tras la declaración XML, como hace `XmlDocument` al guardar:
/// `<?xml ...?>`, el comentario y una línea en blanco.
pub fn con_comentario(texto: &str, comentario: &str) -> String {
    match texto.find("?>\r\n") {
        Some(i) => {
            let (decl, resto) = texto.split_at(i + 4);
            let resto = resto.strip_prefix("\r\n").unwrap_or(resto);
            format!("{decl}<!-- {comentario} -->\r\n\r\n{resto}")
        }
        None => texto.to_string(),
    }
}

impl Guardado {
    pub fn new(carpeta: impl Into<PathBuf>) -> Guardado {
        Guardado {
            carpeta: carpeta.into(),
            copia: Mutex::new(None),
        }
    }

    pub fn copia(&self) -> Option<PathBuf> {
        self.copia.lock().unwrap().clone()
    }

    /// Hace la copia de seguridad si aún no se ha hecho en esta sesión.
    fn asegurar_copia(&self) -> io::Result<()> {
        let mut copia = self.copia.lock().unwrap();
        if copia.is_some() || !self.carpeta.is_dir() {
            return Ok(());
        }
        let nombre = format!(
            "{}-copia-rust-{}",
            self.carpeta
                .file_name()
                .map_or("ajustes".into(), |n| n.to_string_lossy()),
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        );
        let destino = self.carpeta.parent().unwrap_or(&self.carpeta).join(nombre);
        copiar_carpeta(&self.carpeta, &destino)?;
        *copia = Some(destino);
        Ok(())
    }

    /// Escribe `contenido` en `relativo` (dentro de la carpeta de ajustes). Si la copia de
    /// seguridad falla no escribe nada.
    pub fn escribir(&self, relativo: impl AsRef<Path>, contenido: &str) -> io::Result<()> {
        self.asegurar_copia()?;
        let ruta = self.carpeta.join(relativo);
        if let Some(p) = ruta.parent() {
            fs::create_dir_all(p)?;
        }
        let temporal = ruta.with_extension("guardando");
        fs::write(&temporal, contenido)?;
        fs::rename(&temporal, &ruta)
    }

    /// Copia un archivo dentro de la carpeta (para un perfil nuevo a partir de otro).
    pub fn copiar(&self, de: impl AsRef<Path>, a: impl AsRef<Path>) -> io::Result<()> {
        self.asegurar_copia()?;
        fs::copy(self.carpeta.join(de), self.carpeta.join(a)).map(|_| ())
    }

    /// Borra un archivo de la carpeta (queda en la copia de seguridad).
    pub fn borrar(&self, relativo: impl AsRef<Path>) -> io::Result<()> {
        self.asegurar_copia()?;
        fs::remove_file(self.carpeta.join(relativo))
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::ajustes::carpeta_de_prueba;

    #[test]
    fn copia_una_vez_antes_de_escribir() {
        let raiz = carpeta_de_prueba("guardado");
        let carpeta = raiz.join("DS5Control");
        fs::create_dir_all(carpeta.join("Profiles")).unwrap();
        fs::create_dir_all(carpeta.join("Portadas")).unwrap();
        fs::write(carpeta.join("Profiles").join("Default.xml"), "viejo").unwrap();
        fs::write(carpeta.join("Portadas").join("x.jpg"), "x").unwrap();

        let g = Guardado::new(&carpeta);
        g.escribir("Profiles/Default.xml", "nuevo").unwrap();
        let copia = g.copia().expect("tiene que haber copia");
        assert_eq!(
            fs::read_to_string(copia.join("Profiles").join("Default.xml")).unwrap(),
            "viejo"
        );
        assert!(!copia.join("Portadas").exists());
        assert_eq!(
            fs::read_to_string(carpeta.join("Profiles").join("Default.xml")).unwrap(),
            "nuevo"
        );
        // La segunda escritura no hace otra copia.
        g.escribir("Profiles/Default.xml", "otra").unwrap();
        assert_eq!(g.copia(), Some(copia.clone()));
        assert_eq!(
            fs::read_to_string(copia.join("Profiles").join("Default.xml")).unwrap(),
            "viejo"
        );
        assert!(!carpeta.join("Profiles").join("Default.guardando").exists());
        let _ = fs::remove_dir_all(&raiz);
    }

    #[test]
    fn el_comentario_va_tras_la_declaracion() {
        let t = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n\r\n<Actions>\r\n</Actions>";
        assert_eq!(
            con_comentario(t, "Special Actions Configuration Data. 26/08/2026 13:58:56"),
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<!-- Special Actions Configuration Data. 26/08/2026 13:58:56 -->\r\n\r\n<Actions>\r\n</Actions>"
        );
    }

    #[test]
    fn la_fecha_va_como_en_c() {
        let e = encabezado();
        assert_eq!(e.fecha.len(), 19);
        assert_eq!(&e.fecha[2..3], "/");
    }
}
