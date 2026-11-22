//! El panel táctil como stick (`Mouse.TouchpadMouseStick`), con su bola (`FakeTrackball`) y las
//! curvas de salida de `StickOutCurve`.

use std::f64::consts::PI;

use ds5_informes::dualsense::{PANEL_ALTO, PANEL_ANCHO};
use ds5_perfiles::perfil::{CurvaPanel, EjesSalida, PalancaSalida, Perfil, SuavizadoPanel};

use crate::curvas::min_net;
use crate::filtro::ParUnEuro;
use crate::sticks::{direccion, Recorrido, SticksAccion};

/// El filtro del panel como stick: C# lo crea con estos valores y nunca le pasa los del perfil.
const FILTRO_CORTE: f64 = 1.0;
const FILTRO_BETA: f64 = 1.0;
/// La bola del stick: C# no guarda ni su modo ni su fricción, así que siempre va con estos.
const BOLA_FRICCION: f64 = 10.0;
const BOLA_LARGO: usize = 8;
const BOLA_RADIO: f64 = 0.0245;
const BOLA_ESCALA: f64 = 0.004;

/// `TouchMouseStickInfo`.
#[derive(Debug, Clone, PartialEq)]
pub struct AjustesStickPanel {
    pub zona_muerta: i32,
    pub zona_maxima: i32,
    pub stick: PalancaSalida,
    pub ejes: EjesSalida,
    pub antizona: (f64, f64),
    pub invertir: u32,
    pub salida_maxima: Option<f64>,
    pub escala_vertical: i32,
    pub curva: CurvaPanel,
    /// En radianes.
    pub rotacion: f64,
    pub suavizado: bool,
}

impl AjustesStickPanel {
    pub fn desde_perfil(p: &Perfil) -> AjustesStickPanel {
        let s = &p.ajustes.tactil_como_stick;
        AjustesStickPanel {
            zona_muerta: s.zona_muerta,
            zona_maxima: s.zona_maxima,
            stick: s.stick,
            ejes: s.ejes,
            antizona: (s.antizona_x, s.antizona_y),
            invertir: s.invertir,
            salida_maxima: s.salida_maxima_activa.then_some(s.salida_maxima),
            escala_vertical: s.escala_vertical,
            curva: s.curva,
            rotacion: s.rotacion as f64 * PI / 180.0,
            suavizado: s.suavizado.metodo != SuavizadoPanel::None,
        }
    }
}

/// `FakeTrackball`.
#[derive(Debug, Clone)]
struct Bola {
    x: [f64; BOLA_LARGO],
    y: [f64; BOLA_LARGO],
    cola: usize,
    cabeza: usize,
    aceleracion: f64,
    vx: f64,
    vy: f64,
    resto_x: f64,
    resto_y: f64,
}

impl Bola {
    fn nueva() -> Bola {
        let inercia = 2.0 * (45.0 * BOLA_RADIO * BOLA_RADIO) / 5.0;
        Bola {
            x: [0.0; BOLA_LARGO],
            y: [0.0; BOLA_LARGO],
            cola: 0,
            cabeza: 0,
            aceleracion: BOLA_RADIO * BOLA_FRICCION / inercia,
            vx: 0.0,
            vy: 0.0,
            resto_x: 0.0,
            resto_y: 0.0,
        }
    }

    fn apuntar(&mut self, x: f64, y: f64) {
        let i = self.cola;
        self.x[i] = x;
        self.y[i] = y;
        self.cola = (i + 1) % BOLA_LARGO;
        if self.cabeza == self.cola {
            self.cabeza = (self.cabeza + 1) % BOLA_LARGO;
        }
    }

    fn vaciar(&mut self) {
        *self = Bola {
            aceleracion: self.aceleracion,
            ..Bola::nueva()
        };
    }

    /// `CalcTrackballInitVelocities`: la media de lo último que se movió.
    fn lanzar(&mut self) {
        let (mut x, mut y, mut peso) = (0.0, 0.0, 0.0);
        let mut idx = usize::MAX;
        let mut i = 0;
        while i < BOLA_LARGO && idx != self.cabeza {
            idx = (self.cola + BOLA_LARGO - i - 1) % BOLA_LARGO;
            x += self.x[idx];
            y += self.y[idx];
            peso += 1.0;
            i += 1;
        }
        self.vx = x / peso;
        self.vy = y / peso;
    }

