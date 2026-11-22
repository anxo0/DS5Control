//! Los paquetes del protocolo DSU (cemuhook), byte a byte como `UdpServer.cs`.
//!
//! Todos empiezan con la misma cabecera de 16 bytes:
//!
//! | Bytes  | Qué                                                        |
//! |--------|------------------------------------------------------------|
//! | 0..4   | `DSUS` (del servidor) o `DSUC` (del cliente)               |
//! | 4..6   | versión del protocolo, 1001                                |
//! | 6..8   | largo del mensaje sin la cabecera                          |
//! | 8..12  | CRC32 del paquete entero con este campo a cero             |
//! | 12..16 | id de quien lo manda                                       |
//!
//! Detrás va el tipo de mensaje (4 bytes) y su contenido. Todo en little endian.

use ds5_informes::{boton, Conexion, Estado};

use crate::crc::crc32;

/// Cuántos mandos sirve el servidor (`NUMBER_SLOTS`).
pub const RANURAS: usize = 4;
/// La versión del protocolo que se entiende y se responde.
pub const VERSION_PROTOCOLO: u16 = 1001;
/// Largo de la cabecera.
pub const LARGO_CABECERA: usize = 16;
/// Largo de la respuesta de versión.
pub const LARGO_VERSION: usize = 24;
/// Largo de la respuesta de información de un puerto.
pub const LARGO_PUERTO: usize = 32;
/// Largo de la respuesta con los datos del mando (`DATA_RSP_PACKET_LEN`).
pub const LARGO_DATOS: usize = 100;

/// Tipo de la petición y la respuesta de versión.
pub const MENSAJE_VERSION: u32 = 0x10_0000;
/// Tipo de la petición de la lista de puertos y de la respuesta con la información de uno.
pub const MENSAJE_PUERTOS: u32 = 0x10_0001;
/// Tipo de la petición de suscripción a los datos y de la respuesta con los datos.
pub const MENSAJE_DATOS: u32 = 0x10_0002;

/// Si la ranura está ocupada (`DsState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum EstadoRanura {
    #[default]
    Desconectado = 0x00,
    Reservado = 0x01,
    Conectado = 0x02,
}

/// Cómo está conectado el mando (`DsConnection`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum TipoConexion {
    #[default]
    Ninguna = 0x00,
    Usb = 0x01,
    Bluetooth = 0x02,
}

impl From<Conexion> for TipoConexion {
    fn from(c: Conexion) -> Self {
        match c {
            Conexion::Usb => TipoConexion::Usb,
            Conexion::Bt => TipoConexion::Bluetooth,
        }
    }
}

/// El modelo que se anuncia (`DsModel`). C# siempre dice DS4 para un mando conectado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Modelo {
    #[default]
    Ninguno = 0,
    Ds3 = 1,
    Ds4 = 2,
    Generico = 3,
}

/// El nivel de batería en el formato del protocolo (`DsBattery`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum NivelBateria {
    #[default]
    Ninguna = 0x00,
    Agotandose = 0x01,
    Baja = 0x02,
    Media = 0x03,
    Alta = 0x04,
    Llena = 0x05,
    Cargando = 0xEE,
    Cargada = 0xEF,
}

impl NivelBateria {
    /// Como `GetPadDetailForIdx`: del porcentaje y de si está cargando. C# nunca manda
    /// `Cargando`: un mando cargando por debajo del 100 % sale con su nivel.
    pub fn desde_porcentaje(porcentaje: u8, cargando: bool) -> Self {
        match porcentaje {
            p if cargando && p >= 100 => NivelBateria::Cargada,
            95.. => NivelBateria::Llena,
            70.. => NivelBateria::Alta,
            50.. => NivelBateria::Media,
            20.. => NivelBateria::Baja,
            5.. => NivelBateria::Agotandose,
            _ => NivelBateria::Ninguna,
        }
    }
}

