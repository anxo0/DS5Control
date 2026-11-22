//! El suavizado del giroscopio y el acelerómetro para el servidor, como `PrepareDevUDPMotion` en
//! `ControlService.cs`: un filtro One Euro por eje (`OneEuroFilter` y `LowpassFilter`), con la
//! frecuencia sacada del tiempo entre informes. Es el de `UDPServerSmoothingOptions` en
//! Profiles.xml.

use std::f64::consts::PI;

use crate::paquetes::MovimientoDsu;

/// Los ajustes del suavizado (`UdpSmoothMinCutoff` y `UdpSmoothBeta`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Suavizado {
    pub corte_minimo: f64,
    pub beta: f64,
}

impl Default for Suavizado {
    /// `DEFAULT_UDP_SMOOTH_MINCUTOFF` y `DEFAULT_UDP_SMOOTH_BETA`.
    fn default() -> Self {
        Suavizado {
            corte_minimo: 0.4,
            beta: 0.2,
        }
    }
}

impl Suavizado {
    /// Los deja en los límites que aplica C# al leer los ajustes.
    pub fn acotado(self) -> Self {
        Suavizado {
            corte_minimo: self.corte_minimo.clamp(0.00001, 100.0),
            beta: self.beta.clamp(0.0, 1.0),
        }
    }
}

/// `LowpassFilter`.
#[derive(Debug, Clone, Copy, Default)]
struct PasoBajo {
    iniciado: bool,
    anterior: f64,
}

impl PasoBajo {
    fn filtrar(&mut self, x: f64, alfa: f64) -> f64 {
        let v = if self.iniciado {
            alfa * x + (1.0 - alfa) * self.anterior
        } else {
            self.iniciado = true;
            x
        };
        self.anterior = v;
        v
    }
}

/// `OneEuroFilter`.
#[derive(Debug, Clone, Copy)]
pub struct FiltroEuro {
    ajustes: Suavizado,
    iniciado: bool,
    x: PasoBajo,
    dx: PasoBajo,
}

/// El corte de la derivada (`dcutoff`).
const CORTE_DERIVADA: f64 = 1.0;

fn alfa(frecuencia: f64, corte: f64) -> f64 {
    let tau = 1.0 / (2.0 * PI * corte);
    let te = 1.0 / frecuencia;
    1.0 / (1.0 + tau / te)
}

impl FiltroEuro {
    pub fn new(ajustes: Suavizado) -> Self {
        FiltroEuro {
            ajustes,
            iniciado: false,
            x: PasoBajo::default(),
            dx: PasoBajo::default(),
        }
    }

    /// Cambia los ajustes sin perder lo filtrado, como `SetFilterAttrs`.
    pub fn fijar(&mut self, ajustes: Suavizado) {
        self.ajustes = ajustes;
    }

    /// `frecuencia` en hercios: la inversa del tiempo entre informes.
    pub fn filtrar(&mut self, x: f64, frecuencia: f64) -> f64 {
        let dx = if self.iniciado {
            (x - self.x.anterior) * frecuencia
        } else {
            self.iniciado = true;
            0.0
        };
        let edx = self.dx.filtrar(dx, alfa(frecuencia, CORTE_DERIVADA));
        let corte = self.ajustes.corte_minimo + self.ajustes.beta * edx.abs();
        self.x.filtrar(x, alfa(frecuencia, corte))
    }
}

/// Los seis filtros de un mando (`udpEuroPairAccel` y `udpEuroPairGyro`).
#[derive(Debug, Clone, Copy)]
pub struct FiltroMovimiento {
    acel: [FiltroEuro; 3],
    giro: [FiltroEuro; 3],
}

impl FiltroMovimiento {
    pub fn new(ajustes: Suavizado) -> Self {
        FiltroMovimiento {
            acel: [FiltroEuro::new(ajustes); 3],
            giro: [FiltroEuro::new(ajustes); 3],
        }
    }

    pub fn fijar(&mut self, ajustes: Suavizado) {
        for f in self.acel.iter_mut().chain(self.giro.iter_mut()) {
            f.fijar(ajustes);
        }
    }

    /// Filtra el movimiento. `transcurrido` son los segundos desde el informe anterior; con 0
    /// no se puede filtrar y devuelve `false` sin tocar nada (C# no manda ese informe).
    pub fn aplicar(&mut self, m: &mut MovimientoDsu, transcurrido: f64) -> bool {
        if transcurrido == 0.0 {
            return false;
        }
        let f = 1.0 / transcurrido;
        m.acel_x = self.acel[0].filtrar(m.acel_x, f);
        m.acel_y = self.acel[1].filtrar(m.acel_y, f);
        m.acel_z = self.acel[2].filtrar(m.acel_z, f);
        m.vel_yaw = self.giro[0].filtrar(m.vel_yaw, f);
        m.vel_pitch = self.giro[1].filtrar(m.vel_pitch, f);
        m.vel_roll = self.giro[2].filtrar(m.vel_roll, f);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_primer_valor_pasa_tal_cual() {
        let mut f = FiltroEuro::new(Suavizado::default());
        assert_eq!(f.filtrar(12.5, 250.0), 12.5);
    }

    #[test]
    fn segundo_valor_calculado_a_mano() {
        // Con 0.4 y 0.2 a 250 Hz, de 0 a 10:
        // dx = 2500; alfa(250, 1) = 1 / (1 + 250 / 2pi); edx = alfa * 2500;
        // corte = 0.4 + 0.2 * edx; salida = alfa(250, corte) * 10.
        let mut f = FiltroEuro::new(Suavizado::default());
        f.filtrar(0.0, 250.0);
        let a1 = 1.0 / (1.0 + (1.0 / (2.0 * PI)) / (1.0 / 250.0));
        let corte = 0.4 + 0.2 * (a1 * 2500.0);
        let a2 = 1.0 / (1.0 + (1.0 / (2.0 * PI * corte)) / (1.0 / 250.0));
        let v = f.filtrar(10.0, 250.0);
        assert!((v - a2 * 10.0).abs() < 1e-12, "{v}");
        assert!(v > 0.0 && v < 10.0);
    }

    #[test]
    fn suaviza_el_ruido() {
        let mut f = FiltroEuro::new(Suavizado {
            corte_minimo: 0.4,
            beta: 0.0,
        });
        let mut salidas = Vec::new();
        for i in 0..2000 {
            let ruido = if i % 2 == 0 { 1.0 } else { -1.0 };
            salidas.push(f.filtrar(5.0 + ruido, 250.0));
        }
        // La entrada salta 2 en cada informe; la salida apenas se mueve y se queda en la media.
        let (a, b) = (salidas[1998], salidas[1999]);
        assert!((a - b).abs() < 0.05, "{a} {b}");
        assert!((b - 5.0).abs() < 0.05, "{b}");
    }

    #[test]
    fn sin_tiempo_no_filtra() {
        let mut f = FiltroMovimiento::new(Suavizado::default());
        let mut m = MovimientoDsu {
            vel_yaw: 3.0,
            ..Default::default()
        };
        assert!(!f.aplicar(&mut m, 0.0));
        assert_eq!(m.vel_yaw, 3.0);
        assert!(f.aplicar(&mut m, 0.004));
        assert_eq!(m.vel_yaw, 3.0);
    }

    #[test]
    fn acota_como_csharp() {
        let s = Suavizado {
            corte_minimo: 0.0,
            beta: 3.0,
        }
        .acotado();
        assert_eq!(s.corte_minimo, 0.00001);
        assert_eq!(s.beta, 1.0);
    }
}
