//! El ratón que mueven los botones, los sticks, los gatillos y el giroscopio cuando se les asigna
//! una dirección del ratón (`getMouseMapping`, `GetAbsMouseMapping`, `GetMouseWheelMapping` y
//! `calculateFinalMouseMovement`), y la aceleración por velocidad del stick derecho
//! (`DeltaSettingsProcessor`).

use ds5_informes::Estado;
use ds5_perfiles::perfil::{AceleracionPalanca, ModoPalanca, Perfil};

use crate::bezier::byte_net;
use crate::botones::control::*;
use crate::botones::{Campos, Tipo, TIPOS};
use crate::raton::{EventoRaton, Sintetico, RUEDA_ABAJO, RUEDA_ARRIBA};

/// `MOUSESPEEDFACTOR`.
const FACTOR_VELOCIDAD: i32 = 48;

/// `ButtonAbsMouseInfo`: la zona de la pantalla del ratón absoluto.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegionAbsoluta {
    pub ancho: f64,
    pub alto: f64,
    pub centro_x: f64,
    pub centro_y: f64,
    pub antirradio: f64,
    pub volver_al_centro: bool,
}

/// Lo que hace falta del perfil para el ratón desde los controles (`ButtonMouseInfo`).
#[derive(Debug, Clone, PartialEq)]
pub struct AjustesRatonBotones {
    pub sensibilidad: i32,
    pub aceleracion: bool,
    pub desfase: f64,
    pub escala_vertical: f64,
    /// Sin zona muerta en el stick, el ratón deja una de 3 (`getLSDeadzone(device) == 0`).
    pub sin_zona_ls: bool,
    pub sin_zona_rs: bool,
    pub region: RegionAbsoluta,
    /// La aceleración por velocidad del stick derecho, si va como controles. La del izquierdo no
    /// se calcula nunca en C#.
    pub delta_rs: Option<AceleracionPalanca>,
}

impl AjustesRatonBotones {
    pub fn desde_perfil(p: &Perfil) -> AjustesRatonBotones {
        let a = &p.ajustes;
        let r = &a.region_raton_absoluto;
        AjustesRatonBotones {
            sensibilidad: a.sensibilidad_raton_botones,
            aceleracion: a.aceleracion_raton,
            desfase: a.desfase_raton_botones,
            // El perfil guarda el porcentaje; C# usa la proporción.
            escala_vertical: a.escala_vertical_raton_botones * 0.01,
            sin_zona_ls: a.ls_zona_muerta == 0,
            sin_zona_rs: a.rs_zona_muerta == 0,
            region: RegionAbsoluta {
                ancho: r.ancho,
                alto: r.alto,
                centro_x: r.centro_x,
                centro_y: r.centro_y,
                antirradio: r.antirradio,
                volver_al_centro: r.volver_al_centro,
            },
            delta_rs: (a.rs_modo == ModoPalanca::Controls && a.rs_aceleracion.activa)
                .then(|| a.rs_aceleracion.clone()),
        }
    }
}

/// Los ángulos de los sticks (`calculateStickAngles`): en grados de 0 a 360 y lo que pesa cada
/// eje en la dirección.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Angulos {
    pub ls: f64,
    pub rs: f64,
    pub lx: f64,
    pub ly: f64,
    pub rx: f64,
    pub ry: f64,
}

impl Angulos {
    pub fn de(e: &Estado) -> Angulos {
        let calcular = |x: u8, y: u8| {
            let rad = (-(y as i32 - 128) as f64).atan2((x as i32 - 128) as f64);
            let grados = (if rad >= 0.0 {
                rad
            } else {
                2.0 * std::f64::consts::PI + rad
            }) * 180.0
                / std::f64::consts::PI;
            (grados, rad.cos().abs(), rad.sin().abs())
        };
        let (ls, lx, ly) = calcular(e.lx, e.ly);
        let (rs, rx, ry) = calcular(e.rx, e.ry);
        Angulos {
            ls,
            rs,
            lx,
            ly,
            rx,
            ry,
        }
    }
}

