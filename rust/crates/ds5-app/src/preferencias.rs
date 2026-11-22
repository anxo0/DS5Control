//! Las preferencias propias de la interfaz (`InterfazWeb.txt`), con el mismo formato que
//! `ReadInterfaceSettings` y `WriteInterfaceSettings`: una línea `clave=valor` por cada una.
//!
//! Se guarda con `ds5_servicio::Guardado`, que hace antes la copia de seguridad.

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preferencias {
    pub abrir_con_mute: bool,
    pub control_con_mando: bool,
    pub hidhide_ofrecido: bool,
}

pub const ARCHIVO: &str = "InterfazWeb.txt";

impl Preferencias {
    /// Sin archivo, abrir con Mute está activado y lo demás no.
    pub fn desde_texto(texto: &str) -> Preferencias {
        let tiene = |l: &str| texto.lines().any(|x| x == l);
        Preferencias {
            abrir_con_mute: !tiene("abrirConMute=false"),
            control_con_mando: tiene("controlConMando=true"),
            hidhide_ofrecido: tiene("hidHideOfrecido=true"),
        }
    }

    pub fn leer(carpeta: &Path) -> Preferencias {
        Self::desde_texto(&std::fs::read_to_string(carpeta.join(ARCHIVO)).unwrap_or_default())
    }

    pub fn texto(&self) -> String {
        let b = |v: bool| if v { "true" } else { "false" };
        format!(
            "abrirConMute={}\r\ncontrolConMando={}\r\nhidHideOfrecido={}\r\n",
            b(self.abrir_con_mute),
            b(self.control_con_mando),
            b(self.hidhide_ofrecido)
        )
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn sin_archivo_abre_con_mute() {
        let p = Preferencias::desde_texto("");
        assert!(p.abrir_con_mute && !p.control_con_mando && !p.hidhide_ofrecido);
    }

    #[test]
    fn ida_y_vuelta() {
        let p = Preferencias {
            abrir_con_mute: false,
            control_con_mando: true,
            hidhide_ofrecido: true,
        };
        assert_eq!(Preferencias::desde_texto(&p.texto()), p);
    }
}
