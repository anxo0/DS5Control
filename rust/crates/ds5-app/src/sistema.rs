//! Lo que las opciones generales leen o cambian del sistema: el acceso directo de la carpeta Inicio
//! (`StartupMethods.cs`) y la prioridad del proceso (`GeneralOptions.SetPriority`).

use std::path::{Path, PathBuf};

/// `StartupMethods.lnkpath`: `DS5Control.lnk` en la carpeta Inicio del usuario.
fn acceso_directo() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| {
        PathBuf::from(a).join(r"Microsoft\Windows\Start Menu\Programs\Startup\DS5Control.lnk")
    })
}

/// `HasStartProgEntry`.
pub fn inicio_windows() -> bool {
    acceso_directo().is_some_and(|r| r.is_file())
}

/// `WriteStartProgEntry` y `DeleteStartProgEntry`: el acceso directo arranca este programa
/// minimizado (`-m`) desde su carpeta. Un acceso de solo lectura no se borra, como en C#.
pub fn poner_inicio_windows(activar: bool) -> std::io::Result<()> {
    let ruta = acceso_directo().ok_or_else(|| std::io::Error::other("APPDATA"))?;
    if activar {
        let exe = std::env::current_exe()?;
        so::escribir_acceso(&ruta, &exe, "-m", None)
    } else if ruta.is_file() && !std::fs::metadata(&ruta)?.permissions().readonly() {
        std::fs::remove_file(ruta)
    } else {
        Ok(())
    }
}

/// `UpdateStartMenuIcon`: Windows enseña en la barra de tareas el icono del acceso directo del menú
/// Inicio desde el que se abrió el programa. Se crea o se rehace para que abra este programa con el
/// icono de la paleta elegida, también en la versión portable.
pub fn icono_menu_inicio(icono: &Path) -> std::io::Result<()> {
    let ruta = std::env::var_os("APPDATA")
        .map(|a| PathBuf::from(a).join(r"Microsoft\Windows\Start Menu\Programs\DS5Control.lnk"))
        .ok_or_else(|| std::io::Error::other("APPDATA"))?;
    let exe = std::env::current_exe()?;
    so::escribir_acceso(&ruta, &exe, "", Some(icono))
}

/// Como `CheckStartupExeLocation` al arrancar: si el acceso directo de Inicio existe, se vuelve a
/// escribir para que apunte a este programa (por si la carpeta cambió o venía de la versión en C#).
/// Una compilación de desarrollo (en `target`) no lo toca, para no robarle el acceso al programa
/// instalado.
pub fn reparar_inicio_windows() {
    let desarrollo = std::env::current_exe().is_ok_and(|e| {
        e.components()
            .any(|c| c.as_os_str().eq_ignore_ascii_case("target"))
    });
    if inicio_windows() && !desarrollo {
        let _ = poner_inicio_windows(true);
    }
}

#[cfg(windows)]
mod so {
    use std::path::Path;

