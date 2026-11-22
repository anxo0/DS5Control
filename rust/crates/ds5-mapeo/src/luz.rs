//! La luz del mando (`DS5LightBar.updateLightBar`): el color del perfil, por batería, el
//! arcoíris, el aviso de batería baja, la carga, el modo distancia y la luz forzada (extras,
//! macros, batería y calibración del volante). Lo que en C# va con el reloj (`DateTime.UtcNow` y
//! los `Stopwatch`) usa aquí la hora que se le pasa.

use crate::acciones::transicion;
use crate::botones::LuzForzada;
use ds5_perfiles::perfil::{Ajustes as AjustesPerfil, ModoLuz};

/// `BatteryIndicatorDurations`: encendido y apagado según la batería, de 10 en 10.
const DURACIONES_BATERIA: [(u8, u8); 11] = [
    (28, 252),
    (28, 252),
    (56, 224),
    (84, 196),
    (112, 168),
    (140, 140),
    (168, 112),
    (196, 84),
    (224, 56),
    (252, 28),
    (0, 0),
];

const DESTELLO_MS: i64 = 2000;
const DESTELLO_TRAMOS: f64 = (DESTELLO_MS / 40) as f64;
const CARGA_MS: i64 = 4000;
const CARGA_TRAMOS: f64 = ((CARGA_MS / 40) - 2) as f64;

type Color = (u8, u8, u8);
const NEGRO: Color = (0, 0, 0);

/// Los ajustes de la luz en el perfil.
#[derive(Debug, Clone, PartialEq)]
pub struct AjustesLuz {
    pub modo: ModoLuz,
    pub color: Color,
    pub color_bajo: Color,
    pub color_carga: Color,
    pub color_destello: Color,
    /// `ledAsBatteryIndicator`.
    pub por_bateria: bool,
    /// `FlashType`: 0 parpadea, 1 se funde.
    pub tipo_destello: u8,
    /// `flashBatteryAt`: avisa con la batería en este porcentaje o menos.
    pub destello_en: i32,
    /// `Rainbow`: segundos por vuelta; 0 sin arcoíris.
    pub arcoiris: f64,
    /// `MaxSatRainbow`, en fracción.
    pub saturacion: f64,
    /// `ChargingType`: 0 nada, 1 se funde, 2 arcoíris, 3 color de carga.
    pub tipo_carga: i32,
    /// `idleDisconnectTimeout`, en segundos.
    pub desconexion_s: i32,
}

impl AjustesLuz {
    pub fn desde_perfil(p: &AjustesPerfil) -> AjustesLuz {
        let c = |r: &ds5_perfiles::Rgb| (r.0, r.1, r.2);
        AjustesLuz {
            modo: p.modo_luz,
            color: c(&p.color),
            color_bajo: c(&p.color_bajo),
            color_carga: c(&p.color_carga),
            color_destello: c(&p.color_destello),
            por_bateria: p.luz_bateria,
            tipo_destello: p.tipo_destello,
            destello_en: p.destello_bateria_en,
            arcoiris: p.arcoiris,
            saturacion: p.saturacion_arcoiris / 100.0,
            tipo_carga: p.tipo_carga,
            desconexion_s: p.desconexion_inactivo,
        }
    }
}

/// Lo que la luz lee del mando y de la app en cada informe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntradaLuz {
    pub bateria: i32,
    pub cargando: bool,
    pub bluetooth: bool,
    /// El color propio que se ha puesto desde la app (`useCustomLed`), que no se guarda.
    pub personal: Option<Color>,
    /// `defaultLight`: la app pide la luz por defecto.
    pub por_defecto: bool,
    /// `shuttingdown`: la app se cierra y apaga la luz.
    pub apagando: bool,
    /// El perfil es de distancia (su nombre lleva `Distance`): la luz va con la vibración.
    pub distancia: bool,
    /// La vibración fuerte que tiene el mando.
    pub vibracion_fuerte: u8,
    /// `lastActive`: cuándo se usó el mando por última vez.
    pub ultimo_uso_us: i64,
    pub ahora_us: i64,
}

