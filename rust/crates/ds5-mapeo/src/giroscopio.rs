//! El giroscopio como controles: el activador de `Mouse.IsGyroTriggerActive` y la aceleración
//! de salida que calcula `SetCurveAndDeadzone` al final (SX y SZ), que luego leen los controles
//! `GyroXPos`, `GyroZNeg` y compañía al reasignar.

use ds5_informes::{boton, Estado, Movimiento};
use ds5_perfiles::perfil::{Ajustes as AjustesPerfil, SalidaGiroscopio};

use crate::bezier::TipoEje;
use crate::curvas::{modo_curva, CurvaSalida};

/// Un eje del giroscopio como controles: X (inclinar a los lados) o Z (hacia delante y atrás).
#[derive(Debug, Clone, PartialEq)]
pub struct EjeGiro {
    /// De 0 a 1, en fracción de 128.
    pub zona_muerta: f64,
    /// De 0 a 1.
    pub zona_maxima: f64,
    /// De 0 a 1.
    pub antizona: f64,
    pub sensibilidad: f64,
    pub curva: CurvaSalida,
}

impl EjeGiro {
    /// La salida de un eje, de -128 a 128, como en `SetCurveAndDeadzone`.
    fn salida(&self, valor: i32, siempre_curva: bool) -> i32 {
        let zm = (128.0 * self.zona_muerta) as i32;
        let abs = valor.abs();
        let signo = valor.signum();
        let mut r = if zm > 0 || self.zona_maxima < 1.0 || self.antizona > 0.0 {
            let maximo = (self.zona_maxima * 128.0) as i32;
            if abs > zm {
                let ratio = if abs < maximo {
                    (abs - zm) as f64 / (maximo - zm) as f64
                } else {
                    1.0
                };
                signo
                    * 128f64.min(
                        self.sensibilidad * 128.0 * ((1.0 - self.antizona) * ratio + self.antizona),
                    ) as i32
            } else {
                0
            }
        } else {
            signo * 128f64.min(self.sensibilidad * 128.0 * (abs as f64 / 128.0)) as i32
        };

        // La curva de X se aplica siempre; la de Z solo si no es cero.
        if self.curva.modo > 0 && (siempre_curva || r != 0) {
            let t = r as f64 / 128.0;
            let s = signo_doble(t);
            let a = t.abs();
            r = match self.curva.modo {
                1 => {
                    let o = if a <= 0.4 {
                        0.55 * a
                    } else if a <= 0.75 {
                        a - 0.18
                    } else {
                        (a * 1.72) - 0.72
                    };
                    (o * s * 128.0) as i32
                }
                2 => (t * t * s * 128.0) as i32,
                3 => (t * t * t * 128.0) as i32,
                4 => (-(a * (a - 2.0)) * s * 128.0) as i32,
                5 => {
                    let i = a - 1.0;
                    ((i * i * i + 1.0) * s * 128.0) as i32
                }
                _ => {
                    let signo = r.signum();
                    self.curva.valor(r.unsigned_abs().min(128) as usize) as i32 * signo
                }
            };
        }
        r
    }
}