/// `AbsMouseOutput`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SalidaAbsoluta {
    pub x: f64,
    pub y: f64,
    pub sucio_x: bool,
    pub sucio_y: bool,
    pub sucio_antes: bool,
    pub angulo: f64,
}

impl Default for SalidaAbsoluta {
    fn default() -> Self {
        SalidaAbsoluta {
            x: 0.5,
            y: 0.5,
            sucio_x: false,
            sucio_y: false,
            sucio_antes: false,
            angulo: 0.0,
        }
    }
}

/// `DeltaSettingsProcessor`: acelera el ratón cuando el stick se mueve deprisa. El cronómetro de
/// C# va con el reloj; aquí va con la hora del informe.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Delta {
    pub usar: bool,
    pub x: f64,
    pub y: f64,
    anterior_x: f64,
    anterior_y: f64,
    anterior_radial: f64,
    multiplicador: f64,
    multiplicador_suave: f64,
    recorrido: f64,
    recorrido_total: f64,
    /// Cuándo se puso en marcha el cronómetro; `None` si está parado.
    reloj_us: Option<i64>,
}

impl Delta {
    /// `Process`, con los ejes ya restado el centro y el máximo de cada uno.
    #[allow(clippy::too_many_arguments)]
    pub fn procesar(
        &mut self,
        dir_x: f64,
        dir_y: f64,
        crudo_x: f64,
        crudo_y: f64,
        max_x: f64,
        max_y: f64,
        a: &AceleracionPalanca,
        ahora_us: i64,
    ) {
        self.usar = a.activa;
        let multi = a.multiplicador;
        let max_rec = a.recorrido_maximo;
        let min_rec = a.recorrido_minimo;
        let duracion = a.duracion_suavizado;
        let factor = a.factor_minimo.max(1.0);
        let min_parada = 0.1f64.max(min_rec);
        let pendiente = (multi - factor) / (max_rec - min_rec);
        let desfase = factor - (pendiente * min_rec);

        let mut x = dir_x / max_x;
        let mut y = dir_y / max_y;
        let cx = crudo_x / max_x;
        let cy = crudo_y / max_y;
        let hyp = ((cx * cx) + (cy * cy)).sqrt();

        let reiniciar = |d: &mut Delta| {
            d.anterior_radial = hyp;
            d.multiplicador = 0.0;
            d.recorrido = 0.0;
            d.multiplicador_suave = 0.0;
            d.recorrido_total = 0.0;
            d.reloj_us = None;
            d.anterior_x = cx;
            d.anterior_y = cy;
        };

        if self.usar {
            if hyp > 0.0
                && (hyp - self.anterior_radial).abs() >= min_rec
                && (hyp - self.anterior_radial >= 0.0)
            {
                let tramo = (hyp - self.anterior_radial).abs();
                if self.recorrido_total == 0.0 {
                    self.recorrido_total = tramo;
                    self.multiplicador_suave = pendiente * tramo + desfase;
                } else {
                    self.recorrido_total += tramo;
                    self.multiplicador_suave = pendiente * self.recorrido_total + desfase;
                }
                self.multiplicador = pendiente * tramo + desfase;
                x *= self.multiplicador;
                y *= self.multiplicador;
                self.recorrido = tramo;
                self.reloj_us = Some(ahora_us);
                self.anterior_radial = hyp;
                self.anterior_x = cx;
                self.anterior_y = cy;
            } else if hyp > 0.0
                && self.multiplicador > 0.0
                && (self.anterior_radial - hyp).abs() < min_parada
                && !((self.anterior_x >= 0.0) != (cx >= 0.0)
                    && (self.anterior_y >= 0.0) != (cy >= 0.0))
            {
                // `ElapsedMilliseconds`: milisegundos enteros.
                let pasado = self.reloj_us.map_or(0, |r| (ahora_us - r) / 1000) as f64;
                let mut acel = self.multiplicador;
                let mut recorrido = self.recorrido;
                if hyp - self.anterior_radial <= 0.0 {
                    let mezcla = (hyp - self.anterior_radial).abs().min(min_parada);
                    let pend = (min_rec - recorrido) / min_parada;
                    recorrido += pend * mezcla;
                    acel = pendiente * recorrido + desfase;
                }
                let duracion = duracion * (self.multiplicador_suave / multi);
                if duracion > 0.0 && (pasado * 0.001) < duracion {
                    let t = (pasado * 0.001) / duracion;
                    let t = (1.0 - acel) * (t * t * t) + acel;
                    x *= t;
                    y *= t;
                } else {
                    reiniciar(self);
                }
            } else {
                reiniciar(self);
            }
        } else {
            reiniciar(self);
        }
        self.x = x;
        self.y = y;
    }
}

