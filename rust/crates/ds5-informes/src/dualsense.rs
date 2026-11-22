//! Los informes de entrada del DualSense, como `ReadInput` en `DualSenseDevice.cs`.

use crate::movimiento::{Calibracion, Movimiento, Sensores};
use crate::{crc, Conexion};

/// Los botones, en el orden de `ReportRecorder.ButtonBits` (el bit 0 es el cuadrado).
pub mod boton {
    pub const CUADRADO: u64 = 1 << 0;
    pub const TRIANGULO: u64 = 1 << 1;
    pub const CIRCULO: u64 = 1 << 2;
    pub const EQUIS: u64 = 1 << 3;
    pub const ARRIBA: u64 = 1 << 4;
    pub const ABAJO: u64 = 1 << 5;
    pub const IZQUIERDA: u64 = 1 << 6;
    pub const DERECHA: u64 = 1 << 7;
    pub const L1: u64 = 1 << 8;
    pub const L2: u64 = 1 << 9;
    pub const L3: u64 = 1 << 10;
    pub const R1: u64 = 1 << 11;
    pub const R2: u64 = 1 << 12;
    pub const R3: u64 = 1 << 13;
    pub const CREAR: u64 = 1 << 14;
    pub const OPCIONES: u64 = 1 << 15;
    pub const PS: u64 = 1 << 16;
    pub const MUTE: u64 = 1 << 17;
    pub const TOQUE1: u64 = 1 << 18;
    pub const TOQUE2: u64 = 1 << 19;
    pub const PANEL: u64 = 1 << 20;
    pub const PANEL_DERECHA: u64 = 1 << 21;
    pub const PANEL_IZQUIERDA: u64 = 1 << 22;
    pub const UN_DEDO: u64 = 1 << 23;
    pub const DOS_DEDOS: u64 = 1 << 24;
    pub const PANEL_SALIDA: u64 = 1 << 25;
    pub const CAPTURA: u64 = 1 << 26;
    pub const LATERAL_IZQ: u64 = 1 << 27;
    pub const LATERAL_DER: u64 = 1 << 28;
    pub const FN_IZQ: u64 = 1 << 29;
    pub const FN_DER: u64 = 1 << 30;
    pub const PALETA_IZQ: u64 = 1 << 31;
    pub const PALETA_DER: u64 = 1 << 32;
}

/// Ancho del panel táctil (`DS5Touchpad.RESOLUTION_X_MAX`).
pub const PANEL_ANCHO: i32 = 1920;
/// Alto del panel táctil.
pub const PANEL_ALTO: i32 = 942;

/// Largo del informe de entrada por Bluetooth, con su CRC.
pub const LARGO_ENTRADA_BT: usize = 78;
/// Largo del informe de entrada por USB.
pub const LARGO_ENTRADA_USB: usize = 64;

/// Un dedo en el panel táctil.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Toque {
    pub activo: bool,
    pub id: u8,
    pub x: i16,
    pub y: i16,
    /// El byte tal cual: el bit 7 a 1 es que no hay dedo.
    pub crudo: u8,
}

/// El estado decodificado de un informe, como `DS5State`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Estado {
    pub botones: u64,
    pub lx: u8,
    pub ly: u8,
    pub rx: u8,
    pub ry: u8,
    pub l2: u8,
    pub r2: u8,
    pub l2_crudo: u8,
    pub r2_crudo: u8,
    /// Porcentaje: el nivel del mando (de 0 a 10) por diez más cinco, o 100 si está lleno.
    pub bateria: u8,
    pub cargando: bool,
    /// El byte de la batería tal cual: nivel abajo y estado de carga arriba.
    pub estado_bateria: u8,
    /// Bit 0: auriculares; bit 1: micrófono.
    pub audio: u8,
    /// De 0 a 127, como en el DS4.
    pub contador_tramas: u8,
    pub contador_paquetes: u32,
    pub contador_panel: u8,
    pub toque1_id: u8,
    pub toque2_id: u8,
    pub toques: [Toque; 2],
    pub marca_ds5: u16,
    /// Segundos desde el informe anterior.
    pub transcurrido: f64,
    pub total_us: u64,
    /// El mando ha repetido la marca de tiempo y el tiempo sale del reloj del sistema.
    pub marca_repetida: bool,
    /// Cuánto pasa cada stick del borde exterior (`OutputLSOuter`, `OutputRSOuter`). El
    /// decodificador los deja a 0; los calcula el mapeo.
    pub exterior_ls: u8,
    pub exterior_rs: u8,
    pub movimiento: Option<Movimiento>,
    /// `SASteeringWheelEmulationUnit`: lo que sale por el eje del volante. Lo pone el mapeo.
    pub volante: i32,
}