/// `DS5LightbarState`: lo que se manda al mando.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EstadoLuz {
    pub color: Color,
    pub encendido: u8,
    pub apagado: u8,
    /// `LightBarExplicitlyOff`. Ojo: a pesar del nombre de C#, va a `true` siempre que hay un
    /// color puesto; no significa que la luz esté apagada.
    pub apagada: bool,
}

/// La luz de un mando, con lo que C# guarda entre informes.
#[derive(Debug, Clone)]
pub struct Luz {
    /// `counters`: el tono del arcoíris.
    contador: f64,
    /// El `Stopwatch` del fundido: cuándo empezó, si está en marcha.
    reloj: Option<i64>,
    /// `fadedirection`.
    direccion: bool,
    /// `oldnow`.
    antes_us: i64,
    /// Lo que tiene puesto el mando (`currentHap.lightbarState`).
    pub estado: EstadoLuz,
}

/// `HuetoRGB`.
pub fn tono_a_color(tono: f32, sat: u8) -> Color {
    let c = sat;
    let x = (c as f32 * (1.0 - ((tono / 60.0) % 2.0 - 1.0).abs())) as i32 as u8;
    if (0.0..60.0).contains(&tono) {
        (c, x, 0)
    } else if (60.0..120.0).contains(&tono) {
        (x, c, 0)
    } else if (120.0..180.0).contains(&tono) {
        (0, c, x)
    } else if (180.0..240.0).contains(&tono) {
        (0, x, c)
    } else if (240.0..300.0).contains(&tono) {
        (x, 0, c)
    } else if (300.0..360.0).contains(&tono) {
        (c, 0, x)
    } else {
        (255, 0, 0)
    }
}

impl Luz {
    pub fn nueva(ahora_us: i64) -> Luz {
        Luz {
            contador: 0.0,
            reloj: None,
            direccion: false,
            antes_us: ahora_us,
            estado: EstadoLuz::default(),
        }
    }

    /// El fundido de ida y vuelta con el `Stopwatch` de C#: devuelve el porcentaje de negro.
    fn fundido(&mut self, ahora_us: i64, duracion: i64, tramos: f64, recortar: bool) -> f64 {
        let Some(inicio) = self.reloj else {
            let antes = self.direccion;
            self.direccion = !antes;
            self.reloj = Some(ahora_us);
            return if antes { 100.0 } else { 0.0 };
        };
        let pasado = (ahora_us - inicio) / 1000;
        if pasado >= duracion {
            self.reloj = None;
            return if self.direccion { 100.0 } else { 0.0 };
        }
        let mut tramo = pasado / 40;
        if recortar && tramo as f64 > tramos {
            tramo = tramos as i64;
        }
        let v = tramo as f64 / tramos;
        if self.direccion {
            100.0 * v
        } else {
            -100.0 * v + 100.0
        }
    }

