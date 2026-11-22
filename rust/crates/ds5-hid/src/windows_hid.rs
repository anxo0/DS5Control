//! El acceso HID con la API de Windows, como `HidLibrary` en C#: SetupDi para enumerar,
//! `CreateFileW` con E/S solapada para leer y escribir, y `HidD_*` para las características.

use std::fmt;
use std::time::Duration;

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT,
    SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows::Win32::Devices::HumanInterfaceDevice::{
    HidD_FreePreparsedData, HidD_GetAttributes, HidD_GetFeature, HidD_GetHidGuid,
    HidD_GetPreparsedData, HidD_SetNumInputBuffers, HidP_GetCaps, HIDD_ATTRIBUTES, HIDP_CAPS,
    PHIDP_PREPARSED_DATA,
};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_IO_PENDING, GENERIC_READ, GENERIC_WRITE, HANDLE,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

use crate::{InfoMando, Modelo};
use ds5_informes::Conexion;

#[derive(Debug)]
pub enum ErrorHid {
    /// No llegó nada en el tiempo pedido.
    TiempoAgotado,
    /// Windows devolvió un error: el mando se ha desconectado o no se puede abrir.
    Windows(windows::core::Error),
}

impl fmt::Display for ErrorHid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorHid::TiempoAgotado => write!(f, "tiempo agotado"),
            ErrorHid::Windows(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ErrorHid {}

impl From<windows::core::Error> for ErrorHid {
    fn from(e: windows::core::Error) -> Self {
        ErrorHid::Windows(e)
    }
}

/// Un handle que se cierra solo.
struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

// Los handles de Windows se pueden usar desde cualquier hilo.
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

fn abrir_ruta(ruta: &str, acceso: u32) -> windows::core::Result<Handle> {
    let ruta = HSTRING::from(ruta);
    let h = unsafe {
        CreateFileW(
            &ruta,
            acceso,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
            None,
        )?
    };
    Ok(Handle(h))
}

/// Los largos de los informes, de las capacidades del dispositivo.
fn capacidades(h: HANDLE) -> Option<HIDP_CAPS> {
    unsafe {
        let mut datos = PHIDP_PREPARSED_DATA::default();
        if !HidD_GetPreparsedData(h, &mut datos) {
            return None;
        }
        let mut caps = HIDP_CAPS::default();
        let estado = HidP_GetCaps(datos, &mut caps);
        HidD_FreePreparsedData(datos);
        (estado.0 >= 0).then_some(caps)
    }
}

/// Los mandos conectados que se pueden manejar.
pub fn enumerar() -> Vec<InfoMando> {
    let mut mandos = Vec::new();
    unsafe {
        let guid = HidD_GetHidGuid();
        let Ok(lista) = SetupDiGetClassDevsW(
            Some(&guid),
            PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        ) else {
            return mandos;
        };
        let mut indice = 0;
        loop {
            let mut interfaz = SP_DEVICE_INTERFACE_DATA {
                cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                ..Default::default()
            };
            if SetupDiEnumDeviceInterfaces(lista, None, &guid, indice, &mut interfaz).is_err() {
                break;
            }
            indice += 1;
            let mut necesario = 0u32;
            let _ = SetupDiGetDeviceInterfaceDetailW(
                lista,
                &interfaz,
                None,
                0,
                Some(&mut necesario),
                None,
            );
            if necesario == 0 {
                continue;
            }
            // Un búfer de u32 para que quede alineado como la estructura.
            let mut bufer = vec![0u32; (necesario as usize).div_ceil(4)];
            let detalle = bufer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
            (*detalle).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
            if SetupDiGetDeviceInterfaceDetailW(
                lista,
                &interfaz,
                Some(detalle),
                necesario,
                None,
                None,
            )
            .is_err()
            {
                continue;
            }
            let ruta = PCWSTR((*detalle).DevicePath.as_ptr())
                .to_string()
                .unwrap_or_default();
            if let Some(info) = describir(&ruta) {
                mandos.push(info);
            }
        }
        let _ = SetupDiDestroyDeviceInfoList(lista);
    }
    mandos
}

fn describir(ruta: &str) -> Option<InfoMando> {
    // Sin permisos de lectura ni escritura se pueden leer los atributos aunque otro programa
    // tenga el mando abierto.
    let h = abrir_ruta(ruta, 0).ok()?;
    let mut atributos = HIDD_ATTRIBUTES {
        Size: std::mem::size_of::<HIDD_ATTRIBUTES>() as u32,
        ..Default::default()
    };
    if !unsafe { HidD_GetAttributes(h.0, &mut atributos) } {
        return None;
    }
    let modelo = Modelo::desde(atributos.VendorID, atributos.ProductID)?;
    let caps = capacidades(h.0)?;
    // Como `DetermineConnectionType`: por USB el informe de entrada mide 64 bytes.
    let conexion = if caps.InputReportByteLength == 64 {
        Conexion::Usb
    } else {
        Conexion::Bt
    };
    Some(InfoMando {
        ruta: ruta.to_string(),
        vid: atributos.VendorID,
        pid: atributos.ProductID,
        modelo,
        conexion,
        largo_entrada: caps.InputReportByteLength as usize,
        largo_salida: caps.OutputReportByteLength as usize,
        largo_caracteristica: caps.FeatureReportByteLength as usize,
    })
}

/// Un mando abierto para leer y escribir.
pub struct Dispositivo {
    h: Handle,
    evento_lectura: Handle,
    evento_escritura: Handle,
    pub info: InfoMando,
}

impl Dispositivo {
    pub fn abrir(info: &InfoMando) -> Result<Dispositivo, ErrorHid> {
        let h = abrir_ruta(&info.ruta, GENERIC_READ.0 | GENERIC_WRITE.0)?;
        unsafe {
            // Como en C#: pocos informes en cola para que no se acumule retraso.
            let _ = HidD_SetNumInputBuffers(h.0, 3);
        }
        let evento = || -> Result<Handle, ErrorHid> {
            Ok(Handle(unsafe {
                CreateEventW(None, true, false, PCWSTR::null())?
            }))
        };
        Ok(Dispositivo {
            h,
            evento_lectura: evento()?,
            evento_escritura: evento()?,
            info: info.clone(),
        })
    }

    /// Un informe de características. `bufer[0]` lleva el número del informe.
    pub fn leer_caracteristica(&self, bufer: &mut [u8]) -> Result<(), ErrorHid> {
        let ok =
            unsafe { HidD_GetFeature(self.h.0, bufer.as_mut_ptr() as *mut _, bufer.len() as u32) };
        if ok {
            Ok(())
        } else {
            Err(ErrorHid::Windows(windows::core::Error::from_thread()))
        }
    }

    /// Espera al siguiente informe de entrada y devuelve cuántos bytes ha leído.
    pub fn leer(&self, bufer: &mut [u8], espera: Duration) -> Result<usize, ErrorHid> {
        self.solapada(&self.evento_lectura, espera, |ov| unsafe {
            ReadFile(self.h.0, Some(bufer), None, Some(ov))
        })
    }

    /// Manda un informe de salida por el canal de interrupción, como `WriteOutputReportViaInterrupt`.
    pub fn escribir(&self, informe: &[u8], espera: Duration) -> Result<usize, ErrorHid> {
        self.solapada(&self.evento_escritura, espera, |ov| unsafe {
            WriteFile(self.h.0, Some(informe), None, Some(ov))
        })
    }

    fn solapada(
        &self,
        evento: &Handle,
        espera: Duration,
        empezar: impl FnOnce(*mut OVERLAPPED) -> windows::core::Result<()>,
    ) -> Result<usize, ErrorHid> {
        let mut ov = OVERLAPPED {
            hEvent: evento.0,
            ..Default::default()
        };
        if let Err(e) = empezar(&mut ov) {
            if unsafe { GetLastError() } != ERROR_IO_PENDING {
                return Err(e.into());
            }
        }
        let ms = espera.as_millis().min(u32::MAX as u128 - 1) as u32;
        let r = unsafe { WaitForSingleObject(evento.0, ms) };
        if r == WAIT_TIMEOUT {
            unsafe {
                let _ = CancelIoEx(self.h.0, Some(&ov));
                let mut n = 0;
                let _ = GetOverlappedResult(self.h.0, &ov, &mut n, true);
            }
            return Err(ErrorHid::TiempoAgotado);
        }
        if r != WAIT_OBJECT_0 {
            return Err(ErrorHid::Windows(windows::core::Error::from_thread()));
        }
        let mut n = 0u32;
        unsafe { GetOverlappedResult(self.h.0, &ov, &mut n, false)? };
        Ok(n as usize)
    }
}
