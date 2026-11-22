//! HidHide: esconde el mando de verdad a los juegos mientras se les da uno virtual, para que cada
//! pulsación llegue una sola vez. Las mismas llamadas que `HidHideAPIDevice.cs` y las mismas
//! reglas que `HidHideControl.cs`: solo se toca lo que se añadió, y la lista del usuario y el
//! interruptor de HidHide quedan como estaban.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_Device_Interface_PropertyW, CR_BUFFER_SMALL, CR_SUCCESS,
};
use windows::Win32::Devices::Properties::{DEVPKEY_Device_InstanceId, DEVPROPTYPE};
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, QueryDosDeviceW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;

const IOCTL_LEER_BLANCA: u32 = 0x80016000;
const IOCTL_FIJAR_BLANCA: u32 = 0x80016004;
const IOCTL_LEER_NEGRA: u32 = 0x80016008;
const IOCTL_FIJAR_NEGRA: u32 = 0x8001600C;
const IOCTL_LEER_ACTIVO: u32 = 0x80016010;
const IOCTL_FIJAR_ACTIVO: u32 = 0x80016014;
const IOCTL_LEER_INVERSA: u32 = 0x80016018;

/// El dispositivo de control de HidHide abierto.
pub struct HidHide {
    h: HANDLE,
}

impl Drop for HidHide {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.h);
        }
    }
}

