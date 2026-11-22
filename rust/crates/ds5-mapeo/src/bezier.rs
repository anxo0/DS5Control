//! La curva de salida personalizada (`BezierCurve.cs`): una tabla de 256 valores calculada con
//! una curva de Bézier cúbica, como el `KeySpline` de Firefox.

use std::sync::OnceLock;

use regex::Regex;

const TAMANO_TABLA: usize = 11;
const PASO: f64 = 1.0 / (TAMANO_TABLA as f64 - 1.0);
const ITERACIONES_NEWTON: usize = 4;
const PENDIENTE_MINIMA_NEWTON: f64 = 0.001;
const PRECISION_SUBDIVISION: f64 = 0.0000001;
const ITERACIONES_SUBDIVISION: usize = 10;

/// El eje al que se aplica, que decide el rango de la tabla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoEje {
    /// Sticks: centro en 128, 127 pasos a cada lado.
    Palanca,
    /// Gatillos: de 0 a 255.
    Gatillo,
    /// Giroscopio como controles: de 0 a 128.
    Movimiento,
}

/// Los cuatro números de una definición como `"0,70, 0,28, 1,00, 1,00"`, leídos como
/// `BezierCurve.AsString`: la misma expresión regular y `double.TryParse` con la configuración
/// española. Si la expresión no casa, `None` (C# se queda con la curva que tuviera antes, que al
/// arrancar es la lineal); si casa pero un número no se entiende, todo a cero.
pub fn leer_definicion(texto: &str) -> Option<[f64; 4]> {
    static PATRON: OnceLock<Regex> = OnceLock::new();
    let patron = PATRON.get_or_init(|| {
        Regex::new(r"(.{3,9}),\s?(.{3,9}),\s?(.{3,9}),\s?(.{3,9})").expect("patrón válido")
    });
    let c = patron.captures(texto)?;
    let mut v = [0.0; 4];
    for (i, valor) in v.iter_mut().enumerate() {
        match ds5_perfiles::numeros::doble_es(&c[i + 1]) {
            Some(n) => *valor = n,
            None => return Some([0.0; 4]),
        }
    }
    Some(v)
}

fn a(a1: f64, a2: f64) -> f64 {
    1.0 - 3.0 * a2 + 3.0 * a1
}
fn b(a1: f64, a2: f64) -> f64 {
    3.0 * a2 - 6.0 * a1
}
fn c(a1: f64) -> f64 {
    3.0 * a1
}
fn calcular(t: f64, a1: f64, a2: f64) -> f64 {
    ((a(a1, a2) * t + b(a1, a2)) * t + c(a1)) * t
}
fn pendiente(t: f64, a1: f64, a2: f64) -> f64 {
    3.0 * a(a1, a2) * t * t + 2.0 * b(a1, a2) * t + c(a1)
}

struct Curva {
    x1: f64,
    x2: f64,
    muestras: [f64; TAMANO_TABLA],
}

impl Curva {
    fn t_para_x(&self, x: f64) -> f64 {
        let mut inicio = 0.0;
        let mut muestra = 1;
        let ultima = TAMANO_TABLA - 1;
        while muestra != ultima && self.muestras[muestra] <= x {
            inicio += PASO;
            muestra += 1;
        }
        muestra -= 1;
        let dist =
            (x - self.muestras[muestra]) / (self.muestras[muestra + 1] - self.muestras[muestra]);
        let t = inicio + dist * PASO;
        let p = pendiente(t, self.x1, self.x2);
        if p >= PENDIENTE_MINIMA_NEWTON {
            self.newton(x, t)
        } else if p == 0.0 {
            t
        } else {
            self.subdividir(x, inicio, inicio + PASO)
        }
    }

    fn newton(&self, x: f64, mut t: f64) -> f64 {
        for _ in 0..ITERACIONES_NEWTON {
            let p = pendiente(t, self.x1, self.x2);
            if p == 0.0 {
                return t;
            }
            t -= (calcular(t, self.x1, self.x2) - x) / p;
        }
        t
    }

