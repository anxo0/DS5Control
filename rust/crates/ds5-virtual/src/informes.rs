//! Los informes que se mandan a los mandos virtuales y lo que el juego devuelve. No depende de
//! Windows: se prueba sin ViGEmBus.

use ds5_informes::{boton, Estado};

/// Los botones del mando de Xbox 360 (`XUSB_BUTTON`).
pub mod boton_xbox {
    pub const ARRIBA: u16 = 0x0001;
    pub const ABAJO: u16 = 0x0002;
    pub const IZQUIERDA: u16 = 0x0004;
    pub const DERECHA: u16 = 0x0008;
    pub const START: u16 = 0x0010;
    pub const BACK: u16 = 0x0020;
    pub const STICK_IZQ: u16 = 0x0040;
    pub const STICK_DER: u16 = 0x0080;
    pub const LB: u16 = 0x0100;
    pub const RB: u16 = 0x0200;
    pub const GUIA: u16 = 0x0400;
    pub const A: u16 = 0x1000;
    pub const B: u16 = 0x2000;
    pub const X: u16 = 0x4000;
    pub const Y: u16 = 0x8000;
}

/// Los botones del DS4 (`DS4_BUTTONS`). La cruceta va en los cuatro bits bajos.
pub mod boton_ds4 {
    pub const CUADRADO: u16 = 1 << 4;
    pub const EQUIS: u16 = 1 << 5;
    pub const CIRCULO: u16 = 1 << 6;
    pub const TRIANGULO: u16 = 1 << 7;
    pub const L1: u16 = 1 << 8;
    pub const R1: u16 = 1 << 9;
    pub const L2: u16 = 1 << 10;
    pub const R2: u16 = 1 << 11;
    pub const SHARE: u16 = 1 << 12;
    pub const OPTIONS: u16 = 1 << 13;
    pub const L3: u16 = 1 << 14;
    pub const R3: u16 = 1 << 15;
    /// La cruceta suelta.
    pub const CRUCETA_SUELTA: u16 = 8;
    /// Los botones especiales (`DS4_SPECIAL_BUTTONS`).
    pub const PS: u8 = 1;
    pub const PANEL: u8 = 2;
}

const PULSADO: fn(u64, u64) -> bool = |b, m| b & m != 0;

/// El informe de Xbox 360 (`XUSB_REPORT`, 12 bytes).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InformeXbox {
    pub botones: u16,
    pub gatillo_izq: u8,
    pub gatillo_der: u8,
    pub lx: i16,
    pub ly: i16,
    pub rx: i16,
    pub ry: i16,
}

/// Como `Xbox360OutDevice.AxisScale`: de 0..255 a -32768..32767, con `invertir` para la Y.
pub fn escala_eje(valor: u8, invertir: bool) -> i16 {
    let v = valor as i32 - 0x80;
    let reciproco = if v >= 0 { 1.0 / 127f32 } else { 1.0 / 128f32 };
    let mut t = v as f32 * reciproco;
    if invertir {
        t = -t;
    }
    t = (t + 1.0) * 0.5;
    (t * 65535.0 + -32768.0) as i16
}

impl InformeXbox {
    /// Como `Xbox360OutDevice.ConvertandSendReport` sin volante.
    pub fn desde(e: &Estado) -> InformeXbox {
        use boton_xbox::*;
        let b = e.botones;
        let mut botones = 0u16;
        for (m, x) in [
            (boton::CREAR, BACK),
            (boton::L3, STICK_IZQ),
            (boton::R3, STICK_DER),
            (boton::OPCIONES, START),
            (boton::ARRIBA, ARRIBA),
            (boton::DERECHA, DERECHA),
            (boton::ABAJO, ABAJO),
            (boton::IZQUIERDA, IZQUIERDA),
            (boton::L1, LB),
            (boton::R1, RB),
            (boton::TRIANGULO, Y),
            (boton::CIRCULO, B),
            (boton::EQUIS, A),
            (boton::CUADRADO, X),
            (boton::PS, GUIA),
        ] {
            if PULSADO(b, m) {
                botones |= x;
            }
        }
        InformeXbox {
            botones,
            gatillo_izq: e.l2,
            gatillo_der: e.r2,
            lx: escala_eje(e.lx, false),
            ly: escala_eje(e.ly, true),
            rx: escala_eje(e.rx, false),
            ry: escala_eje(e.ry, true),
        }
    }

