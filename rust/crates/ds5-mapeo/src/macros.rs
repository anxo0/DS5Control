//! Las macros asignadas a un control (`PlayMacro`, `PlayMacroTask`, `PlayMacroCodeValue` y
//! `EndMacro`). En C# cada macro corre en su propio hilo con esperas de verdad; aquí avanza con la
//! hora de cada informe, así que lo que hay detrás de una espera sale en el primer informe que
//! llega después de que acabe.
//!
//! Cada paso de una macro es un número:
//! - de 0 a 255, una tecla; de 256 a 260, un botón del ratón; de 261 a 288, un botón del mando
//!   virtual. Cada vez que aparece se pulsa o se suelta, alternando;
//! - de 300 a 999.999, una espera de ese número menos 300 milisegundos;
//! - de 1.000.000 en adelante, la vibración (`1FFFSSS`: fuerte y suave);
//! - de 1.000.000.000 en adelante, la luz (`1RRRGGGBBB`), o volver a la normal si es justo ese.

use ds5_informes::{boton, Estado};

use crate::botones::LuzForzada;
use crate::raton::{
    EventoRaton, CENTRAL_BAJA, CENTRAL_SUBE, DERECHO_BAJA, DERECHO_SUBE, IZQUIERDO_BAJA,
    IZQUIERDO_SUBE, X_BAJA, X_SUBE,
};

/// `DS5KeyType`.
const EXPLORACION: u8 = 1;
const MANTENER: u8 = 16;
/// `RepeatMacro`: la macro de una acción especial vuelve a sonar mientras se mantiene.
pub const REPETIR: u8 = 32;

/// `MAX_MACRO_VALUE`.
const MAXIMO_MANDO: i32 = 288;
const TECLA_TAB: u32 = 0x09;
const TECLA_ALT: u32 = 0x12;

/// Lo que hace un paso de la macro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Paso {
    Sigue,
    /// Esperar estos milisegundos.
    Espera(i64),
    /// C# se para con una excepción (un índice fuera de su vector) y la macro se queda a medias.
    Rota,
}

/// Una macro sonando.
#[derive(Debug, Clone)]
struct Reproduccion {
    pasos: Vec<i32>,
    siguiente: usize,
    tipo: u8,
    /// El control que la lanzó (su índice en `macrodone`), o `None`.
    control: Option<usize>,
    pulsadas: Vec<bool>,
    /// Hasta cuándo espera.
    hasta_us: i64,
    /// Ya ha soltado lo pulsado y solo espera los 50 ms de `HoldMacro`.
    acabada: bool,
    /// Se ha parado con una excepción: lo que va detrás en C# no llega a hacerse.
    rota: bool,
    /// `keepKeyState` de la acción especial: al acabar no suelta lo pulsado ni apaga la luz.
    mantener: bool,
    /// La acción especial que la lanzó (su índice en `actionDone`), para volver a lanzarla con
    /// `RepeatMacro`.
    accion: Option<usize>,
    /// La cola de las macros sincronizadas (los controles de la acción): suenan una detrás de otra.
    cola: Option<String>,
}

impl Reproduccion {
    fn nueva(pasos: &[i32], tipo: u8, control: Option<usize>, ahora_us: i64) -> Reproduccion {
        Reproduccion {
            pasos: pasos.to_vec(),
            siguiente: 0,
            tipo,
            control,
            pulsadas: vec![false; 512],
            hasta_us: ahora_us,
            acabada: false,
            rota: false,
            mantener: false,
            accion: None,
            cola: None,
        }
    }
}

/// Cómo lanza una acción especial su macro.
#[derive(Debug, Clone, Default)]
pub struct OpcionesMacro {
    pub mantener_teclas: bool,
    /// El índice de la acción, si la macro se repite mientras se mantiene (`RepeatMacro`).
    pub accion: Option<usize>,
    /// La cola de las macros sincronizadas.
    pub cola: Option<String>,
}

/// Lo que comparten todas las macros de un mando (en C#, estáticos de `Mapping`).
#[derive(Debug, Clone)]
pub struct Macros {
    /// `macrodone`: la macro de este control ya ha empezado y no se repite hasta soltarlo.
    hechas: [bool; 52],
    /// `macroControl` y `macroCount`: los botones del mando virtual que tienen pulsados.
    mando: [bool; 26],
    cuenta: u32,
    en_curso: Vec<Reproduccion>,
    /// Las macros sincronizadas que esperan a que acabe la de delante en su cola.
    esperando: Vec<Reproduccion>,
    /// `altTabDone`, `altTabNow` y `oldAltTabNow`.
    alt_tab_hecho: bool,
    alt_tab_us: i64,
    alt_tab_antes_us: i64,
    /// La última vibración que ha pedido una macro (fuerte, suave), directa al mando, sin el
    /// refuerzo del perfil.
    pub vibracion: Option<(u8, u8)>,
    /// Las acciones especiales cuya macro con `RepeatMacro` ha acabado: vuelven a poder lanzarse.
    pub acciones_acabadas: Vec<usize>,
}