/// Lo que se cuenta de un mando aparte de sus controles (`DualShockPadMeta`). Va en la
/// respuesta de puertos y en la cabecera de cada paquete de datos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Meta {
    /// La ranura, de 0 a 3 (`PadId`).
    pub ranura: u8,
    pub estado: EstadoRanura,
    pub modelo: Modelo,
    pub conexion: TipoConexion,
    /// `None` si no se sabe: sale como seis ceros.
    pub mac: Option<[u8; 6]>,
    pub bateria: NivelBateria,
    /// Si se está tocando algún control (`!isDS5Idle()`).
    pub activo: bool,
}

impl Meta {
    /// Una ranura sin mando, como `GetPadDetailForIdx` cuando no hay dispositivo.
    pub fn desconectado(ranura: u8) -> Self {
        Meta {
            ranura,
            ..Default::default()
        }
    }

    /// La de un mando conectado, como `GetPadDetailForIdx`.
    ///
    /// - `mac`: la del mando. Si es todo ceros (`BLANK_SERIAL`) la ranura sale desconectada,
    ///   pero con el resto de datos, igual que en C#.
    /// - `vivo`: si el mando está sincronizado o ya manda informes buenos (`isSynced() ||
    ///   IsAlive()`). Si no, la ranura sale como reservada.
    /// - `estado`: el último informe del mando, para la batería y para saber si está activo.
    pub fn de_mando(
        ranura: u8,
        mac: [u8; 6],
        conexion: Conexion,
        vivo: bool,
        estado: &Estado,
    ) -> Self {
        let estado_ranura = if mac == [0; 6] {
            EstadoRanura::Desconectado
        } else if vivo {
            EstadoRanura::Conectado
        } else {
            EstadoRanura::Reservado
        };
        Meta {
            ranura,
            estado: estado_ranura,
            modelo: Modelo::Ds4,
            conexion: conexion.into(),
            mac: Some(mac),
            bateria: NivelBateria::desde_porcentaje(estado.bateria, estado.cargando),
            activo: !inactivo(estado),
        }
    }

    fn mac_o_ceros(&self) -> [u8; 6] {
        self.mac.unwrap_or([0; 6])
    }
}

/// Lee una MAC como la guarda el mando (`AA:BB:CC:DD:EE:FF`), con o sin separadores (`:` o
/// `-`). `None` si no son seis bytes en hexadecimal.
pub fn leer_mac(texto: &str) -> Option<[u8; 6]> {
    let limpio: String = texto
        .trim()
        .chars()
        .filter(|c| *c != ':' && *c != '-')
        .collect();
    if limpio.len() != 12 || !limpio.is_ascii() {
        return None;
    }
    let mut mac = [0u8; 6];
    for (i, b) in mac.iter_mut().enumerate() {
        *b = u8::from_str_radix(&limpio[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(mac)
}

/// Si no se está tocando ningún control, como `DS5Device.isDS5Idle`.
pub fn inactivo(e: &Estado) -> bool {
    const BOTONES: u64 = boton::CUADRADO
        | boton::EQUIS
        | boton::CIRCULO
        | boton::TRIANGULO
        | boton::ARRIBA
        | boton::IZQUIERDA
        | boton::ABAJO
        | boton::DERECHA
        | boton::L3
        | boton::R3
        | boton::L1
        | boton::R1
        | boton::CREAR
        | boton::OPCIONES
        | boton::PS
        | boton::TOQUE1
        | boton::TOQUE2
        | boton::PANEL;
    const HOLGURA: u8 = 64;
    let fuera = |v: u8| v <= 127 - HOLGURA || v >= 128 + HOLGURA;
    !(e.pulsado(BOTONES)
        || e.l2 != 0
        || e.r2 != 0
        || fuera(e.lx)
        || fuera(e.ly)
        || fuera(e.rx)
        || fuera(e.ry))
}

/// Un dedo en el panel táctil, como va en el paquete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToqueDsu {
    pub activo: bool,
    pub id: u8,
    pub x: u16,
    pub y: u16,
}

/// El giroscopio y el acelerómetro de un informe. Se guardan en `f64` porque el suavizado
/// trabaja en doble precisión, como en C#; al paquete van como `f32`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MovimientoDsu {
    /// Microsegundos acumulados del mando (`totalMicroSec`).
    pub total_us: u64,
    /// En g. El eje Z se manda cambiado de signo.
    pub acel_x: f64,
    pub acel_y: f64,
    pub acel_z: f64,
    /// En grados por segundo.
    pub vel_pitch: f64,
    pub vel_yaw: f64,
    pub vel_roll: f64,
}

/// Los controles de un informe tal como los necesita el paquete de datos. Se saca de un
/// [`Estado`] con [`DatosMando::desde_estado`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DatosMando {
    /// `PacketCounter`.
    pub contador_paquetes: u32,
    /// Cruceta, opciones, sticks pulsados y crear: izquierda 0x80, abajo 0x40, derecha 0x20,
    /// arriba 0x10, opciones 0x08, R3 0x04, L3 0x02, crear 0x01.
    pub botones1: u8,
    /// Cuadrado 0x80, equis 0x40, círculo 0x20, triángulo 0x10, R1 0x08, L1 0x04, R2 0x02,
    /// L2 0x01.
    pub botones2: u8,
    pub ps: bool,
    /// El clic del panel táctil (`TouchButton`).
    pub panel: bool,
    /// Los sticks como vienen del mando; el eje Y se invierte al escribir el paquete.
    pub lx: u8,
    pub ly: u8,
    pub rx: u8,
    pub ry: u8,
    pub l2: u8,
    pub r2: u8,
    pub toques: [ToqueDsu; 2],
    /// Segundos desde el informe anterior (`elapsedTime`): lo usa el suavizado.
    pub transcurrido: f64,
    /// `None` si el informe no trae movimiento: la marca de tiempo y los ejes van a cero.
    pub movimiento: Option<MovimientoDsu>,
}

