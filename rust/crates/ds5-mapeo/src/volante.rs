//! El volante con el giroscopio (`Mapping.Scale360degreeGyroAxis`) y su calibración
//! (`SAWheelEmulationCalibration`). El ángulo sale de la aceleración en X y Z comparada con tres
//! puntos del mando: recto y girado 90 grados a cada lado. Va en décimas de grado.

use crate::botones::LuzForzada;
use crate::filtro::UnEuro;
use ds5_informes::{boton, Estado};
use ds5_perfiles::perfil::{Ajustes as AjustesPerfil, EjeVolante, MandoVirtual};

/// `C_WHEEL_ANGLE_PRECISION`: el ángulo va en décimas de grado.
const PRECISION: i32 = 10;

const COLOR_CALIBRACION: [(u8, u8, u8); 4] = [
    (0xA0, 0x00, 0x00),
    (0xFF, 0xFF, 0x00),
    (0x00, 0x50, 0x50),
    (0x00, 0xC0, 0x00),
];

/// `WheelCalibrationPoint`.
pub mod punto_calibrado {
    pub const CENTRO: u8 = 1;
    pub const DERECHA_90: u8 = 2;
    pub const IZQUIERDA_90: u8 = 4;
    pub const TODOS: u8 = CENTRO | DERECHA_90 | IZQUIERDA_90;
}

/// Un punto de la aceleración (`System.Drawing.Point`). En C# un punto a (0, 0) cuenta como
/// vacío.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Punto {
    pub x: i32,
    pub y: i32,
}

impl Punto {
    fn vacio(self) -> bool {
        self.x == 0 && self.y == 0
    }
}

/// Lo que se guarda de cada mando en `ControllerConfigs.xml`: el centro y los dos puntos de
/// 90 grados.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CalibracionVolante {
    pub centro: Punto,
    pub izquierda_90: Punto,
    pub derecha_90: Punto,
}

/// Los ajustes del volante en el perfil.
#[derive(Debug, Clone, PartialEq)]
pub struct AjustesVolante {
    pub eje: EjeVolante,
    /// `SASteeringWheelEmulationRange`, en grados.
    pub rango: i32,
    /// `SASteeringWheelFuzz`.
    pub difuminado: i32,
    pub suavizado: bool,
    pub corte_minimo: f64,
    pub beta: f64,
    /// `SXDeadZone`: la zona muerta del eje X del giroscopio, en fracción de 20 grados.
    pub zona_muerta: f64,
    /// `SXAntiDeadZone`, en fracción.
    pub antizona: f64,
    /// El mando virtual: el DS4 va de 0 a 255 y el Xbox de -32768 a 32767.
    pub mando: MandoVirtual,
    /// El color de la luz del perfil (`getMainColor`), al que vuelve al acabar de calibrar.
    pub color: (u8, u8, u8),
}

impl AjustesVolante {
    pub fn desde_perfil(p: &AjustesPerfil) -> AjustesVolante {
        let s = &p.suavizado_volante;
        AjustesVolante {
            eje: p.eje_volante,
            rango: p.rango_volante,
            difuminado: p.difuminado_volante,
            suavizado: s.activo,
            corte_minimo: s.corte_minimo,
            beta: s.beta,
            zona_muerta: p.sx_zona_muerta,
            antizona: (p.sx_antizona * 0.01).clamp(0.0, 1.0),
            mando: p.mando_virtual,
            color: (p.color.0, p.color.1, p.color.2),
        }
    }
}

/// `ClampInt`: sin fallar si el mínimo pasa del máximo.
fn limitar(min: i32, v: i32, max: i32) -> i32 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

/// `Convert.ToInt32(double)`: redondea al par más cercano.
fn entero(v: f64) -> i32 {
    v.round_ties_even() as i32
}

