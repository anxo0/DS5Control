//! Zonas muertas, sensibilidad y curvas de sticks y gatillos: `Mapping.SetCurveAndDeadzone`.
//! Va justo después de decodificar el informe y antes de los mapeos (la línea `cd` de las
//! grabaciones).

use std::collections::VecDeque;
use std::f64::consts::PI;

use ds5_informes::Estado;
use ds5_perfiles::perfil::{Ajustes as AjustesPerfil, TipoZonaMuerta, ZonasAxiales};

use crate::bezier::{self, byte_net, TipoEje};
use crate::giroscopio::AjustesGiro;

/// La zona muerta de un eje cuando el stick es axial (`AxisDeadZoneInfo`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZonaEje {
    pub zona_muerta: i32,
    pub antizona: i32,
    pub zona_maxima: i32,
    pub salida_maxima: f64,
}

impl ZonaEje {
    fn activa(&self) -> bool {
        self.zona_muerta > 0
            || self.antizona > 0
            || self.zona_maxima != 100
            || self.salida_maxima != 100.0
    }
}

/// Las zonas de un stick (`StickDeadZoneInfo`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZonaPalanca {
    pub axial: bool,
    pub zona_muerta: i32,
    pub antizona: i32,
    pub zona_maxima: i32,
    pub salida_maxima: f64,
    pub forzar_salida_maxima: bool,
    pub escala_vertical: f64,
    pub difuminado: i32,
    pub zona_exterior: f64,
    pub invertir_exterior: bool,
    pub x: ZonaEje,
    pub y: ZonaEje,
}

/// La zona de un gatillo (`TriggerDeadZoneZInfo`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZonaGatillo {
    pub zona_muerta: u8,
    pub antizona: i32,
    pub zona_maxima: i32,
    pub salida_maxima: f64,
}

/// El antirrebote del stick (`StickAntiSnapbackInfo`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Antirrebote {
    pub activo: bool,
    pub delta: f64,
    pub tiempo_ms: i64,
}

/// Una curva de salida: 0 lineal, 1 a 5 las fijas y 6 la personalizada, con su tabla.
#[derive(Debug, Clone, PartialEq)]
pub struct CurvaSalida {
    pub modo: u8,
    pub tabla: Option<Box<[u8; 256]>>,
}

impl CurvaSalida {
    pub(crate) fn nueva(modo: u8, personalizada: Option<&str>, eje: TipoEje) -> CurvaSalida {
        // Como `setOutBezierCurveObjArrayItem`: solo la personalizada usa la tabla.
        let tabla = (modo == 6).then(|| {
            let def = personalizada
                .and_then(bezier::leer_definicion)
                .unwrap_or([0.0; 4]);
            Box::new(bezier::tabla(def, eje))
        });
        CurvaSalida { modo, tabla }
    }

    pub(crate) fn valor(&self, i: usize) -> u8 {
        self.tabla.as_ref().map_or(i as u8, |t| t[i])
    }
}

/// Lo que usa este bloque de un perfil, ya como lo tiene C# en memoria.
#[derive(Debug, Clone, PartialEq)]
pub struct AjustesCurvas {
    pub rotacion_ls: f64,
    pub rotacion_rs: f64,
    pub antirrebote_ls: Antirrebote,
    pub antirrebote_rs: Antirrebote,
    pub ls: ZonaPalanca,
    pub rs: ZonaPalanca,
    pub l2: ZonaGatillo,
    pub r2: ZonaGatillo,
    /// LS, RS, L2, R2, SX, SZ.
    pub sensibilidad: [f64; 6],
    pub cuadrado_ls: Option<f64>,
    pub cuadrado_rs: Option<f64>,
    pub curva_ls: CurvaSalida,
    pub curva_rs: CurvaSalida,
    pub curva_l2: CurvaSalida,
    pub curva_r2: CurvaSalida,
    /// Deriva corregida: LX, LY, RX, RY.
    pub deriva: [i8; 4],
    /// El giroscopio como controles (SX y SZ).
    pub giro: AjustesGiro,
}