    fn subdividir(&self, x: f64, mut a: f64, mut b: f64) -> f64 {
        let mut i = 0;
        loop {
            let t = a + (b - a) / 2.0;
            let actual = calcular(t, self.x1, self.x2) - x;
            if actual > 0.0 {
                b = t;
            } else {
                a = t;
            }
            i += 1;
            if !(actual.abs() > PRECISION_SUBDIVISION && i < ITERACIONES_SUBDIVISION) {
                return t;
            }
        }
    }
}

/// Como el `(byte)` de C# sobre un `double`: se trunca a entero y se queda el byte bajo.
pub fn byte_net(v: f64) -> u8 {
    v as i32 as u8
}

/// La tabla de una curva, como `InitBezierCurve`. Las definiciones con `x1 = 99` son las curvas
/// fijas de siempre (91 a 95).
pub fn tabla(def: [f64; 4], eje: TipoEje) -> [u8; 256] {
    let [x1, y1, x2, y2] = def;
    let (maximo, centro): (f64, f64) = match eje {
        TipoEje::Palanca => (127.0, 128.0),
        TipoEje::Gatillo => (255.0, 0.0),
        TipoEje::Movimiento => (128.0, 0.0),
    };
    let mut t = [0u8; 256];
    let palanca = eje == TipoEje::Palanca;
    let centro_i = centro as usize;
    let max_i = maximo as usize;
    // Cada valor de la mitad positiva y, en los sticks, su reflejo en la negativa.
    let poner = |t: &mut [u8; 256], idx: usize, v: u8| {
        t[idx + centro_i] = v;
        if palanca {
            t[127 - idx] = 255 - v;
        }
    };

    if x1 == 99.0 {
        let fija: Option<fn(f64) -> f64> = match y1 {
            91.0 => Some(|a| {
                if a <= 0.4 {
                    0.55 * a
                } else if a <= 0.75 {
                    a - 0.18
                } else {
                    (a * 1.72) - 0.72
                }
            }),
            92.0 => Some(|a| a * a),
            93.0 => Some(|a| a * a * a),
            94.0 => Some(|a| -(a * (a - 2.0))),
            95.0 => Some(|a| {
                let i = a - 1.0;
                1.0 * ((i * i * i) + 1.0)
            }),
            _ => None,
        };
        if let Some(f) = fija {
            for idx in 0..=max_i {
                let a = idx as f64 / maximo;
                poner(&mut t, idx, byte_net(f(a) * maximo + centro));
            }
            return t;
        }
    }

    let (mx1, my1, mx2, my2) = if !(0.0..=1.0).contains(&x1) || !(0.0..=1.0).contains(&x2) {
        (0.0, 0.0, 0.0, 0.0)
    } else {
        (x1, y1, x2, y2)
    };
    if x1 == 0.0 && y1 == 0.0 && ((x2 == 0.0 && y2 == 0.0) || (x2 == 1.0 && y2 == 1.0)) {
        for (i, v) in t.iter_mut().enumerate() {
            *v = i as u8;
        }
        return t;
    }
    let mut muestras = [0.0; TAMANO_TABLA];
    for (i, m) in muestras.iter_mut().enumerate() {
        *m = calcular(i as f64 * PASO, mx1, mx2);
    }
    let curva = Curva {
        x1: mx1,
        x2: mx2,
        muestras,
    };
    for idx in 0..=max_i {
        let y = calcular(curva.t_para_x(idx as f64 / maximo), my1, my2) * maximo;
        let v = y.round_ties_even().clamp(0.0, maximo) + centro;
        poner(&mut t, idx, byte_net(v));
    }
    t
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn las_definiciones_se_leen_como_en_c() {
        assert_eq!(
            leer_definicion("0,70, 0,28, 1,00, 1,00"),
            Some([0.7, 0.28, 1.0, 1.0])
        );
        // Con la configuración española el punto es de millares: "0.70" es 70.
        assert_eq!(
            leer_definicion("0.70, 0.28, 1.00, 1.00"),
            Some([70.0, 28.0, 100.0, 100.0])
        );
        assert_eq!(leer_definicion(""), None);
    }

    #[test]
    fn la_lineal_es_la_identidad() {
        let t = tabla([0.0, 0.0, 1.0, 1.0], TipoEje::Gatillo);
        assert!(t.iter().enumerate().all(|(i, &v)| v as usize == i));
    }
}