/// `CalculateControllerAngle`: el ángulo del mando en décimas de grado, de -1800 a 1800.
pub fn angulo(x: i32, z: i32, c: &Puntos) -> i32 {
    let centro = c.centro;
    if x == centro.x && (z - centro.y).abs() <= 1 {
        return 0;
    }
    let circulo = if x >= centro.x {
        c.circulo_derecha
    } else {
        c.circulo_izquierda
    };
    let ab = ((centro.x - circulo.x) as f64, (centro.y - circulo.y) as f64);
    let cd = ((x - circulo.x) as f64, (z - circulo.y) as f64);
    let producto = ab.0 * cd.0 + ab.1 * cd.1;
    let mod_ab = (ab.0 * ab.0 + ab.1 * ab.1).sqrt();
    let mod_cd = (cd.0 * cd.0 + cd.1 * cd.1).sqrt();
    let mut r = if mod_ab == 0.0 || mod_cd == 0.0 {
        0
    } else {
        let a = (producto / (mod_ab * mod_cd)).clamp(-1.0, 1.0).acos();
        // `Math.Round(grados, 1)`: con .NET multiplica por 10, redondea al par y divide.
        let grados = (a * (180.0 / std::f64::consts::PI) * 10.0).round_ties_even() / 10.0;
        let lim = 180.0 * PRECISION as f64;
        entero((grados * PRECISION as f64).clamp(-lim, lim))
    };
    if x < centro.x {
        r = -r;
    }
    r
}

/// Los puntos con los que se calcula el ángulo: los guardados y los centros de los dos
/// círculos.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Puntos {
    pub centro: Punto,
    pub izquierda_90: Punto,
    pub derecha_90: Punto,
    pub circulo_izquierda: Punto,
    pub circulo_derecha: Punto,
}

/// Lo que el volante pide a la app.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SalidaVolante {
    /// Guardar esta calibración en `ControllerConfigs.xml` (`SaveControllerConfigs`).
    pub guardar: Option<CalibracionVolante>,
}

/// El volante de un mando: lo que C# guarda en el mando y en `Mapping`.
#[derive(Debug, Clone)]
pub struct Volante {
    /// La calibración guardada del mando, la que C# lee de `ControllerConfigs.xml` cuando el
    /// centro está vacío.
    pub guardada: Option<CalibracionVolante>,
    pub puntos: Puntos,
    /// `WheelRecalibrateActiveState`: 0 sin calibrar, 1 empezar, 2 calibrando, 3 acabar y
    /// 4 cancelar.
    pub calibrando: u8,
    /// `wheelCalibratedAxisBitmask`.
    pub calibrados: u8,
    /// `wheelPrevRecalibrateTime`, en microsegundos; `None` es el año 2500 de C#.
    momento_recalibrar_us: Option<i64>,
    angulo_fisico_antes: i32,
    angulo_total_antes: i32,
    vueltas: i32,
    /// `lastWheelGyroValues`: la aceleración que dejó pasar el difuminado.
    ultimo: Punto,
    filtro: UnEuro,
    pub salida: SalidaVolante,
}

impl Volante {
    pub fn nuevo(guardada: Option<CalibracionVolante>) -> Volante {
        Volante {
            guardada,
            puntos: Puntos::default(),
            calibrando: 0,
            calibrados: 0,
            momento_recalibrar_us: None,
            angulo_fisico_antes: 0,
            angulo_total_antes: 0,
            vueltas: 0,
            ultimo: Punto::default(),
            // `SetupInitialHookEvents`: el filtro toma los valores del perfil y los sigue cuando
            // cambian, así que se ponen en cada informe.
            filtro: UnEuro::nuevo(0.1, 0.1),
            salida: SalidaVolante::default(),
        }
    }