impl Default for Macros {
    fn default() -> Self {
        Macros {
            hechas: [false; 52],
            mando: [false; 26],
            cuenta: 0,
            en_curso: Vec::new(),
            esperando: Vec::new(),
            alt_tab_hecho: true,
            alt_tab_us: 0,
            alt_tab_antes_us: -1_000_000,
            vibracion: None,
            acciones_acabadas: Vec::new(),
        }
    }
}

/// `DS5ControltoInt`: los controles después de `SideR` no tienen sitio propio y usan el 0.
fn indice(c: usize) -> usize {
    if c <= 51 {
        c
    } else {
        0
    }
}

/// Si la macro es la de Alt+Tab (`164/9/9/164` o `18/9/9/18`).
fn es_alt_tab(p: &[i32]) -> bool {
    p.len() >= 4 && ((p[..4] == [164, 9, 9, 164]) || (p[..4] == [18, 9, 9, 18]))
}

/// Los tres dígitos de un trozo del número, como hace C# con el texto.
fn tres(d: &[u8], i: usize) -> u8 {
    let n = |j: usize| (d[j] - b'0') as i32;
    (n(i) * 100 + n(i + 1) * 10 + n(i + 2)) as u8
}

impl Macros {
    /// Una macro asignada a un control, en cada informe: mientras el control está activo se lanza
    /// (una vez por pulsación, o en bucle con `HoldMacro`); al soltarlo puede volver a lanzarse.
    #[allow(clippy::too_many_arguments)]
    pub fn control(
        &mut self,
        c: usize,
        pasos: &[i32],
        tipo: u8,
        activo: bool,
        ahora_us: i64,
        luz: &mut LuzForzada,
        eventos: &mut Vec<EventoRaton>,
    ) {
        if activo {
            if es_alt_tab(pasos) {
                self.alt_tab(pasos, ahora_us, eventos);
                self.hechas[indice(c)] = true;
            } else if !self.hechas[indice(c)] {
                self.hechas[indice(c)] = true;
                let r = Reproduccion::nueva(pasos, tipo, Some(indice(c)), ahora_us);
                self.empezar(r, ahora_us, luz, eventos);
            }
        } else {
            if es_alt_tab(pasos) && !self.alt_tab_hecho && self.alt_tab_us < ahora_us - 10_000 {
                self.soltar_alt_tab(ahora_us, eventos);
            }
            self.hechas[indice(c)] = false;
        }
    }

    /// `PlayMacro` desde una acción especial, sin control.
    #[allow(clippy::too_many_arguments)]
    pub fn lanzar(
        &mut self,
        pasos: &[i32],
        tipo: u8,
        opciones: OpcionesMacro,
        ahora_us: i64,
        luz: &mut LuzForzada,
        eventos: &mut Vec<EventoRaton>,
    ) {
        let mut r = Reproduccion::nueva(pasos, tipo, None, ahora_us);
        r.mantener = opciones.mantener_teclas;
        r.accion = opciones.accion;
        r.cola = opciones.cola;
        let ocupada = r.cola.is_some()
            && self
                .en_curso
                .iter()
                .chain(&self.esperando)
                .any(|o| o.cola == r.cola);
        if ocupada {
            self.esperando.push(r);
        } else {
            self.empezar(r, ahora_us, luz, eventos);
        }
    }

    /// `EndMacro` sin control: solo suelta el Alt+Tab.
    pub fn terminar(&mut self, pasos: &[i32], ahora_us: i64, eventos: &mut Vec<EventoRaton>) {
        if es_alt_tab(pasos) && !self.alt_tab_hecho {
            self.soltar_alt_tab(ahora_us, eventos);
        }
    }

    /// Empieza a sonar una macro: hace sus pasos hasta la primera espera.
    fn empezar(
        &mut self,
        r: Reproduccion,
        ahora_us: i64,
        luz: &mut LuzForzada,
        eventos: &mut Vec<EventoRaton>,
    ) {
        if r.control.is_none() && es_alt_tab(&r.pasos) {
            // La de Alt+Tab de una acción no espera: pulsa lo que toque y acaba.
            self.alt_tab(&r.pasos, ahora_us, eventos);
            self.acabar(&r, ahora_us, luz, eventos);
            return;
        }
        self.en_curso.push(r);
        let n = self.en_curso.len() - 1;
        if self.seguir(n, ahora_us, luz, eventos) {
            let r = self.en_curso.remove(n);
            self.acabar(&r, ahora_us, luz, eventos);
        }
    }

