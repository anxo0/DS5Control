//! Abrir un programa desde una acción especial, como la parte de `LaunchProgram` de
//! `Mapping.MapCustomAction`: con `UseShellExecute`, y con `$hidden` en los argumentos la ventana
//! va oculta (y los `.bat` y `.cmd` se abren con el intérprete de `COMSPEC`).

/// Lo que se le pasa a `ShellExecute`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lanzamiento {
    pub archivo: String,
    pub argumentos: Option<String>,
    pub oculto: bool,
}

/// La posición de `$hidden` sin distinguir mayúsculas (`OrdinalIgnoreCase`).
fn posicion_oculto(texto: &str) -> Option<usize> {
    texto.to_ascii_lowercase().find("$hidden")
}

pub fn preparar(ruta: &str, argumentos: Option<&str>, comspec: Option<&str>) -> Lanzamiento {
    let Some(extra) = argumentos.filter(|a| !a.is_empty()) else {
        return Lanzamiento {
            archivo: ruta.to_string(),
            argumentos: None,
            oculto: false,
        };
    };
    let Some(pos) = posicion_oculto(extra) else {
        return Lanzamiento {
            archivo: ruta.to_string(),
            argumentos: Some(extra.to_string()),
            oculto: false,
        };
    };
    // Se quita la primera aparición de `$hidden`, que solo era el modificador.
    let resto = format!("{}{}", &extra[..pos], &extra[pos + 7..]);
    let extension = std::path::Path::new(ruta)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase());
    let (archivo, argumentos) = match extension.as_deref() {
        Some("bat") | Some("cmd") => (
            comspec.unwrap_or_default().to_string(),
            format!("/C \"{ruta}\" {resto}"),
        ),
        _ => (ruta.to_string(), resto),
    };
    Lanzamiento {
        archivo,
        argumentos: Some(argumentos),
        oculto: true,
    }
}

/// Lo abre. Devuelve si Windows lo ha aceptado.
pub fn abrir(l: &Lanzamiento) -> bool {
    so::abrir(l)
}

#[cfg(windows)]
mod so {
    use super::Lanzamiento;
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNORMAL};

    pub fn abrir(l: &Lanzamiento) -> bool {
        let archivo = HSTRING::from(l.archivo.as_str());
        let argumentos = l.argumentos.as_deref().map(HSTRING::from);
        let r = unsafe {
            ShellExecuteW(
                None,
                PCWSTR::null(),
                &archivo,
                argumentos
                    .as_ref()
                    .map_or(PCWSTR::null(), |a| PCWSTR(a.as_ptr())),
                PCWSTR::null(),
                if l.oculto { SW_HIDE } else { SW_SHOWNORMAL },
            )
        };
        // Por encima de 32 es que ha ido bien.
        r.0 as isize > 32
    }
}

#[cfg(not(windows))]
mod so {
    pub fn abrir(_l: &super::Lanzamiento) -> bool {
        false
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn sin_argumentos_ni_oculto() {
        assert_eq!(
            preparar("C:\\a.exe", None, None),
            Lanzamiento {
                archivo: "C:\\a.exe".into(),
                argumentos: None,
                oculto: false
            }
        );
        assert_eq!(preparar("C:\\a.exe", Some(""), None).argumentos, None);
        assert_eq!(
            preparar("C:\\a.exe", Some("-x"), None)
                .argumentos
                .as_deref(),
            Some("-x")
        );
    }

    #[test]
    fn hidden_se_quita_una_vez_y_sin_distinguir_mayusculas() {
        let l = preparar("C:\\a.exe", Some("-x $HIDDEN -y $hidden"), None);
        assert!(l.oculto);
        assert_eq!(l.argumentos.as_deref(), Some("-x  -y $hidden"));
    }

    #[test]
    fn los_bat_van_con_comspec() {
        let l = preparar("C:\\s.BAT", Some("$hidden uno"), Some("C:\\cmd.exe"));
        assert_eq!(l.archivo, "C:\\cmd.exe");
        assert_eq!(l.argumentos.as_deref(), Some("/C \"C:\\s.BAT\"  uno"));
    }
}
