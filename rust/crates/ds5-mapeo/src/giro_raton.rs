//! El giroscopio como ratón (`MouseCursor.sixaxisMoved`), como stick (`Mouse.SixMouseStick`) y
//! con gestos (`Mouse.SixDirectionalSwipe`). C# lo hace en el hilo del mando cada vez que llega un
//! informe con movimiento, antes de las curvas y de la reasignación.

use ds5_informes::{Estado, Movimiento};
use ds5_perfiles::perfil::{EjeDeslizar, EjesSalida, PalancaSalida, Perfil, SalidaGiroscopio};
use ds5_perfiles::MetodoSuavizado;

use crate::filtro::ParUnEuro;
use crate::giroscopio::{gatillos_pulsados, Activacion, Activador};
use crate::panel::dos_decimales;
use crate::raton::EventoRaton;
use crate::sticks::{direccion, Recorrido, SticksAccion};

/// `GyroMouseSensDualSense`.
const COEFICIENTE: f64 = 0.009;
const DESFASE: f64 = 0.15;
/// El filtro del stick: C# lo crea con estos valores y nunca le pasa los del perfil.
const STICK_CORTE: f64 = 0.4;
const STICK_BETA: f64 = 0.7;
const SUAVE_LARGO: usize = 3;

/// El giroscopio como ratón.
#[derive(Debug, Clone, PartialEq)]
pub struct AjustesRatonGiro {
    /// `GyroSensitivity`, en porcentaje. Con 0 no se mueve.
    pub sensibilidad: i32,
    pub escala_vertical: i32,
    /// Bit 2 invierte la X; bit 1, la Y.
    pub invertir: i32,
    /// 0 es girar (yaw) y 1 es inclinar (roll).
    pub eje_horizontal: i32,
    pub zona_muerta: i32,
    pub umbral: f64,
    pub compensar_temblor: bool,
    pub suavizado: bool,
    pub metodo: MetodoSuavizado,
    pub corte_minimo: f64,
    pub beta: f64,
    pub activacion: Activacion,
}

/// El giroscopio como stick (`GyroMouseStickInfo`).
#[derive(Debug, Clone, PartialEq)]
pub struct AjustesStickGiro {
    pub eje_horizontal: i32,
    pub zona_muerta: i32,
    pub zona_maxima: i32,
    pub antizona: (f64, f64),
    pub escala_vertical: i32,
    pub salida_maxima: Option<f64>,
    pub invertir: u32,
    pub suavizado: bool,
    pub metodo: MetodoSuavizado,
    /// De 0 a 1.
    pub peso: f64,
    pub compensar_temblor: bool,
    pub stick: PalancaSalida,
    pub ejes: EjesSalida,
    pub activacion: Activacion,
}