/// Lo que recuerda el ratón desde los controles entre informes.
#[derive(Debug, Clone, PartialEq)]
pub struct RatonBotones {
    /// `activeButtonSensitivity` y `tempButtonSensitivity`: los extras pueden cambiarla mientras
    /// se mantiene un control.
    pub sensibilidad: i32,
    pub sensibilidad_temporal: i32,
    resto_h: f64,
    resto_v: f64,
    /// `mcounter`, `mouseaccel` y `prevmouseaccel`.
    contador: i32,
    aceleracion: i32,
    aceleracion_antes: i32,
    pub(crate) absoluto: SalidaAbsoluta,
    pub(crate) delta_rs: Delta,
    /// `stickWheelRemainder` y `stickWheelDownDir`.
    rueda_resto: f64,
    pub(crate) rueda_abajo: bool,
}

impl Default for RatonBotones {
    /// Con la sensibilidad por defecto; la reasignación pone la del perfil.
    fn default() -> Self {
        RatonBotones {
            sensibilidad: 25,
            sensibilidad_temporal: -1,
            resto_h: 0.0,
            resto_v: 0.0,
            contador: 34,
            aceleracion: 0,
            aceleracion_antes: 0,
            absoluto: SalidaAbsoluta::default(),
            delta_rs: Delta::default(),
            rueda_resto: 0.0,
            rueda_abajo: false,
        }
    }
}

impl RatonBotones {
    pub fn nuevo(a: &AjustesRatonBotones) -> RatonBotones {
        RatonBotones {
            sensibilidad: a.sensibilidad,
            sensibilidad_temporal: -1,
            resto_h: 0.0,
            resto_v: 0.0,
            contador: 34,
            aceleracion: 0,
            aceleracion_antes: 0,
            absoluto: SalidaAbsoluta::default(),
            delta_rs: Delta::default(),
            rueda_resto: 0.0,
            rueda_abajo: false,
        }
    }