impl Estado {
    pub fn pulsado(&self, b: u64) -> bool {
        self.botones & b != 0
    }
}

/// Decodifica los informes de un mando. Guarda lo que hace falta del informe anterior: la marca
/// de tiempo, los contadores y la calibración del giroscopio.
#[derive(Debug, Clone)]
pub struct Decodificador {
    conexion: Conexion,
    sensores: Sensores,
    marca_iniciada: bool,
    marca_anterior: u32,
    reloj_anterior_us: i64,
    anterior: Estado,
}

impl Decodificador {
    /// `ahora_us` es cuando empieza a leer, en el mismo reloj que se pase a `decodificar`.
    pub fn new(conexion: Conexion, calibracion: Option<Calibracion>, ahora_us: i64) -> Self {
        Decodificador {
            conexion,
            sensores: Sensores::new(calibracion, ahora_us),
            marca_iniciada: false,
            marca_anterior: 0,
            reloj_anterior_us: 0,
            anterior: Estado::default(),
        }
    }

    pub fn conexion(&self) -> Conexion {
        self.conexion
    }

    pub fn sensores(&mut self) -> &mut Sensores {
        &mut self.sensores
    }

    /// Si es un informe que hay que decodificar: por Bluetooth tiene que ser el 0x31 con el CRC
    /// bien.
    pub fn valido(&self, informe: &[u8]) -> bool {
        match self.conexion {
            Conexion::Usb => informe.len() >= LARGO_ENTRADA_USB,
            Conexion::Bt => {
                informe.len() >= LARGO_ENTRADA_BT
                    && informe[0] == 0x31
                    && crc::valido(crc::CABECERA_ENTRADA, &informe[..LARGO_ENTRADA_BT])
            }
        }
    }

    /// Corrige el tiempo acumulado del último informe. Solo sirve para las pruebas: cuando el
    /// mando repite la marca de tiempo, C# usó un reloj que no está en la grabación.
    #[doc(hidden)]
    pub fn fijar_total_us(&mut self, total_us: u64) {
        self.anterior.total_us = total_us;
    }

    /// Un informe con el CRC mal: no se decodifica, pero cuenta como paquete perdido.
    pub fn perdido(&mut self) {
        self.anterior.contador_paquetes = self.anterior.contador_paquetes.wrapping_add(1);
    }