    pub fn bytes(&self) -> [u8; 12] {
        let mut r = [0u8; 12];
        r[0..2].copy_from_slice(&self.botones.to_le_bytes());
        r[2] = self.gatillo_izq;
        r[3] = self.gatillo_der;
        r[4..6].copy_from_slice(&self.lx.to_le_bytes());
        r[6..8].copy_from_slice(&self.ly.to_le_bytes());
        r[8..10].copy_from_slice(&self.rx.to_le_bytes());
        r[10..12].copy_from_slice(&self.ry.to_le_bytes());
        r
    }
}

/// El eje del mando virtual por el que sale el volante (`SASteeringWheelEmulationAxisType`, sin
/// los de vJoy, que no se mudan).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EjeVolante {
    LX,
    LY,
    RX,
    RY,
    /// Girar a un lado aprieta un gatillo y al otro el otro.
    L2R2,
}

impl InformeXbox {
    /// La parte del volante de `Xbox360OutDevice.ConvertandSendReport`: su eje sale tal cual,
    /// sin escalar. En los gatillos, el valor negativo se corta a un byte, como en C#.
    pub fn poner_volante(&mut self, eje: EjeVolante, unidad: i32) {
        match eje {
            EjeVolante::LX => self.lx = unidad as i16,
            EjeVolante::LY => self.ly = unidad as i16,
            EjeVolante::RX => self.rx = unidad as i16,
            EjeVolante::RY => self.ry = unidad as i16,
            EjeVolante::L2R2 => {
                self.gatillo_izq = 0;
                self.gatillo_der = 0;
                if unidad >= 0 {
                    self.gatillo_izq = unidad as u8;
                } else {
                    self.gatillo_der = unidad as u8;
                }
            }
        }
    }
}

/// La parte del volante de `DS4OutDeviceExt.ConvertandSendReport`.
pub fn poner_volante_ds4(r: &mut [u8; LARGO_DS4_EX], eje: EjeVolante, unidad: i32) {
    match eje {
        EjeVolante::LX => r[0] = unidad as u8,
        EjeVolante::LY => r[1] = unidad as u8,
        EjeVolante::RX => r[2] = unidad as u8,
        EjeVolante::RY => r[3] = unidad as u8,
        EjeVolante::L2R2 => {
            r[7] = 0;
            r[8] = 0;
            if unidad >= 0 {
                r[7] = unidad as u8;
            } else {
                r[8] = unidad as u8;
            }
        }
    }
}

/// La cruceta del DS4 en el orden de comprobación de C#.
pub fn cruceta(b: u64) -> u16 {
    let (ar, ab, iz, de) = (
        PULSADO(b, boton::ARRIBA),
        PULSADO(b, boton::ABAJO),
        PULSADO(b, boton::IZQUIERDA),
        PULSADO(b, boton::DERECHA),
    );
    if ar && de {
        1
    } else if ar && iz {
        7
    } else if ar {
        0
    } else if de && ab {
        3
    } else if de {
        2
    } else if ab && iz {
        5
    } else if ab {
        4
    } else if iz {
        6
    } else {
        boton_ds4::CRUCETA_SUELTA
    }
}

/// El informe extendido del DS4 (`DS4_REPORT_EX`, 63 bytes): botones, sticks, giroscopio,
/// panel táctil y batería. ViGEmBus le pone delante el número de informe.
pub const LARGO_DS4_EX: usize = 63;