    /// Lo que pasa cuando acaba una macro: la acción que se repite puede volver a lanzarse y, en
    /// su cola, empieza la siguiente.
    fn acabar(
        &mut self,
        r: &Reproduccion,
        ahora_us: i64,
        luz: &mut LuzForzada,
        eventos: &mut Vec<EventoRaton>,
    ) {
        if let (Some(a), false) = (r.accion, r.rota) {
            if r.tipo & REPETIR != 0 {
                self.acciones_acabadas.push(a);
            }
        }
        if r.cola.is_some() {
            if let Some(i) = self.esperando.iter().position(|o| o.cola == r.cola) {
                let mut siguiente = self.esperando.remove(i);
                siguiente.hasta_us = ahora_us;
                self.empezar(siguiente, ahora_us, luz, eventos);
            }
        }
    }

    /// `AltTabSwappingRelease`.
    fn soltar_alt_tab(&mut self, ahora_us: i64, eventos: &mut Vec<EventoRaton>) {
        self.alt_tab_hecho = true;
        eventos.push(EventoRaton::SueltaTecla(TECLA_TAB));
        eventos.push(EventoRaton::SueltaTecla(TECLA_ALT));
        self.alt_tab_us = ahora_us;
        self.alt_tab_antes_us = ahora_us - 86_400_000_000;
    }

    /// `AltTabSwapping`: la primera vez pulsa Tab; luego lo pulsa y suelta cada cierto tiempo.
    fn alt_tab(&mut self, pasos: &[i32], ahora_us: i64, eventos: &mut Vec<EventoRaton>) {
        let ultimo = *pasos.last().unwrap();
        let espera = if ultimo <= 300 || ultimo > u16::MAX as i32 {
            1000
        } else {
            ultimo - 300
        } as i64;
        if self.alt_tab_hecho {
            self.alt_tab_hecho = false;
            eventos.push(EventoRaton::Tecla(TECLA_TAB));
        } else {
            self.alt_tab_us = ahora_us;
            if ahora_us >= self.alt_tab_antes_us + espera * 1000 {
                self.alt_tab_antes_us = ahora_us;
                eventos.push(EventoRaton::Tecla(TECLA_TAB));
                eventos.push(EventoRaton::SueltaTecla(TECLA_TAB));
            }
        }
    }

    /// Sigue las macros que esperaban, al llegar un informe nuevo.
    pub fn avanzar(&mut self, ahora_us: i64, luz: &mut LuzForzada, eventos: &mut Vec<EventoRaton>) {
        let mut i = 0;
        while i < self.en_curso.len() {
            if self.en_curso[i].hasta_us <= ahora_us && self.seguir(i, ahora_us, luz, eventos) {
                let r = self.en_curso.remove(i);
                self.acabar(&r, ahora_us, luz, eventos);
            } else {
                i += 1;
            }
        }
    }

    /// Hace los pasos de una macro hasta la siguiente espera. Devuelve si ha terminado.
    fn seguir(
        &mut self,
        n: usize,
        ahora_us: i64,
        luz: &mut LuzForzada,
        eventos: &mut Vec<EventoRaton>,
    ) -> bool {
        let mut r = std::mem::replace(
            &mut self.en_curso[n],
            Reproduccion {
                acabada: true,
                ..Reproduccion::nueva(&[], 0, None, 0)
            },
        );
        let fin = self.seguir_reproduccion(&mut r, ahora_us, luz, eventos);
        self.en_curso[n] = r;
        fin
    }

