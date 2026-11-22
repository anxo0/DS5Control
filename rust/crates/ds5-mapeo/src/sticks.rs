//! Lo que el giroscopio y el panel mandan a los sticks cuando hacen de stick
//! (`Mapping.PostMapStickData`), lo que tienen en común al calcularlo y cómo se mezcla al final de
//! la reasignación con lo que ya sale por los sticks.

use ds5_informes::Estado;
use ds5_perfiles::perfil::{EjesSalida, PalancaSalida};

use crate::bezier::byte_net;
use crate::curvas::{max_net, min_net};

/// `PostMapStickData`: el valor de cada eje que ha pedido el giroscopio o el panel. Solo se vacía
/// cuando se mezcla, así que sin reasignación se queda con el más alejado del centro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SticksAccion {
    pub lx: u8,
    pub ly: u8,
    pub rx: u8,
    pub ry: u8,
    pub sucio: bool,
}

impl Default for SticksAccion {
    fn default() -> Self {
        SticksAccion {
            lx: 128,
            ly: 128,
            rx: 128,
            ry: 128,
            sucio: false,
        }
    }
}

fn lejos(a: u8, b: u8) -> bool {
    (a as i32 - 128).abs() > (b as i32 - 128).abs()
}

impl SticksAccion {
    /// El final de `SixMouseStick` y `TouchpadMouseStick`: cada eje que sale se apunta si va más
    /// lejos del centro que lo que ya había.
    pub fn poner(&mut self, stick: PalancaSalida, ejes: EjesSalida, x: u8, y: u8) {
        let (sx, sy) = match stick {
            PalancaSalida::LeftStick => (&mut self.lx, &mut self.ly),
            PalancaSalida::RightStick => (&mut self.rx, &mut self.ry),
            PalancaSalida::None => return,
        };
        if matches!(ejes, EjesSalida::XY | EjesSalida::X) && lejos(x, *sx) {
            *sx = x;
            self.sucio = true;
        }
        if matches!(ejes, EjesSalida::XY | EjesSalida::Y) && lejos(y, *sy) {
            *sy = y;
            self.sucio = true;
        }
    }

    /// El final de `MapCustom`: cada eje pedido sale si va más lejos del centro que el del estado.
    pub fn mezclar(&mut self, e: &mut Estado) {
        if !self.sucio {
            return;
        }
        for (v, s) in [
            (self.lx, &mut e.lx),
            (self.ly, &mut e.ly),
            (self.rx, &mut e.rx),
            (self.ry, &mut e.ry),
        ] {
            if v != 128 && lejos(v, *s) {
                *s = v;
            }
        }
        *self = SticksAccion::default();
    }
}

/// Lo que comparten el giroscopio y el panel como stick de la zona máxima en adelante.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Recorrido {
    pub dx: i32,
    pub dy: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub norm_x: f64,
    pub norm_y: f64,
    /// `maxDirX` y `maxDirY`: 127 o -128. C# los saca del signo de antes de rotar y de
    /// suavizar, así que no siempre van con `dx` y `dy`.
    pub dir_x: f64,
    pub dir_y: f64,
}

impl Recorrido {
    /// `xratio` e `yratio`.
    pub fn proporcion(&self) -> (f64, f64) {
        let x = if self.dx != 0 {
            self.dx as f64 / self.max_x as f64
        } else {
            0.0
        };
        let y = if self.dy != 0 {
            self.dy as f64 / self.max_y as f64
        } else {
            0.0
        };
        (x, y)
    }

    /// La salida máxima (`maxOutputEnabled`): un poco de más en diagonal, porque con el giroscopio
    /// casi nunca sale una línea recta.
    pub fn limitar(&self, x: f64, y: f64, maxima: f64) -> (f64, f64) {
        let r = maxima / 100.0;
        let mx = min_net(self.norm_x / 0.95, 1.0) * r;
        let my = min_net(self.norm_y / 0.95, 1.0) * r;
        (min_net(max_net(x, 0.0), mx), min_net(max_net(y, 0.0), my))
    }

    /// De la proporción de cada eje al byte del stick, con la antizona y la inversión.
    pub fn salida(&self, x: f64, y: f64, anti: (f64, f64), invertir: u32) -> (u8, u8) {
        let ax = anti.0 * self.norm_x;
        let ay = anti.1 * self.norm_y;
        let nx = if x != 0.0 { (1.0 - ax) * x + ax } else { 0.0 };
        let ny = if y != 0.0 { (1.0 - ay) * y + ay } else { 0.0 };
        let (mut dir_x, mut dir_y) = (self.dir_x, self.dir_y);
        if invertir & 1 == 1 {
            dir_x = if self.dx >= 0 { -128.0 } else { 127.0 };
        }
        if invertir & 2 == 2 {
            dir_y = if self.dy >= 0 { -128.0 } else { 127.0 };
        }
        (byte_net(nx * dir_x + 128.0), byte_net(ny * dir_y + 128.0))
    }
}

/// `maxDirX`: 127 hacia la derecha o abajo (o en el centro) y -128 hacia el otro lado.
pub(crate) fn direccion(v: i32) -> f64 {
    if v >= 0 {
        127.0
    } else {
        -128.0
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// Sin reasignación no se vacía, así que se queda con lo más lejos del centro.
    #[test]
    fn se_queda_con_lo_mas_lejos_hasta_mezclar() {
        let mut s = SticksAccion::default();
        s.poner(PalancaSalida::RightStick, EjesSalida::XY, 200, 100);
        s.poner(PalancaSalida::RightStick, EjesSalida::XY, 150, 10);
        assert_eq!((s.rx, s.ry), (200, 10));
        let mut e = Estado {
            lx: 128,
            ly: 128,
            rx: 60,
            ry: 128,
            ..Default::default()
        };
        s.mezclar(&mut e);
        assert_eq!((e.rx, e.ry), (200, 10));
        assert_eq!(s, SticksAccion::default());
    }
}