/// Como `DS4OutDeviceExt.ConvertandSendReport` sin volante.
pub fn informe_ds4(e: &Estado) -> [u8; LARGO_DS4_EX] {
    use boton_ds4::*;
    let b = e.botones;
    let mut botones = 0u16;
    for (m, x) in [
        (boton::CREAR, SHARE),
        (boton::L3, L3),
        (boton::R3, R3),
        (boton::OPCIONES, OPTIONS),
        (boton::L1, L1),
        (boton::R1, R1),
        (boton::L2, L2),
        (boton::R2, R2),
        (boton::TRIANGULO, TRIANGULO),
        (boton::CIRCULO, CIRCULO),
        (boton::EQUIS, EQUIS),
        (boton::CUADRADO, CUADRADO),
    ] {
        if PULSADO(b, m) {
            botones |= x;
        }
    }
    botones |= cruceta(b);
    let mut especiales = 0u8;
    if PULSADO(b, boton::PS) {
        especiales |= PS;
    }
    // En C# es `OutputTouchButton`, que sin mapeo es el clic del panel.
    if PULSADO(b, boton::PANEL) {
        especiales |= PANEL;
    }

    let mut r = [0u8; LARGO_DS4_EX];
    r[0] = e.lx;
    r[1] = e.ly;
    r[2] = e.rx;
    r[3] = e.ry;
    r[4..6].copy_from_slice(&botones.to_le_bytes());
    r[6] = especiales | (e.contador_tramas << 2);
    r[7] = e.l2;
    r[8] = e.r2;
    r[9..11].copy_from_slice(&e.marca_ds5.to_le_bytes());
    if let Some(m) = &e.movimiento {
        let ejes = [
            m.giro_pitch_completo,
            -m.giro_yaw_completo,
            -m.giro_roll_completo,
            -m.acel_x_completo,
            -m.acel_y_completo,
            m.acel_z_completo,
        ];
        for (i, v) in ejes.into_iter().enumerate() {
            r[12 + i * 2..14 + i * 2].copy_from_slice(&(v as i16).to_le_bytes());
        }
    }
    r[29] = e.bateria / 11;
    r[32] = 1;
    r[33] = e.contador_panel;
    for (i, t) in e.toques.iter().enumerate() {
        let o = 34 + i * 4;
        let (x, y) = (t.x as i32, t.y as i32);
        r[o] = t.crudo;
        r[o + 1] = (x & 0xFF) as u8;
        r[o + 2] = (((x >> 8) & 0x0F) | ((y << 4) & 0xF0)) as u8;
        r[o + 3] = (y >> 4) as u8;
    }
    r
}

/// El informe del DS4 en reposo, como `DS4OutDeviceExt.ResetState`.
pub fn informe_ds4_reposo() -> [u8; LARGO_DS4_EX] {
    let mut r = [0u8; LARGO_DS4_EX];
    r[..4].fill(0x80);
    r[4] = boton_ds4::CRUCETA_SUELTA as u8;
    r
}

/// Lo que pide el juego al mando virtual.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Respuesta {
    /// Motor grande (izquierdo, lento).
    pub motor_grande: u8,
    /// Motor pequeño (derecho, rápido).
    pub motor_pequeno: u8,
    /// Solo el DS4: el color de la barra de luz, si el juego lo ha cambiado.
    pub color: Option<(u8, u8, u8)>,
    /// Solo el DS4: el parpadeo (encendido, apagado), si el juego lo ha cambiado.
    pub parpadeo: Option<(u8, u8)>,
    /// Si el informe traía la vibración (el DS4 la puede dejar fuera).
    pub con_vibracion: bool,
}

const VIBRACION: u8 = 0x01;
const LUZ: u8 = 0x02;
const PARPADEO: u8 = 0x04;