fn zona_eje(z: &ZonasAxiales, eje_x: bool) -> ZonaEje {
    if eje_x {
        ZonaEje {
            zona_muerta: z.zona_muerta_x,
            antizona: z.antizona_x,
            zona_maxima: z.zona_maxima_x,
            salida_maxima: z.salida_maxima_x,
        }
    } else {
        ZonaEje {
            zona_muerta: z.zona_muerta_y,
            antizona: z.antizona_y,
            zona_maxima: z.zona_maxima_y,
            salida_maxima: z.salida_maxima_y,
        }
    }
}

pub(crate) fn modo_curva(c: ds5_perfiles::Curva) -> u8 {
    use ds5_perfiles::Curva::*;
    match c {
        Lineal => 0,
        PrecisionMejorada => 1,
        Cuadratica => 2,
        Cubica => 3,
        SalidaCuadratica => 4,
        SalidaCubica => 5,
        Personalizada => 6,
    }
}

impl AjustesCurvas {
    /// Como `ProfileDTO.MapTo` para estos campos.
    pub fn desde_perfil(p: &AjustesPerfil) -> AjustesCurvas {
        let palanca = |axial: TipoZonaMuerta,
                       zm: i32,
                       az: i32,
                       zmax: i32,
                       smax: f64,
                       forzar: bool,
                       escala: f64,
                       dif: i32,
                       ext: f64,
                       inv: bool,
                       axiales: &ZonasAxiales| ZonaPalanca {
            axial: axial == TipoZonaMuerta::Axial,
            zona_muerta: zm,
            antizona: az,
            zona_maxima: zmax,
            salida_maxima: smax,
            forzar_salida_maxima: forzar,
            escala_vertical: escala,
            difuminado: dif,
            zona_exterior: ext,
            invertir_exterior: inv,
            x: zona_eje(axiales, true),
            y: zona_eje(axiales, false),
        };
        // C# guarda la rotación en radianes: la lee en grados, la limita y la convierte.
        let radianes = |g: f64| g.clamp(-180.0, 180.0) * PI / 180.0;
        AjustesCurvas {
            rotacion_ls: radianes(p.ls_rotacion),
            rotacion_rs: radianes(p.rs_rotacion),
            antirrebote_ls: Antirrebote {
                activo: p.ls_antirrebote,
                delta: p.ls_antirrebote_delta,
                tiempo_ms: p.ls_antirrebote_tiempo as i64,
            },
            antirrebote_rs: Antirrebote {
                activo: p.rs_antirrebote,
                delta: p.rs_antirrebote_delta,
                tiempo_ms: p.rs_antirrebote_tiempo as i64,
            },
            ls: palanca(
                p.ls_tipo_zona_muerta,
                p.ls_zona_muerta,
                p.ls_antizona,
                p.ls_zona_maxima,
                p.ls_salida_maxima,
                p.ls_forzar_salida_maxima,
                p.ls_escala_vertical,
                p.ls_difuminado,
                p.ls_zona_exterior,
                p.ls_invertir_exterior,
                &p.ls_zonas_axiales,
            ),
            rs: palanca(
                p.rs_tipo_zona_muerta,
                p.rs_zona_muerta,
                p.rs_antizona,
                p.rs_zona_maxima,
                p.rs_salida_maxima,
                p.rs_forzar_salida_maxima,
                p.rs_escala_vertical,
                p.rs_difuminado,
                p.rs_zona_exterior,
                p.rs_invertir_exterior,
                &p.rs_zonas_axiales,
            ),
            l2: ZonaGatillo {
                zona_muerta: p.l2_zona_muerta,
                antizona: p.l2_antizona,
                zona_maxima: p.l2_zona_maxima,
                salida_maxima: p.l2_salida_maxima,
            },
            r2: ZonaGatillo {
                zona_muerta: p.r2_zona_muerta,
                antizona: p.r2_antizona,
                zona_maxima: p.r2_zona_maxima,
                salida_maxima: p.r2_salida_maxima,
            },
            sensibilidad: p.sensibilidades,
            cuadrado_ls: p.ls_cuadrado.then_some(p.ls_redondez),
            cuadrado_rs: p.rs_cuadrado.then_some(p.rs_redondez),
            curva_ls: CurvaSalida::nueva(
                modo_curva(p.ls_curva),
                p.ls_curva_personalizada.as_deref(),
                TipoEje::Palanca,
            ),
            curva_rs: CurvaSalida::nueva(
                modo_curva(p.rs_curva),
                p.rs_curva_personalizada.as_deref(),
                TipoEje::Palanca,
            ),
            curva_l2: CurvaSalida::nueva(
                modo_curva(p.l2_curva),
                p.l2_curva_personalizada.as_deref(),
                TipoEje::Gatillo,
            ),
            curva_r2: CurvaSalida::nueva(
                modo_curva(p.r2_curva),
                p.r2_curva_personalizada.as_deref(),
                TipoEje::Gatillo,
            ),
            deriva: [
                p.deriva_izq_x,
                p.deriva_izq_y,
                p.deriva_der_x,
                p.deriva_der_y,
            ],
            giro: AjustesGiro::desde_perfil(p),
        }
    }
}