    /// `getMouseMapping`: cuánto mueve el control el ratón. `sentido` es 0 arriba, 1 abajo, 2 a
    /// la izquierda y 3 a la derecha; `transcurrido_ms`, el tiempo desde el informe anterior.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn mover(
        &mut self,
        a: &AjustesRatonBotones,
        c: usize,
        f: &Campos,
        e: &Estado,
        ang: &Angulos,
        sentido: u8,
        transcurrido_ms: f64,
    ) -> f64 {
        let zl = if a.sin_zona_ls { 3 } else { 0 };
        let zr = if a.sin_zona_rs { 3 } else { 0 };
        let velocidad = self.sensibilidad;
        const RAIZ: f64 = 1.002;
        const DIVIDE: f64 = 10000.0;
        let base = RAIZ + velocidad as f64 / DIVIDE;
        let vertical = sentido <= 1;
        let escala = if vertical { a.escala_vertical } else { 1.0 };
        let mut v = 0.0;
        match TIPOS[c] {
            Tipo::Boton => {
                v = if f.botones[c] {
                    base.powf(100.0) - 1.0
                } else {
                    0.0
                };
                if vertical {
                    v *= escala;
                }
            }
            Tipo::Eje => {
                let td = transcurrido_ms * 0.001;
                let mut vel = velocidad * FACTOR_VELOCIDAD;
                let desfase = a.desfase * vel as f64;
                if vertical {
                    vel = (vel as f64 * escala) as i32;
                }
                let vel = vel as f64;
                let delta = self.delta_rs;
                // El eje, su zona, su peso en la dirección y si usa la aceleración del stick.
                let (valor, zona, unidad, delta_xy) = match c {
                    LX_NEG | LX_POS => (e.lx, zl, ang.lx, None),
                    LY_NEG | LY_POS => (e.ly, zl, ang.ly, None),
                    RX_NEG | RX_POS => (e.rx, zr, ang.rx, Some(delta.x)),
                    _ => (e.ry, zr, ang.ry, Some(delta.y)),
                };
                let delta_xy = delta_xy.filter(|_| delta.usar);
                let valor = valor as i32;
                let off = unidad * desfase;
                if matches!(c, LX_NEG | LY_NEG | RX_NEG | RY_NEG) {
                    if valor < 128 - zona {
                        let diff = match delta_xy {
                            Some(d) => -d,
                            None => (-(valor - 128 - zona)) as f64 / (0 - 128 - zona) as f64,
                        };
                        v = (vel - off) * td * diff + (-off * td);
                    }
                } else if valor > 128 + zona {
                    let diff = match delta_xy {
                        Some(d) => d,
                        None => (valor - 128 + zona) as f64 / (255 - 128 + zona) as f64,
                    };
                    v = (vel - off) * td * diff + (off * td);
                }
            }
            Tipo::Gatillo => {
                v = base.powf(f.gatillos[c] as f64 / 2.0) - 1.0;
                if vertical {
                    v *= escala;
                }
            }
            Tipo::Giro => {
                let g = f.giro[c];
                let g = match c {
                    GIRO_X_POS | GIRO_Z_POS => g,
                    _ => -g,
                };
                v = byte_net(if g > 0 { base.powf(g as f64) } else { 0.0 }) as f64;
                if vertical {
                    v *= escala;
                }
            }
            _ => {}
        }

        if a.aceleracion {
            if v > 0.0 {
                self.contador = 34;
                self.aceleracion += 1;
            }
            if self.aceleracion == self.aceleracion_antes {
                self.contador -= 1;
            }
            if self.contador <= 0 {
                self.aceleracion = 0;
                self.contador = 34;
            }
            v *= 1.0 + 20000.min(self.aceleracion) as f64 / 10000.0;
            self.aceleracion_antes = self.aceleracion;
        }
        v
    }

    /// `GetAbsMouseMapping`: dónde pone el control el ratón absoluto y si lo mueve. `salida` es
    /// la dirección asignada: 0 arriba, 1 abajo, 2 a la izquierda y 3 a la derecha.
    pub(crate) fn absoluto(
        a: &AjustesRatonBotones,
        c: usize,
        f: &Campos,
        e: &Estado,
        ang: &Angulos,
        salida: u8,
    ) -> (f64, bool) {
        let r = &a.region;
        let positivo = salida == 1 || salida == 3;
        let vertical = salida <= 1;
        let con_anti = r.antirradio != 0.0;
        let (mitad, centro) = if vertical {
            (r.alto / 2.0, r.centro_y)
        } else {
            (r.ancho / 2.0, r.centro_x)
        };
        let maximo = if positivo {
            centro + mitad
        } else {
            centro - mitad
        };
        let medio = centro;
        let mut resultado = medio;
        let mut movido = false;
        match TIPOS[c] {
            Tipo::Eje => {
                let (valor, unidad) = match c {
                    LX_NEG | LX_POS => (e.lx, ang.lx),
                    LY_NEG | LY_POS => (e.ly, ang.ly),
                    RX_NEG | RX_POS => (e.rx, ang.rx),
                    _ => (e.ry, ang.ry),
                };
                let negativo = matches!(c, LX_NEG | LY_NEG | RX_NEG | RY_NEG);
                let diff = if negativo && valor < 128 {
                    Some(-(valor as f64 - 128.0) / 127.0)
                } else if !negativo && valor > 128 {
                    Some((valor as f64 - 128.0) / 128.0)
                } else {
                    None
                };
                if let Some(mut diff) = diff {
                    if con_anti {
                        let anti = r.antirradio * unidad;
                        diff = (1.0 - anti) * diff + anti;
                    }
                    resultado = (maximo - medio) * diff + medio;
                    movido = true;
                }
            }
            Tipo::Boton => {
                let activo = f.botones[c];
                let bajo = if con_anti {
                    (maximo - medio) * r.antirradio + medio
                } else {
                    0.0
                };
                resultado = if activo { maximo } else { bajo };
                movido = activo;
            }
            Tipo::Gatillo => {
                let t = f.gatillos[c];
                let mut diff = t as f64 / 255.0;
                if con_anti {
                    let anti = r.antirradio;
                    diff = (1.0 - anti) * diff + anti;
                }
                resultado = mitad * (if positivo { diff } else { -diff }) + centro;
                movido = t > 0;
            }
            Tipo::Giro => {
                let g = f.giro[c];
                let g = match c {
                    GIRO_X_POS | GIRO_Z_POS => g,
                    _ => -g,
                };
                let mut diff = g as f64 / 128.0;
                if con_anti {
                    let anti = r.antirradio;
                    diff = (1.0 - anti) * diff + anti;
                }
                resultado = (maximo - medio) * diff + medio;
                movido = g > 0;
            }
            _ => {}
        }
        (resultado, movido)
    }

    /// `GetMouseWheelMapping`: un eje, un gatillo o el giroscopio hacen girar la rueda según lo
    /// que se aprieten, como mucho cada 10 ms. Tres pasadas a tope son un paso de rueda.
    pub(crate) fn rueda(
        &mut self,
        valor: u8,
        abajo: bool,
        s: &mut Sintetico,
        eventos: &mut Vec<EventoRaton>,
    ) {
        if s.ahora_us >= s.antes_us + 10_000 && !s.repetir {
            s.antes_us = s.ahora_us;
            let sentido = if abajo { RUEDA_ABAJO } else { RUEDA_ARRIBA };
            let proporcion = (1.0 - 0.05) * (valor as f64 / 255.0) + 0.05;
            let rueda = proporcion / 3.0 + self.rueda_resto;
            if rueda >= 1.0 {
                eventos.push(EventoRaton::Rueda(rueda as i32 * sentido, 0));
                self.rueda_resto = rueda - (rueda as i32) as f64;
            } else {
                self.rueda_resto = rueda;
            }
        }
    }

    /// Cambia el sentido de la rueda con un stick: lo que sobraba se tira.
    pub(crate) fn sentido_rueda(&mut self, abajo: bool) {
        if self.rueda_abajo != abajo {
            self.rueda_resto = 0.0;
            self.rueda_abajo = abajo;
        }
    }

    /// `calculateFinalMouseMovement`: los píxeles enteros que se mueven y lo que sobra, que se
    /// suma al siguiente si va en el mismo sentido.
    pub(crate) fn final_movimiento(&mut self, mut x: f64, mut y: f64) -> (i32, i32) {
        fn recorte(d: f64) -> f64 {
            d - (d as i32) as f64
        }
        if (x > 0.0 && self.resto_h > 0.0) || (x < 0.0 && self.resto_h < 0.0) {
            x += self.resto_h;
        } else {
            self.resto_h = 0.0;
        }
        let xt = x - (recorte(x * 100.0) / 100.0);
        let mx = xt as i32;
        self.resto_h = xt - mx as f64;

        if (y > 0.0 && self.resto_v > 0.0) || (y < 0.0 && self.resto_v < 0.0) {
            y += self.resto_v;
        } else {
            self.resto_v = 0.0;
        }
        let yt = y - (recorte(y * 100.0) / 100.0);
        let my = yt as i32;
        self.resto_v = yt - my as f64;
        (mx, my)
    }

    /// El final de `MapCustom` para el ratón absoluto: si algún control lo ha movido, va a su
    /// sitio; si lo ha dejado de mover, vuelve al centro de la zona.
    pub(crate) fn final_absoluto(
        &mut self,
        a: &AjustesRatonBotones,
        eventos: &mut Vec<EventoRaton>,
    ) {
        let s = &mut self.absoluto;
        let sucio = s.sucio_x || s.sucio_y;
        if sucio || s.sucio_antes {
            if sucio {
                eventos.push(EventoRaton::Absoluto(
                    s.x.clamp(0.0, 1.0),
                    s.y.clamp(0.0, 1.0),
                ));
            } else if a.region.volver_al_centro {
                let r = &a.region;
                let xd = s.angulo.cos() * r.antirradio;
                let yd = -s.angulo.sin() * r.antirradio;
                let x = ((r.ancho / 2.0) * xd + r.centro_x).clamp(0.0, 1.0);
                let y = ((r.alto / 2.0) * yd + r.centro_y).clamp(0.0, 1.0);
                eventos.push(EventoRaton::Absoluto(x, y));
            }
            s.sucio_antes = sucio;
            s.sucio_x = false;
            s.sucio_y = false;
            s.angulo = (-(s.y - 0.5)).atan2(s.x - 0.5);
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// Lo que no llega a un píxel se guarda y se suma al siguiente movimiento en el mismo
    /// sentido; si cambia de sentido, se tira.
    #[test]
    fn el_resto_se_suma_si_sigue_el_sentido() {
        let a = AjustesRatonBotones::desde_perfil(&Perfil::default());
        let mut r = RatonBotones::nuevo(&a);
        assert_eq!(r.final_movimiento(0.6, 0.0), (0, 0));
        assert_eq!(r.final_movimiento(0.6, 0.0), (1, 0));
        assert_eq!(r.final_movimiento(-0.6, 0.0), (0, 0));
    }

    /// La rueda con un gatillo a fondo: un paso cada tres pasadas, y como mucho una pasada cada
    /// 10 ms.
    #[test]
    fn la_rueda_con_un_gatillo_va_cada_tres_pasadas() {
        let a = AjustesRatonBotones::desde_perfil(&Perfil::default());
        let mut r = RatonBotones::nuevo(&a);
        let mut s = Sintetico::default();
        let mut e = Vec::new();
        for ms in (0..60).step_by(5) {
            s.ahora_us = ms * 1000;
            r.rueda(255, false, &mut s, &mut e);
        }
        // Pasadas a los 0, 10, 20, 30, 40 y 50 ms: dos pasos de rueda.
        assert_eq!(e, vec![EventoRaton::Rueda(120, 0); 2]);
    }

    /// La aceleración por velocidad: un empujón rápido multiplica la salida y, si el stick se
    /// queda quieto, vuelve poco a poco a 1 en el tiempo de suavizado.
    #[test]
    fn la_aceleracion_del_stick_se_suaviza_con_el_tiempo() {
        let a = AceleracionPalanca {
            activa: true,
            ..Default::default()
        };
        let mut d = Delta::default();
        d.procesar(64.0, 0.0, 64.0, 0.0, 127.0, 127.0, &a, 0);
        let empujon = d.x / (64.0 / 127.0);
        assert!(empujon > 1.5, "{empujon}");
        d.procesar(64.0, 0.0, 64.0, 0.0, 127.0, 127.0, &a, 10_000);
        let luego = d.x / (64.0 / 127.0);
        assert!(luego < empujon && luego > 1.0, "{luego}");
        d.procesar(64.0, 0.0, 64.0, 0.0, 127.0, 127.0, &a, 2_000_000);
        assert_eq!(d.x, 64.0 / 127.0);
    }
}