    /// `Scale360degreeGyroAxis`: lo que sale por el eje del volante. `crudo` es el estado del
    /// mando sin curvas, del que salen la aceleración, el tiempo entre informes y, al calibrar,
    /// la Equis y el giro.
    pub fn escalar(
        &mut self,
        crudo: &Estado,
        a: &AjustesVolante,
        ahora_us: i64,
        luz: &mut LuzForzada,
    ) -> i32 {
        self.salida = SalidaVolante::default();
        let Some(m) = crudo.movimiento else {
            return 0;
        };
        if self.calibrando > 0 {
            self.calibrar(crudo, a, ahora_us, luz);
            return 0;
        }
        let (mut x, mut z) = (m.acel_x, m.acel_z);

        if self.puntos.centro.vacio() {
            let p = &mut self.puntos;
            match self.guardada.filter(|g| !g.centro.vacio()) {
                Some(g) => {
                    p.centro = g.centro;
                    p.izquierda_90 = g.izquierda_90;
                    p.derecha_90 = g.derecha_90;
                }
                None => {
                    // Sin calibrar: unos valores estimados.
                    p.centro = Punto { x, y: z };
                    p.derecha_90 = Punto {
                        x: x + 113,
                        y: z + 110,
                    };
                    p.izquierda_90 = Punto {
                        x: x - 127,
                        y: p.derecha_90.y,
                    };
                }
            }
            p.circulo_derecha = Punto {
                x: p.centro.x,
                y: p.derecha_90.y,
            };
            p.circulo_izquierda = Punto {
                x: p.centro.x,
                y: p.izquierda_90.y,
            };
        }

        let max_der = a.rango / 2 * PRECISION;
        let max_izq = -max_der;

        if a.difuminado != 0 {
            let u = self.ultimo;
            if !(x == 0 || x == 128 || x == -128 || (x - u.x).abs() > a.difuminado) {
                x = u.x;
            }
            if !(z == 0 || z == 128 || z == -128 || (z - u.y).abs() > a.difuminado) {
                z = u.y;
            }
            self.ultimo = Punto { x, y: z };
        }

        let mut r = angulo(x, z, &self.puntos);

        if a.zona_muerta > 0.0 {
            let zona = entero(20.0 * PRECISION as f64 * a.zona_muerta);
            if r.abs() <= zona {
                r = 0;
            } else {
                r -= if r < 0 { -zona } else { zona };
            }
        }

        let media_vuelta = 180 * PRECISION;
        let mut vueltas = self.vueltas;
        if self.angulo_fisico_antes < 0 && r > 0 {
            if r - self.angulo_fisico_antes > media_vuelta {
                if max_der > media_vuelta {
                    vueltas -= 1;
                } else {
                    r = max_izq;
                }
            }
        } else if self.angulo_fisico_antes > 0
            && r < 0
            && self.angulo_fisico_antes - r > media_vuelta
        {
            if max_der > media_vuelta {
                vueltas += 1;
            } else {
                r = max_der;
            }
        }
        self.angulo_fisico_antes = r;

        if vueltas != 0 {
            r = r.wrapping_add(vueltas.wrapping_mul(media_vuelta * 2));
        }
        if (r - self.angulo_total_antes).abs() <= media_vuelta {
            self.angulo_total_antes = r;
            self.vueltas = vueltas;
        } else {
            r = self.angulo_total_antes;
        }

        r = limitar(max_izq, r, max_der);
        if a.suavizado {
            let frecuencia = 1.0 / crudo.transcurrido;
            self.filtro.corte_minimo = a.corte_minimo;
            self.filtro.beta = a.beta;
            r = self.filtro.filtrar(r as f64 * 1.0005, frecuencia) as i32;
            r = limitar(max_izq, r, max_der);
        }

        salida_eje(r, a, max_izq, max_der)
    }

