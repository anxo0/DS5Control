//! Dónde estaba la ventana (`Ventana.txt`), con el mismo formato que `SavePlacement` y
//! `RestorePlacement` de C#: `x;y;ancho;alto;maximizada`, en píxeles lógicos y con punto decimal.

use std::time::Duration;

pub const ARCHIVO: &str = "Ventana.txt";
/// El temporizador `placement` de C#: se guarda cuando la ventana lleva este rato quieta.
pub const ESPERA: Duration = Duration::from_millis(400);
pub const ANCHO_MINIMO: f64 = 960.0;
pub const ALTO_MINIMO: f64 = 640.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colocacion {
    pub x: f64,
    pub y: f64,
    pub ancho: f64,
    pub alto: f64,
    pub maximizada: bool,
}

/// `double.ToString(CultureInfo.InvariantCulture)`: sin decimales si es entero.
fn numero(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

impl Colocacion {
    pub fn leer(texto: &str) -> Option<Colocacion> {
        let partes: Vec<&str> = texto.trim().split(';').collect();
        if partes.len() != 5 {
            return None;
        }
        let mut v = [0.0; 4];
        for (i, p) in partes[..4].iter().enumerate() {
            v[i] = p.trim().parse::<f64>().ok().filter(|x| x.is_finite())?;
        }
        Some(Colocacion {
            x: v[0],
            y: v[1],
            ancho: v[2].max(ANCHO_MINIMO),
            alto: v[3].max(ALTO_MINIMO),
            maximizada: partes[4] == "True",
        })
    }

    pub fn texto(&self) -> String {
        format!(
            "{};{};{};{};{}",
            numero(self.x),
            numero(self.y),
            numero(self.ancho),
            numero(self.alto),
            if self.maximizada { "True" } else { "False" }
        )
    }

    /// Un monitor que ya no está no debe dejar la ventana fuera de la vista: tiene que tocar la
    /// pantalla virtual (`x, y, ancho, alto`).
    pub fn se_ve_en(&self, pantalla: (f64, f64, f64, f64)) -> bool {
        let (px, py, pa, pl) = pantalla;
        self.x < px + pa && px < self.x + self.ancho && self.y < py + pl && py < self.y + self.alto
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn lee_y_escribe_como_c() {
        let c = Colocacion::leer("311;95;1320;820;False\r\n").unwrap();
        assert_eq!(
            c,
            Colocacion {
                x: 311.0,
                y: 95.0,
                ancho: 1320.0,
                alto: 820.0,
                maximizada: false
            }
        );
        assert_eq!(c.texto(), "311;95;1320;820;False");
        let d = Colocacion::leer("10.5;-3;500;300;True").unwrap();
        assert_eq!(d.ancho, ANCHO_MINIMO);
        assert_eq!(d.texto(), "10.5;-3;960;640;True");
        assert!(Colocacion::leer("1;2;3").is_none());
        assert!(Colocacion::leer("a;2;3;4;False").is_none());
    }

    #[test]
    fn fuera_de_la_pantalla_no_vale() {
        let c = Colocacion::leer("3000;100;1320;820;False").unwrap();
        assert!(!c.se_ve_en((0.0, 0.0, 1920.0, 1080.0)));
        assert!(c.se_ve_en((0.0, 0.0, 3840.0, 1080.0)));
    }
}