    /// Decodifica un informe ya validado. `ahora_us` es cuando se ha leído: hace falta para el
    /// tiempo entre informes si el mando repite la marca y para la calibración continua.
    pub fn decodificar(&mut self, r: &[u8], ahora_us: i64) -> Estado {
        let o = match self.conexion {
            Conexion::Usb => 0,
            Conexion::Bt => 1,
        };
        let ms_reloj = (ahora_us - self.reloj_anterior_us) as f64 / 1000.0;
        self.reloj_anterior_us = ahora_us;

        let mut e = Estado {
            contador_paquetes: self.anterior.contador_paquetes.wrapping_add(1),
            lx: r[1 + o],
            ly: r[2 + o],
            rx: r[3 + o],
            ry: r[4 + o],
            l2: r[5 + o],
            r2: r[6 + o],
            l2_crudo: r[5 + o],
            r2_crudo: r[6 + o],
            contador_tramas: r[7 + o] % 128,
            ..Default::default()
        };
        let mut b = 0u64;
        let mut pon = |cond: bool, bit: u64| {
            if cond {
                b |= bit;
            }
        };
        let x = r[8 + o];
        pon(x & 0x80 != 0, boton::TRIANGULO);
        pon(x & 0x40 != 0, boton::CIRCULO);
        pon(x & 0x20 != 0, boton::EQUIS);
        pon(x & 0x10 != 0, boton::CUADRADO);
        let (arriba, abajo, izq, der) = match x & 0x0F {
            0 => (true, false, false, false),
            1 => (true, false, false, true),
            2 => (false, false, false, true),
            3 => (false, true, false, true),
            4 => (false, true, false, false),
            5 => (false, true, true, false),
            6 => (false, false, true, false),
            7 => (true, false, true, false),
            _ => (false, false, false, false),
        };
        pon(arriba, boton::ARRIBA);
        pon(abajo, boton::ABAJO);
        pon(izq, boton::IZQUIERDA);
        pon(der, boton::DERECHA);
        let x = r[9 + o];
        pon(x & 0x80 != 0, boton::R3);
        pon(x & 0x40 != 0, boton::L3);
        pon(x & 0x20 != 0, boton::OPCIONES);
        pon(x & 0x10 != 0, boton::CREAR);
        pon(x & 0x08 != 0, boton::R2);
        pon(x & 0x04 != 0, boton::L2);
        pon(x & 0x02 != 0, boton::R1);
        pon(x & 0x01 != 0, boton::L1);
        let x = r[10 + o];
        pon(x & 0x01 != 0, boton::PS);
        pon(x & 0x02 != 0, boton::PANEL);
        pon(x & 0x02 != 0, boton::PANEL_SALIDA);
        pon(x & 0x04 != 0, boton::MUTE);
        pon(x & 0x10 != 0, boton::FN_IZQ);
        pon(x & 0x20 != 0, boton::FN_DER);
        pon(x & 0x40 != 0, boton::PALETA_IZQ);
        pon(x & 0x80 != 0, boton::PALETA_DER);

        // Batería: nivel de 0 a 10 abajo, estado arriba (0 descargando, 1 cargando, 2 lleno).
        e.audio = r[54 + o];
        let bateria = r[53 + o];
        e.estado_bateria = bateria;
        e.cargando = bateria >> 4 == 1;
        e.bateria = if bateria >> 4 == 2 {
            100
        } else {
            ((bateria & 0x0F) as u32 * 10 + 5).min(100) as u8
        };

        // La marca de tiempo del mando va en tercios de microsegundo.
        let marca = u32::from_le_bytes([r[28 + o], r[29 + o], r[30 + o], r[31 + o]]);
        let delta = if !self.marca_iniciada {
            self.marca_iniciada = true;
            marca / 3
        } else {
            marca.wrapping_sub(self.marca_anterior) / 3
        };
        let dt = if delta != 0 {
            e.total_us = self.anterior.total_us.wrapping_add(delta as u64);
            0.000001 * delta as f64
        } else {
            // Marca repetida: se usa el reloj del sistema.
            e.marca_repetida = true;
            let dt = ms_reloj * 0.001;
            e.total_us = self
                .anterior
                .total_us
                .wrapping_add((dt * 1_000_000.0) as u32 as u64);
            dt
        };
        e.transcurrido = dt;
        e.marca_ds5 = ((marca / 16) % u16::MAX as u32) as u16;
        self.marca_anterior = marca;

        let toque = |i: usize| Toque {
            crudo: r[i],
            id: r[i] & 0x7F,
            activo: r[i] & 0x80 == 0,
            x: ((((r[i + 2] & 0x0F) as u16) << 8) | r[i + 1] as u16) as i16,
            y: (((r[i + 3] as u16) << 4) | ((r[i + 2] & 0xF0) as u16 >> 4)) as i16,
        };
        let p = 33 + o;
        e.toques = [toque(p), toque(p + 4)];
        e.contador_panel = r[p + 8];
        let t1 = r[p] >> 7 == 0;
        let t2 = r[p + 4] >> 7 == 0;
        e.toque1_id = r[p] & 0x7F;
        e.toque2_id = r[p + 4] & 0x7F;
        pon(t1, boton::TOQUE1);
        pon(t2, boton::TOQUE2);
        pon(t1 || t2, boton::UN_DEDO);
        pon(t1 && t2, boton::DOS_DEDOS);
        let x = (((r[p + 2] & 0x0F) as i32) << 8) | r[p + 1] as i32;
        let limite = PANEL_ANCHO * 2 / 5;
        pon(x < limite, boton::PANEL_IZQUIERDA);
        pon(x >= limite, boton::PANEL_DERECHA);
        e.botones = b;

        e.movimiento = self.sensores.procesar(&r[16 + o..28 + o], dt, ahora_us);
        self.anterior = e.clone();
        e
    }
}