    /// `Process`: frena y da lo que se mueve en este informe.
    fn paso(&mut self, t: f64) -> (i32, i32) {
        let angulo = (-self.vy).atan2(self.vx);
        let nx = angulo.cos().abs();
        let ny = angulo.sin().abs();
        let sx = signo(self.vx);
        let sy = signo(self.vy);
        let frena_x = self.vx.abs().min(self.aceleracion * t * nx);
        let frena_y = self.vy.abs().min(self.aceleracion * t * ny);
        let vx = self.vx - (frena_x * sx);
        let vy = self.vy - (frena_y * sy);
        let mut mx = (vx * t) / BOLA_ESCALA;
        let mut my = (vy * t) / BOLA_ESCALA;
        if mx != 0.0 {
            mx += self.resto_x;
        } else {
            self.resto_x = 0.0;
        }
        let dx = mx as i32;
        self.resto_x = mx - dx as f64;
        if my != 0.0 {
            my += self.resto_y;
        } else {
            self.resto_y = 0.0;
        }
        let dy = my as i32;
        self.resto_y = my - dy as f64;
        self.vx = vx;
        self.vy = vy;
        (dx, dy)
    }
}

/// `Math.Sign` de un `double`.
fn signo(v: f64) -> f64 {
    if v > 0.0 {
        1.0
    } else if v < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Lo que recuerda el panel como stick entre informes.
#[derive(Debug, Clone)]
pub struct StickPanel {
    bola: Bola,
    bola_activa: bool,
    filtro: ParUnEuro,
    /// El último movimiento, para repetirlo una vez si el dedo se queda quieto.
    antes: (i32, i32),
    repetido: bool,
}

impl Default for StickPanel {
    fn default() -> Self {
        StickPanel {
            bola: Bola::nueva(),
            bola_activa: false,
            filtro: ParUnEuro::nuevo(FILTRO_CORTE, FILTRO_BETA),
            antes: (0, 0),
            repetido: false,
        }
    }
}

impl StickPanel {
    /// `touchesBegan` y `touchesEnded`, en todos los modos: se olvida el último movimiento.
    pub(crate) fn olvidar(&mut self) {
        self.antes = (0, 0);
        self.repetido = false;
    }

    /// `touchesBegan` en el modo stick.
    pub(crate) fn empiezan(&mut self) {
        self.bola.vaciar();
        self.bola_activa = false;
    }

    /// `touchesMoved` en el modo stick.
    pub(crate) fn mueven(
        &mut self,
        dx: i32,
        dy: i32,
        activo: bool,
        t: f64,
        a: &AjustesStickPanel,
        sticks: &mut SticksAccion,
    ) {
        self.repetido = false;
        if activo {
            self.bola.apuntar(
                dx as f64 * BOLA_ESCALA / 0.004,
                dy as f64 * BOLA_ESCALA / 0.004,
            );
            self.mover(dx, dy, t, a, sticks);
            self.antes = (dx, dy);
        } else {
            self.bola.apuntar(0.0, 0.0);
        }
    }

    /// `touchesEnded` en el modo stick: la bola sale rodando.
    pub(crate) fn acaban(&mut self, t: f64, a: &AjustesStickPanel, sticks: &mut SticksAccion) {
        if !self.bola_activa {
            self.bola.lanzar();
            self.bola_activa = true;
            let (dx, dy) = self.bola.paso(t);
            if dx == 0 && dy == 0 {
                self.bola_activa = false;
            } else {
                self.mover(dx, dy, t, a, sticks);
            }
        }
    }

    /// `touchUnchanged` en el modo stick. `dedo` es si hay un dedo en el panel (`Touch1`).
    pub(crate) fn sin_cambios(
        &mut self,
        dedo: bool,
        t: f64,
        a: &AjustesStickPanel,
        sticks: &mut SticksAccion,
    ) {
        if self.bola_activa {
            let (dx, dy) = self.bola.paso(t);
            self.mover(dx, dy, t, a, sticks);
        } else if dedo && !self.repetido {
            self.repetido = true;
            let (dx, dy) = self.antes;
            self.mover(dx, dy, t, a, sticks);
        } else if !dedo {
            let f = 1.0 / t;
            self.filtro.x.filtrar(0.0, f);
            self.filtro.y.filtrar(0.0, f);
        }
    }

    /// `TouchpadMouseStick`.
    fn mover(
        &mut self,
        mut dx: i32,
        mut dy: i32,
        t: f64,
        a: &AjustesStickPanel,
        sticks: &mut SticksAccion,
    ) {
        // C# saca el sentido de la salida de antes de girar.
        let mut dir = (direccion(dx), direccion(dy));
        if a.rotacion != 0.0 {
            let (s, c) = (a.rotacion.sin(), a.rotacion.cos());
            let (x, y) = (dx as f64, dy as f64);
            dx = acotar(x * c - y * s, PANEL_ANCHO as f64) as i32;
            dy = acotar(x * s + y * c, PANEL_ALTO as f64) as i32;
        }
        let angulo = (-dy as f64).atan2(dx as f64);
        let nx = angulo.cos().abs();
        let ny = angulo.sin().abs();
        let sx = dx.signum();
        let sy = dy.signum();
        let zx = (nx * a.zona_muerta as f64).abs() as i32;
        let radial_y = (ny * a.zona_muerta as f64).abs() as i32;
        let maxz = a.zona_maxima;
        let mut max_x = sx * maxz;
        let mut max_y = sy * maxz;
        let axial_max = (maxz as f64 * 0.30) as i32;
        let axial_min = (maxz as f64 * 0.06) as i32;
        let tope = |v: i32, max: i32| {
            if (v < 0 && v < max) || (v > 0 && v > max) {
                max
            } else {
                v
            }
        };

        if dx * dx + dy * dy <= 0 {
            dx = 0;
            dy = 0;
        } else {
            // En forma de pajarita: cuanto más se mueve un eje, más zona muerta tiene el otro.
            let rx = dx.abs() as f64 / (max_x as f64).abs();
            let ry = dy.abs() as f64 / (max_y as f64).abs();
            let rango = (axial_max - axial_min) as f64;
            let axial_x = (rango * min_net(1.0, ry) + axial_min as f64) as i32;
            let axial_y = (rango * min_net(1.0, rx) + axial_min as f64) as i32;
            if dx.abs() > axial_x {
                let usar = if zx > axial_x { zx } else { axial_x };
                dx -= sx * usar;
                let nuevo_max = (max_x as f64).abs() - usar as f64;
                let escala = dx.abs() as f64 / nuevo_max;
                dx = tope((max_x as f64 * escala) as i32, max_x);
            } else {
                dx = 0;
            }
            if dy.abs() > axial_y {
                let usar = if radial_y > axial_y {
                    radial_y
                } else {
                    axial_y
                };
                dy -= sy * usar;
                let nuevo_max = (max_y as f64).abs() - usar as f64;
                let escala = dy.abs() as f64 / nuevo_max;
                dy = tope((max_y as f64 * escala) as i32, max_y);
            } else {
                dy = 0;
            }
        }

        if a.suavizado {
            let f = 1.0 / t;
            // C# sube un poco la entrada para que el redondeo del filtro no se coma el valor.
            let mut tx = self.filtro.x.filtrar(dx as f64 * 1.0005, f);
            let mut ty = self.filtro.y.filtrar(dy as f64 * 1.0005, f);
            if tx.abs() < 0.0001 {
                tx = 0.0;
            }
            if ty.abs() < 0.0001 {
                ty = 0.0;
            }
            dx = acotar(tx, maxz as f64) as i32;
            dy = acotar(ty, maxz as f64) as i32;
            max_x = if dx < 0 { -maxz } else { maxz };
            max_y = if dy < 0 { -maxz } else { maxz };
            dir = (direccion(dx), direccion(dy));
        }

        let mut r = Recorrido {
            dx,
            dy,
            max_x,
            max_y,
            norm_x: nx,
            norm_y: ny,
            dir_x: dir.0,
            dir_y: dir.1,
        };
        let (mut x, mut y) = r.proporcion();
        // C# escala la Y después de calcular la proporción: solo cambia hacia dónde se invierte.
        if a.escala_vertical != 100 {
            r.dy = tope(
                (dy as f64 * (a.escala_vertical as f64 * 0.01)) as i32,
                max_y,
            );
        }
        if let Some(maxima) = a.salida_maxima {
            (x, y) = r.limitar(x, y, maxima);
        }
        if a.curva != CurvaPanel::Linear {
            (x, y) = curva(a.curva, x, y);
        }
        let (bx, by) = r.salida(x, y, a.antizona, a.invertir);
        sticks.poner(a.stick, a.ejes, bx, by);
    }
}

/// `Global.Clamp(-max, v, max)`.
fn acotar(v: f64, max: f64) -> f64 {
    if v < -max {
        -max
    } else if v > max {
        max
    } else {
        v
    }
}

/// `StickOutCurve.CalcOutValue` para las curvas que no son lineales.
fn curva(tipo: CurvaPanel, x: f64, y: f64) -> (f64, f64) {
    let r = y.atan2(x);
    let mut cap_x = r.cos().abs();
    let mut cap_y = r.sin().abs();
    if x.abs() > cap_x {
        cap_x = x.abs();
    }
    if y.abs() > cap_y {
        cap_y = y.abs();
    }
    let rx = if cap_x > 0.0 { x / cap_x } else { 0.0 };
    let ry = if cap_y > 0.0 { y / cap_y } else { 0.0 };
    let sx = if rx >= 0.0 { 1.0 } else { -1.0 };
    let sy = if ry >= 0.0 { 1.0 } else { -1.0 };
    let precision = |a: f64| {
        if a <= 0.4 {
            0.8 * a
        } else if a <= 0.75 {
            a - 0.08
        } else if a > 0.75 {
            (a * 1.32) - 0.32
        } else {
            0.0
        }
    };
    match tipo {
        CurvaPanel::EnhancedPrecision => (
            sx * precision(rx.abs()) * cap_x,
            sy * precision(ry.abs()) * cap_y,
        ),
        CurvaPanel::Quadratic => (sx * rx * rx * cap_x, sy * ry * ry * cap_y),
        CurvaPanel::Cubic => (rx * rx * rx * cap_x, ry * ry * ry * cap_y),
        CurvaPanel::EaseoutQuad => {
            let (ax, ay) = (rx.abs(), ry.abs());
            (
                -(ax * (ax - 2.0)) * sx * cap_x,
                -(ay * (ay - 2.0)) * sy * cap_y,
            )
        }
        CurvaPanel::EaseoutCubic => {
            let (ix, iy) = (rx.abs() - 1.0, ry.abs() - 1.0);
            (
                (ix * ix * ix + 1.0) * sx * cap_x,
                (iy * iy * iy + 1.0) * sy * cap_y,
            )
        }
        CurvaPanel::Linear => (x, y),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn ajustes() -> AjustesStickPanel {
        AjustesStickPanel {
            zona_muerta: 0,
            zona_maxima: 100,
            stick: PalancaSalida::RightStick,
            ejes: EjesSalida::XY,
            antizona: (0.4, 0.4),
            invertir: 0,
            salida_maxima: None,
            escala_vertical: 100,
            curva: CurvaPanel::Linear,
            rotacion: 0.0,
            suavizado: false,
        }
    }

    /// Con la Y a tope, la zona muerta de la X es el 30 % de la zona máxima: 31 ya sale.
    #[test]
    fn la_zona_muerta_en_pajarita_es_el_30_por_ciento() {
        let a = ajustes();
        let mut s = SticksAccion::default();
        StickPanel::default().mover(31, 200, 0.004, &a, &mut s);
        assert_ne!(s.rx, 128);
        let mut s = SticksAccion::default();
        StickPanel::default().mover(30, 200, 0.004, &a, &mut s);
        assert_eq!(s.rx, 128);
    }

    /// La escala vertical va después de la proporción: con la Y invertida, un movimiento hacia
    /// arriba que la escala deja en 0 sale hacia arriba en vez de hacia abajo.
    #[test]
    fn la_escala_vertical_solo_cambia_la_inversion() {
        let mut a = ajustes();
        a.zona_maxima = 8;
        a.invertir = 2;
        let mut s = SticksAccion::default();
        StickPanel::default().mover(0, -1, 0.004, &a, &mut s);
        assert!(s.ry > 128, "{}", s.ry);
        a.escala_vertical = 50;
        let mut s = SticksAccion::default();
        StickPanel::default().mover(0, -1, 0.004, &a, &mut s);
        assert!(s.ry < 128, "{}", s.ry);
    }
}
