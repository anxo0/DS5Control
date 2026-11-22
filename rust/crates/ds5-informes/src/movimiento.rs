//! El giroscopio y el acelerómetro, como `DS5Sixaxis.cs`: la calibración de fábrica que manda el
//! mando y la calibración continua de los primeros cinco segundos, que resta la deriva del
//! giroscopio en reposo.

/// Unidades del acelerómetro por g.
pub const ACEL_POR_G: i32 = 8192;
/// Unidades del giroscopio por grado por segundo.
pub const GIRO_POR_GRADO: i32 = 16;

/// Lo que se sabe del movimiento en un informe, como `SixAxis` en C#.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Movimiento {
    pub giro_yaw: i32,
    pub giro_pitch: i32,
    pub giro_roll: i32,
    pub acel_x: i32,
    pub acel_y: i32,
    pub acel_z: i32,
    pub giro_yaw_completo: i32,
    pub giro_pitch_completo: i32,
    pub giro_roll_completo: i32,
    pub acel_x_completo: i32,
    pub acel_y_completo: i32,
    pub acel_z_completo: i32,
    /// Grados por segundo.
    pub vel_yaw: f64,
    pub vel_pitch: f64,
    pub vel_roll: f64,
    /// En g.
    pub acel_x_g: f64,
    pub acel_y_g: f64,
    pub acel_z_g: f64,
    /// Segundos desde el informe anterior.
    pub transcurrido: f64,
    /// La aceleración que sale como controles (`outputAccelX`, `Y`, `Z`): el decodificador la
    /// deja a cero y la calcula el mapeo.
    pub salida_acel_x: i32,
    pub salida_acel_y: i32,
    pub salida_acel_z: i32,
    /// Si el giroscopio cuenta como controles en este informe (`outputGyroControls`): lo decide
    /// el activador del giroscopio.
    pub giro_controles: bool,
}

impl Movimiento {
    /// Como `SixAxis.populate`. Los ejes vienen ya calibrados.
    pub fn nuevo(yaw: i32, pitch: i32, roll: i32, ax: i32, ay: i32, az: i32, dt: f64) -> Self {
        let giro = GIRO_POR_GRADO as f32;
        let acel = ACEL_POR_G as f32;
        let (yf, pf, rf) = (yaw.wrapping_neg(), pitch, roll.wrapping_neg());
        let (xf, yaf, zf) = (ax.wrapping_neg(), ay.wrapping_neg(), az);
        Movimiento {
            giro_yaw: yaw.wrapping_neg() / 256,
            giro_pitch: pitch / 256,
            giro_roll: roll.wrapping_neg() / 256,
            acel_x: ax.wrapping_neg() / 64,
            acel_y: ay.wrapping_neg() / 64,
            acel_z: az / 64,
            giro_yaw_completo: yf,
            giro_pitch_completo: pf,
            giro_roll_completo: rf,
            acel_x_completo: xf,
            acel_y_completo: yaf,
            acel_z_completo: zf,
            // En C# se dividen como float.
            vel_yaw: (yf as f32 / giro) as f64,
            vel_pitch: (pf as f32 / giro) as f64,
            vel_roll: (rf as f32 / giro) as f64,
            acel_x_g: (xf as f32 / acel) as f64,
            acel_y_g: (yaf as f32 / acel) as f64,
            acel_z_g: (zf as f32 / acel) as f64,
            transcurrido: dt,
            ..Default::default()
        }
    }
}

/// La calibración de un eje: se resta el sesgo y se multiplica por el factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eje {
    pub sesgo: i32,
    pub numerador: i32,
    pub denominador: i32,
}

impl Eje {
    fn aplicar(&self, v: i32) -> i32 {
        // Como en C#: `(int)(v * (numerador / (float)denominador))`, todo en float.
        let factor = self.numerador as f32 / self.denominador as f32;
        (v.wrapping_sub(self.sesgo) as f32 * factor) as i32
    }
}

/// La calibración de fábrica de los seis ejes: pitch, yaw, roll y acelerómetro X, Y, Z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Calibracion {
    pub ejes: [Eje; 6],
}

fn s16(b: &[u8], i: usize) -> i32 {
    i16::from_le_bytes([b[i], b[i + 1]]) as i32
}

