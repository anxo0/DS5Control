//! Lo que hace la sección de juegos con la lista, como `AutoProfilesPanel.cs`: cada juego pone el
//! mismo perfil a todos los mandos. Guardar el archivo le toca a quien llama.

use ds5_perfiles::otros::Autoperfil;
use ds5_perfiles::Autoperfiles;

use crate::regla::NINGUNO;

/// Un juego de la lista, como lo describe `Describe`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Juego {
    pub ruta: String,
    /// El nombre del ejecutable sin la extensión.
    pub nombre: String,
    /// El perfil del primer mando (vacío si es `(none)`).
    pub perfil: String,
}

/// Como `Path.GetFileNameWithoutExtension` en Windows.
pub fn nombre_sin_extension(ruta: &str) -> &str {
    // `C:juego.exe`: la unidad cuenta como raíz.
    let raiz = match ruta.as_bytes() {
        [l, b':', ..] if l.is_ascii_alphabetic() => 2,
        _ => 0,
    };
    let inicio = ruta[raiz..]
        .rfind(['\\', '/'])
        .map(|i| raiz + i + 1)
        .unwrap_or(raiz);
    let nombre = &ruta[inicio..];
    match nombre.rfind('.') {
        Some(i) => &nombre[..i],
        None => nombre,
    }
}

pub fn describir(lista: &Autoperfiles) -> Vec<Juego> {
    lista
        .programas
        .iter()
        .map(|p| Juego {
            ruta: p.ruta.clone(),
            nombre: nombre_sin_extension(&p.ruta).to_string(),
            perfil: if p.perfiles[0] == NINGUNO {
                String::new()
            } else {
                p.perfiles[0].clone()
            },
        })
        .collect()
}

fn asignar(p: &mut Autoperfil, perfil: &str) {
    for n in p.perfiles.iter_mut() {
        *n = perfil.to_string();
    }
}

/// Añade un juego. Devuelve `false` (y no hay que guardar) si ya estaba esa ruta exacta.
pub fn anadir(lista: &mut Autoperfiles, ruta: &str, perfil: &str) -> bool {
    if lista.programas.iter().any(|p| p.ruta == ruta) {
        return false;
    }
    let mut p = Autoperfil {
        ruta: ruta.to_string(),
        ..Default::default()
    };
    asignar(&mut p, perfil);
    lista.programas.push(p);
    true
}

/// Cambia el perfil del primer juego con esa ruta. Devuelve `false` si no está.
pub fn poner_perfil(lista: &mut Autoperfiles, ruta: &str, perfil: &str) -> bool {
    match lista.programas.iter_mut().find(|p| p.ruta == ruta) {
        Some(p) => {
            asignar(p, perfil);
            true
        }
        None => false,
    }
}

/// Quita el primer juego con esa ruta. Devuelve `false` si no está.
pub fn quitar(lista: &mut Autoperfiles, ruta: &str) -> bool {
    match lista.programas.iter().position(|p| p.ruta == ruta) {
        Some(i) => {
            lista.programas.remove(i);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombres() {
        assert_eq!(nombre_sin_extension(r"C:\Juegos\Mi.Juego.exe"), "Mi.Juego");
        assert_eq!(nombre_sin_extension("C:/Juegos/juego.exe"), "juego");
        assert_eq!(nombre_sin_extension("C:juego.exe"), "juego");
        assert_eq!(nombre_sin_extension("juego"), "juego");
        assert_eq!(nombre_sin_extension(r"C:\Juegos\"), "");
        assert_eq!(nombre_sin_extension(""), "");
        assert_eq!(nombre_sin_extension(r"C:\a\.exe"), "");
    }

    #[test]
    fn anadir_cambiar_y_quitar() {
        let mut l = Autoperfiles::default();
        assert!(anadir(&mut l, r"C:\A.exe", "Carreras"));
        // La misma ruta exacta no se repite; con otras mayúsculas sí, como en C#.
        assert!(!anadir(&mut l, r"C:\A.exe", "Otro"));
        assert!(anadir(&mut l, r"c:\a.exe", "Otro"));
        let p = &l.programas[0];
        assert!(p.perfiles.iter().all(|n| n == "Carreras"));
        assert_eq!(p.titulo, "");
        assert!(!p.apagar);

        assert!(poner_perfil(&mut l, r"C:\A.exe", NINGUNO));
        assert!(!poner_perfil(&mut l, r"C:\B.exe", "X"));
        assert_eq!(
            describir(&l),
            vec![
                Juego {
                    ruta: r"C:\A.exe".into(),
                    nombre: "A".into(),
                    perfil: String::new(),
                },
                Juego {
                    ruta: r"c:\a.exe".into(),
                    nombre: "a".into(),
                    perfil: "Otro".into(),
                },
            ]
        );

        assert!(quitar(&mut l, r"C:\A.exe"));
        assert!(!quitar(&mut l, r"C:\A.exe"));
        assert_eq!(l.programas.len(), 1);
    }
}
