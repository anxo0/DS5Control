//! El flick stick (`ProcessFlickStick` y `HandleFlickStickAngle`): al empujar el stick, la cámara
//! gira de golpe hacia donde apunta; al girarlo empujado, gira con él. Mueve el ratón en
//! horizontal y usa el estado sin curvas del mando.

use ds5_perfiles::perfil::FlickStick;

/// `FlickStickMappingData`. C# guarda uno por mando, que comparten los dos sticks.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Flick {
    progreso: f64,
    tamano: f64,
    resto: f64,
}

/// `WarpEaseOut`.
fn suavizar(t: f64) -> f64 {
    let f = 1.0 - t;
    1.0 - f * f
}

impl Flick {
    /// `HandleFlickStickAngle`: cuánto gira la cámara, en radianes. `transcurrido` es el tiempo
    /// del informe en segundos.
    fn angulo(&mut self, x: u8, y: u8, px: u8, py: u8, a: &FlickStick, transcurrido: f64) -> f64 {
        let mut r = 0.0;
        let norma = |v: u8| {
            let max = if v >= 128 { 127.0 } else { -128.0 };
            (v as i32 - 128) as f64 / max
        };
        let (lx, ly, cx, cy) = (norma(px), norma(py), norma(x), norma(y));
        let largo_antes = (lx * lx) + (ly * ly);
        let largo = (cx * cx) + (cy * cy);
        let umbral = a.umbral * a.umbral;
        let angulo_de = |x: u8, y: u8| ((x as i32 - 128) as f64).atan2((-(y as i32 - 128)) as f64);

        if largo >= umbral {
            if largo_antes < umbral {
                // Empieza un flick.
                self.progreso = 0.0;
                self.tamano = angulo_de(x, y);
            } else {
                // Gira la cámara con el stick.
                let cambio = angulo_de(x, y) - angulo_de(px, py);
                let mut cambio = (cambio + std::f64::consts::PI) % (2.0 * std::f64::consts::PI);
                if cambio < 0.0 {
                    cambio += 2.0 * std::f64::consts::PI;
                }
                cambio -= std::f64::consts::PI;
                r += cambio;
            }
        } else {
            r = 0.0;
        }

        // Sigue el giro del flick.
        let antes = self.progreso;
        let tiempo = a.tiempo;
        if antes < tiempo {
            self.progreso = (self.progreso + transcurrido).min(tiempo);
            let t_antes = suavizar(antes / tiempo);
            let t_ahora = suavizar(self.progreso / tiempo);
            r += (t_ahora - t_antes) * self.tamano;
        }
        r
    }

    /// `ProcessFlickStick`: lo que mueve el ratón en horizontal. `x` e `y` son el stick sin
    /// curvas y `px`, `py` los del informe anterior.
    #[allow(clippy::too_many_arguments)]
    pub fn procesar(
        &mut self,
        x: u8,
        y: u8,
        px: u8,
        py: u8,
        a: &FlickStick,
        transcurrido: f64,
    ) -> f64 {
        let cambio = self.angulo(x, y, px, py, a, transcurrido);
        let mut grados = cambio * 180.0 / std::f64::consts::PI;
        if grados == 0.0 {
            self.resto = 0.0;
        } else if grados >= 0.0 && self.resto >= 0.0 {
            grados += self.resto;
        }
        self.resto = 0.0;
        if (a.angulo_minimo == 0.0 && grados != 0.0) || grados.abs() >= a.angulo_minimo {
            grados * a.calibracion
        } else {
            self.resto = grados;
            0.0
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// Al empujar el stick a la derecha, la cámara gira un cuarto de vuelta repartido en el
    /// tiempo del flick.
    #[test]
    fn un_flick_gira_hacia_donde_apunta_el_stick() {
        let a = FlickStick {
            calibracion: 1.0,
            umbral: 0.9,
            tiempo: 0.1,
            angulo_minimo: 0.0,
        };
        let mut f = Flick::default();
        let mut total = 0.0;
        let (mut px, mut py) = (128, 128);
        for _ in 0..20 {
            total += f.procesar(255, 128, px, py, &a, 0.01);
            (px, py) = (255, 128);
        }
        assert!((total - 90.0).abs() < 1e-9, "{total}");
    }
}