/// Los gestos con el giroscopio (`GyroDirectionalSwipeInfo`).
#[derive(Debug, Clone, PartialEq)]
pub struct AjustesGestosGiro {
    /// En grados por segundo.
    pub zona_x: i32,
    pub zona_y: i32,
    pub balanceo: bool,
    pub retardo_ms: i32,
    /// Sin modo alterno: C# no lo mira para los gestos.
    pub activacion: Activacion,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AjustesGiroRaton {
    pub modo: SalidaGiroscopio,
    pub raton: AjustesRatonGiro,
    pub stick: AjustesStickGiro,
    pub gestos: AjustesGestosGiro,
}

impl AjustesGiroRaton {
    pub fn desde_perfil(p: &Perfil) -> AjustesGiroRaton {
        let a = &p.ajustes;
        // El modo alterno es siempre el de los controles: `IsGyroTriggerActive` solo mira ese.
        let alterna = a.controles_giroscopio.alterna;
        let sr = &a.suavizado_giroscopio_raton;
        let ss = &a.suavizado_giroscopio_stick;
        let g = &a.deslizar_giroscopio;
        AjustesGiroRaton {
            modo: a.salida_giroscopio,
            raton: AjustesRatonGiro {
                sensibilidad: a.sensibilidad_giroscopio,
                escala_vertical: a.escala_vertical_giroscopio,
                invertir: a.invertir_giroscopio,
                eje_horizontal: a.eje_horizontal_giroscopio_raton,
                zona_muerta: a.zona_muerta_giroscopio_raton,
                umbral: a.umbral_giroscopio_raton,
                compensar_temblor: a.compensar_temblor_giroscopio_raton,
                suavizado: sr.activo,
                metodo: sr.metodo,
                corte_minimo: sr.corte_minimo,
                beta: sr.beta,
                activacion: Activacion::nueva(
                    &a.gatillos_giroscopio,
                    a.condicion_giroscopio,
                    a.gatillo_alterna_giroscopio,
                    alterna,
                ),
            },
            stick: AjustesStickGiro {
                eje_horizontal: a.eje_horizontal_giroscopio_stick,
                zona_muerta: a.zona_muerta_giroscopio_stick,
                zona_maxima: a.zona_maxima_giroscopio_stick,
                antizona: (a.antizona_giroscopio_stick_x, a.antizona_giroscopio_stick_y),
                escala_vertical: a.escala_vertical_giroscopio_stick,
                salida_maxima: a
                    .salida_maxima_giroscopio_stick_activa
                    .then_some(a.salida_maxima_giroscopio_stick),
                invertir: a.invertir_giroscopio_stick,
                suavizado: ss.activo,
                metodo: ss.metodo,
                peso: ss.peso * 0.01,
                compensar_temblor: a.compensar_temblor_giroscopio_stick,
                stick: a.stick_giroscopio,
                ejes: a.ejes_giroscopio_stick,
                activacion: Activacion::nueva(
                    &a.gatillos_giroscopio_stick,
                    a.condicion_giroscopio_stick,
                    a.gatillo_alterna_giroscopio_stick,
                    alterna,
                ),
            },
            gestos: AjustesGestosGiro {
                zona_x: g.zona_muerta_x,
                zona_y: g.zona_muerta_y,
                balanceo: g.eje_x == EjeDeslizar::Roll,
                retardo_ms: g.retardo,
                activacion: Activacion::nueva(&g.gatillos, g.condicion, g.gatillo_alterna, false),
            },
        }
    }
}

/// `GyroSwipeData`: hacia dónde va cada eje y desde cuándo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct EjeGesto {
    /// -1, 0 o 1.
    sentido: i32,
    desde_us: i64,
    activo: bool,
}

impl EjeGesto {
    /// Si el gesto cuenta: con retardo, solo después de ir ese tiempo hacia el mismo lado.
    fn mover(&mut self, sentido: i32, retardo_ms: i32, ahora_us: i64) -> bool {
        if self.sentido != sentido {
            self.desde_us = ahora_us;
            self.sentido = sentido;
            self.activo = retardo_ms == 0;
        }
        if !self.activo {
            self.activo = self.desde_us + retardo_ms as i64 * 1000 < ahora_us;
        }
        self.activo
    }
}

/// Lo que recuerda el giroscopio entre informes.
#[derive(Debug, Clone)]
pub struct GiroRaton {
    // MouseCursor
    resto_h: f64,
    resto_v: f64,
    suave_x: [f64; SUAVE_LARGO],
    suave_y: [f64; SUAVE_LARGO],
    suave_cola: usize,
    filtro_raton: ParUnEuro,
    // Mouse (stick)
    stick_x: [i32; SUAVE_LARGO],
    stick_y: [i32; SUAVE_LARGO],
    stick_cola: usize,
    filtro_stick: ParUnEuro,
    // Gestos: izquierda, derecha, arriba y abajo, ahora y en el informe anterior.
    gestos: [bool; 4],
    gestos_antes: [bool; 4],
    gesto_x: EjeGesto,
    gesto_y: EjeGesto,
}

impl GiroRaton {
    /// Como queda al conectar el mando: el filtro del ratón con los valores del perfil
    /// (`SetupLateOneEuroFilters`) y el del stick con los de siempre.
    pub fn nuevo(a: &AjustesGiroRaton) -> GiroRaton {
        GiroRaton {
            resto_h: 0.0,
            resto_v: 0.0,
            suave_x: [0.0; SUAVE_LARGO],
            suave_y: [0.0; SUAVE_LARGO],
            suave_cola: 0,
            filtro_raton: ParUnEuro::nuevo(a.raton.corte_minimo, a.raton.beta),
            stick_x: [0; SUAVE_LARGO],
            stick_y: [0; SUAVE_LARGO],
            stick_cola: 0,
            filtro_stick: ParUnEuro::nuevo(STICK_CORTE, STICK_BETA),
            gestos: [false; 4],
            gestos_antes: [false; 4],
            gesto_x: EjeGesto::default(),
            gesto_y: EjeGesto::default(),
        }
    }

    /// Los gestos de ahora y los del informe anterior, para la reasignación.
    pub fn gestos(&self) -> ([bool; 4], [bool; 4]) {
        (self.gestos, self.gestos_antes)
    }