    /// `SAWheelEmulationCalibration`.
    fn calibrar(
        &mut self,
        crudo: &Estado,
        a: &AjustesVolante,
        ahora_us: i64,
        luz: &mut LuzForzada,
    ) {
        let m = crudo.movimiento.unwrap_or_default();
        let (x, z) = (m.acel_x, m.acel_z);
        let p = &mut self.puntos;
        match self.calibrando {
            1 => {
                self.calibrando = 2;
                self.angulo_fisico_antes = 0;
                self.angulo_total_antes = 0;
                self.vueltas = 0;
                p.centro = Punto { x, y: z };
                p.derecha_90.x = x + 20;
                p.izquierda_90.x = x - 20;
                self.calibrados = 0;
                self.momento_recalibrar_us = None;
            }
            3 => {
                if self.calibrados & punto_calibrado::TODOS == punto_calibrado::TODOS {
                    let c = CalibracionVolante {
                        centro: p.centro,
                        izquierda_90: p.izquierda_90,
                        derecha_90: p.derecha_90,
                    };
                    self.guardada = Some(c);
                    self.salida.guardar = Some(c);
                } else {
                    p.centro = Punto::default();
                }
                self.calibrando = 0;
                self.momento_recalibrar_us = Some(ahora_us);
            }
            4 => {
                self.calibrando = 0;
                self.momento_recalibrar_us = Some(ahora_us);
            }
            _ => {}
        }

        if self.calibrando > 0 {
            if crudo.pulsado(boton::EQUIS) {
                self.momento_recalibrar_us = Some(ahora_us);
            }
            let quieto = m.vel_pitch.abs() < 0.5 && m.vel_yaw.abs() < 0.5 && m.vel_roll.abs() < 0.5;
            if quieto
                && self
                    .momento_recalibrar_us
                    .is_some_and(|t| ahora_us - t > 1_000_000)
            {
                self.momento_recalibrar_us = None;
                if self.calibrados == 0 {
                    p.centro = Punto { x, y: z };
                    self.calibrados |= punto_calibrado::CENTRO;
                } else if p.derecha_90.x < x {
                    p.derecha_90 = Punto { x, y: z };
                    p.circulo_derecha = Punto {
                        x: p.centro.x,
                        y: z,
                    };
                    self.calibrados |= punto_calibrado::DERECHA_90;
                } else if p.izquierda_90.x > x {
                    p.izquierda_90 = Punto { x, y: z };
                    p.circulo_izquierda = Punto {
                        x: p.centro.x,
                        y: z,
                    };
                    self.calibrados |= punto_calibrado::IZQUIERDA_90;
                }
            }

            let bits = self.calibrados.count_ones() as usize;
            luz.color = COLOR_CALIBRACION[bits.min(3)];
            let r = angulo(x, z, p);
            let c = self.calibrados;
            let marca = (c & punto_calibrado::CENTRO != 0 && r.abs() <= PRECISION)
                || (c & punto_calibrado::IZQUIERDA_90 != 0
                    && (-91 * PRECISION..=-89 * PRECISION).contains(&r))
                || (c & punto_calibrado::DERECHA_90 != 0
                    && (89 * PRECISION..=91 * PRECISION).contains(&r))
                || (c & punto_calibrado::IZQUIERDA_90 != 0 && r.abs() >= 179 * PRECISION);
            luz.destello = if marca { 2 } else { 0 };
            luz.activa = true;
        } else {
            luz.destello = 0;
            luz.color = a.color;
            luz.activa = false;
        }
    }
}