    use windows::core::{Interface, HSTRING};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    /// Un `.lnk` hacia `exe` con sus argumentos, su carpeta de trabajo y su icono.
    pub fn escribir_acceso(
        ruta: &Path,
        exe: &Path,
        argumentos: &str,
        icono: Option<&Path>,
    ) -> std::io::Result<()> {
        let carpeta = exe.parent().unwrap_or(exe);
        let fallo = |e: windows::core::Error| std::io::Error::other(e.to_string());
        unsafe {
            // Puede estar ya iniciado en este hilo; da igual.
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let enlace: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).map_err(fallo)?;
            enlace.SetPath(&HSTRING::from(exe)).map_err(fallo)?;
            enlace
                .SetArguments(&HSTRING::from(argumentos))
                .map_err(fallo)?;
            enlace
                .SetWorkingDirectory(&HSTRING::from(carpeta))
                .map_err(fallo)?;
            enlace
                .SetIconLocation(&HSTRING::from(icono.unwrap_or(exe)), 0)
                .map_err(fallo)?;
            enlace
                .SetDescription(&HSTRING::from("DS5Control"))
                .map_err(fallo)?;
            let archivo: IPersistFile = enlace.cast().map_err(fallo)?;
            archivo.Save(&HSTRING::from(ruta), true).map_err(fallo)
        }
    }

    use std::sync::atomic::{AtomicIsize, Ordering};

    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateIconFromResourceEx, DestroyIcon, GetSystemMetrics, SendMessageW, HICON, ICON_BIG,
        LR_DEFAULTCOLOR, SM_CXICON, WM_SETICON,
    };

    /// El último icono grande que se puso, para soltarlo al poner otro.
    static ICONO_GRANDE: AtomicIsize = AtomicIsize::new(0);

    /// La imagen de un `.ico` que mejor encaja con `lado`: la de ese tamaño o, si no hay, la más
    /// pequeña de las mayores (o la mayor de todas).
    pub(super) fn imagen_ico(ico: &[u8], lado: u32) -> Option<&[u8]> {
        let numero = u16::from_le_bytes(ico.get(4..6)?.try_into().ok()?) as usize;
        let mut mejor: Option<(u32, &[u8])> = None;
        for i in 0..numero {
            let e = ico.get(6 + i * 16..6 + i * 16 + 16)?;
            let ancho = if e[0] == 0 { 256 } else { u32::from(e[0]) };
            let largo = u32::from_le_bytes(e[8..12].try_into().ok()?) as usize;
            let desde = u32::from_le_bytes(e[12..16].try_into().ok()?) as usize;
            let datos = ico.get(desde..desde + largo)?;
            let vale = |a: u32| (a >= lado, std::cmp::Reverse(a.abs_diff(lado)));
            if mejor.is_none_or(|(a, _)| vale(ancho) > vale(a)) {
                mejor = Some((ancho, datos));
            }
        }
        mejor.map(|(_, d)| d)
    }

    /// `DisconnectBT` de C#: pide a cada radio Bluetooth que suelte el mando con esta MAC.
    pub fn desconectar_bt(mac: [u8; 6]) -> bool {
        use windows::Win32::Devices::Bluetooth::{
            BluetoothFindFirstRadio, BluetoothFindNextRadio, BluetoothFindRadioClose,
            BLUETOOTH_FIND_RADIO_PARAMS,
        };
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::System::IO::DeviceIoControl;
        const IOCTL_BTH_DISCONNECT_DEVICE: u32 = 0x0041_000C;

        // La dirección va al revés que la MAC escrita: el último byte primero.
        let mut direccion = [0u8; 8];
        for (i, b) in mac.iter().enumerate() {
            direccion[5 - i] = *b;
        }
        let direccion = u64::from_le_bytes(direccion);
        let parametros = BLUETOOTH_FIND_RADIO_PARAMS {
            dwSize: std::mem::size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32,
        };
        let mut radio = HANDLE::default();
        let Ok(busqueda) = (unsafe { BluetoothFindFirstRadio(&parametros, &mut radio) }) else {
            return false;
        };
        let hecho = loop {
            let mut n = 0u32;
            let hecho = unsafe {
                DeviceIoControl(
                    radio,
                    IOCTL_BTH_DISCONNECT_DEVICE,
                    Some(&direccion as *const u64 as *const _),
                    8,
                    None,
                    0,
                    Some(&mut n),
                    None,
                )
            }
            .is_ok();
            let _ = unsafe { CloseHandle(radio) };
            if hecho || unsafe { BluetoothFindNextRadio(busqueda, &mut radio) }.is_err() {
                break hecho;
            }
        };
        let _ = unsafe { BluetoothFindRadioClose(busqueda) };
        hecho
    }

    pub fn identidad(id: &str) {
        let _ = unsafe {
            windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(id))
        };
    }

    pub fn icono_grande(ventana: isize, ico: &[u8]) {
        let lado = unsafe { GetSystemMetrics(SM_CXICON) }.max(32);
        let Some(datos) = imagen_ico(ico, lado as u32) else {
            return;
        };
        let Ok(icono) = (unsafe {
            CreateIconFromResourceEx(datos, true, 0x0003_0000, lado, lado, LR_DEFAULTCOLOR)
        }) else {
            return;
        };
        unsafe {
            SendMessageW(
                HWND(ventana as _),
                WM_SETICON,
                Some(WPARAM(ICON_BIG as usize)),
                Some(LPARAM(icono.0 as isize)),
            );
        }
        let anterior = ICONO_GRANDE.swap(icono.0 as isize, Ordering::SeqCst);
        if anterior != 0 {
            let _ = unsafe { DestroyIcon(HICON(anterior as _)) };
        }
    }

    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetPriorityClass, SetPriorityClass, ABOVE_NORMAL_PRIORITY_CLASS,
        HIGH_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, PROCESS_CREATION_FLAGS,
    };

    const CLASES: [PROCESS_CREATION_FLAGS; 3] = [
        NORMAL_PRIORITY_CLASS,
        ABOVE_NORMAL_PRIORITY_CLASS,
        HIGH_PRIORITY_CLASS,
    ];

    pub fn prioridad() -> usize {
        let actual = unsafe { GetPriorityClass(GetCurrentProcess()) };
        CLASES.iter().position(|c| c.0 == actual).unwrap_or(0)
    }

    pub fn poner_prioridad(i: usize) {
        if let Some(c) = CLASES.get(i) {
            let _ = unsafe { SetPriorityClass(GetCurrentProcess(), *c) };
        }
    }
}