/// Lo que este bloque recuerda entre informes: los últimos valores del filtro de ruido y el
/// historial del antirrebote.
#[derive(Debug, Clone)]
pub struct Curvas {
    ultimos: [u8; 4],
    historial: [VecDeque<(f64, f64, i64)>; 2],
}

impl Default for Curvas {
    fn default() -> Self {
        Curvas {
            ultimos: [128; 4],
            historial: [VecDeque::new(), VecDeque::new()],
        }
    }
}

fn clamp(min: f64, v: f64, max: f64) -> f64 {
    // `Global.Clamp`.
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

/// `Math.Max` de .NET: si uno es NaN, da NaN (el `max` de Rust se queda con el otro).
pub(crate) fn max_net(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// `Math.Min` de .NET, con NaN igual que `max_net`.
pub(crate) fn min_net(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

fn rotar(x: &mut u8, y: &mut u8, radianes: f64) {
    let (s, c) = (radianes.sin(), radianes.cos());
    let (tx, ty) = (*x as f64 - 128.0, *y as f64 - 128.0);
    *x = byte_net(clamp(-128.0, tx * c - ty * s, 127.0) + 128.0);
    *y = byte_net(clamp(-128.0, tx * s + ty * c, 127.0) + 128.0);
}

impl Curvas {
    /// `CalcStickAxisFuzz`. Los índices del valor anterior son los de C#: LS usa 0 y 3, RS 2 y 1.
    fn difuminar(&mut self, palanca: usize, delta: i32, x: u8, y: u8) -> (u8, u8) {
        let ix = if palanca == 0 { 0 } else { 2 };
        let iy = if palanca == 1 { 1 } else { 3 };
        let (mut ux, mut uy) = (self.ultimos[ix], self.ultimos[iy]);
        let dx = x as i32 - self.ultimos[ix] as i32;
        let dy = y as i32 - self.ultimos[iy] as i32;
        let mag = dx * dx + dy * dy;
        let d2 = delta * delta;
        if x == 0 || x == 255 || mag > d2 {
            ux = x;
            self.ultimos[ix] = x;
        }
        if y == 0 || y == 255 || mag > d2 {
            uy = y;
            self.ultimos[iy] = y;
        }
        (ux, uy)
    }

    /// `CalcAntiSnapbackStick`: si el stick ha cruzado el centro de golpe hace poco, se deja en
    /// el centro. `ahora_ms` es la hora del reloj en milisegundos.
    fn antirrebote(
        &mut self,
        palanca: usize,
        a: &Antirrebote,
        x: u8,
        y: u8,
        ahora_ms: i64,
    ) -> (u8, u8) {
        let cola = &mut self.historial[palanca];
        while cola.front().is_some_and(|v| v.2 < ahora_ms - a.tiempo_ms) {
            cola.pop_front();
        }
        let (fx, fy) = (x as f64, y as f64);
        let cruza = cola.iter().any(|&(ox, oy, _)| {
            let dist2 = (fx - ox).powi(2) + (fy - oy).powi(2);
            if dist2 >= a.delta * a.delta {
                let t = ((128.0 - fx) * (ox - fx) + (128.0 - fy) * (oy - fy)) / dist2;
                let t = t.clamp(0.0, 1.0);
                let medio2 =
                    (128.0 - (fx + t * (ox - fx))).powi(2) + (128.0 - (fy + t * (oy - fy))).powi(2);
                medio2 <= 15.0 * 15.0
            } else {
                false
            }
        });
        cola.push_back((fx, fy, ahora_ms));
        if cruza {
            (128, 128)
        } else {
            (x, y)
        }
    }

    /// `SetCurveAndDeadzone` para los sticks y los gatillos. El giroscopio como controles (SX y
    /// SZ) va con el bloque del giroscopio.
    pub fn aplicar(&mut self, entrada: &Estado, a: &AjustesCurvas, ahora_ms: i64) -> Estado {
        let mut c = entrada.clone();
        if a.rotacion_ls != 0.0 {
            rotar(&mut c.lx, &mut c.ly, a.rotacion_ls);
        }
        if a.rotacion_rs != 0.0 {
            rotar(&mut c.rx, &mut c.ry, a.rotacion_rs);
        }
        if a.antirrebote_ls.activo {
            (c.lx, c.ly) = self.antirrebote(0, &a.antirrebote_ls, c.lx, c.ly, ahora_ms);
        }
        if a.antirrebote_rs.activo {
            (c.rx, c.ry) = self.antirrebote(1, &a.antirrebote_rs, c.rx, c.ry, ahora_ms);
        }
        if a.ls.difuminado > 0 {
            (c.lx, c.ly) = self.difuminar(0, a.ls.difuminado, c.lx, c.ly);
        }
        if a.rs.difuminado > 0 {
            (c.rx, c.ry) = self.difuminar(1, a.rs.difuminado, c.rx, c.ry);
        }

        // `ApplyStickCalibration`: desde aquí C# trabaja sobre un solo estado.
        let mut d = c;
        let corregir = |v: &mut u8, deriva: i8| {
            if deriva != 0 {
                *v = (*v as i32 - deriva as i32).clamp(0, 255) as u8;
            }
        };
        corregir(&mut d.rx, a.deriva[2]);
        corregir(&mut d.ry, a.deriva[3]);
        corregir(&mut d.lx, a.deriva[0]);
        corregir(&mut d.ly, a.deriva[1]);

        zona_palanca(&mut d.lx, &mut d.ly, &mut d.exterior_ls, &a.ls);
        zona_palanca(&mut d.rx, &mut d.ry, &mut d.exterior_rs, &a.rs);
        zona_gatillo(&mut d.l2, &a.l2);
        zona_gatillo(&mut d.r2, &a.r2);

        // La sensibilidad antigua solo se aplica con zona muerta radial.
        let sens = |x: &mut u8, y: &mut u8, s: f64| {
            if s != 1.0 {
                *x = byte_net(clamp(0.0, s * (*x as f64 - 128.0) + 128.0, 255.0));
                *y = byte_net(clamp(0.0, s * (*y as f64 - 128.0) + 128.0, 255.0));
            }
        };
        if !a.ls.axial {
            sens(&mut d.lx, &mut d.ly, a.sensibilidad[0]);
        }
        if !a.rs.axial {
            sens(&mut d.rx, &mut d.ry, a.sensibilidad[1]);
        }
        for (v, s) in [
            (&mut d.l2, a.sensibilidad[2]),
            (&mut d.r2, a.sensibilidad[3]),
        ] {
            if s != 1.0 {
                *v = byte_net(clamp(0.0, s * *v as f64, 255.0));
            }
        }

        if let Some(r) = a.cuadrado_ls {
            cuadrado(&mut d.lx, &mut d.ly, r);
        }
        curva_palanca(&mut d.lx, &mut d.ly, &a.curva_ls, a.ls.axial);
        if let Some(r) = a.cuadrado_rs {
            cuadrado(&mut d.rx, &mut d.ry, r);
        }
        curva_palanca(&mut d.rx, &mut d.ry, &a.curva_rs, a.rs.axial);
        curva_gatillo(&mut d.l2, &a.curva_l2);
        curva_gatillo(&mut d.r2, &a.curva_r2);
        if let Some(m) = d.movimiento.as_mut() {
            a.giro.salida(m);
        }
        d
    }
}

fn zona_radial(x: &mut u8, y: &mut u8, z: &ZonaPalanca) {
    let interpretar = z.antizona > 0
        || z.zona_maxima != 100
        || z.salida_maxima != 100.0
        || z.forzar_salida_maxima
        || z.escala_vertical != 100.0;
    if !(z.zona_muerta > 0 || interpretar) {
        return;
    }
    let (fx, fy) = (*x as f64, *y as f64);
    let cuadrado = (fx - 128.0).powf(2.0) + (fy - 128.0).powf(2.0);
    let zm2 = (z.zona_muerta as f64).powf(2.0);
    if z.zona_muerta > 0 && cuadrado <= zm2 {
        *x = 128;
        *y = 128;
        return;
    }
    let r = (-(fy - 128.0)).atan2(fx - 128.0);
    let max_x = if fx >= 128.0 { 127.0 } else { -128.0 };
    let max_y = if fy >= 128.0 { 127.0 } else { -128.0 };
    let ratio = z.zona_maxima as f64 / 100.0;
    let ratio_salida = z.salida_maxima / 100.0;
    let escala = z.escala_vertical / 100.0;
    let neg = (ratio * -128.0) + 128.0;
    let pos = (ratio * 127.0) + 128.0;
    let zona_x = if fx >= 128.0 {
        pos - 128.0
    } else {
        neg - 128.0
    };
    let zona_y = if fy >= 128.0 {
        pos - 128.0
    } else {
        neg - 128.0
    };

    let (mut sx, mut sy) = (0.0, 0.0);
    if z.zona_muerta > 0 {
        let mx = r.cos().abs() * (z.zona_muerta as f64 / 127.0) * max_x;
        let my = r.sin().abs() * (z.zona_muerta as f64 / 127.0) * max_y;
        if cuadrado > zm2 {
            sx = (clamp(neg, fx, pos) - 128.0 - mx) / (zona_x - mx);
            sy = (clamp(neg, fy, pos) - 128.0 - my) / (zona_y - my);
        }
    } else {
        sx = (clamp(neg, fx, pos) - 128.0) / zona_x;
        sy = (clamp(neg, fy, pos) - 128.0) / zona_y;
    }
    if z.escala_vertical != 100.0 {
        sy = min_net(max_net(sy * escala, 0.0), 1.0);
    }
    if z.salida_maxima != 100.0 || z.forzar_salida_maxima {
        let mx = min_net(r.cos().abs() * ratio_salida / 0.99, 1.0);
        let my = min_net(r.sin().abs() * ratio_salida / 0.99, 1.0);
        sx = min_net(max_net(sx, 0.0), mx);
        sy = min_net(max_net(sy, 0.0), my);
    }
    let (mut ax, mut ay) = (0.0, 0.0);
    if z.antizona > 0 {
        ax = (z.antizona as f64 * 0.01) * r.cos().abs();
        ay = (z.antizona as f64 * 0.01) * r.sin().abs();
    }
    *x = if sx > 0.0 {
        byte_net(((1.0 - ax) * sx + ax) * max_x + 128.0)
    } else {
        128
    };
    *y = if sy > 0.0 {
        byte_net(((1.0 - ay) * sy + ay) * max_y + 128.0)
    } else {
        128
    };
}

fn zona_axial(v: &mut u8, z: &ZonaEje) {
    if !z.activa() {
        return;
    }
    let dist = (*v as i32 - 128).abs();
    if z.zona_muerta > 0 && dist <= z.zona_muerta {
        *v = 128;
        return;
    }
    let f = *v as f64;
    let max = if f >= 128.0 { 127.0 } else { -128.0 };
    let ratio = z.zona_maxima as f64 / 100.0;
    let mut ratio_salida = z.salida_maxima / 100.0;
    let neg = (ratio * -128.0) + 128.0;
    let pos = (ratio * 127.0) + 128.0;
    let zona = if f >= 128.0 { pos - 128.0 } else { neg - 128.0 };
    let muerta = if z.zona_muerta > 0 {
        (z.zona_muerta as f64 / 127.0) * max
    } else {
        0.0
    };
    let mut s = (clamp(neg, f, pos) - 128.0 - muerta) / (zona - muerta);
    if z.salida_maxima != 100.0 {
        ratio_salida = min_net(ratio_salida / 0.99, 1.0);
        s = min_net(max_net(s, 0.0), ratio_salida);
    }
    let anti = if z.antizona > 0 {
        z.antizona as f64 * 0.01
    } else {
        0.0
    };
    *v = if s > 0.0 {
        byte_net(((1.0 - anti) * s + anti) * max + 128.0)
    } else {
        128
    };
}

fn zona_palanca(x: &mut u8, y: &mut u8, exterior: &mut u8, z: &ZonaPalanca) {
    if z.axial {
        zona_axial(x, &z.x);
        zona_axial(y, &z.y);
        return;
    }
    zona_radial(x, y, z);
    // La salida del borde exterior, solo con zona radial.
    *exterior = 0;
    if *x != 128 || *y != 128 {
        let ax = *x as i32 - 128;
        let ay = *y as i32 - 128;
        let r = (-(ay as f64)).atan2(ax as f64);
        let hip = ((ax * ax + ay * ay) as f64).sqrt();
        if hip != 0.0 {
            let tx = (r.cos().abs() * if *x >= 128 { 127.0 } else { 128.0 }) as i32;
            let ty = (r.sin().abs() * if *y >= 128 { 127.0 } else { 128.0 }) as i32;
            let maximo = ((tx * tx + ty * ty) as f64).sqrt();
            let mut ratio = hip / maximo;
            if ratio > 1.0 {
                ratio = 1.0;
            }
            let actual = ratio * 255.0;
            let muerta = z.zona_exterior * 0.01 * 255.0;
            if !z.invertir_exterior && actual > muerta {
                *exterior = byte_net((actual - muerta) / (255.0 - muerta) * 255.0);
            } else if z.invertir_exterior && actual < muerta {
                *exterior = byte_net((muerta - actual) / muerta * 255.0);
            }
        }
    }
}

fn zona_gatillo(v: &mut u8, z: &ZonaGatillo) {
    if !(z.zona_muerta > 0 || z.antizona > 0 || z.zona_maxima != 100 || z.salida_maxima != 100.0) {
        return;
    }
    let maximo = 255.0 * (z.zona_maxima as f64 / 100.0);
    let mut s;
    if z.zona_muerta > 0 {
        if *v > z.zona_muerta {
            let actual = clamp(0.0, *v as f64, maximo);
            s = (actual - z.zona_muerta as f64) / (maximo - z.zona_muerta as f64);
        } else {
            s = 0.0;
        }
    } else {
        s = clamp(0.0, *v as f64, maximo) / maximo;
    }
    if z.salida_maxima != 100.0 {
        s = min_net(max_net(s, 0.0), z.salida_maxima / 100.0);
    }
    let anti = if z.antizona > 0 {
        z.antizona as f64 * 0.01
    } else {
        0.0
    };
    *v = if s > 0.0 {
        byte_net(((1.0 - anti) * s + anti) * 255.0)
    } else {
        0
    };
}

/// `DS5SquareStick.CircleToSquare` sobre un stick.
fn cuadrado(x: &mut u8, y: &mut u8, redondez: f64) {
    if *x == 128 && *y == 128 {
        return;
    }
    let cap_x = if *x >= 128 { 127.0 } else { 128.0 };
    let cap_y = if *y >= 128 { 127.0 } else { 128.0 };
    let mut cx = (*x as f64 - 128.0) / cap_x;
    let mut cy = (*y as f64 - 128.0) / cap_y;
    const PI4: f64 = PI / 4.0;
    let angulo = cy.atan2(-cx) + PI;
    let cos = angulo.cos();
    let factor_pared = if angulo <= PI4 || angulo > 7.0 * PI4 {
        Some(1.0 / cos)
    } else if angulo > PI4 && angulo <= 3.0 * PI4 {
        Some(1.0 / angulo.sin())
    } else if angulo > 3.0 * PI4 && angulo <= 5.0 * PI4 {
        Some(-1.0 / cos)
    } else if angulo > 5.0 * PI4 && angulo <= 7.0 * PI4 {
        Some(-1.0 / angulo.sin())
    } else {
        None
    };
    if let Some(f) = factor_pared {
        let (qx, qy) = (cx * f, cy * f);
        let largo = cx / cos;
        let factor = largo.powf(redondez);
        cx += (qx - cx) * factor;
        cy += (qy - cy) * factor;
    }
    let cx = cx.clamp(-1.0, 1.0);
    let cy = cy.clamp(-1.0, 1.0);
    *x = byte_net(cx * cap_x + 128.0);
    *y = byte_net(cy * cap_y + 128.0);
}

/// Las curvas fijas 1 a 5 sobre un valor de 0 a 1.
fn curva_fija_palanca(modo: u8, a: f64) -> f64 {
    match modo {
        1 => {
            if a <= 0.4 {
                0.8 * a
            } else if a <= 0.75 {
                a - 0.08
            } else {
                (a * 1.32) - 0.32
            }
        }
        _ => a,
    }
}

fn curva_palanca(x: &mut u8, y: &mut u8, curva: &CurvaSalida, axial: bool) {
    if curva.modo == 0 || (*x == 128 && *y == 128) {
        return;
    }
    let (fx, fy) = (*x as f64, *y as f64);
    let (cap_x, cap_y, rx, ry);
    if !axial {
        let r = (-(fy - 128.0)).atan2(fx - 128.0);
        let (mx, my) = (r.cos().abs(), r.sin().abs());
        let mut cx = if *x >= 128 { mx * 127.0 } else { mx * 128.0 };
        let mut cy = if *y >= 128 { my * 127.0 } else { my * 128.0 };
        let (ax, ay) = ((fx - 128.0).abs(), (fy - 128.0).abs());
        if ax > cx {
            cx = ax;
        }
        if ay > cy {
            cy = ay;
        }
        cap_x = cx;
        cap_y = cy;
        rx = if cx > 0.0 { (fx - 128.0) / cx } else { 0.0 };
        ry = if cy > 0.0 { (fy - 128.0) / cy } else { 0.0 };
    } else {
        cap_x = if *x >= 128 { 127.0 } else { 128.0 };
        cap_y = if *y >= 128 { 127.0 } else { 128.0 };
        rx = (fx - 128.0) / cap_x;
        ry = (fy - 128.0) / cap_y;
    }
    let sx = if rx >= 0.0 { 1.0 } else { -1.0 };
    let sy = if ry >= 0.0 { 1.0 } else { -1.0 };
    match curva.modo {
        1 => {
            let (ox, oy) = (
                curva_fija_palanca(1, rx.abs()),
                curva_fija_palanca(1, ry.abs()),
            );
            *x = byte_net(ox * sx * cap_x + 128.0);
            *y = byte_net(oy * sy * cap_y + 128.0);
        }
        2 => {
            *x = byte_net(rx * rx * sx * cap_x + 128.0);
            *y = byte_net(ry * ry * sy * cap_y + 128.0);
        }
        3 => {
            *x = byte_net(rx * rx * rx * cap_x + 128.0);
            *y = byte_net(ry * ry * ry * cap_y + 128.0);
        }
        4 => {
            let (ax, ay) = (rx.abs(), ry.abs());
            *x = byte_net(-(ax * (ax - 2.0)) * sx * cap_x + 128.0);
            *y = byte_net(-(ay * (ay - 2.0)) * sy * cap_y + 128.0);
        }
        5 => {
            let (ix, iy) = (rx.abs() - 1.0, ry.abs() - 1.0);
            *x = byte_net(1.0 * (ix * ix * ix + 1.0) * sx * cap_x + 128.0);
            *y = byte_net(1.0 * (iy * iy * iy + 1.0) * sy * cap_y + 128.0);
        }
        6 => {
            if !axial {
                let mx = if *x >= 128 { 127.0 } else { 128.0 };
                let my = if *y >= 128 { 127.0 } else { 128.0 };
                let tx = curva.valor(byte_net(rx * mx + 128.0) as usize);
                let ty = curva.valor(byte_net(ry * my + 128.0) as usize);
                let ox = (tx as f64 - 128.0) / mx;
                let oy = (ty as f64 - 128.0) / my;
                *x = byte_net(ox * cap_x + 128.0);
                *y = byte_net(oy * cap_y + 128.0);
            } else {
                *x = curva.valor(*x as usize);
                *y = curva.valor(*y as usize);
            }
        }
        _ => {}
    }
}

fn curva_gatillo(v: &mut u8, curva: &CurvaSalida) {
    if curva.modo == 0 || *v == 0 {
        return;
    }
    let t = *v as f64 / 255.0;
    *v = match curva.modo {
        1 => {
            let o = if t <= 0.4 {
                0.55 * t
            } else if t <= 0.75 {
                t - 0.18
            } else {
                (t * 1.72) - 0.72
            };
            byte_net(o * 255.0)
        }
        2 => byte_net(t * t * 255.0),
        3 => byte_net(t * t * t * 255.0),
        4 => byte_net(-(t * (t - 2.0)) * 255.0),
        // C# lo niega como si fuera la 4: da un valor negativo que al pasarlo a byte da la vuelta.
        5 => {
            let i = t.abs() - 1.0;
            byte_net(-(i * i * i + 1.0) * 255.0)
        }
        6 => curva.valor(*v as usize),
        _ => *v,
    };
}