impl Respuesta {
    /// El informe de salida que el juego mandó al DS4 virtual (el de `AwaitRawOutputReport`).
    /// Como en `ControlService.EstablishOutFeedback`: `[1]` dice qué trae, `[4]` es el motor
    /// rápido y `[5]` el lento.
    pub fn desde_ds4(b: &[u8]) -> Option<Respuesta> {
        if b.len() < 11 {
            return None;
        }
        let banderas = b[1];
        let vibracion = banderas & VIBRACION != 0;
        Some(Respuesta {
            motor_grande: if vibracion { b[5] } else { 0 },
            motor_pequeno: if vibracion { b[4] } else { 0 },
            color: (banderas & LUZ != 0).then_some((b[6], b[7], b[8])),
            parpadeo: (banderas & PARPADEO != 0).then_some((b[9], b[10])),
            con_vibracion: vibracion,
        })
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use ds5_informes::{Movimiento, Toque};

    /// El volante sale tal cual por su eje; en los gatillos, el negativo se corta a un byte.
    #[test]
    fn el_volante_sale_por_su_eje() {
        let mut x = InformeXbox::desde(&Estado::default());
        x.poner_volante(EjeVolante::LX, -12000);
        assert_eq!(x.lx, -12000);
        x.poner_volante(EjeVolante::L2R2, -100);
        assert_eq!((x.gatillo_izq, x.gatillo_der), (0, 156));
        let mut r = informe_ds4(&Estado::default());
        poner_volante_ds4(&mut r, EjeVolante::RY, 200);
        assert_eq!(r[3], 200);
        poner_volante_ds4(&mut r, EjeVolante::L2R2, 90);
        assert_eq!((r[7], r[8]), (90, 0));
    }

    #[test]
    fn los_ejes_se_escalan_como_en_c() {
        assert_eq!(escala_eje(0, false), -32768);
        assert_eq!(escala_eje(255, false), 32767);
        assert_eq!(escala_eje(128, false), 0);
        assert_eq!(escala_eje(0, true), 32767);
        assert_eq!(escala_eje(255, true), -32768);
        assert_eq!(escala_eje(128, true), 0);
        // Valores sueltos calculados con la misma cuenta en float.
        assert_eq!(escala_eje(64, false), -16384);
        assert_eq!(escala_eje(200, false), 18576);
    }

    #[test]
    fn la_cruceta_sigue_el_orden_de_c() {
        assert_eq!(cruceta(0), 8);
        assert_eq!(cruceta(boton::ARRIBA), 0);
        assert_eq!(cruceta(boton::ARRIBA | boton::DERECHA), 1);
        assert_eq!(cruceta(boton::ABAJO | boton::IZQUIERDA), 5);
        // Arriba y abajo a la vez: gana arriba, como en C#.
        assert_eq!(cruceta(boton::ARRIBA | boton::ABAJO), 0);
    }

    fn estado() -> Estado {
        Estado {
            botones: boton::EQUIS | boton::PS | boton::CREAR | boton::PANEL | boton::L2,
            lx: 10,
            ly: 20,
            rx: 30,
            ry: 40,
            l2: 50,
            r2: 60,
            bateria: 100,
            contador_tramas: 5,
            contador_panel: 9,
            marca_ds5: 0x1234,
            toques: [
                Toque {
                    activo: true,
                    id: 1,
                    x: 1000,
                    y: 500,
                    crudo: 1,
                },
                Toque {
                    activo: false,
                    id: 2,
                    x: 0x123,
                    y: 0x456,
                    crudo: 0x82,
                },
            ],
            movimiento: Some(Movimiento {
                giro_yaw_completo: 100,
                giro_pitch_completo: -200,
                giro_roll_completo: 300,
                acel_x_completo: 1,
                acel_y_completo: -2,
                acel_z_completo: 8192,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn el_informe_de_xbox() {
        let x = InformeXbox::desde(&estado());
        assert_eq!(
            x.botones,
            boton_xbox::A | boton_xbox::GUIA | boton_xbox::BACK
        );
        assert_eq!((x.gatillo_izq, x.gatillo_der), (50, 60));
        let b = x.bytes();
        assert_eq!(b[0..2], (x.botones).to_le_bytes());
        assert_eq!(b[4..6], escala_eje(10, false).to_le_bytes());
        assert_eq!(b[6..8], escala_eje(20, true).to_le_bytes());
    }

    #[test]
    fn el_informe_del_ds4() {
        let r = informe_ds4(&estado());
        assert_eq!(r[..4], [10, 20, 30, 40]);
        let botones = u16::from_le_bytes([r[4], r[5]]);
        assert_eq!(
            botones,
            boton_ds4::EQUIS | boton_ds4::SHARE | boton_ds4::L2 | 8
        );
        assert_eq!(r[6], boton_ds4::PS | boton_ds4::PANEL | (5 << 2));
        assert_eq!((r[7], r[8]), (50, 60));
        assert_eq!(r[9..11], [0x34, 0x12]);
        let eje = |i: usize| i16::from_le_bytes([r[12 + i * 2], r[13 + i * 2]]);
        assert_eq!(
            (0..6).map(eje).collect::<Vec<_>>(),
            [-200, -100, -300, -1, 2, 8192]
        );
        assert_eq!(r[29], 9);
        assert_eq!((r[32], r[33]), (1, 9));
        // Segundo dedo: X 0x123, Y 0x456.
        assert_eq!(r[38..42], [0x82, 0x23, 0x61, 0x45]);
    }

    #[test]
    fn la_respuesta_del_ds4() {
        let mut b = [0u8; 64];
        b[0] = 0x05;
        b[1] = VIBRACION | LUZ;
        b[4] = 11;
        b[5] = 22;
        b[6..9].copy_from_slice(&[1, 2, 3]);
        let r = Respuesta::desde_ds4(&b).unwrap();
        assert_eq!((r.motor_grande, r.motor_pequeno), (22, 11));
        assert_eq!(r.color, Some((1, 2, 3)));
        assert_eq!(r.parpadeo, None);
        b[1] = LUZ;
        assert!(!Respuesta::desde_ds4(&b).unwrap().con_vibracion);
    }
}