impl HidHide {
    /// `None` si HidHide no está instalado.
    pub fn abrir() -> Option<HidHide> {
        let h = unsafe {
            CreateFileW(
                &HSTRING::from(r"\\.\HidHide"),
                GENERIC_READ.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }
        .ok()?;
        Some(HidHide { h })
    }

    fn llamar(&self, codigo: u32, entrada: &[u8], salida: &mut [u8]) -> Option<u32> {
        let mut n = 0u32;
        unsafe {
            DeviceIoControl(
                self.h,
                codigo,
                (!entrada.is_empty()).then_some(entrada.as_ptr() as *const _),
                entrada.len() as u32,
                (!salida.is_empty()).then_some(salida.as_mut_ptr() as *mut _),
                salida.len() as u32,
                Some(&mut n),
                None,
            )
        }
        .ok()
        .map(|_| n)
    }

    fn leer_bool(&self, codigo: u32) -> bool {
        let mut b = [0u8];
        self.llamar(codigo, &[], &mut b);
        b[0] != 0
    }

    fn leer_lista(&self, codigo: u32) -> Vec<String> {
        // Primero se pregunta cuánto ocupa, como en C#.
        let Some(n) = self.llamar(codigo, &[], &mut []) else {
            return Vec::new();
        };
        if n == 0 {
            return Vec::new();
        }
        let mut b = vec![0u8; n as usize];
        if self.llamar(codigo, &[], &mut b).is_none() {
            return Vec::new();
        }
        desde_multi_sz(&b)
    }

    /// Si HidHide está encendido.
    pub fn activo(&self) -> bool {
        self.leer_bool(IOCTL_LEER_ACTIVO)
    }

    pub fn fijar_activo(&self, activo: bool) -> bool {
        self.llamar(IOCTL_FIJAR_ACTIVO, &[activo as u8], &mut [])
            .is_some()
    }

    /// Los dispositivos escondidos (sus identificadores de instancia).
    pub fn lista_negra(&self) -> Vec<String> {
        self.leer_lista(IOCTL_LEER_NEGRA)
    }

    pub fn fijar_lista_negra(&self, lista: &[String]) -> bool {
        self.llamar(IOCTL_FIJAR_NEGRA, &a_multi_sz(lista), &mut [])
            .is_some()
    }

    /// Los programas que sí ven los dispositivos escondidos (rutas de dispositivo DOS).
    pub fn lista_blanca(&self) -> Vec<String> {
        self.leer_lista(IOCTL_LEER_BLANCA)
    }

    pub fn fijar_lista_blanca(&self, lista: &[String]) -> bool {
        self.llamar(IOCTL_FIJAR_BLANCA, &a_multi_sz(lista), &mut [])
            .is_some()
    }

    /// Si la lista blanca está invertida (entonces no hace falta apuntarse en ella).
    pub fn inversa(&self) -> bool {
        self.leer_bool(IOCTL_LEER_INVERSA)
    }

    /// Como `CheckHidHidePresence`: apunta el programa en la lista blanca para que siga viendo
    /// los mandos escondidos. Devuelve si lo ha añadido (antes no estaba).
    pub fn permitir_programa(&self, exe: &Path) -> bool {
        if self.inversa() {
            return false;
        }
        let Some(ruta) = ruta_dos(exe) else {
            return false;
        };
        let mut lista = self.lista_blanca();
        if lista.iter().any(|r| r == &ruta) {
            return false;
        }
        lista.push(ruta);
        self.fijar_lista_blanca(&lista)
    }

    /// Quita el programa de la lista blanca.
    pub fn retirar_programa(&self, exe: &Path) -> bool {
        let Some(ruta) = ruta_dos(exe) else {
            return false;
        };
        let mut lista = self.lista_blanca();
        let antes = lista.len();
        lista.retain(|r| r != &ruta);
        lista.len() != antes && self.fijar_lista_blanca(&lista)
    }
}

/// Una lista de cadenas UTF-16 separadas por ceros y con un cero más al final.
pub fn a_multi_sz(lista: &[String]) -> Vec<u8> {
    let mut b = Vec::new();
    for s in lista {
        for u in s.encode_utf16().chain([0]) {
            b.extend_from_slice(&u.to_le_bytes());
        }
    }
    b.extend_from_slice(&[0, 0]);
    b
}

pub fn desde_multi_sz(b: &[u8]) -> Vec<String> {
    let u: Vec<u16> = b
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    u.split(|&c| c == 0)
        .filter(|s| !s.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

/// `C:\...\ds5.exe` como lo guarda HidHide: `\Device\HarddiskVolume3\...\ds5.exe`.
pub fn ruta_dos(exe: &Path) -> Option<String> {
    // Si la carpeta es un enlace (lo hace Scoop), la ruta de verdad.
    let exe = fs::canonicalize(exe).unwrap_or_else(|_| exe.to_path_buf());
    let texto = exe.to_string_lossy();
    let texto = texto.strip_prefix(r"\\?\").unwrap_or(&texto);
    let (unidad, resto) = texto.split_at_checked(2)?;
    if !unidad.ends_with(':') {
        return None;
    }
    let mut bufer = [0u16; 512];
    let n = unsafe { QueryDosDeviceW(&HSTRING::from(unidad), Some(&mut bufer)) };
    if n == 0 {
        return None;
    }
    let destino = PCWSTR(bufer.as_ptr());
    let destino = unsafe { destino.to_string() }.ok()?;
    let destino = destino.strip_prefix(r"\??\").unwrap_or(&destino);
    Some(format!(
        "{}\\{}",
        destino.trim_end_matches('\\'),
        resto.trim_start_matches('\\')
    ))
}

/// El identificador de instancia del dispositivo de una ruta HID, como `InstanceOf`.
pub fn instancia_de(ruta: &str) -> Option<String> {
    if ruta.is_empty() {
        return None;
    }
    let ruta_w = HSTRING::from(ruta);
    let mut tipo = DEVPROPTYPE::default();
    let mut largo = 0u32;
    let r = unsafe {
        CM_Get_Device_Interface_PropertyW(
            &ruta_w,
            &DEVPKEY_Device_InstanceId,
            &mut tipo,
            None,
            &mut largo,
            0,
        )
    };
    if r == CR_BUFFER_SMALL && largo > 0 {
        let mut b = vec![0u8; largo as usize];
        let r = unsafe {
            CM_Get_Device_Interface_PropertyW(
                &ruta_w,
                &DEVPKEY_Device_InstanceId,
                &mut tipo,
                Some(b.as_mut_ptr()),
                &mut largo,
                0,
            )
        };
        if r == CR_SUCCESS {
            if let Some(s) = desde_multi_sz(&b).into_iter().next() {
                return Some(s);
            }
        }
    }
    instancia_desde_ruta(ruta)
}

/// `\\?\hid#vid_054c&pid_0ce6&mi_03#8&e01d75d&0&0000#{guid}` da
/// `HID\VID_054C&PID_0CE6&MI_03\8&E01D75D&0&0000`.
pub fn instancia_desde_ruta(ruta: &str) -> Option<String> {
    if ruta.is_empty() {
        return None;
    }
    let mut r = ruta.strip_prefix(r"\\?\").unwrap_or(ruta);
    if let Some(i) = r.rfind("#{") {
        if i > 0 {
            r = &r[..i];
        }
    }
    Some(r.replace('#', "\\").to_uppercase())
}

/// Lo que se ha escondido, recordado en un archivo por si el programa se cierra de golpe.
/// Mismo formato que `HidHide.txt` de C#.
pub struct Ocultador {
    archivo: PathBuf,
    anadidos: BTreeSet<String>,
    encendido_por_nosotros: bool,
    ultimo: Option<BTreeSet<String>>,
}

const LINEA_ENCENDIDO: &str = "encendido=true";
const PREFIJO_OCULTO: &str = "oculto=";

impl Ocultador {
    pub fn new(archivo: PathBuf) -> Ocultador {
        let mut o = Ocultador {
            archivo,
            anadidos: BTreeSet::new(),
            encendido_por_nosotros: false,
            ultimo: None,
        };
        if let Ok(texto) = fs::read_to_string(&o.archivo) {
            for linea in texto.lines() {
                if linea == LINEA_ENCENDIDO {
                    o.encendido_por_nosotros = true;
                } else if let Some(i) = linea.strip_prefix(PREFIJO_OCULTO) {
                    o.anadidos.insert(i.to_uppercase());
                }
            }
        }
        o
    }

    /// Esconde exactamente estos dispositivos; con una lista vacía lo deshace todo.
    pub fn aplicar(&mut self, esconder: &[String]) {
        let queridos: BTreeSet<String> = esconder.iter().map(|s| s.to_uppercase()).collect();
        if self.ultimo.as_ref() == Some(&queridos) {
            return;
        }
        let Some(hh) = HidHide::abrir() else {
            return;
        };
        let mut lista = hh.lista_negra();
        let mut cambiado = false;
        let sobran: Vec<String> = self
            .anadidos
            .iter()
            .filter(|i| !queridos.contains(*i))
            .cloned()
            .collect();
        for i in sobran {
            let antes = lista.len();
            lista.retain(|l| !l.eq_ignore_ascii_case(&i));
            cambiado |= lista.len() != antes;
            self.anadidos.remove(&i);
        }
        for i in &queridos {
            if lista.iter().any(|l| l.eq_ignore_ascii_case(i)) {
                continue;
            }
            lista.push(i.clone());
            self.anadidos.insert(i.clone());
            cambiado = true;
        }
        if cambiado && !hh.fijar_lista_negra(&lista) {
            return;
        }
        if !queridos.is_empty() && !hh.activo() {
            hh.fijar_activo(true);
            self.encendido_por_nosotros = hh.activo();
        } else if queridos.is_empty() && self.encendido_por_nosotros {
            hh.fijar_activo(false);
            self.encendido_por_nosotros = false;
        }
        self.ultimo = Some(queridos);
        self.guardar();
    }

    fn guardar(&self) {
        let mut lineas: Vec<String> = self
            .anadidos
            .iter()
            .map(|i| format!("{PREFIJO_OCULTO}{i}"))
            .collect();
        if self.encendido_por_nosotros {
            lineas.insert(0, LINEA_ENCENDIDO.to_string());
        }
        if lineas.is_empty() {
            let _ = fs::remove_file(&self.archivo);
        } else {
            let _ = fs::write(&self.archivo, lineas.join("\r\n") + "\r\n");
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_instancia_sale_de_la_ruta() {
        assert_eq!(
            instancia_desde_ruta(
                r"\\?\hid#vid_054c&pid_0ce6&mi_03#8&e01d75d&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}"
            )
            .as_deref(),
            Some(r"HID\VID_054C&PID_0CE6&MI_03\8&E01D75D&0&0000")
        );
    }

    #[test]
    fn las_listas_van_y_vuelven() {
        let l = vec!["uno".to_string(), "dós".to_string()];
        let b = a_multi_sz(&l);
        assert_eq!(b.len(), (3 + 1 + 3 + 1 + 1) * 2);
        assert_eq!(desde_multi_sz(&b), l);
        assert!(desde_multi_sz(&a_multi_sz(&[])).is_empty());
    }
}