impl Calibracion {
    /// Lee el informe de características 0x05, como `setCalibrationData` con el orden del
    /// DualSense (`useAltGyroCalib`). `None` si algún divisor es cero y no se puede usar.
    pub fn desde_informe(c: &[u8]) -> Option<Calibracion> {
        if c.len() < 35 {
            return None;
        }
        let (pitch_mas, pitch_menos) = (s16(c, 7), s16(c, 9));
        let (yaw_mas, yaw_menos) = (s16(c, 11), s16(c, 13));
        let (roll_mas, roll_menos) = (s16(c, 15), s16(c, 17));
        let velocidad = (s16(c, 19) + s16(c, 21)) * GIRO_POR_GRADO;
        let giro = |sesgo, mas: i32, menos: i32| Eje {
            sesgo,
            numerador: velocidad,
            denominador: mas - menos,
        };
        let acel = |i| {
            let (mas, menos) = (s16(c, i), s16(c, i + 2));
            let rango = mas - menos;
            Eje {
                sesgo: mas - rango / 2,
                numerador: 2 * ACEL_POR_G,
                denominador: rango,
            }
        };
        let ejes = [
            giro(s16(c, 1), pitch_mas, pitch_menos),
            giro(s16(c, 3), yaw_mas, yaw_menos),
            giro(s16(c, 5), roll_mas, roll_menos),
            acel(23),
            acel(27),
            acel(31),
        ];
        // C# comprueba los tres del giroscopio y el último del acelerómetro.
        let valida = ejes[..3].iter().all(|e| e.denominador != 0) && ejes[5].denominador != 0;
        valida.then_some(Calibracion { ejes })
    }
}

const VENTANAS: usize = 3;
const MS_VENTANA: i32 = 5000;
const MS_CALIBRACION: i64 = 5000;

/// `Convert.ToInt32(double)`: redondea al par más cercano.
fn redondear_ms(us: i64) -> i32 {
    ((us as f64) / 1000.0).round_ties_even() as i32
}

#[derive(Debug, Clone, Copy, Default)]
struct Ventana {
    x: i32,
    y: i32,
    z: i32,
    muestras: i32,
    inicio_us: i64,
    fin_us: i64,
}

impl Ventana {
    fn reiniciar(&mut self, ahora: i64) {
        *self = Ventana {
            inicio_us: ahora,
            fin_us: ahora,
            ..Default::default()
        };
    }

    fn duracion_ms(&self) -> i32 {
        redondear_ms(self.fin_us - self.inicio_us)
    }
}

/// Decodifica el giroscopio y el acelerómetro y lleva la calibración continua.
#[derive(Debug, Clone)]
pub struct Sensores {
    calibracion: Option<Calibracion>,
    ventanas: [Ventana; VENTANAS],
    frente: usize,
    /// Cuándo empezó la calibración continua, o `None` si ya terminó.
    calibrando_desde: Option<i64>,
    desvio: (i32, i32, i32),
    ultimo: Option<Movimiento>,
}

impl Sensores {
    /// `ahora_us` es cuando empieza a leer el mando: ahí arranca la calibración continua.
    pub fn new(calibracion: Option<Calibracion>, ahora_us: i64) -> Self {
        let mut s = Sensores {
            calibracion,
            ventanas: [Ventana::default(); VENTANAS],
            frente: 0,
            calibrando_desde: None,
            desvio: (0, 0, 0),
            ultimo: None,
        };
        s.reiniciar_calibracion_continua(ahora_us);
        s
    }

    pub fn calibracion(&self) -> Option<Calibracion> {
        self.calibracion
    }

    pub fn set_calibracion(&mut self, calibracion: Option<Calibracion>) {
        self.calibracion = calibracion;
    }

    /// Como `ResetContinuousCalibration`: vuelve a medir la deriva durante cinco segundos.
    /// El desvío medido antes se sigue restando hasta que acaba la nueva medida.
    pub fn reiniciar_calibracion_continua(&mut self, ahora_us: i64) {
        for v in &mut self.ventanas {
            v.reiniciar(ahora_us);
        }
        self.calibrando_desde = Some(ahora_us);
    }

    /// El desvío del giroscopio que se resta: yaw, pitch y roll.
    pub fn desvio(&self) -> (i32, i32, i32) {
        self.desvio
    }

