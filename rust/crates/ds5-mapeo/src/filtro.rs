//! El filtro One Euro (`Sensorit.Base.OneEuroFilter`) con el que C# suaviza el giroscopio como
//! ratón y como stick, y el panel como stick.

use std::f64::consts::PI;

/// `LowpassFilter`.
#[derive(Debug, Clone, Copy, Default)]
struct PasoBajo {
    usado: bool,
    anterior: f64,
}

impl PasoBajo {
    fn filtrar(&mut self, x: f64, alfa: f64) -> f64 {
        let v = if self.usado {
            alfa * x + (1.0 - alfa) * self.anterior
        } else {
            self.usado = true;
            x
        };
        self.anterior = v;
        v
    }
}

/// `OneEuroFilter`.
#[derive(Debug, Clone, Copy)]
pub struct UnEuro {
    pub corte_minimo: f64,
    pub beta: f64,
    usado: bool,
    x: PasoBajo,
    dx: PasoBajo,
}

impl UnEuro {
    pub fn nuevo(corte_minimo: f64, beta: f64) -> UnEuro {
        UnEuro {
            corte_minimo,
            beta,
            usado: false,
            x: PasoBajo::default(),
            dx: PasoBajo::default(),
        }
    }

    fn alfa(frecuencia: f64, corte: f64) -> f64 {
        let tau = 1.0 / (2.0 * PI * corte);
        let te = 1.0 / frecuencia;
        1.0 / (1.0 + tau / te)
    }

    /// `Filter(x, rate)`, con la frecuencia en informes por segundo.
    pub fn filtrar(&mut self, x: f64, frecuencia: f64) -> f64 {
        let dx = if self.usado {
            (x - self.x.anterior) * frecuencia
        } else {
            self.usado = true;
            0.0
        };
        let edx = self.dx.filtrar(dx, Self::alfa(frecuencia, 1.0));
        let corte = self.corte_minimo + self.beta * edx.abs();
        self.x.filtrar(x, Self::alfa(frecuencia, corte))
    }

    /// `Reset`: como C#, el filtro de la derivada sigue dando su primer valor por usado.
    pub fn reiniciar(&mut self) {
        self.x = PasoBajo::default();
        self.dx = PasoBajo::default();
    }
}

/// `OneEuroFilterPair`: uno por eje.
#[derive(Debug, Clone, Copy)]
pub struct ParUnEuro {
    pub x: UnEuro,
    pub y: UnEuro,
}

impl ParUnEuro {
    pub fn nuevo(corte_minimo: f64, beta: f64) -> ParUnEuro {
        ParUnEuro {
            x: UnEuro::nuevo(corte_minimo, beta),
            y: UnEuro::nuevo(corte_minimo, beta),
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// El primer valor sale tal cual y después se acerca poco a poco al nuevo.
    #[test]
    fn el_filtro_suaviza_un_salto() {
        let mut f = UnEuro::nuevo(1.0, 0.0);
        assert_eq!(f.filtrar(10.0, 250.0), 10.0);
        let v = f.filtrar(20.0, 250.0);
        assert!(v > 10.0 && v < 11.0, "{v}");
    }
}
