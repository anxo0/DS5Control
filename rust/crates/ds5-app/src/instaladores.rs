//! Los instaladores de ViGEmBus y HidHide que van con el programa, como `DriverSetup.cs`: se
//! abren con permisos de administrador desde la carpeta `drivers` y se espera a que acaben.

use std::path::PathBuf;

pub const VIGEM: &str = "ViGEmBus_1.22.0_x64_x86_arm64.exe";
pub const HIDHIDE: &str = "HidHide_1.5.230_x64.exe";
/// `ERROR_SUCCESS_REBOOT_REQUIRED`.
pub const HAY_QUE_REINICIAR: u32 = 3010;

/// Cómo acabó un instalador.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fin {
    /// Acabó con este código de salida.
    Codigo(u32),
    /// El usuario dijo que no a los permisos de administrador.
    Cancelado,
    /// No se pudo abrir.
    Fallo,
}

pub fn ruta(nombre: &str) -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default()
        .join("drivers")
        .join(nombre)
}

#[cfg(windows)]
pub fn ejecutar(ruta: &std::path::Path, argumentos: Option<&str>) -> Fin {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED};
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let verbo = HSTRING::from("runas");
    let archivo = HSTRING::from(ruta.as_os_str());
    let args = argumentos.map(HSTRING::from);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verbo.as_ptr()),
        lpFile: PCWSTR(archivo.as_ptr()),
        lpParameters: args.as_ref().map_or(PCWSTR::null(), |a| PCWSTR(a.as_ptr())),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    if let Err(e) = unsafe { ShellExecuteExW(&mut info) } {
        return if e.code() == ERROR_CANCELLED.to_hresult() {
            Fin::Cancelado
        } else {
            Fin::Fallo
        };
    }
    if info.hProcess.is_invalid() {
        return Fin::Fallo;
    }
    let mut codigo = 0u32;
    unsafe {
        WaitForSingleObject(info.hProcess, INFINITE);
        let _ = GetExitCodeProcess(info.hProcess, &mut codigo);
        let _ = CloseHandle(info.hProcess);
    }
    Fin::Codigo(codigo)
}

#[cfg(not(windows))]
pub fn ejecutar(_ruta: &std::path::Path, _argumentos: Option<&str>) -> Fin {
    Fin::Fallo
}

/// Cómo se llaman en "Aplicaciones instaladas" de Windows.
pub const NOMBRE_VIGEM: &str = "ViGEm Bus Driver";
pub const NOMBRE_HIDHIDE: &str = "HidHide";

/// Parte una orden de desinstalar (`MsiExec.exe /X{...}` o `"C:\a b\x.exe" /uninstall`) en el
/// programa y sus argumentos.
pub fn partir_orden(orden: &str) -> Option<(String, Option<String>)> {
    let orden = orden.trim();
    if orden.is_empty() {
        return None;
    }
    let (programa, resto) = if let Some(sin) = orden.strip_prefix('"') {
        let fin = sin.find('"')?;
        (&sin[..fin], &sin[fin + 1..])
    } else {
        match orden.find(' ') {
            Some(i) => (&orden[..i], &orden[i..]),
            None => (orden, ""),
        }
    };
    let resto = resto.trim();
    Some((
        programa.to_string(),
        (!resto.is_empty()).then(|| resto.to_string()),
    ))
}

/// La orden de desinstalar del programa con ese nombre en "Aplicaciones instaladas", si está.
#[cfg(windows)]
pub fn orden_desinstalar(nombre: &str) -> Option<String> {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_LOCAL_MACHINE,
        KEY_READ, RRF_RT_REG_SZ,
    };

    fn valor(clave: HKEY, sub: &str, nombre: &str) -> Option<String> {
        let mut bufer = vec![0u16; 1024];
        let mut largo = (bufer.len() * 2) as u32;
        let r = unsafe {
            RegGetValueW(
                clave,
                &HSTRING::from(sub),
                &HSTRING::from(nombre),
                RRF_RT_REG_SZ,
                None,
                Some(bufer.as_mut_ptr().cast()),
                Some(&mut largo),
            )
        };
        if r.is_err() {
            return None;
        }
        let n = (largo as usize / 2).saturating_sub(1);
        Some(String::from_utf16_lossy(&bufer[..n.min(bufer.len())]))
    }

    for raiz in [
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ] {
        let mut clave = HKEY::default();
        let abierta = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                &HSTRING::from(raiz),
                None,
                KEY_READ,
                &mut clave,
            )
        };
        if abierta.is_err() {
            continue;
        }
        let mut i = 0;
        let mut hallada = None;
        loop {
            let mut nombre_sub = [0u16; 256];
            let mut largo = nombre_sub.len() as u32;
            let r = unsafe {
                RegEnumKeyExW(
                    clave,
                    i,
                    Some(windows::core::PWSTR(nombre_sub.as_mut_ptr())),
                    &mut largo,
                    None,
                    None,
                    None,
                    None,
                )
            };
            if r.is_err() {
                break;
            }
            i += 1;
            let sub = String::from_utf16_lossy(&nombre_sub[..largo as usize]);
            if valor(clave, &sub, "DisplayName").as_deref() == Some(nombre) {
                hallada = valor(clave, &sub, "UninstallString");
                break;
            }
        }
        unsafe {
            let _ = RegCloseKey(clave);
        }
        let _ = PCWSTR::null();
        if hallada.is_some() {
            return hallada;
        }
    }
    None
}