impl DatosMando {
    /// Saca lo que manda C# del estado decodificado del mando.
    pub fn desde_estado(e: &Estado) -> Self {
        let bit = |b: u64, valor: u8| if e.pulsado(b) { valor } else { 0 };
        let botones1 = bit(boton::IZQUIERDA, 0x80)
            | bit(boton::ABAJO, 0x40)
            | bit(boton::DERECHA, 0x20)
            | bit(boton::ARRIBA, 0x10)
            | bit(boton::OPCIONES, 0x08)
            | bit(boton::R3, 0x04)
            | bit(boton::L3, 0x02)
            | bit(boton::CREAR, 0x01);
        let botones2 = bit(boton::CUADRADO, 0x80)
            | bit(boton::EQUIS, 0x40)
            | bit(boton::CIRCULO, 0x20)
            | bit(boton::TRIANGULO, 0x10)
            | bit(boton::R1, 0x08)
            | bit(boton::L1, 0x04)
            | bit(boton::R2, 0x02)
            | bit(boton::L2, 0x01);
        let toque = |i: usize| ToqueDsu {
            activo: e.toques[i].activo,
            id: e.toques[i].id,
            x: e.toques[i].x as u16,
            y: e.toques[i].y as u16,
        };
        DatosMando {
            contador_paquetes: e.contador_paquetes,
            botones1,
            botones2,
            ps: e.pulsado(boton::PS),
            panel: e.pulsado(boton::PANEL),
            lx: e.lx,
            ly: e.ly,
            rx: e.rx,
            ry: e.ry,
            l2: e.l2,
            r2: e.r2,
            toques: [toque(0), toque(1)],
            transcurrido: e.transcurrido,
            movimiento: e.movimiento.map(|m| MovimientoDsu {
                total_us: e.total_us,
                acel_x: m.acel_x_g,
                acel_y: m.acel_y_g,
                acel_z: m.acel_z_g,
                vel_pitch: m.vel_pitch,
                vel_yaw: m.vel_yaw,
                vel_roll: m.vel_roll,
            }),
        }
    }
}

/// Escribe la cabecera, el cuerpo y el CRC, como `BeginPacket` y `FinishPacket`. El cuerpo
/// empieza por el tipo de mensaje.
pub fn armar_paquete(id_servidor: u32, cuerpo: &[u8]) -> Vec<u8> {
    let mut p = Vec::with_capacity(LARGO_CABECERA + cuerpo.len());
    p.extend_from_slice(b"DSUS");
    p.extend_from_slice(&VERSION_PROTOCOLO.to_le_bytes());
    p.extend_from_slice(&(cuerpo.len() as u16).to_le_bytes());
    p.extend_from_slice(&[0; 4]);
    p.extend_from_slice(&id_servidor.to_le_bytes());
    p.extend_from_slice(cuerpo);
    firmar(&mut p);
    p
}

