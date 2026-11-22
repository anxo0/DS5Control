//! La ventana en primer plano y su proceso, preguntándole a Windows.

use ::windows::core::PWSTR;
use ::windows::Win32::Foundation::{CloseHandle, HWND};
use ::windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
};

use crate::observador::Escritorio;

/// Lo que cabe en una ruta larga de Windows.
const MAXIMO_RUTA: usize = 32 * 1024;

/// El escritorio de verdad.
#[derive(Debug, Default, Clone, Copy)]
pub struct EscritorioWindows;

fn ventana(v: isize) -> HWND {
    HWND(v as *mut core::ffi::c_void)
}

impl Escritorio for EscritorioWindows {
    fn ventana_primer_plano(&mut self) -> Option<isize> {
        let v = unsafe { GetForegroundWindow() };
        (!v.is_invalid()).then_some(v.0 as isize)
    }

    fn proceso_de_ventana(&mut self, v: isize) -> u32 {
        let mut proceso = 0u32;
        unsafe { GetWindowThreadProcessId(ventana(v), Some(&mut proceso)) };
        proceso
    }

    fn ruta_proceso(&mut self, proceso: u32) -> Option<String> {
        let manejador =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, proceso) }.ok()?;
        let mut texto = vec![0u16; MAXIMO_RUTA];
        let mut largo = texto.len() as u32;
        let leido = unsafe {
            QueryFullProcessImageNameW(
                manejador,
                PROCESS_NAME_WIN32,
                PWSTR(texto.as_mut_ptr()),
                &mut largo,
            )
        };
        unsafe {
            let _ = CloseHandle(manejador);
        }
        // Si se abre el proceso pero no se lee la ruta, queda vacía.
        Some(match leido {
            Ok(()) => String::from_utf16_lossy(&texto[..largo as usize]),
            Err(_) => String::new(),
        })
    }

    fn titulo_ventana(&mut self, v: isize) -> String {
        let v = ventana(v);
        let largo = unsafe { GetWindowTextLengthW(v) };
        if largo <= 0 {
            return String::new();
        }
        let mut texto = vec![0u16; largo as usize + 1];
        let leido = unsafe { GetWindowTextW(v, &mut texto) };
        if leido <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&texto[..leido as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_proceso_actual_tiene_ruta() {
        let mut e = EscritorioWindows;
        let ruta = e.ruta_proceso(std::process::id()).unwrap();
        let esperada = std::env::current_exe().unwrap();
        assert_eq!(
            ruta.to_lowercase(),
            esperada.to_string_lossy().to_lowercase()
        );
    }

    #[test]
    fn un_proceso_que_no_existe_no_tiene_ruta() {
        // Los identificadores de proceso son múltiplos de 4: este no puede existir.
        assert_eq!(EscritorioWindows.ruta_proceso(3), None);
    }

    #[test]
    fn la_ventana_cero_no_tiene_titulo_ni_proceso() {
        let mut e = EscritorioWindows;
        assert_eq!(e.titulo_ventana(0), "");
        assert_eq!(e.proceso_de_ventana(0), 0);
    }
}