#[cfg(not(windows))]
pub fn orden_desinstalar(_nombre: &str) -> Option<String> {
    None
}

/// Los textos del registro tras desinstalar.
pub fn mensaje_desinstalar(nombre: &str, fin: Fin, sigue: bool) -> (String, bool) {
    match fin {
        Fin::Cancelado => (format!("Desinstalación de {nombre} cancelada."), true),
        Fin::Codigo(HAY_QUE_REINICIAR) => (
            format!("{nombre} desinstalado. Reinicia el PC para acabar."),
            true,
        ),
        _ if !sigue => (format!("{nombre} desinstalado."), false),
        _ => (format!("{nombre} no se ha desinstalado."), true),
    }
}

/// Los textos del registro tras instalar HidHide (`InstallHidHideAsync`). Devuelve el texto y
/// si es un aviso.
pub fn mensaje_hidhide(fin: Fin, instalado: bool) -> (String, bool) {
    match fin {
        Fin::Cancelado => ("Instalación de HidHide cancelada.".into(), true),
        Fin::Codigo(HAY_QUE_REINICIAR) => (
            "HidHide instalado. Reinicia el PC para que empiece a funcionar.".into(),
            true,
        ),
        _ if instalado => (
            "HidHide instalado. Apaga y enciende el mando para que los juegos dejen de verlo."
                .into(),
            false,
        ),
        _ => ("HidHide no se ha instalado.".into(), true),
    }
}

/// Los textos del registro tras instalar ViGEmBus (`InstallViGEmAsync`).
pub fn mensaje_vigem(fin: Fin, instalado: bool) -> (String, bool) {
    match fin {
        Fin::Cancelado => ("Instalación de ViGEmBus cancelada.".into(), true),
        _ if instalado => ("ViGEmBus instalado.".into(), false),
        _ => (
            "ViGEmBus no se ha instalado. Si lo acabas de instalar, reinicia el PC.".into(),
            true,
        ),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// Solo lee el registro de este PC: `cargo test -p ds5-app -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn busca_los_desinstaladores_en_este_pc() {
        println!("ViGEmBus: {:?}", orden_desinstalar(NOMBRE_VIGEM));
        println!("HidHide: {:?}", orden_desinstalar(NOMBRE_HIDHIDE));
    }

    #[test]
    fn parte_las_ordenes_de_desinstalar() {
        assert_eq!(
            partir_orden("MsiExec.exe /X{966606F3-2745-49E9-BF15-5C3EAA4E9077}"),
            Some((
                "MsiExec.exe".into(),
                Some("/X{966606F3-2745-49E9-BF15-5C3EAA4E9077}".into())
            ))
        );
        assert_eq!(
            partir_orden("\"C:\\Archivos\\x y\\setup.exe\" /uninstall /quiet"),
            Some((
                "C:\\Archivos\\x y\\setup.exe".into(),
                Some("/uninstall /quiet".into())
            ))
        );
        assert_eq!(partir_orden("C:\\a.exe"), Some(("C:\\a.exe".into(), None)));
        assert_eq!(partir_orden("  "), None);
        assert!(mensaje_desinstalar("HidHide", Fin::Codigo(0), false)
            .0
            .contains("desinstalado"));
    }

    #[test]
    fn los_mensajes_son_los_de_c() {
        assert!(mensaje_hidhide(Fin::Cancelado, false).1);
        assert!(mensaje_hidhide(Fin::Codigo(3010), true)
            .0
            .contains("Reinicia el PC"));
        assert!(!mensaje_hidhide(Fin::Codigo(0), true).1);
        assert!(mensaje_vigem(Fin::Codigo(0), false).1);
        assert!(ruta(VIGEM).ends_with(r"drivers\ViGEmBus_1.22.0_x64_x86_arm64.exe"));
    }
}