    /// `updateLightBar`. Devuelve si ha puesto la luz; sin modo de luz no la toca.
    pub fn actualizar(&mut self, a: &AjustesLuz, en: &EntradaLuz, forzada: &LuzForzada) -> bool {
        let mut color = NEGRO;
        let forzar = forzada.activa;
        let rutina = a.modo == ModoLuz::DS5Win;
        let bateria = en.bateria;
        let mut usar = rutina;
        if !en.por_defecto && !forzar && rutina {
            if let Some(p) = en.personal {
                color = p;
            } else if a.arcoiris > 0.0 {
                if en.ahora_us >= self.antes_us + 10_000 {
                    // `TimeSpan.Milliseconds`: solo la parte de milisegundos, sin los segundos.
                    let ms = ((en.ahora_us - self.antes_us) / 1000 % 1000) as f64;
                    self.antes_us = en.ahora_us;
                    let paso = 360.0 * (ms / 1000.0 / a.arcoiris);
                    if en.cargando {
                        self.contador -= paso;
                    } else {
                        self.contador += paso;
                    }
                }
                if self.contador < 0.0 {
                    self.contador = 180000.0;
                } else if self.contador > 180000.0 {
                    self.contador = 0.0;
                }
                let tono = self.contador as f32 % 360.0;
                let sat = if a.por_bateria {
                    let s = if a.saturacion == 1.0 {
                        bateria as f64 * 2.55
                    } else {
                        bateria as f64 * 2.55 * a.saturacion
                    };
                    s as i32 as u8
                } else if a.saturacion == 1.0 {
                    255
                } else {
                    (255.0 * a.saturacion) as i32 as u8
                };
                color = tono_a_color(tono, sat);
            } else if a.por_bateria {
                color = transicion(a.color_bajo, a.color, bateria as f64);
            } else {
                color = a.color;
            }

            if bateria <= a.destello_en && !en.cargando {
                if a.color_destello != NEGRO {
                    color = a.color_destello;
                }
                if a.tipo_destello == 1 {
                    let r = self.fundido(en.ahora_us, DESTELLO_MS, DESTELLO_TRAMOS, false);
                    color = transicion(color, NEGRO, r);
                }
            }

            if a.desconexion_s > 0 && a.por_bateria && (!en.cargando || bateria >= 100) {
                let pasado_ms = (en.ahora_us - en.ultimo_uso_us) as f64 / 1000.0;
                let total_ms = a.desconexion_s as f64 * 1000.0;
                let r = 100.0 * (pasado_ms / total_ms);
                if (50.0..100.0).contains(&r) {
                    let e = 0.02 * (r - 50.0);
                    color = transicion(color, NEGRO, (-100.0 * e * (e - 2.0)) as u32 as f64);
                } else if r >= 100.0 {
                    color = transicion(color, NEGRO, 100.0);
                }
            }

            if en.cargando && bateria < 100 {
                match a.tipo_carga {
                    1 => {
                        let r = self.fundido(en.ahora_us, CARGA_MS, CARGA_TRAMOS, true);
                        color = transicion(color, NEGRO, r);
                    }
                    2 => {
                        self.contador += 0.167;
                        color = tono_a_color(self.contador as f32 % 360.0, 255);
                    }
                    3 => color = a.color_carga,
                    _ => {}
                }
            }
        } else if forzar {
            color = forzada.color;
            usar = true;
        } else if en.apagando {
            color = NEGRO;
            usar = true;
        } else if rutina {
            color = if en.bluetooth { (32, 64, 64) } else { NEGRO };
        }

        if !usar {
            return false;
        }
        let vib = en.vibracion_fuerte;
        if en.distancia && !en.por_defecto {
            let fuerza = vib as f32 / 2.55;
            let max = color.0.max(color.1).max(color.2);
            let amarillo = (max, max, 0);
            if vib > 100 {
                color = transicion(amarillo, (255, 0, 0), fuerza as f64);
            } else {
                let medio = transicion(amarillo, (255, 0, 0), 39.6078f32 as f64);
                color = transicion(color, medio, vib as f64);
            }
        }

        let mut nuevo = EstadoLuz {
            color,
            ..EstadoLuz::default()
        };
        if nuevo.color != NEGRO {
            if forzar && forzada.destello > 0 {
                let d = 25u8.wrapping_sub(forzada.destello);
                nuevo.encendido = d;
                nuevo.apagado = d;
                nuevo.apagada = true;
            } else if bateria <= a.destello_en
                && a.tipo_destello == 0
                && !en.por_defecto
                && !en.cargando
            {
                let nivel = (bateria / 10).clamp(i32::MIN, 10);
                // Una batería negativa no se da en el mando; C# fallaría con el índice.
                let (on, off) = DURACIONES_BATERIA[nivel.max(0) as usize];
                nuevo.encendido = on;
                nuevo.apagado = off;
            } else if en.distancia && vib > 155 {
                let d = (265 - vib as i32) as u8;
                nuevo.encendido = d;
                nuevo.apagado = d;
                nuevo.apagada = true;
            } else {
                nuevo.apagada = true;
            }
        } else {
            nuevo.apagada = true;
        }

        let antes = self.estado.encendido;
        if antes != nuevo.encendido && antes != 1 && nuevo.encendido == 0 {
            nuevo.encendido = 1;
            nuevo.apagado = 1;
        }
        self.estado = nuevo;
        true
    }
}