#[cfg(not(windows))]
mod so {
    pub fn escribir_acceso(
        _ruta: &std::path::Path,
        _exe: &std::path::Path,
        _argumentos: &str,
        _icono: Option<&std::path::Path>,
    ) -> std::io::Result<()> {
        Ok(())
    }
    pub fn prioridad() -> usize {
        0
    }
    pub fn poner_prioridad(_i: usize) {}
    pub fn icono_grande(_ventana: isize, _ico: &[u8]) {}
    pub fn identidad(_id: &str) {}
    pub fn desconectar_bt(_mac: [u8; 6]) -> bool {
        false
    }
}

/// Desconecta por Bluetooth el mando con esta MAC, como `DisconnectBT` de C#.
pub fn desconectar_bt(mac: [u8; 6]) -> bool {
    so::desconectar_bt(mac)
}

/// La identidad del programa en la barra de tareas. Sin ella, Windows reconoce el programa por la
/// ruta del `.exe` y pinta el icono del `.exe` (el de fuego) en lugar del de la ventana, así que el
/// icono de la paleta no se veía nunca en la barra. Las notificaciones llevan su propio
/// identificador y no cambian.
pub fn identidad(id: &str) {
    so::identidad(id)
}

/// El icono grande de la ventana, el que enseña la barra de tareas. `set_icon` de Tauri solo cambia
/// el pequeño (el de la vista previa); en C#, `Window.Icon` cambia los dos.
pub fn icono_grande(ventana: isize, ico: &[u8]) {
    so::icono_grande(ventana, ico)
}

/// La prioridad del proceso como índice de `generales::PRIORIDADES` (0 si es otra).
pub fn prioridad() -> usize {
    so::prioridad()
}

pub fn poner_prioridad(i: usize) {
    so::poner_prioridad(i)
}

#[cfg(all(test, windows))]
mod pruebas {
    #[test]
    fn escribe_un_acceso_directo() {
        let carpeta = std::env::temp_dir().join("ds5-prueba-acceso");
        let _ = std::fs::create_dir_all(&carpeta);
        let ruta = carpeta.join("DS5Control.lnk");
        let _ = std::fs::remove_file(&ruta);
        let exe = std::env::current_exe().unwrap();
        super::so::escribir_acceso(&ruta, &exe, "-m", None).unwrap();
        assert!(std::fs::metadata(&ruta).unwrap().len() > 0);
        let _ = std::fs::remove_dir_all(carpeta);
    }

    #[test]
    fn elige_la_imagen_del_ico_que_toca() {
        let ico = include_bytes!("../../../../assets/logo/final/ds5control-fuego.ico");
        for lado in [16, 32, 48, 256] {
            let d = super::so::imagen_ico(ico, lado).unwrap();
            assert!(d.len() > 40);
        }
    }
}