    /// `Mouse.sixaxisMoved` en los modos ratón, stick y gestos. `e` es el estado del mando, del
    /// que se leen los botones de activación; el modo controles lo hace `Activador::aplicar`.
    pub fn procesar(
        &mut self,
        e: &Estado,
        a: &AjustesGiroRaton,
        activador: &mut Activador,
        sticks: &mut SticksAccion,
        eventos: &mut Vec<EventoRaton>,
        ahora_us: i64,
    ) {
        let Some(m) = e.movimiento.as_ref() else {
            return;
        };
        match a.modo {
            SalidaGiroscopio::Mouse if a.raton.sensibilidad > 0 => {
                let pulsado = activador.pulsado(e, &a.raton.activacion);
                if pulsado == a.raton.activacion.con_pulsados {
                    self.raton(m, &a.raton, eventos);
                } else {
                    self.raton_reposo(m, &a.raton);
                }
            }
            SalidaGiroscopio::MouseJoystick => {
                let pulsado = activador.pulsado(e, &a.stick.activacion);
                if pulsado == a.stick.activacion.con_pulsados {
                    self.stick(m, &a.stick, sticks);
                } else {
                    self.stick_reposo(m, &a.stick);
                }
            }
            SalidaGiroscopio::DirectionalSwipe => {
                let g = &a.gestos;
                let pulsado = gatillos_pulsados(e, &g.activacion);
                self.gestos_antes = self.gestos;
                if pulsado == g.activacion.con_pulsados {
                    self.gestos_mover(m, g, ahora_us);
                } else {
                    self.gestos = [false; 4];
                }
            }
            _ => {}
        }
    }

    /// `MouseCursor.sixaxisMoved`.
    fn raton(&mut self, m: &Movimiento, a: &AjustesRatonGiro, eventos: &mut Vec<EventoRaton>) {
        let mut dx = if a.eje_horizontal == 0 {
            m.giro_yaw_completo
        } else {
            m.giro_roll_completo
        };
        let mut dy = -m.giro_pitch_completo;
        let tiempo = m.transcurrido * 200.0;
        let coef = (a.sensibilidad as f64 * 0.01) * COEFICIENTE;

        let angulo = (-dy as f64).atan2(dx as f64);
        let nx = angulo.cos().abs();
        let ny = angulo.sin().abs();
        let sx = dx.signum();
        let sy = dy.signum();

        if dx == 0 || ((self.resto_h > 0.0) != (dx > 0)) {
            self.resto_h = 0.0;
        }
        if dy == 0 || ((self.resto_v > 0.0) != (dy > 0)) {
            self.resto_v = 0.0;
        }

        let zx = (nx * a.zona_muerta as f64).abs() as i32;
        let zy = (ny * a.zona_muerta as f64).abs() as i32;
        dx = if dx.abs() > zx { dx - sx * zx } else { 0 };
        dy = if dy.abs() > zy { dy - sy * zy } else { 0 };

        let mut mx = if dx != 0 {
            coef * (dx as f64 * tiempo) + (nx * (DESFASE * sx as f64))
        } else {
            0.0
        };
        let escala = a.escala_vertical as f64 * 0.01;
        let mut my = if dy != 0 {
            (coef * escala) * (dy as f64 * tiempo) + (ny * (DESFASE * sy as f64))
        } else {
            0.0
        };

        if a.compensar_temblor {
            const UMBRAL: f64 = 0.26;
            let umbral_f = 0.26f32 as f64;
            let ax = mx.abs();
            if ax <= nx * UMBRAL {
                mx = sx as f64 * (ax / umbral_f).powf(1.408) * UMBRAL;
            }
            let ay = my.abs();
            if ay <= ny * UMBRAL {
                my = sy as f64 * (ay / umbral_f).powf(1.408) * UMBRAL;
            }
        }

        if mx != 0.0 {
            mx += self.resto_h;
        } else {
            self.resto_h = 0.0;
        }
        if my != 0.0 {
            my += self.resto_v;
        } else {
            self.resto_v = 0.0;
        }

        if a.suavizado {
            if a.metodo == MetodoSuavizado::UnEuro {
                let f = 1.0 / m.transcurrido;
                mx = self.filtro_raton.x.filtrar(mx, f);
                my = self.filtro_raton.y.filtrar(my, f);
            } else {
                // C# no lee el peso del perfil: lo deja a cero, así que sale el valor de ahora.
                let i = self.suave_cola % SUAVE_LARGO;
                self.suave_x[i] = mx;
                self.suave_y[i] = my;
                self.suave_cola = i + 1;
                let (mut peso, mut total, mut x, mut y) = (1.0, 0.0, 0.0, 0.0);
                for k in 0..SUAVE_LARGO {
                    let idx = (self.suave_cola + SUAVE_LARGO - k - 1) % SUAVE_LARGO;
                    x += self.suave_x[idx] * peso;
                    y += self.suave_y[idx] * peso;
                    total += peso;
                    peso *= 0.0;
                }
                mx = x / total;
                my = y / total;
            }
        }

        self.resto_h = 0.0;
        self.resto_v = 0.0;
        mx = dos_decimales(mx);
        my = dos_decimales(my);

        let dist = mx * mx + my * my;
        let mut ax = mx as i32;
        let mut ay = my as i32;
        if a.umbral == 1.0 || dist >= a.umbral * a.umbral {
            self.resto_h = mx - ax as f64;
            self.resto_v = my - ay as f64;
        } else {
            self.resto_h = mx;
            self.resto_v = my;
            ax = 0;
            ay = 0;
        }

        if a.invertir & 2 == 2 {
            ax = -ax;
        }
        if a.invertir & 1 == 1 {
            ay = -ay;
        }
        if ax != 0 || ay != 0 {
            eventos.push(EventoRaton::Mover(ax, ay));
        }
    }

