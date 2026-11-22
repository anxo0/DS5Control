//! Lo que el servicio lee de la carpeta de ajustes: el perfil de cada mando y las acciones
//! especiales. Solo lee: guardar cambios llega más adelante, con copia de seguridad.

use std::fs;
use std::path::PathBuf;

use ds5_perfiles::{Acciones, AjustesApp, Perfil};

/// El perfil que se usa si el mando no tiene ninguno elegido.
pub const PERFIL_POR_DEFECTO: &str = "Default";

/// La carpeta de ajustes y lo que hay en ella.
#[derive(Debug, Clone)]
pub struct Carpeta {
    pub ruta: PathBuf,
}

impl Carpeta {
    pub fn new(ruta: impl Into<PathBuf>) -> Carpeta {
        Carpeta { ruta: ruta.into() }
    }

    fn leer(&self, nombre: &str) -> Option<String> {
        fs::read_to_string(self.ruta.join(nombre)).ok()
    }

    /// `Profiles.xml`, o los valores por defecto si no está o no se entiende.
    pub fn ajustes(&self) -> AjustesApp {
        self.leer("Profiles.xml")
            .and_then(|t| AjustesApp::leer(&t).ok())
            .unwrap_or_default()
    }

    pub fn acciones(&self) -> Acciones {
        self.leer("Actions.xml")
            .and_then(|t| Acciones::leer(&t).ok())
            .unwrap_or_default()
    }

    pub fn ruta_perfil(&self, nombre: &str) -> PathBuf {
        self.ruta.join("Profiles").join(format!("{nombre}.xml"))
    }

    /// Los nombres de los perfiles guardados, en orden alfabético sin distinguir mayúsculas.
    pub fn perfiles(&self) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(self.ruta.join("Profiles"))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                (p.extension().is_some_and(|x| x.eq_ignore_ascii_case("xml")))
                    .then(|| p.file_stem()?.to_str().map(str::to_string))
                    .flatten()
            })
            .collect();
        v.sort_by_key(|n| n.to_lowercase());
        v
    }

    /// El perfil elegido para la ranura, cargado. Si no hay ninguno o no se puede leer, el perfil
    /// por defecto; y si tampoco, los valores por defecto (como C#, que se queda con ellos al
    /// fallar la carga).
    pub fn perfil_de(&self, ranura: usize) -> (String, Perfil) {
        let elegido = self
            .ajustes()
            .perfiles
            .get(ranura)
            .filter(|n| !n.is_empty())
            .cloned()
            .unwrap_or_else(|| PERFIL_POR_DEFECTO.to_string());
        let perfil = self.cargar(&elegido).unwrap_or_default();
        (elegido, perfil)
    }

    pub fn cargar(&self, nombre: &str) -> Option<Perfil> {
        let texto = fs::read_to_string(self.ruta_perfil(nombre)).ok()?;
        Perfil::leer(&texto).ok()
    }
}

/// Las rutas de prueba se crean en una carpeta temporal propia.
#[cfg(test)]
pub(crate) fn carpeta_de_prueba(nombre: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("ds5-servicio-{nombre}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(p.join("Profiles")).unwrap();
    p
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn sin_ajustes_usa_el_perfil_por_defecto() {
        let c = Carpeta::new(carpeta_de_prueba("vacia"));
        let (nombre, perfil) = c.perfil_de(0);
        assert_eq!(nombre, PERFIL_POR_DEFECTO);
        assert_eq!(perfil, Perfil::default());
        assert!(c.perfiles().is_empty());
    }

    #[test]
    fn lista_los_perfiles_sin_distinguir_mayusculas() {
        let r = carpeta_de_prueba("lista");
        for n in ["zeta", "Alfa", "beta"] {
            fs::write(r.join("Profiles").join(format!("{n}.xml")), "x").unwrap();
        }
        fs::write(r.join("Profiles").join("nota.txt"), "x").unwrap();
        assert_eq!(Carpeta::new(&r).perfiles(), ["Alfa", "beta", "zeta"]);
    }
}