    fn seguir_reproduccion(
        &mut self,
        r: &mut Reproduccion,
        ahora_us: i64,
        luz: &mut LuzForzada,
        eventos: &mut Vec<EventoRaton>,
    ) -> bool {
        if r.acabada {
            // Los 50 ms de `HoldMacro`: después la macro puede volver a lanzarse.
            if let Some(c) = r.control {
                self.hechas[c] = false;
            }
            return true;
        }
        // La hora de la macro sigue desde donde acabó la espera, para que no se acumule el retraso
        // de cada informe.
        let mut hora = r.hasta_us;
        while r.siguiente < r.pasos.len() {
            let v = r.pasos[r.siguiente];
            r.siguiente += 1;
            match self.paso(v, r.tipo, &mut r.pulsadas, luz, eventos) {
                Paso::Sigue => {}
                Paso::Rota => {
                    r.rota = true;
                    return true;
                }
                Paso::Espera(ms) => {
                    hora += ms * 1000;
                    if hora > ahora_us {
                        r.hasta_us = hora;
                        return false;
                    }
                }
            }
        }
        // Se suelta lo que haya quedado pulsado y la luz vuelve a la normal, salvo que la acción
        // pida dejarlo como está.
        if !r.mantener {
            for i in 0..r.pulsadas.len() {
                if r.pulsadas[i]
                    && self.paso(i as i32, r.tipo, &mut r.pulsadas, luz, eventos) == Paso::Rota
                {
                    r.rota = true;
                    return true;
                }
            }
            luz.destello = 0;
            luz.activa = false;
        }
        if r.tipo & MANTENER != 0 {
            r.acabada = true;
            r.hasta_us = hora + 50_000;
            return if r.hasta_us <= ahora_us {
                if let Some(c) = r.control {
                    self.hechas[c] = false;
                }
                true
            } else {
                false
            };
        }
        true
    }

    /// `PlayMacroCodeValue`.
    fn paso(
        &mut self,
        v: i32,
        tipo: u8,
        pulsadas: &mut [bool],
        luz: &mut LuzForzada,
        eventos: &mut Vec<EventoRaton>,
    ) -> Paso {
        if (261..=MAXIMO_MANDO).contains(&v) {
            let i = (v - 261) as usize;
            if i >= self.mando.len() {
                return Paso::Rota;
            }
            let u = v as usize;
            if !pulsadas[u] {
                self.mando[i] = true;
                pulsadas[u] = true;
                self.cuenta += 1;
            } else {
                self.mando[i] = false;
                pulsadas[u] = false;
                if self.cuenta > 0 {
                    self.cuenta -= 1;
                }
            }
        } else if v < 300 {
            if v < 0 {
                return Paso::Rota;
            }
            let u = v as usize;
            let alt = tipo & EXPLORACION != 0;
            let pulsa = !pulsadas[u];
            eventos.push(match (v, pulsa) {
                (256, true) => EventoRaton::Boton(IZQUIERDO_BAJA),
                (257, true) => EventoRaton::Boton(DERECHO_BAJA),
                (258, true) => EventoRaton::Boton(CENTRAL_BAJA),
                (259, true) => EventoRaton::BotonX(X_BAJA, 1),
                (260, true) => EventoRaton::BotonX(X_BAJA, 2),
                (256, false) => EventoRaton::Boton(IZQUIERDO_SUBE),
                (257, false) => EventoRaton::Boton(DERECHO_SUBE),
                (258, false) => EventoRaton::Boton(CENTRAL_SUBE),
                (259, false) => EventoRaton::BotonX(X_SUBE, 1),
                (260, false) => EventoRaton::BotonX(X_SUBE, 2),
                (_, true) if alt => EventoRaton::TeclaAlt(v as u32),
                (_, true) => EventoRaton::Tecla(v as u32),
                (_, false) if alt => EventoRaton::SueltaTeclaAlt(v as u32),
                (_, false) => EventoRaton::SueltaTecla(v as u32),
            });
            pulsadas[u] = pulsa;
        } else if v >= 1_000_000_000 {
            if v > 1_000_000_000 {
                let d = v.to_string();
                let d = &d.as_bytes()[1..];
                *luz = LuzForzada {
                    activa: true,
                    destello: 0,
                    color: (tres(d, 0), tres(d, 3), tres(d, 6)),
                };
            } else {
                luz.destello = 0;
                luz.activa = false;
            }
        } else if v >= 1_000_000 {
            let d = v.to_string();
            let d = &d.as_bytes()[1..];
            self.vibracion = Some((tres(d, 0), tres(d, 3)));
        } else {
            return Paso::Espera((v - 300) as i64);
        }
        Paso::Sigue
    }