/// La última parte de `Scale360degreeGyroAxis`: el ángulo pasado al eje de salida.
fn salida_eje(r: i32, a: &AjustesVolante, max_izq: i32, max_der: i32) -> i32 {
    let mut anti = a.antizona;
    let (max, min, cero) = if a.mando == MandoVirtual::DS4 {
        (255, 0, 128)
    } else {
        (32767, -32768, 0)
    };
    match a.eje {
        EjeVolante::LX | EjeVolante::LY | EjeVolante::RX | EjeVolante::RY => {
            if r == 0 {
                return cero;
            }
            if anti > 0.0 {
                anti *= ((if r < 0 { min } else { max }) - cero) as f64;
                let ai = anti as i32;
                if r < 0 {
                    ((min - ai) as f64 * (r as f64 / max_izq as f64) + ai as f64) as i32
                } else {
                    ((max - ai) as f64 * (r as f64 / max_der as f64) + ai as f64) as i32
                }
            } else {
                (r - max_izq)
                    .wrapping_mul(max - min)
                    .checked_div(max_der - max_izq)
                    .unwrap_or(0)
                    + min
            }
        }
        EjeVolante::L2R2 => {
            if r == 0 {
                return 0;
            }
            let mut g = entero(r as f64 / PRECISION as f64);
            if g < 0 {
                g = -181 - g;
            }
            if anti > 0.0 {
                let ai = entero(anti * 255.0);
                if g < 0 {
                    ((g + 180) * (-ai + 255)) / 180 - 255
                } else {
                    (g * (255 - ai)) / 180 + ai
                }
            } else {
                ((g + 180) * 510) / 360 - 255
            }
        }
        EjeVolante::VJoy1X
        | EjeVolante::VJoy1Y
        | EjeVolante::VJoy1Z
        | EjeVolante::VJoy2X
        | EjeVolante::VJoy2Y
        | EjeVolante::VJoy2Z => {
            if r == 0 {
                return 16384;
            }
            if anti > 0.0 {
                let ai = entero(anti * 16384.0);
                if r < 0 {
                    ((r - max_izq) * (16384 - ai))
                        .checked_div(-max_izq)
                        .unwrap_or(0)
                } else {
                    (r * (32767 - (16384 + ai)))
                        .checked_div(max_der)
                        .unwrap_or(0)
                        + 16384
                        + ai
                }
            } else {
                (r - max_izq)
                    .wrapping_mul(32767)
                    .checked_div(max_der - max_izq)
                    .unwrap_or(0)
            }
        }
        EjeVolante::None => 0,
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use ds5_informes::Movimiento;

    fn ajustes() -> AjustesVolante {
        AjustesVolante {
            eje: EjeVolante::LX,
            rango: 360,
            difuminado: 0,
            suavizado: false,
            corte_minimo: 0.1,
            beta: 0.1,
            zona_muerta: 0.0,
            antizona: 0.0,
            mando: MandoVirtual::X360,
            color: (1, 2, 3),
        }
    }

    fn estado(x: i32, z: i32, vel: f64, equis: bool) -> Estado {
        let m = Movimiento {
            acel_x: x,
            acel_z: z,
            vel_yaw: vel,
            ..Movimiento::default()
        };
        Estado {
            botones: if equis { boton::EQUIS } else { 0 },
            transcurrido: 0.004,
            movimiento: Some(m),
            ..Estado::default()
        }
    }

    /// Sin calibrar, el centro es la primera lectura y girar 90 grados a la derecha (113, 110
    /// más allá) da medio eje.
    #[test]
    fn sin_calibrar_estima_los_puntos() {
        let a = ajustes();
        let mut v = Volante::nuevo(None);
        let mut luz = LuzForzada::default();
        assert_eq!(v.escalar(&estado(10, 20, 0.0, false), &a, 0, &mut luz), 0);
        let r = v.escalar(&estado(123, 130, 0.0, false), &a, 4000, &mut luz);
        assert_eq!(r, (900 + 1800) * 65535 / 3600 - 32768);
    }

    /// La calibración: Equis y quieto más de un segundo guarda el centro, luego la derecha y la
    /// izquierda; al acabar pide guardarla. Mientras, la luz va cambiando de color.
    #[test]
    fn la_calibracion_captura_los_tres_puntos() {
        let a = ajustes();
        let mut v = Volante::nuevo(None);
        let mut luz = LuzForzada::default();
        v.calibrando = 1;
        let mut t = 0;
        let mut paso = |v: &mut Volante, x, z, equis| {
            t += 4000;
            v.escalar(&estado(x, z, 0.0, equis), &a, t, &mut luz);
            (luz.activa, luz.color)
        };
        assert_eq!(paso(&mut v, 0, 100, false), (true, COLOR_CALIBRACION[0]));
        assert_eq!(v.calibrando, 2);
        // Sin pulsar Equis no captura nada, aunque esté quieto.
        for _ in 0..300 {
            paso(&mut v, 0, 100, false);
        }
        assert_eq!(v.calibrados, 0);
        paso(&mut v, 0, 100, true);
        for _ in 0..251 {
            paso(&mut v, 0, 100, false);
        }
        assert_eq!(v.calibrados, punto_calibrado::CENTRO);
        assert_eq!(paso(&mut v, 0, 100, false).1, COLOR_CALIBRACION[1]);
        paso(&mut v, 110, 200, true);
        for _ in 0..251 {
            paso(&mut v, 110, 200, false);
        }
        paso(&mut v, -120, 210, true);
        for _ in 0..251 {
            paso(&mut v, -120, 210, false);
        }
        assert_eq!(v.calibrados, punto_calibrado::TODOS);
        assert_eq!(paso(&mut v, 0, 100, false).1, COLOR_CALIBRACION[3]);
        v.calibrando = 3;
        assert_eq!(paso(&mut v, 0, 100, false), (false, (1, 2, 3)));
        let g = v.salida.guardar.expect("pide guardar la calibración");
        assert_eq!(g.centro, Punto { x: 0, y: 100 });
        assert_eq!(g.derecha_90, Punto { x: 110, y: 200 });
        assert_eq!(g.izquierda_90, Punto { x: -120, y: 210 });
        assert_eq!(v.calibrando, 0);
    }

    /// Los puntos estimados sin calibrar: 113 y 110 a la derecha, 127 a la izquierda.
    #[test]
    fn los_puntos_estimados_son_los_de_c() {
        let a = ajustes();
        let mut v = Volante::nuevo(None);
        let mut luz = LuzForzada::default();
        v.escalar(&estado(10, 20, 0.0, false), &a, 0, &mut luz);
        assert_eq!(v.puntos.derecha_90, Punto { x: 123, y: 130 });
        assert_eq!(v.puntos.izquierda_90, Punto { x: -117, y: 130 });
        assert_eq!(v.puntos.circulo_izquierda, Punto { x: 10, y: 130 });
    }

    /// La zona muerta se redondea (`Convert.ToInt32`): 1,7 décimas son 2, no 1.
    #[test]
    fn la_zona_muerta_se_redondea() {
        let mut a = ajustes();
        a.zona_muerta = 0.0085;
        let guardada = CalibracionVolante {
            centro: Punto { x: 0, y: 100 },
            izquierda_90: Punto { x: -1000, y: 1000 },
            derecha_90: Punto { x: 1000, y: 1000 },
        };
        let mut v = Volante::nuevo(Some(guardada));
        let mut luz = LuzForzada::default();
        // Un ángulo de unas 2 décimas de grado: con zona 2 queda en 0.
        let x = (900.0 * (0.2f64).to_radians().tan()).round() as i32;
        v.escalar(&estado(x, 100, 0.0, false), &a, 0, &mut luz);
        assert_eq!(angulo(x, 100, &v.puntos), 2);
        assert_eq!(
            v.escalar(&estado(x, 100, 0.0, false), &a, 4000, &mut luz),
            0
        );
    }

    /// Una vuelta cuenta solo si el salto pasa de 180 grados; justo 180 no.
    #[test]
    fn justo_media_vuelta_no_cuenta() {
        let mut a = ajustes();
        a.rango = 720;
        a.zona_muerta = 0.0;
        let mut v = Volante::nuevo(None);
        v.angulo_fisico_antes = -900;
        v.angulo_total_antes = -900;
        // El centro no puede ser (0, 0): C# lo toma por vacío y vuelve a estimarlo.
        v.puntos = Puntos {
            centro: Punto { x: 0, y: 1 },
            izquierda_90: Punto { x: -100, y: 100 },
            derecha_90: Punto { x: 100, y: 100 },
            circulo_izquierda: Punto { x: 0, y: 100 },
            circulo_derecha: Punto { x: 0, y: 100 },
        };
        let mut luz = LuzForzada::default();
        // De -90 a +90 grados: 1800 décimas, no pasa de 180.
        v.escalar(&estado(100, 100, 0.0, false), &a, 0, &mut luz);
        assert_eq!(v.vueltas, 0);
    }

    /// Por vJoy sin antizona, el ángulo va de 0 a 32767 en todo el rango.
    #[test]
    fn por_vjoy_sin_antizona_va_de_0_a_32767() {
        let mut a = ajustes();
        a.eje = EjeVolante::VJoy1X;
        assert_eq!(salida_eje(1800, &a, -1800, 1800), 32767);
        assert_eq!(salida_eje(-900, &a, -1800, 1800), 900 * 32767 / 3600);
    }

    /// Si al acabar faltan puntos, no se guarda y el centro se vacía: vuelve a la guardada.
    #[test]
    fn la_calibracion_a_medias_no_se_guarda() {
        let a = ajustes();
        let guardada = CalibracionVolante {
            centro: Punto { x: 5, y: 6 },
            izquierda_90: Punto { x: -100, y: 120 },
            derecha_90: Punto { x: 110, y: 120 },
        };
        let mut v = Volante::nuevo(Some(guardada));
        let mut luz = LuzForzada::default();
        v.calibrando = 1;
        v.escalar(&estado(0, 0, 0.0, false), &a, 0, &mut luz);
        v.calibrando = 3;
        v.escalar(&estado(0, 0, 0.0, false), &a, 4000, &mut luz);
        assert_eq!(v.salida.guardar, None);
        v.escalar(&estado(5, 6, 0.0, false), &a, 8000, &mut luz);
        assert_eq!(v.puntos.centro, guardada.centro);
    }
}