/// Pone a cero el campo del CRC, lo calcula sobre el paquete entero y lo escribe.
fn firmar(p: &mut [u8]) {
    p[8..12].fill(0);
    let crc = crc32(p);
    p[8..12].copy_from_slice(&crc.to_le_bytes());
}

/// La respuesta a una petición de versión.
pub fn respuesta_version(id_servidor: u32) -> Vec<u8> {
    let mut cuerpo = [0u8; 8];
    cuerpo[..4].copy_from_slice(&MENSAJE_VERSION.to_le_bytes());
    cuerpo[4..6].copy_from_slice(&VERSION_PROTOCOLO.to_le_bytes());
    armar_paquete(id_servidor, &cuerpo)
}

/// Los 11 bytes que describen el mando, iguales en la respuesta de puertos y en la de datos:
/// ranura, estado, modelo, conexión, MAC y batería.
fn escribir_meta(destino: &mut [u8], meta: &Meta) {
    destino[0] = meta.ranura;
    destino[1] = meta.estado as u8;
    destino[2] = meta.modelo as u8;
    destino[3] = meta.conexion as u8;
    destino[4..10].copy_from_slice(&meta.mac_o_ceros());
    destino[10] = meta.bateria as u8;
}

/// La respuesta con la información de un puerto. El último byte va a cero (C# no manda si
/// está activo aquí).
pub fn respuesta_puerto(id_servidor: u32, meta: &Meta) -> Vec<u8> {
    let mut cuerpo = [0u8; 16];
    cuerpo[..4].copy_from_slice(&MENSAJE_PUERTOS.to_le_bytes());
    escribir_meta(&mut cuerpo[4..15], meta);
    armar_paquete(id_servidor, &cuerpo)
}

/// La respuesta con los datos del mando (`PadDataRspPacket`), los 100 bytes.
pub fn respuesta_datos(id_servidor: u32, meta: &Meta, d: &DatosMando) -> [u8; LARGO_DATOS] {
    let mut p = [0u8; LARGO_DATOS];
    p[..4].copy_from_slice(b"DSUS");
    p[4..6].copy_from_slice(&VERSION_PROTOCOLO.to_le_bytes());
    p[6..8].copy_from_slice(&((LARGO_DATOS - LARGO_CABECERA) as u16).to_le_bytes());
    p[12..16].copy_from_slice(&id_servidor.to_le_bytes());
    p[16..20].copy_from_slice(&MENSAJE_DATOS.to_le_bytes());

    escribir_meta(&mut p[20..31], meta);
    p[31] = meta.activo as u8;
    p[32..36].copy_from_slice(&d.contador_paquetes.to_le_bytes());

    let analogico = |b: u8, mascara: u8| if b & mascara != 0 { 0xFF } else { 0 };
    p[36] = d.botones1;
    p[37] = d.botones2;
    p[38] = d.ps as u8;
    p[39] = d.panel as u8;
    // El eje Y de los sticks se invierte por convención.
    p[40] = d.lx;
    p[41] = 255 - d.ly;
    p[42] = d.rx;
    p[43] = 255 - d.ry;
    // El DualSense no tiene botones analógicos: 0xFF si está pulsado.
    p[44] = analogico(d.botones1, 0x80); // izquierda
    p[45] = analogico(d.botones1, 0x40); // abajo
    p[46] = analogico(d.botones1, 0x20); // derecha
    p[47] = analogico(d.botones1, 0x10); // arriba
    p[48] = analogico(d.botones2, 0x80); // cuadrado
    p[49] = analogico(d.botones2, 0x40); // equis
    p[50] = analogico(d.botones2, 0x20); // círculo
    p[51] = analogico(d.botones2, 0x10); // triángulo
    p[52] = analogico(d.botones2, 0x08); // R1
    p[53] = analogico(d.botones2, 0x04); // L1
    p[54] = d.r2;
    p[55] = d.l2;

    for (i, t) in d.toques.iter().enumerate() {
        let o = 56 + i * 6;
        p[o] = t.activo as u8;
        p[o + 1] = t.id;
        p[o + 2..o + 4].copy_from_slice(&t.x.to_le_bytes());
        p[o + 4..o + 6].copy_from_slice(&t.y.to_le_bytes());
    }

    if let Some(m) = &d.movimiento {
        p[68..76].copy_from_slice(&m.total_us.to_le_bytes());
        let ejes = [
            m.acel_x,
            m.acel_y,
            -m.acel_z,
            m.vel_pitch,
            m.vel_yaw,
            m.vel_roll,
        ];
        for (i, v) in ejes.iter().enumerate() {
            let o = 76 + i * 4;
            p[o..o + 4].copy_from_slice(&(*v as f32).to_le_bytes());
        }
    }

    firmar(&mut p);
    p
}