    /// El final de `MapCustom`: los botones del mando que tienen pulsados las macros salen
    /// pulsados.
    pub fn volcar(&self, m: &mut Estado) {
        if self.cuenta == 0 {
            return;
        }
        let b = &self.mando;
        let botones = [
            (0, boton::EQUIS),
            (1, boton::CIRCULO),
            (2, boton::CUADRADO),
            (3, boton::TRIANGULO),
            (4, boton::OPCIONES),
            (5, boton::CREAR),
            (6, boton::ARRIBA),
            (7, boton::ABAJO),
            (8, boton::IZQUIERDA),
            (9, boton::DERECHA),
            (10, boton::PS),
            (11, boton::L1),
            (12, boton::R1),
            (15, boton::L3),
            (16, boton::R3),
            (25, boton::PANEL_SALIDA),
        ];
        for (i, bit) in botones {
            if b[i] {
                m.botones |= bit;
            }
        }
        if b[13] {
            m.l2 = 255;
        }
        if b[14] {
            m.r2 = 255;
        }
        for (i, eje, v) in [
            (17, 0, 255),
            (18, 0, 0),
            (19, 1, 255),
            (20, 1, 0),
            (21, 2, 255),
            (22, 2, 0),
            (23, 3, 255),
            (24, 3, 0),
        ] {
            if b[i] {
                match eje {
                    0 => m.lx = v,
                    1 => m.ly = v,
                    2 => m.rx = v,
                    _ => m.ry = v,
                }
            }
        }
    }

    /// Toca una macro entera de golpe, sin esperar (`PlayMacroTask` sin control): para comparar
    /// con C# lo que hace cada paso.
    pub fn tocar_entera(
        &mut self,
        pasos: &[i32],
        tipo: u8,
        luz: &mut LuzForzada,
    ) -> Vec<EventoRaton> {
        let mut eventos = Vec::new();
        let mut r = Reproduccion::nueva(pasos, tipo, None, 0);
        // Con la hora muy adelantada, ninguna espera para.
        self.seguir_reproduccion(&mut r, i64::MAX / 2, luz, &mut eventos);
        eventos
    }

    /// Los botones del mando virtual que tienen pulsados las macros.
    pub fn botones_mando(&self) -> [bool; 26] {
        self.mando
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// Una macro con espera: lo de antes sale al pulsar y lo de después cuando pasa el tiempo.
    /// Al acabar suelta lo que quedaba pulsado. No vuelve a sonar hasta soltar el control.
    #[test]
    fn la_macro_espera_y_suelta_al_acabar() {
        let mut m = Macros::default();
        let mut luz = LuzForzada::default();
        let pasos = [65, 65, 66, 400, 67];
        let mut e = Vec::new();
        m.control(15, &pasos, 0, true, 1_000_000, &mut luz, &mut e);
        assert_eq!(
            e,
            vec![
                EventoRaton::Tecla(65),
                EventoRaton::SueltaTecla(65),
                EventoRaton::Tecla(66)
            ]
        );
        e.clear();
        m.avanzar(1_050_000, &mut luz, &mut e);
        assert!(e.is_empty());
        m.avanzar(1_100_000, &mut luz, &mut e);
        assert_eq!(
            e,
            vec![
                EventoRaton::Tecla(67),
                EventoRaton::SueltaTecla(66),
                EventoRaton::SueltaTecla(67)
            ]
        );
        e.clear();
        m.control(15, &pasos, 0, true, 1_200_000, &mut luz, &mut e);
        assert!(e.is_empty());
        m.control(15, &pasos, 0, false, 1_210_000, &mut luz, &mut e);
        m.control(15, &pasos, 0, true, 1_220_000, &mut luz, &mut e);
        assert_eq!(e[0], EventoRaton::Tecla(65));
    }

    /// Con `HoldMacro` la macro vuelve a sonar mientras se mantiene, 50 ms después de acabar.
    #[test]
    fn la_macro_mantenida_se_repite() {
        let mut m = Macros::default();
        let mut luz = LuzForzada::default();
        let mut veces = 0;
        for ms in (0..=200).step_by(10) {
            let mut e = Vec::new();
            m.avanzar(ms * 1000, &mut luz, &mut e);
            m.control(18, &[70, 70], MANTENER, true, ms * 1000, &mut luz, &mut e);
            veces += e.iter().filter(|x| **x == EventoRaton::Tecla(70)).count();
        }
        // A los 0, 50, 100, 150 y 200 ms.
        assert_eq!(veces, 5);
    }

    /// Los botones del mando virtual de la macro salen en el estado mientras están pulsados.
    #[test]
    fn los_botones_del_mando_salen_en_el_estado() {
        let mut m = Macros::default();
        let mut luz = LuzForzada::default();
        let mut e = Vec::new();
        m.control(15, &[261, 1300, 261], 0, true, 0, &mut luz, &mut e);
        let mut s = Estado::default();
        m.volcar(&mut s);
        assert!(s.pulsado(boton::EQUIS));
        m.avanzar(1_000_000, &mut luz, &mut e);
        let mut s = Estado::default();
        m.volcar(&mut s);
        assert!(!s.pulsado(boton::EQUIS));
    }
}