/// La macro de luz de un control (`LightbarMacro`): una lista de colores con cuánto dura cada
/// uno, que suena al pulsar o al soltar el control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroLuz {
    /// Los colores y cuántos milisegundos dura cada uno.
    pub pasos: Vec<(Color, u32)>,
    /// `Trigger`: al soltar en vez de al pulsar.
    pub al_soltar: bool,
    /// `CancelCurrent`: corta la que esté sonando; si no, no suena hasta que acabe.
    pub cortar: bool,
}

impl MacroLuz {
    /// `GetMacroFromString`. Solo devuelve las activas; una que no se puede leer no suena.
    pub fn leer(texto: &str) -> Option<MacroLuz> {
        let partes: Vec<&str> = texto.split('/').collect();
        if !(3..=4).contains(&partes.len()) {
            return None;
        }
        let logico = |s: &str| match s.trim().to_ascii_lowercase().as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };
        if !logico(partes[0])? {
            return None;
        }
        let mut pasos = Vec::new();
        for elemento in partes[1].split(';') {
            if elemento.is_empty() {
                return None;
            }
            let mut e = elemento.split(':');
            let rgb: Vec<u8> = e
                .next()?
                .split(',')
                .map(|v| v.trim().parse().ok())
                .collect::<Option<_>>()?;
            if rgb.len() != 3 {
                return None;
            }
            let largo: u32 = e.next()?.trim().parse().ok()?;
            pasos.push(((rgb[0], rgb[1], rgb[2]), largo));
        }
        // `Enum.TryParse`: lo que no es `Release` (sin distinguir mayúsculas) es al pulsar.
        let al_soltar = partes[2].trim().eq_ignore_ascii_case("release") || partes[2].trim() == "1";
        let cortar = if partes.len() == 3 {
            true
        } else {
            logico(partes[3])?
        };
        Some(MacroLuz {
            pasos,
            al_soltar,
            cortar,
        })
    }
}

/// La macro de luz que está sonando (`RunLightbarMacro`): solo una a la vez para todos los
/// controles. En C# corre en su hilo cada 10 ms; aquí avanza con cada informe.
#[derive(Debug, Clone, Default)]
pub struct AnimacionLuz {
    pasos: Vec<(Color, u32)>,
    paso: usize,
    marca_us: i64,
    pub en_marcha: bool,
}

impl AnimacionLuz {
    /// Empieza una macro, cortando la que hubiera.
    pub fn empezar(&mut self, m: &MacroLuz, ahora_us: i64, luz: &mut LuzForzada) {
        self.pasos = m.pasos.clone();
        self.paso = 0;
        self.marca_us = ahora_us;
        self.en_marcha = true;
        luz.activa = true;
        self.poner(luz);
    }

    fn poner(&mut self, luz: &mut LuzForzada) {
        match self.pasos.get(self.paso) {
            Some((color, _)) => luz.color = *color,
            None => {
                self.en_marcha = false;
                luz.activa = false;
            }
        }
    }