/// `Math.Sign` de un `double`.
fn signo_doble(v: f64) -> f64 {
    if v > 0.0 {
        1.0
    } else if v < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Qué botones encienden el giroscopio y cómo (`Triggers`, `TriggerCond`...).
#[derive(Debug, Clone, PartialEq)]
pub struct Activacion {
    /// Los botones, por su número en `getDS5ControlsByName` (-1 es siempre). `None` es un
    /// número que no se entiende.
    pub gatillos: Vec<Option<i32>>,
    /// Si la lista empieza vacía, no se mira ningún botón.
    pub vacia: bool,
    /// Todos los botones (`and`) o cualquiera.
    pub todos: bool,
    /// Si el giroscopio va con los botones pulsados (`TriggerTurns`) o con ellos sueltos.
    pub con_pulsados: bool,
    /// Cada pulsación lo enciende o lo apaga (`Toggle`).
    pub alterna: bool,
}

impl Activacion {
    /// La lista como la parte C#: `Split(',')` e `int.TryParse` de cada trozo.
    pub fn nueva(texto: &str, todos: bool, con_pulsados: bool, alterna: bool) -> Activacion {
        let trozos: Vec<&str> = texto.split(',').collect();
        Activacion {
            vacia: trozos[0].is_empty(),
            gatillos: trozos
                .iter()
                .map(|t| t.trim().parse::<i32>().ok())
                .collect(),
            todos,
            con_pulsados,
            alterna,
        }
    }
}

/// `getDS5ControlsByName` de `Mouse.cs`, sobre el estado del mando.
pub(crate) fn boton_activo(e: &Estado, n: i32) -> bool {
    let b = |bit| e.pulsado(bit);
    match n {
        -1 => true,
        0 => b(boton::EQUIS),
        1 => b(boton::CIRCULO),
        2 => b(boton::CUADRADO),
        3 => b(boton::TRIANGULO),
        4 => b(boton::L1),
        5 => e.l2 > 128,
        6 => b(boton::R1),
        7 => e.r2 > 128,
        8 => b(boton::ARRIBA),
        9 => b(boton::ABAJO),
        10 => b(boton::IZQUIERDA),
        11 => b(boton::DERECHA),
        12 => b(boton::L3),
        13 => b(boton::R3),
        14 => b(boton::UN_DEDO),
        15 => b(boton::DOS_DEDOS),
        16 => b(boton::OPCIONES),
        17 => b(boton::CREAR),
        18 => b(boton::PS),
        19 => b(boton::PANEL),
        20 => b(boton::MUTE),
        21 => b(boton::LATERAL_IZQ),
        22 => b(boton::LATERAL_DER),
        23 => b(boton::FN_IZQ),
        24 => b(boton::FN_DER),
        25 => b(boton::PALETA_IZQ),
        26 => b(boton::PALETA_DER),
        _ => false,
    }
}

/// Si los botones de activación cuentan como pulsados, sin el modo alterno.
pub(crate) fn gatillos_pulsados(e: &Estado, a: &Activacion) -> bool {
    let mut activo = a.todos;
    if !a.vacia {
        for g in &a.gatillos {
            let pulsado = g.is_some_and(|n| boton_activo(e, n));
            if a.todos && !pulsado {
                activo = false;
                break;
            } else if !a.todos && pulsado {
                activo = true;
                break;
            }
        }
    }
    activo
}

/// Lo que recuerda el activador entre informes. En C# es uno por mando y lo comparten todos los
/// modos del giroscopio.
#[derive(Debug, Clone, Default)]
pub struct Activador {
    anterior: bool,
    alterno: bool,
}

impl Activador {
    /// `IsGyroTriggerActive`: si los botones de activación cuentan como pulsados.
    pub fn pulsado(&mut self, e: &Estado, a: &Activacion) -> bool {
        let activo = gatillos_pulsados(e, a);
        if a.alterna {
            if activo && activo != self.anterior {
                self.alterno = !self.alterno;
            }
            self.anterior = activo;
            self.alterno
        } else {
            self.anterior = activo;
            activo
        }
    }

    /// `Mouse.sixaxisMoved` en el modo controles: decide si el giroscopio cuenta en este informe
    /// y lo apunta en el movimiento del estado.
    pub fn aplicar(&mut self, e: &mut Estado, a: &AjustesGiro) {
        if !a.controles || e.movimiento.is_none() {
            return;
        }
        let pulsado = self.pulsado(e, &a.activacion);
        if let Some(m) = e.movimiento.as_mut() {
            m.giro_controles = pulsado == a.activacion.con_pulsados;
        }
    }
}

/// El giroscopio como controles de un perfil.
#[derive(Debug, Clone, PartialEq)]
pub struct AjustesGiro {
    /// Si el giroscopio sale como controles (`GyroOutputMode` es `Controls`).
    pub controles: bool,
    pub x: EjeGiro,
    pub z: EjeGiro,
    pub activacion: Activacion,
}

impl AjustesGiro {
    pub fn desde_perfil(p: &AjustesPerfil) -> AjustesGiro {
        let eje = |zm: f64, zmax: f64, az: f64, sens: f64, curva, personalizada: Option<&str>| {
            EjeGiro {
                zona_muerta: zm,
                // C# las guarda en fracción; el perfil, en porcentaje.
                zona_maxima: (zmax * 0.01).clamp(0.0, 1.0),
                antizona: (az * 0.01).clamp(0.0, 1.0),
                sensibilidad: sens,
                curva: CurvaSalida::nueva(modo_curva(curva), personalizada, TipoEje::Movimiento),
            }
        };
        let c = &p.controles_giroscopio;
        AjustesGiro {
            controles: p.salida_giroscopio == SalidaGiroscopio::Controls,
            x: eje(
                p.sx_zona_muerta,
                p.sx_zona_maxima,
                p.sx_antizona,
                p.sensibilidades[4],
                p.sx_curva,
                p.sx_curva_personalizada.as_deref(),
            ),
            z: eje(
                p.sz_zona_muerta,
                p.sz_zona_maxima,
                p.sz_antizona,
                p.sensibilidades[5],
                p.sz_curva,
                p.sz_curva_personalizada.as_deref(),
            ),
            // En el modo controles, el sentido es el del giroscopio en general
            // (`GyroTriggerTurns`), y el modo alterno el de los controles.
            activacion: Activacion::nueva(
                &c.gatillos,
                c.condicion,
                p.gatillo_alterna_giroscopio,
                c.alterna,
            ),
        }
    }

    /// La parte final de `SetCurveAndDeadzone`: la aceleración que sale como controles.
    pub fn salida(&self, m: &mut Movimiento) {
        if self.controles && m.giro_controles {
            m.salida_acel_x = self.x.salida(m.acel_x, true);
            m.salida_acel_z = self.z.salida(m.acel_z, false);
        }
    }
}