    /// `mouseRemainderReset`: con el giroscopio apagado se pierde lo que quedaba por mover.
    fn raton_reposo(&mut self, m: &Movimiento, a: &AjustesRatonGiro) {
        self.resto_h = 0.0;
        self.resto_v = 0.0;
        let i = self.suave_cola % SUAVE_LARGO;
        self.suave_x[i] = 0.0;
        self.suave_y[i] = 0.0;
        self.suave_cola = i + 1;
        if a.metodo == MetodoSuavizado::UnEuro {
            let f = 1.0 / m.transcurrido;
            self.filtro_raton.x.filtrar(0.0, f);
            self.filtro_raton.y.filtrar(0.0, f);
        }
    }

    /// `SixMouseStick`.
    fn stick(&mut self, m: &Movimiento, a: &AjustesStickGiro, sticks: &mut SticksAccion) {
        let mut dx = if a.eje_horizontal == 0 {
            m.giro_yaw_completo
        } else {
            m.giro_roll_completo
        };
        let mut dy = -m.giro_pitch_completo;
        let mut dir = (direccion(dx), direccion(dy));

        let angulo = (-dy as f64).atan2(dx as f64);
        let nx = angulo.cos().abs();
        let ny = angulo.sin().abs();
        let sx = dx.signum();
        let sy = dy.signum();
        let zx = (nx * a.zona_muerta as f64).abs() as i32;
        let zy = (ny * a.zona_muerta as f64).abs() as i32;
        let mut max_x = sx * a.zona_maxima;
        let mut max_y = sy * a.zona_maxima;
        let tope = |v: i32, max: i32| {
            if (v < 0 && v < max) || (v > 0 && v > max) {
                max
            } else {
                v
            }
        };

        dx = if dx.abs() > zx {
            tope(dx - sx * zx, max_x)
        } else {
            0
        };
        dy = if dy.abs() > zy {
            tope(dy - sy * zy, max_y)
        } else {
            0
        };

        if a.compensar_temblor {
            const UMBRAL: f64 = 2.0;
            let ax = dx.abs() as f64;
            if ax <= nx * UMBRAL {
                dx = (sx as f64 * (ax / UMBRAL).powf(1.408) * UMBRAL) as i32;
            }
            let ay = dy.abs() as f64;
            if ay <= ny * UMBRAL {
                dy = (sy as f64 * (ay / UMBRAL).powf(1.408) * UMBRAL) as i32;
            }
        }

        if a.suavizado {
            if a.metodo == MetodoSuavizado::UnEuro {
                let f = 1.0 / m.transcurrido;
                dx = self.filtro_stick.x.filtrar(dx as f64, f) as i32;
                dy = self.filtro_stick.y.filtrar(dy as f64, f) as i32;
            } else if a.metodo == MetodoSuavizado::MediaPonderada {
                let i = self.stick_cola % SUAVE_LARGO;
                self.stick_x[i] = dx;
                self.stick_y[i] = dy;
                self.stick_cola = i + 1;
                let (mut peso, mut total, mut x, mut y) = (1.0, 0.0, 0.0, 0.0);
                for k in 0..SUAVE_LARGO {
                    let idx = (self.stick_cola + SUAVE_LARGO - k - 1) % SUAVE_LARGO;
                    x += self.stick_x[idx] as f64 * peso;
                    y += self.stick_y[idx] as f64 * peso;
                    total += peso;
                    peso *= a.peso;
                }
                dx = (x / total) as i32;
                dy = (y / total) as i32;
            }
            max_x = if dx < 0 {
                -a.zona_maxima
            } else {
                a.zona_maxima
            };
            max_y = if dy < 0 {
                -a.zona_maxima
            } else {
                a.zona_maxima
            };
            dir = (direccion(dx), direccion(dy));
        }

        if a.escala_vertical != 100 {
            dy = tope(
                (dy as f64 * (a.escala_vertical as f64 * 0.01)) as i32,
                max_y,
            );
        }

        let r = Recorrido {
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
        if let Some(maxima) = a.salida_maxima {
            (x, y) = r.limitar(x, y, maxima);
        }
        let (bx, by) = r.salida(x, y, a.antizona, a.invertir);
        sticks.poner(a.stick, a.ejes, bx, by);
    }

    /// `SixMouseReset`.
    fn stick_reposo(&mut self, m: &Movimiento, a: &AjustesStickGiro) {
        let i = self.stick_cola % SUAVE_LARGO;
        self.stick_x[i] = 0;
        self.stick_y[i] = 0;
        self.stick_cola = i + 1;
        if a.metodo == MetodoSuavizado::UnEuro {
            let f = 1.0 / m.transcurrido;
            self.filtro_stick.x.filtrar(0.0, f);
            self.filtro_stick.y.filtrar(0.0, f);
        }
    }

    /// `SixDirectionalSwipe`. C# mide el retardo con el reloj; aquí, con la hora del informe.
    fn gestos_mover(&mut self, m: &Movimiento, a: &AjustesGestosGiro, ahora_us: i64) {
        let vx = if a.balanceo { m.vel_roll } else { m.vel_yaw };
        let vy = m.vel_pitch;
        let zx = a.zona_x.abs();
        let zy = a.zona_y.abs();

        self.gestos[0] = false;
        self.gestos[1] = false;
        if vx.abs() > zx as f64 {
            let sentido = if vx > 0.0 { 1 } else { -1 };
            if self.gesto_x.mover(sentido, a.retardo_ms, ahora_us) {
                self.gestos[if sentido > 0 { 1 } else { 0 }] = true;
            }
        } else {
            self.gesto_x.sentido = 0;
        }

        self.gestos[2] = false;
        self.gestos[3] = false;
        if vy.abs() > zy as f64 {
            let sentido = if vy > 0.0 { 1 } else { -1 };
            if self.gesto_y.mover(sentido, a.retardo_ms, ahora_us) {
                self.gestos[if sentido > 0 { 2 } else { 3 }] = true;
            }
        } else {
            self.gesto_y.sentido = 0;
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn girando(vel_yaw: f64) -> Estado {
        Estado {
            movimiento: Some(Movimiento {
                vel_yaw,
                transcurrido: 0.004,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Con retardo, el gesto solo cuenta cuando se lleva ese tiempo girando hacia el mismo lado.
    #[test]
    fn el_gesto_espera_su_retardo() {
        let mut a = AjustesGiroRaton::desde_perfil(&Perfil::default());
        a.modo = SalidaGiroscopio::DirectionalSwipe;
        a.gestos.retardo_ms = 50;
        let mut g = GiroRaton::nuevo(&a);
        let mut activador = Activador::default();
        let mut sticks = SticksAccion::default();
        let mut eventos = Vec::new();
        let mut paso = |g: &mut GiroRaton, vel: f64, ms: i64| {
            g.procesar(
                &girando(vel),
                &a,
                &mut activador,
                &mut sticks,
                &mut eventos,
                ms * 1000,
            );
            g.gestos().0
        };
        assert_eq!(paso(&mut g, 200.0, 0), [false; 4]);
        assert_eq!(paso(&mut g, 200.0, 40), [false; 4]);
        assert_eq!(paso(&mut g, 200.0, 51), [false, true, false, false]);
        // Al cambiar de lado vuelve a esperar.
        assert_eq!(paso(&mut g, -200.0, 60), [false; 4]);
        assert_eq!(paso(&mut g, -200.0, 111), [true, false, false, false]);
        assert_eq!(g.gestos().1, [false; 4]);
    }
}