    /// Procesa los 12 bytes del giroscopio y el acelerómetro. Devuelve el último movimiento
    /// conocido: si el acelerómetro viene a cero se queda el anterior, como en C#.
    pub fn procesar(&mut self, datos: &[u8], dt: f64, ahora_us: i64) -> Option<Movimiento> {
        let mut pitch = s16(datos, 0);
        let mut yaw = s16(datos, 2);
        let mut roll = s16(datos, 4);
        let mut ax = s16(datos, 6);
        let mut ay = s16(datos, 8);
        let mut az = s16(datos, 10);
        if let Some(c) = &self.calibracion {
            pitch = c.ejes[0].aplicar(pitch);
            yaw = c.ejes[1].aplicar(yaw);
            roll = c.ejes[2].aplicar(roll);
            ax = c.ejes[3].aplicar(ax);
            ay = c.ejes[4].aplicar(ay);
            az = c.ejes[5].aplicar(az);
        }
        if let Some(desde) = self.calibrando_desde {
            self.acumular(yaw, pitch, roll, ahora_us);
            // `Stopwatch.ElapsedMilliseconds > 5000`.
            if (ahora_us - desde).div_euclid(1000) > MS_CALIBRACION {
                self.calibrando_desde = None;
                if let Some(d) = self.promediar() {
                    self.desvio = d;
                }
            }
        }
        yaw = yaw.wrapping_sub(self.desvio.0);
        pitch = pitch.wrapping_sub(self.desvio.1);
        roll = roll.wrapping_sub(self.desvio.2);
        if ax != 0 || ay != 0 || az != 0 {
            self.ultimo = Some(Movimiento::nuevo(yaw, pitch, roll, ax, ay, az, dt));
        }
        self.ultimo
    }

    /// Como `PushSensorSamples`.
    fn acumular(&mut self, x: i32, y: i32, z: i32, ahora: i64) {
        let v = &mut self.ventanas[self.frente];
        if redondear_ms(ahora - v.inicio_us) >= MS_VENTANA {
            self.frente = (self.frente + VENTANAS - 1) % VENTANAS;
            self.ventanas[self.frente].reiniciar(ahora);
        } else {
            v.fin_us = ahora;
        }
        let v = &mut self.ventanas[self.frente];
        v.muestras += 1;
        v.x = v.x.wrapping_add(x);
        v.y = v.y.wrapping_add(y);
        v.z = v.z.wrapping_add(z);
    }

    /// Como `AverageGyro`: la media ponderada de las ventanas de los últimos cinco segundos.
    fn promediar(&self) -> Option<(i32, i32, i32)> {
        let mut peso = 0.0f64;
        let (mut tx, mut ty, mut tz) = (0.0f64, 0.0f64, 0.0f64);
        let mut quedan = MS_VENTANA;
        for i in 0..VENTANAS {
            if quedan <= 0 {
                break;
            }
            let v = &self.ventanas[(i + self.frente) % VENTANAS];
            let duracion = v.duracion_ms();
            if v.muestras == 0 || duracion == 0 {
                continue;
            }
            let n = v.muestras as f64;
            let este = if quedan < duracion {
                let p = (quedan as f32 / duracion as f32) as f64;
                quedan = 0;
                p
            } else {
                quedan -= duracion;
                (duracion as f64 / MS_VENTANA as f64).min(1.0)
            };
            tx += (v.x as f64 / n) * este;
            ty += (v.y as f64 / n) * este;
            tz += (v.z as f64 / n) * este;
            peso += este;
        }
        (peso > 0.0).then(|| ((tx / peso) as i32, (ty / peso) as i32, (tz / peso) as i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn calibracion_del_mando_de_las_grabaciones() {
        let c = Calibracion::desde_informe(&hex(
            "0500000700F9FF882277DDBE2252DD902257DD1C021C02062006E0D81FBADFE51FE4DF060005F43DD9",
        ))
        .unwrap();
        assert_eq!(c.ejes[1].sesgo, 7);
        assert_eq!(c.ejes[3].aplicar(-174), -180);
        assert_eq!(c.ejes[4].denominador, 16_414);
    }

    #[test]
    fn redondeo_como_net() {
        assert_eq!(redondear_ms(4_999_500), 5000);
        assert_eq!(redondear_ms(2_500), 2);
        assert_eq!(redondear_ms(3_500), 4);
    }
}
