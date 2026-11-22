//! El cliente de ViGEmBus: enchufa mandos virtuales de Xbox 360 y DS4, les manda informes y
//! recoge lo que devuelve el juego (vibración y, en el DS4, la luz). Habla con el driver con las
//! mismas llamadas que la biblioteca nativa de ViGEm que usa C#.

use std::fmt;
use std::mem::size_of;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use windows::core::{GUID, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT,
    SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER, ERROR_IO_PENDING,
    GENERIC_READ, GENERIC_WRITE, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT, WIN32_ERROR,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_NO_BUFFERING, FILE_FLAG_OVERLAPPED,
    FILE_FLAG_WRITE_THROUGH, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows::Win32::System::IO::{CancelIoEx, DeviceIoControl, GetOverlappedResult, OVERLAPPED};

use crate::informes::{InformeXbox, Respuesta, LARGO_DS4_EX};

/// La interfaz que publica ViGEmBus.
const GUID_BUS: GUID = GUID::from_u128(0x96E42B22_F5E9_42F8_B043_ED0F932F014F);

const IOCTL_ENCHUFAR: u32 = 0x2AA004;
const IOCTL_DESENCHUFAR: u32 = 0x2AA008;
const IOCTL_VERSION: u32 = 0x2AA00C;
const IOCTL_ESPERAR_LISTO: u32 = 0x2AA010;
const IOCTL_XUSB_AVISO: u32 = 0x2AE804;
const IOCTL_XUSB_INFORME: u32 = 0x2AA808;
const IOCTL_DS4_INFORME: u32 = 0x2AA80C;
const IOCTL_DS4_AVISO: u32 = 0x2AA810;
const IOCTL_XUSB_INDICE: u32 = 0x2AE81C;
const IOCTL_DS4_SALIDA: u32 = 0x2AE820;

/// La versión de la interfaz común (`VIGEM_COMMON_VERSION`).
const VERSION_COMUN: u32 = 0x0001;

#[derive(Debug)]
pub enum ErrorVirtual {
    /// ViGEmBus no está instalado o no responde.
    SinBus,
    /// No llegó nada en el tiempo pedido.
    TiempoAgotado,
    Windows(windows::core::Error),
}

impl fmt::Display for ErrorVirtual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorVirtual::SinBus => write!(f, "ViGEmBus no está instalado"),
            ErrorVirtual::TiempoAgotado => write!(f, "tiempo agotado"),
            ErrorVirtual::Windows(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ErrorVirtual {}

impl From<windows::core::Error> for ErrorVirtual {
    fn from(e: windows::core::Error) -> Self {
        ErrorVirtual::Windows(e)
    }
}

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

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

fn evento() -> Result<Handle, ErrorVirtual> {
    Ok(Handle(unsafe {
        CreateEventW(None, true, false, PCWSTR::null())?
    }))
}

/// La conexión con el driver. Se comparte entre todos los mandos virtuales.
pub struct Bus {
    h: Handle,
}

impl Bus {
    /// Abre ViGEmBus. `SinBus` si no está instalado.
    pub fn abrir() -> Result<Arc<Bus>, ErrorVirtual> {
        for ruta in rutas_del_bus() {
            let ruta = windows::core::HSTRING::from(ruta);
            let Ok(h) = (unsafe {
                CreateFileW(
                    &ruta,
                    GENERIC_READ.0 | GENERIC_WRITE.0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL
                        | FILE_FLAG_NO_BUFFERING
                        | FILE_FLAG_WRITE_THROUGH
                        | FILE_FLAG_OVERLAPPED,
                    None,
                )
            }) else {
                continue;
            };
            let bus = Bus { h: Handle(h) };
            let mut version = [8u32, VERSION_COMUN];
            if bus
                .llamar(IOCTL_VERSION, &mut version, false, Duration::from_secs(2))
                .is_ok()
            {
                return Ok(Arc::new(bus));
            }
        }
        Err(ErrorVirtual::SinBus)
    }

    /// Una llamada al driver con el mismo búfer de entrada y, si `devuelve`, de salida.
    fn llamar<T>(
        &self,
        codigo: u32,
        datos: &mut T,
        devuelve: bool,
        espera: Duration,
    ) -> Result<(), ErrorVirtual> {
        let ev = evento()?;
        self.llamar_con(&ev, codigo, datos, devuelve, espera)
    }

    fn llamar_con<T>(
        &self,
        ev: &Handle,
        codigo: u32,
        datos: &mut T,
        devuelve: bool,
        espera: Duration,
    ) -> Result<(), ErrorVirtual> {
        let mut ov = OVERLAPPED {
            hEvent: ev.0,
            ..Default::default()
        };
        let p = datos as *mut T as *mut core::ffi::c_void;
        let largo = size_of::<T>() as u32;
        let r = unsafe {
            DeviceIoControl(
                self.h.0,
                codigo,
                Some(p),
                largo,
                devuelve.then_some(p),
                if devuelve { largo } else { 0 },
                None,
                Some(&mut ov),
            )
        };
        if let Err(e) = r {
            if unsafe { GetLastError() } != ERROR_IO_PENDING {
                return Err(e.into());
            }
        }
        let ms = espera.as_millis().min(u32::MAX as u128 - 1) as u32;
        let w = unsafe { WaitForSingleObject(ev.0, ms) };
        if w == WAIT_TIMEOUT {
            unsafe {
                let _ = CancelIoEx(self.h.0, Some(&ov));
                let mut n = 0;
                let _ = GetOverlappedResult(self.h.0, &ov, &mut n, true);
            }
            return Err(ErrorVirtual::TiempoAgotado);
        }
        if w != WAIT_OBJECT_0 {
            return Err(windows::core::Error::from_thread().into());
        }
        let mut n = 0u32;
        unsafe { GetOverlappedResult(self.h.0, &ov, &mut n, false)? };
        Ok(())
    }
}

fn rutas_del_bus() -> Vec<String> {
    let mut rutas = Vec::new();
    unsafe {
        let Ok(lista) = SetupDiGetClassDevsW(
            Some(&GUID_BUS),
            PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        ) else {
            return rutas;
        };
        let mut indice = 0;
        loop {
            let mut interfaz = SP_DEVICE_INTERFACE_DATA {
                cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                ..Default::default()
            };
            if SetupDiEnumDeviceInterfaces(lista, None, &GUID_BUS, indice, &mut interfaz).is_err() {
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
            let mut bufer = vec![0u32; (necesario as usize).div_ceil(4)];
            let detalle = bufer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
            (*detalle).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
            if SetupDiGetDeviceInterfaceDetailW(
                lista,
                &interfaz,
                Some(detalle),
                necesario,
                None,
                None,
            )
            .is_ok()
            {
                if let Ok(r) = PCWSTR((*detalle).DevicePath.as_ptr()).to_string() {
                    rutas.push(r);
                }
            }
        }
        let _ = SetupDiDestroyDeviceInfoList(lista);
    }
    rutas
}

/// El tipo de mando virtual.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tipo {
    Xbox360,
    Ds4,
}

impl Tipo {
    fn codigo(self) -> i32 {
        match self {
            Tipo::Xbox360 => 0,
            Tipo::Ds4 => 2,
        }
    }

    /// Los mismos que pone la biblioteca de ViGEm por defecto.
    fn vid_pid(self) -> (u16, u16) {
        match self {
            Tipo::Xbox360 => (0x045E, 0x028E),
            Tipo::Ds4 => (0x054C, 0x05C4),
        }
    }
}

#[repr(C)]
struct Enchufar {
    largo: u32,
    serie: u32,
    tipo: i32,
    vid: u16,
    pid: u16,
}

#[repr(C)]
struct InformeXusb {
    largo: u32,
    serie: u32,
    informe: [u8; 12],
}

/// Empaquetada: 71 bytes, como la manda la biblioteca nativa de ViGEm.
#[repr(C, packed)]
struct InformeDs4Ex {
    largo: u32,
    serie: u32,
    informe: [u8; LARGO_DS4_EX],
}

#[repr(C)]
struct AvisoXusb {
    largo: u32,
    serie: u32,
    motor_grande: u8,
    motor_pequeno: u8,
    led: u8,
    _relleno: u8,
}

#[repr(C)]
struct AvisoDs4 {
    largo: u32,
    serie: u32,
    motor_pequeno: u8,
    motor_grande: u8,
    rojo: u8,
    verde: u8,
    azul: u8,
    _relleno: [u8; 3],
}

#[repr(C)]
struct SalidaDs4 {
    largo: u32,
    serie: u32,
    bufer: [u8; 64],
}

/// Un mando virtual enchufado. Se desenchufa al soltarlo.
pub struct MandoVirtual {
    bus: Arc<Bus>,
    pub tipo: Tipo,
    serie: u32,
    /// El evento de los informes: se manda desde un solo hilo cada vez.
    envio: Mutex<Handle>,
    /// Si el driver es antiguo y no tiene la salida completa del DS4.
    ds4_sin_salida: std::sync::atomic::AtomicBool,
}

impl MandoVirtual {
    /// Enchufa un mando nuevo y espera a que Windows lo tenga listo.
    pub fn enchufar(bus: &Arc<Bus>, tipo: Tipo) -> Result<MandoVirtual, ErrorVirtual> {
        let (vid, pid) = tipo.vid_pid();
        // Como la biblioteca de ViGEm: el primer número de serie libre.
        let mut serie = 1;
        loop {
            let mut e = Enchufar {
                largo: size_of::<Enchufar>() as u32,
                serie,
                tipo: tipo.codigo(),
                vid,
                pid,
            };
            match bus.llamar(IOCTL_ENCHUFAR, &mut e, false, Duration::from_secs(5)) {
                Ok(()) => break,
                Err(ErrorVirtual::Windows(_)) if serie < 64 => serie += 1,
                Err(e) => return Err(e),
            }
        }
        let mando = MandoVirtual {
            bus: bus.clone(),
            tipo,
            serie,
            envio: Mutex::new(evento()?),
            ds4_sin_salida: false.into(),
        };
        let mut listo = [8u32, serie];
        match bus.llamar(
            IOCTL_ESPERAR_LISTO,
            &mut listo,
            false,
            Duration::from_secs(5),
        ) {
            // Los drivers anteriores a la 1.17 no tienen esta llamada.
            Err(ErrorVirtual::Windows(e)) if e.code() == ERROR_INVALID_PARAMETER.to_hresult() => {}
            r => r?,
        }
        Ok(mando)
    }

    fn enviar<T>(&self, codigo: u32, datos: &mut T) -> Result<(), ErrorVirtual> {
        let ev = self.envio.lock().unwrap_or_else(|e| e.into_inner());
        match self
            .bus
            .llamar_con(&ev, codigo, datos, false, Duration::from_millis(100))
        {
            // Como la biblioteca nativa: el envío solo falla con acceso denegado (el mando ya no
            // existe) o parámetro no válido. El driver devuelve otros avisos aunque lo reciba.
            Err(ErrorVirtual::Windows(e))
                if e.code() != ERROR_ACCESS_DENIED.to_hresult()
                    && e.code() != ERROR_INVALID_PARAMETER.to_hresult() =>
            {
                Ok(())
            }
            r => r,
        }
    }

    /// Manda el estado al mando de Xbox 360.
    pub fn enviar_xbox(&self, informe: &InformeXbox) -> Result<(), ErrorVirtual> {
        let mut d = InformeXusb {
            largo: size_of::<InformeXusb>() as u32,
            serie: self.serie,
            informe: informe.bytes(),
        };
        self.enviar(IOCTL_XUSB_INFORME, &mut d)
    }

    /// Manda el informe extendido al DS4.
    pub fn enviar_ds4(&self, informe: &[u8; LARGO_DS4_EX]) -> Result<(), ErrorVirtual> {
        let mut d = InformeDs4Ex {
            largo: size_of::<InformeDs4Ex>() as u32,
            serie: self.serie,
            informe: *informe,
        };
        self.enviar(IOCTL_DS4_INFORME, &mut d)
    }

    /// El número de jugador de XInput (0 a 3), como `UserIndex`.
    pub fn indice_xinput(&self) -> Option<u32> {
        if self.tipo != Tipo::Xbox360 {
            return None;
        }
        let mut d = [12u32, self.serie, 0];
        self.bus
            .llamar(IOCTL_XUSB_INDICE, &mut d, true, Duration::from_millis(500))
            .ok()
            .map(|_| d[2])
    }

    /// Espera a que el juego mande algo al mando virtual. Se llama en bucle desde un hilo propio.
    pub fn esperar_respuesta(&self, espera: Duration) -> Result<Respuesta, ErrorVirtual> {
        match self.tipo {
            Tipo::Xbox360 => {
                let mut d = AvisoXusb {
                    largo: size_of::<AvisoXusb>() as u32,
                    serie: self.serie,
                    motor_grande: 0,
                    motor_pequeno: 0,
                    led: 0,
                    _relleno: 0,
                };
                self.bus.llamar(IOCTL_XUSB_AVISO, &mut d, true, espera)?;
                Ok(Respuesta {
                    motor_grande: d.motor_grande,
                    motor_pequeno: d.motor_pequeno,
                    con_vibracion: true,
                    ..Default::default()
                })
            }
            Tipo::Ds4 => {
                use std::sync::atomic::Ordering;
                if !self.ds4_sin_salida.load(Ordering::Relaxed) {
                    let mut d = SalidaDs4 {
                        largo: size_of::<SalidaDs4>() as u32,
                        serie: self.serie,
                        bufer: [0; 64],
                    };
                    match self.bus.llamar(IOCTL_DS4_SALIDA, &mut d, true, espera) {
                        Ok(()) => {
                            return Respuesta::desde_ds4(&d.bufer)
                                .ok_or(ErrorVirtual::TiempoAgotado)
                        }
                        Err(ErrorVirtual::Windows(e))
                            if e.code() == WIN32_ERROR(1).to_hresult()
                                || e.code() == ERROR_INVALID_PARAMETER.to_hresult() =>
                        {
                            // Driver anterior a la 1.19: se usa el aviso de siempre.
                            self.ds4_sin_salida.store(true, Ordering::Relaxed);
                        }
                        Err(e) => return Err(e),
                    }
                }
                let mut d = AvisoDs4 {
                    largo: size_of::<AvisoDs4>() as u32,
                    serie: self.serie,
                    motor_pequeno: 0,
                    motor_grande: 0,
                    rojo: 0,
                    verde: 0,
                    azul: 0,
                    _relleno: [0; 3],
                };
                self.bus.llamar(IOCTL_DS4_AVISO, &mut d, true, espera)?;
                Ok(Respuesta {
                    motor_grande: d.motor_grande,
                    motor_pequeno: d.motor_pequeno,
                    color: Some((d.rojo, d.verde, d.azul)),
                    con_vibracion: true,
                    ..Default::default()
                })
            }
        }
    }
}

impl Drop for MandoVirtual {
    fn drop(&mut self) {
        let mut d = [8u32, self.serie];
        let _ = self
            .bus
            .llamar(IOCTL_DESENCHUFAR, &mut d, false, Duration::from_secs(2));
    }
}