/// Lo que pide un cliente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Peticion {
    /// La versión del protocolo.
    Version,
    /// La información de estas ranuras (todas por debajo de [`RANURAS`]).
    Puertos(Vec<u8>),
    /// Suscribirse a los datos de los mandos durante cinco segundos.
    ///
    /// - `banderas` a 0: todos los mandos.
    /// - bit 0: el de la ranura `ranura`.
    /// - bit 1: el de la MAC `mac`.
    Datos {
        banderas: u8,
        ranura: u8,
        mac: [u8; 6],
    },
}

/// Una petición ya comprobada, con el id del cliente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeticionCliente {
    pub id_cliente: u32,
    pub peticion: Peticion,
}

fn u16_en(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_en(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

/// Lee una petición como `ProcessIncoming`. `None` si hay que ignorarla: no empieza por
/// `DSUC`, la versión es mayor que 1001, el largo no cuadra, el CRC está mal, el tipo es
/// desconocido o le faltan bytes. Lo que sobre detrás del largo declarado se descarta.
pub fn leer_peticion(datos: &[u8]) -> Option<PeticionCliente> {
    if datos.len() < LARGO_CABECERA + 4 || &datos[..4] != b"DSUC" {
        return None;
    }
    if u16_en(datos, 4) > VERSION_PROTOCOLO {
        return None;
    }
    let largo = u16_en(datos, 6) as usize + LARGO_CABECERA;
    if largo > datos.len() || largo < LARGO_CABECERA + 4 {
        return None;
    }
    let mut p = datos[..largo].to_vec();
    let crc = u32_en(&p, 8);
    p[8..12].fill(0);
    if crc32(&p) != crc {
        return None;
    }
    let id_cliente = u32_en(&p, 12);
    let peticion = match u32_en(&p, 16) {
        MENSAJE_VERSION => Peticion::Version,
        MENSAJE_PUERTOS => {
            let n = i32::from_le_bytes(p.get(20..24)?.try_into().ok()?);
            if !(0..=RANURAS as i32).contains(&n) {
                return None;
            }
            let ranuras = p.get(24..24 + n as usize)?;
            if ranuras.iter().any(|&r| r as usize >= RANURAS) {
                return None;
            }
            Peticion::Puertos(ranuras.to_vec())
        }
        MENSAJE_DATOS => {
            let resto = p.get(20..28)?;
            let mut mac = [0u8; 6];
            mac.copy_from_slice(&resto[2..8]);
            Peticion::Datos {
                banderas: resto[0],
                ranura: resto[1],
                mac,
            }
        }
        _ => return None,
    };
    Some(PeticionCliente {
        id_cliente,
        peticion,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ds5_informes::{Movimiento, Toque};

    // Los paquetes esperados se han armado aparte siguiendo los campos de `PadDataRspPacket` y
    // de `ProcessIncoming`, con el CRC32 de zip.

    const VERSION: [u8; 24] = [
        0x44, 0x53, 0x55, 0x53, 0xE9, 0x03, 0x08, 0x00, 0xFB, 0x24, 0xA7, 0x81, 0x78, 0x56, 0x34,
        0x12, 0x00, 0x00, 0x10, 0x00, 0xE9, 0x03, 0x00, 0x00,
    ];

    const PUERTO: [u8; 32] = [
        0x44, 0x53, 0x55, 0x53, 0xE9, 0x03, 0x10, 0x00, 0x6D, 0x85, 0xAD, 0x62, 0x78, 0x56, 0x34,
        0x12, 0x01, 0x00, 0x10, 0x00, 0x01, 0x02, 0x02, 0x02, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
        0x04, 0x00,
    ];

    const DATOS: [u8; 100] = [
        0x44, 0x53, 0x55, 0x53, 0xE9, 0x03, 0x54, 0x00, 0xC3, 0xBF, 0x6D, 0x48, 0xEF, 0xBE, 0xAD,
        0xDE, 0x02, 0x00, 0x10, 0x00, 0x00, 0x02, 0x02, 0x01, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
        0x05, 0x01, 0x04, 0x03, 0x02, 0x01, 0x89, 0x46, 0x01, 0x01, 0x0A, 0xEB, 0xC8, 0x00, 0xFF,
        0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x40, 0x01, 0x05, 0xE8, 0x03,
        0xF4, 0x01, 0x00, 0x06, 0x7F, 0x07, 0xAD, 0x03, 0x55, 0x44, 0x33, 0x22, 0x11, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x3F, 0x00, 0x00, 0x80, 0xBF, 0x00, 0x00, 0x80, 0xBE, 0x00, 0x00,
        0x28, 0x41, 0x00, 0x00, 0xA0, 0xC1, 0x00, 0x00, 0x44, 0x40,
    ];

    const DATOS_SIN_MOVIMIENTO: [u8; 100] = [
        0x44, 0x53, 0x55, 0x53, 0xE9, 0x03, 0x54, 0x00, 0x39, 0x20, 0x16, 0x4F, 0xEF, 0xBE, 0xAD,
        0xDE, 0x02, 0x00, 0x10, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x7F, 0x80, 0x7F, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    /// Una petición de datos de la ranura 2 del cliente 0xCAFEBABE.
    pub(crate) const PETICION_DATOS: [u8; 28] = [
        0x44, 0x53, 0x55, 0x43, 0xE9, 0x03, 0x0C, 0x00, 0xA8, 0x2F, 0xBE, 0x14, 0xBE, 0xBA, 0xFE,
        0xCA, 0x02, 0x00, 0x10, 0x00, 0x01, 0x02, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60,
    ];

    /// Arma una petición de cliente con su CRC.
    pub(crate) fn peticion(cuerpo: &[u8]) -> Vec<u8> {
        let mut p = armar_paquete(0x0102_0304, cuerpo);
        p[..4].copy_from_slice(b"DSUC");
        firmar(&mut p);
        p
    }

    fn estado_completo() -> Estado {
        let mut e = Estado {
            botones: boton::IZQUIERDA
                | boton::OPCIONES
                | boton::CREAR
                | boton::EQUIS
                | boton::R2
                | boton::L1
                | boton::PS
                | boton::PANEL,
            lx: 10,
            ly: 20,
            rx: 200,
            ry: 255,
            l2: 0x40,
            r2: 0xFF,
            contador_paquetes: 0x0102_0304,
            total_us: 0x11_2233_4455,
            transcurrido: 0.004,
            ..Default::default()
        };
        e.toques = [
            Toque {
                activo: true,
                id: 5,
                x: 1000,
                y: 500,
                crudo: 5,
            },
            Toque {
                activo: false,
                id: 6,
                x: 1919,
                y: 941,
                crudo: 0x86,
            },
        ];
        e.movimiento = Some(Movimiento {
            acel_x_g: 0.5,
            acel_y_g: -1.0,
            acel_z_g: 0.25,
            vel_pitch: 10.5,
            vel_yaw: -20.0,
            vel_roll: 3.0625,
            ..Default::default()
        });
        e
    }

    #[test]
    fn respuesta_de_version_byte_a_byte() {
        assert_eq!(respuesta_version(0x1234_5678), VERSION);
    }

    #[test]
    fn respuesta_de_puerto_byte_a_byte() {
        let meta = Meta {
            ranura: 1,
            estado: EstadoRanura::Conectado,
            modelo: Modelo::Ds4,
            conexion: TipoConexion::Bluetooth,
            mac: Some([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]),
            bateria: NivelBateria::Alta,
            activo: true,
        };
        assert_eq!(respuesta_puerto(0x1234_5678, &meta), PUERTO);
    }

    #[test]
    fn respuesta_de_datos_byte_a_byte() {
        let meta = Meta {
            ranura: 0,
            estado: EstadoRanura::Conectado,
            modelo: Modelo::Ds4,
            conexion: TipoConexion::Usb,
            mac: Some([1, 2, 3, 4, 5, 6]),
            bateria: NivelBateria::Llena,
            activo: true,
        };
        let d = DatosMando::desde_estado(&estado_completo());
        assert_eq!(d.botones1, 0x89);
        assert_eq!(d.botones2, 0x46);
        assert_eq!(respuesta_datos(0xDEAD_BEEF, &meta, &d), DATOS);
    }

    #[test]
    fn sin_movimiento_la_marca_y_los_ejes_van_a_cero() {
        let mut e = Estado {
            lx: 128,
            ly: 128,
            rx: 128,
            ry: 128,
            contador_paquetes: 7,
            // Se ignora porque no hay movimiento.
            total_us: 999,
            ..Default::default()
        };
        e.toques[0].activo = true;
        e.toques[1].activo = true;
        let p = respuesta_datos(
            0xDEAD_BEEF,
            &Meta::desconectado(3),
            &DatosMando::desde_estado(&e),
        );
        assert_eq!(p, DATOS_SIN_MOVIMIENTO);
    }

    #[test]
    fn el_crc_del_paquete_cuadra() {
        let p = respuesta_version(42);
        let mut sin_crc = p.clone();
        sin_crc[8..12].fill(0);
        assert_eq!(crc32(&sin_crc), u32_en(&p, 8));
    }

    #[test]
    fn lee_la_peticion_de_datos() {
        let p = leer_peticion(&PETICION_DATOS).unwrap();
        assert_eq!(p.id_cliente, 0xCAFE_BABE);
        assert_eq!(
            p.peticion,
            Peticion::Datos {
                banderas: 1,
                ranura: 2,
                mac: [0x10, 0x20, 0x30, 0x40, 0x50, 0x60],
            }
        );
    }

    #[test]
    fn ignora_lo_que_sobra_detras_del_largo() {
        let mut p = PETICION_DATOS.to_vec();
        p.extend_from_slice(&[0xAB; 20]);
        assert!(leer_peticion(&p).is_some());
    }

    #[test]
    fn rechaza_peticiones_malas() {
        // CRC mal.
        let mut p = PETICION_DATOS;
        p[8] ^= 1;
        assert_eq!(leer_peticion(&p), None);
        // Un dato cambiado sin rehacer el CRC.
        let mut p = PETICION_DATOS;
        p[21] = 3;
        assert_eq!(leer_peticion(&p), None);
        // Firma de servidor.
        let mut p = PETICION_DATOS;
        p[3] = b'S';
        assert_eq!(leer_peticion(&p), None);
        // Versión mayor.
        let mut p = peticion(&MENSAJE_VERSION.to_le_bytes());
        p[4..6].copy_from_slice(&1002u16.to_le_bytes());
        firmar(&mut p);
        assert_eq!(leer_peticion(&p), None);
        // Más corto que el largo declarado.
        assert_eq!(leer_peticion(&PETICION_DATOS[..27]), None);
        // Tipo desconocido.
        assert_eq!(leer_peticion(&peticion(&0x10_0003u32.to_le_bytes())), None);
        // Petición de datos sin la MAC.
        let mut cuerpo = MENSAJE_DATOS.to_le_bytes().to_vec();
        cuerpo.extend_from_slice(&[0, 0]);
        assert_eq!(leer_peticion(&peticion(&cuerpo)), None);
        // Basura.
        assert_eq!(leer_peticion(b"DSUC"), None);
        assert_eq!(leer_peticion(&[]), None);
    }

    #[test]
    fn lee_la_version_y_los_puertos() {
        let p = leer_peticion(&peticion(&MENSAJE_VERSION.to_le_bytes())).unwrap();
        assert_eq!(p.peticion, Peticion::Version);
        assert_eq!(p.id_cliente, 0x0102_0304);

        let cuerpo = |n: i32, ranuras: &[u8]| {
            let mut c = MENSAJE_PUERTOS.to_le_bytes().to_vec();
            c.extend_from_slice(&n.to_le_bytes());
            c.extend_from_slice(ranuras);
            peticion(&c)
        };
        let p = leer_peticion(&cuerpo(4, &[0, 1, 2, 3])).unwrap();
        assert_eq!(p.peticion, Peticion::Puertos(vec![0, 1, 2, 3]));
        let p = leer_peticion(&cuerpo(0, &[])).unwrap();
        assert_eq!(p.peticion, Peticion::Puertos(vec![]));
        // Una ranura que no existe invalida toda la petición.
        assert_eq!(leer_peticion(&cuerpo(2, &[0, 4])), None);
        assert_eq!(leer_peticion(&cuerpo(5, &[0, 1, 2, 3, 0])), None);
        assert_eq!(leer_peticion(&cuerpo(-1, &[])), None);
        // Faltan ranuras.
        assert_eq!(leer_peticion(&cuerpo(3, &[0, 1])), None);
    }

    #[test]
    fn nivel_de_bateria_como_csharp() {
        use NivelBateria::*;
        let casos = [
            (100, true, Cargada),
            (100, false, Llena),
            (95, true, Llena),
            (94, false, Alta),
            (70, false, Alta),
            (69, false, Media),
            (50, false, Media),
            (49, false, Baja),
            (20, false, Baja),
            (19, false, Agotandose),
            (5, false, Agotandose),
            (4, false, Ninguna),
        ];
        for (p, c, esperado) in casos {
            assert_eq!(NivelBateria::desde_porcentaje(p, c), esperado, "{p} {c}");
        }
    }

    #[test]
    fn meta_de_un_mando() {
        let mut e = Estado {
            lx: 128,
            ly: 128,
            rx: 128,
            ry: 128,
            bateria: 75,
            ..Default::default()
        };
        let mac = [1, 2, 3, 4, 5, 6];
        let m = Meta::de_mando(2, mac, Conexion::Bt, true, &e);
        assert_eq!(m.estado, EstadoRanura::Conectado);
        assert_eq!(m.modelo, Modelo::Ds4);
        assert_eq!(m.conexion, TipoConexion::Bluetooth);
        assert_eq!(m.bateria, NivelBateria::Alta);
        assert!(!m.activo);
        e.botones = boton::EQUIS;
        assert!(Meta::de_mando(2, mac, Conexion::Usb, true, &e).activo);
        assert_eq!(
            Meta::de_mando(2, mac, Conexion::Usb, false, &e).estado,
            EstadoRanura::Reservado
        );
        assert_eq!(
            Meta::de_mando(2, [0; 6], Conexion::Usb, true, &e).estado,
            EstadoRanura::Desconectado
        );
    }

    #[test]
    fn inactivo_como_csharp() {
        let mut e = Estado {
            lx: 127,
            ly: 128,
            rx: 100,
            ry: 191,
            ..Default::default()
        };
        assert!(inactivo(&e));
        e.ry = 192;
        assert!(!inactivo(&e));
        e.ry = 128;
        e.lx = 63;
        assert!(!inactivo(&e));
        e.lx = 64;
        assert!(inactivo(&e));
        e.l2 = 1;
        assert!(!inactivo(&e));
        e.l2 = 0;
        e.botones = boton::TOQUE1;
        assert!(!inactivo(&e));
        // El mute y los botones de los mandos Edge no cuentan.
        e.botones = boton::MUTE | boton::FN_IZQ;
        assert!(inactivo(&e));
    }

    #[test]
    fn lee_macs() {
        assert_eq!(
            leer_mac("AA:bb:0C:dd:EE:01"),
            Some([0xAA, 0xBB, 0x0C, 0xDD, 0xEE, 0x01])
        );
        assert_eq!(leer_mac("aabb0cddee01"), leer_mac("AA-BB-0C-DD-EE-01"));
        assert_eq!(leer_mac("AA:BB:CC:DD:EE"), None);
        assert_eq!(leer_mac("ZZ:BB:CC:DD:EE:FF"), None);
        assert_eq!(leer_mac(""), None);
    }
}