    /// En cada informe: pasa al siguiente color cuando acaba el que suena.
    pub fn avanzar(&mut self, ahora_us: i64, luz: &mut LuzForzada) {
        if !self.en_marcha {
            return;
        }
        if let Some((_, largo)) = self.pasos.get(self.paso) {
            if ahora_us - self.marca_us >= *largo as i64 * 1000 {
                self.paso += 1;
                self.marca_us = ahora_us;
            }
        }
        self.poner(luz);
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn ajustes() -> AjustesLuz {
        AjustesLuz {
            modo: ModoLuz::DS5Win,
            color: (0, 0, 255),
            color_bajo: (255, 0, 0),
            color_carga: (0, 255, 0),
            color_destello: NEGRO,
            por_bateria: false,
            tipo_destello: 0,
            destello_en: 0,
            arcoiris: 0.0,
            saturacion: 1.0,
            tipo_carga: 0,
            desconexion_s: 0,
        }
    }

    fn entrada(ms: i64) -> EntradaLuz {
        EntradaLuz {
            bateria: 80,
            cargando: false,
            bluetooth: true,
            personal: None,
            por_defecto: false,
            apagando: false,
            distancia: false,
            vibracion_fuerte: 0,
            ultimo_uso_us: 0,
            ahora_us: ms * 1000,
        }
    }

    /// El arcoíris da una vuelta en los segundos que diga el perfil.
    #[test]
    fn el_arcoiris_avanza_con_el_tiempo() {
        let mut a = ajustes();
        a.arcoiris = 2.0;
        let mut l = Luz::nueva(0);
        let f = LuzForzada::default();
        l.actualizar(&a, &entrada(0), &f);
        assert_eq!(l.estado.color, (255, 0, 0));
        for ms in (20..=500).step_by(20) {
            l.actualizar(&a, &entrada(ms), &f);
        }
        // Medio segundo de dos: un cuarto de vuelta, 90 grados.
        assert_eq!(l.estado.color, tono_a_color(90.0, 255));
    }

    /// Con la batería baja y fundido, va y viene del negro cada dos segundos.
    #[test]
    fn el_aviso_de_bateria_se_funde() {
        let mut a = ajustes();
        a.destello_en = 90;
        a.tipo_destello = 1;
        let mut l = Luz::nueva(0);
        let f = LuzForzada::default();
        l.actualizar(&a, &entrada(0), &f);
        assert_eq!(l.estado.color, (0, 0, 255));
        l.actualizar(&a, &entrada(1000), &f);
        assert_eq!(l.estado.color, transicion((0, 0, 255), NEGRO, 50.0));
        l.actualizar(&a, &entrada(2000), &f);
        assert_eq!(l.estado.color, NEGRO);
    }

    /// La macro de luz pone cada color el tiempo que dice y al acabar suelta la luz.
    #[test]
    fn la_macro_de_luz_va_color_a_color() {
        let m = MacroLuz::leer("True/255,0,0:100;0,255,0:50/Press/False").unwrap();
        assert!(!m.al_soltar && !m.cortar);
        assert_eq!(MacroLuz::leer("False/255,0,0:100/Press/False"), None);
        let mut a = AnimacionLuz::default();
        let mut luz = LuzForzada::default();
        a.empezar(&m, 0, &mut luz);
        assert_eq!((luz.activa, luz.color), (true, (255, 0, 0)));
        a.avanzar(60_000, &mut luz);
        assert_eq!(luz.color, (255, 0, 0));
        a.avanzar(100_000, &mut luz);
        assert_eq!(luz.color, (0, 255, 0));
        a.avanzar(150_000, &mut luz);
        assert!(!luz.activa && !a.en_marcha);
    }

    /// Cargando con arcoíris, el tono sube 0,167 grados en cada informe.
    #[test]
    fn el_arcoiris_de_carga_sube_por_informe() {
        let mut a = ajustes();
        a.tipo_carga = 2;
        let mut l = Luz::nueva(0);
        let f = LuzForzada::default();
        let mut en = entrada(0);
        en.cargando = true;
        for _ in 0..600 {
            l.actualizar(&a, &en, &f);
        }
        assert_eq!(
            l.estado.color,
            tono_a_color((600.0 * 0.167f64) as f32 % 360.0, 255)
        );
    }

    /// Sin usar el mando, la luz por batería se apaga poco a poco hacia la desconexión.
    #[test]
    fn la_luz_se_apaga_sin_usar_el_mando() {
        let mut a = ajustes();
        a.por_bateria = true;
        a.desconexion_s = 100;
        let mut l = Luz::nueva(0);
        let f = LuzForzada::default();
        l.actualizar(&a, &entrada(10_000), &f);
        let lleno = l.estado.color;
        l.actualizar(&a, &entrada(75_000), &f);
        assert!(l.estado.color.2 < lleno.2);
        l.actualizar(&a, &entrada(100_000), &f);
        assert_eq!(l.estado.color, NEGRO);
    }
}
