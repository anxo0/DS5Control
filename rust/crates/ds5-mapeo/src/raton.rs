//! El teclado y el ratón virtuales: lo que se les manda (`VirtualKBMBase`) y `Mapping.Commit`, que
//! convierte los clics y las teclas de cada informe en pulsaciones y sueltas.

/// Una llamada al teclado o al ratón virtual.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventoRaton {
    /// `MoveRelativeMouse`.
    Mover(i32, i32),
    /// `MoveAbsoluteMouse`, de 0 a 1.
    Absoluto(f64, f64),
    /// `PerformMouseWheelEvent(vertical, horizontal)`.
    Rueda(i32, i32),
    /// `PerformMouseButtonEvent`, con las banderas de `SendInput`.
    Boton(u32),
    /// `PerformMouseButtonEventAlt`, para los botones 4 y 5.
    BotonX(u32, i32),
    /// `PerformKeyPress`.
    Tecla(u32),
    /// `PerformKeyPressAlt`: la tecla por su código de exploración.
    TeclaAlt(u32),
    /// `PerformKeyRelease`.
    SueltaTecla(u32),
    /// `PerformKeyReleaseAlt`.
    SueltaTeclaAlt(u32),
}

pub const IZQUIERDO_BAJA: u32 = 2;
pub const IZQUIERDO_SUBE: u32 = 4;
pub const DERECHO_BAJA: u32 = 8;
pub const DERECHO_SUBE: u32 = 16;
pub const CENTRAL_BAJA: u32 = 32;
pub const CENTRAL_SUBE: u32 = 64;
pub const X_BAJA: u32 = 128;
pub const X_SUBE: u32 = 256;
pub const RUEDA_ARRIBA: i32 = 120;
pub const RUEDA_ABAJO: i32 = -120;

/// `Mapping.Click`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clic {
    Izquierdo,
    Central,
    Derecho,
    Cuarto,
    Quinto,
    RuedaArriba,
    RuedaAbajo,
}

/// `SyntheticState.MouseClick`: cuántas veces se ha pedido cada clic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Clics {
    pub izquierdo: i32,
    pub central: i32,
    pub derecho: i32,
    pub cuarto: i32,
    pub quinto: i32,
    pub rueda_arriba: i32,
    pub rueda_abajo: i32,
    pub alterna: i32,
    pub alterna_activa: bool,
}

/// `SyntheticState.KeyPress`: cuántas veces se ha pedido una tecla.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pulsacion {
    pub normal: i32,
    pub exploracion: i32,
    pub repeticion: i32,
    pub alterna: i32,
    pub alterna_activa: bool,
}

/// `SyntheticState.KeyPresses`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Tecla {
    previa: Pulsacion,
    actual: Pulsacion,
}

/// El estado sintético del mando y el global (`deviceState` y `globalState`), con un solo mando.
#[derive(Debug, Clone)]
pub struct Sintetico {
    pub actual: Clics,
    previo: Clics,
    global: Clics,
    global_previo: Clics,
    /// Las teclas del mando y las globales, en el orden en que se pidieron por primera vez, como
    /// los diccionarios de C#.
    teclas: Vec<(u16, Tecla)>,
    teclas_global: Vec<(u16, Tecla)>,
    rueda: i32,
    /// `oldnow`: lo comparten la rueda, la repetición de teclas y la rueda con un stick.
    pub antes_us: i64,
    /// `pressagain`: la tecla mantenida ya se repite.
    pub repetir: bool,
    /// `keyshelddown`.
    mantenida: u16,
    /// `fakeKeyRepeat`: con `SendInput` la tecla mantenida se repite cada 25 ms pasado medio
    /// segundo.
    pub repeticion_falsa: bool,
    /// La hora del informe que se está procesando.
    pub ahora_us: i64,
}

impl Default for Sintetico {
    fn default() -> Self {
        Sintetico {
            actual: Clics::default(),
            previo: Clics::default(),
            global: Clics::default(),
            global_previo: Clics::default(),
            teclas: Vec::new(),
            teclas_global: Vec::new(),
            rueda: 0,
            antes_us: i64::MIN / 2,
            repetir: false,
            mantenida: 0,
            repeticion_falsa: false,
            ahora_us: 0,
        }
    }
}

impl Sintetico {
    /// `Mapping.MapClick`.
    pub fn clic(&mut self, c: Clic) {
        let a = &mut self.actual;
        match c {
            Clic::Izquierdo => a.izquierdo += 1,
            Clic::Central => a.central += 1,
            Clic::Derecho => a.derecho += 1,
            Clic::Cuarto => a.cuarto += 1,
            Clic::Quinto => a.quinto += 1,
            Clic::RuedaArriba => a.rueda_arriba += 1,
            Clic::RuedaAbajo => a.rueda_abajo += 1,
        }
    }

    /// Lo que se ha pedido de una tecla en este informe (`deviceState.keyPresses[tecla]`).
    pub fn tecla(&mut self, codigo: u16) -> &mut Pulsacion {
        let i = match self.teclas.iter().position(|(c, _)| *c == codigo) {
            Some(i) => i,
            None => {
                self.teclas.push((codigo, Tecla::default()));
                self.teclas.len() - 1
            }
        };
        &mut self.teclas[i].1.actual
    }

    /// `Mapping.Commit`.
    pub fn confirmar(&mut self, eventos: &mut Vec<EventoRaton>) {
        let (s, p) = (self.actual, self.previo);
        let g = &mut self.global;
        g.izquierdo += s.izquierdo - p.izquierdo;
        g.central += s.central - p.central;
        g.derecho += s.derecho - p.derecho;
        g.cuarto += s.cuarto - p.cuarto;
        g.quinto += s.quinto - p.quinto;
        g.rueda_arriba += s.rueda_arriba - p.rueda_arriba;
        g.rueda_abajo += s.rueda_abajo - p.rueda_abajo;
        g.alterna += s.alterna - p.alterna;
        g.alterna_activa = s.alterna_activa;
        let (g, gp) = (self.global, self.global_previo);
        let empieza = |a: i32, b: i32| a != 0 && b == 0;
        let acaba = |a: i32, b: i32| a == 0 && b != 0;
        let mut e = |v| eventos.push(v);

        if empieza(g.alterna, gp.alterna) {
            let (izq, der, cen, x) = if g.alterna_activa {
                (IZQUIERDO_BAJA, DERECHO_BAJA, CENTRAL_BAJA, X_BAJA)
            } else {
                (IZQUIERDO_SUBE, DERECHO_SUBE, CENTRAL_SUBE, X_SUBE)
            };
            if empieza(g.izquierdo, gp.izquierdo) {
                e(EventoRaton::Boton(izq));
            }
            if empieza(g.derecho, gp.derecho) {
                e(EventoRaton::Boton(der));
            }
            if empieza(g.central, gp.central) {
                e(EventoRaton::Boton(cen));
            }
            if empieza(g.cuarto, gp.cuarto) {
                e(EventoRaton::BotonX(x, 1));
            }
            if empieza(g.quinto, gp.quinto) {
                e(EventoRaton::BotonX(x, 2));
            }
        }

        if g.alterna == 0 && gp.alterna == 0 {
            let pares = [
                (g.izquierdo, gp.izquierdo, IZQUIERDO_BAJA, IZQUIERDO_SUBE),
                (g.central, gp.central, CENTRAL_BAJA, CENTRAL_SUBE),
                (g.derecho, gp.derecho, DERECHO_BAJA, DERECHO_SUBE),
            ];
            for (a, b, baja, sube) in pares {
                if empieza(a, b) {
                    e(EventoRaton::Boton(baja));
                } else if acaba(a, b) {
                    e(EventoRaton::Boton(sube));
                }
            }
            for (a, b, n) in [(g.cuarto, gp.cuarto, 1), (g.quinto, gp.quinto, 2)] {
                if empieza(a, b) {
                    e(EventoRaton::BotonX(X_BAJA, n));
                } else if acaba(a, b) {
                    e(EventoRaton::BotonX(X_SUBE, n));
                }
            }
            if empieza(g.rueda_arriba, gp.rueda_arriba) {
                e(EventoRaton::Rueda(RUEDA_ARRIBA, 0));
                self.antes_us = self.ahora_us;
                self.rueda = RUEDA_ARRIBA;
            } else if acaba(g.rueda_arriba, gp.rueda_arriba) {
                self.rueda = 0;
            }
            if empieza(g.rueda_abajo, gp.rueda_abajo) {
                e(EventoRaton::Rueda(RUEDA_ABAJO, 0));
                self.antes_us = self.ahora_us;
                self.rueda = RUEDA_ABAJO;
            }
            if acaba(g.rueda_abajo, gp.rueda_abajo) {
                self.rueda = 0;
            }
        }

        // La rueda sigue girando mientras se mantiene, cada 150 ms.
        if self.rueda != 0 && self.ahora_us >= self.antes_us + 150_000 && !self.repetir {
            self.antes_us = self.ahora_us;
            e(EventoRaton::Rueda(self.rueda, 0));
        }

        self.confirmar_teclas(eventos);

        self.global_previo = self.global;
        self.previo = self.actual;
        self.actual = Clics {
            alterna_activa: self.actual.alterna_activa,
            ..Clics::default()
        };
        for (_, t) in self.teclas_global.iter_mut() {
            t.previa = t.actual;
        }
        for (_, t) in self.teclas.iter_mut() {
            t.previa = t.actual;
            t.actual = Pulsacion {
                alterna_activa: t.actual.alterna_activa,
                ..Pulsacion::default()
            };
        }
    }

    /// La parte de las teclas de `Mapping.Commit`: cada tecla del mando se suma a la global y se
    /// pulsa, se suelta o se repite. Con `SendInput` el código que se manda es la propia tecla.
    fn confirmar_teclas(&mut self, eventos: &mut Vec<EventoRaton>) {
        let ahora = self.ahora_us;
        for &(codigo, t) in &self.teclas {
            let (s, p) = (t.actual, t.previa);
            let g = match self.teclas_global.iter().position(|(c, _)| *c == codigo) {
                Some(i) => {
                    let g = &mut self.teclas_global[i].1.actual;
                    g.normal += s.normal - p.normal;
                    g.exploracion += s.exploracion - p.exploracion;
                    g.repeticion += s.repeticion - p.repeticion;
                    g.alterna_activa = s.alterna_activa;
                    g.alterna += s.alterna - p.alterna;
                    self.teclas_global[i].1
                }
                None => {
                    let g = Tecla {
                        previa: Pulsacion::default(),
                        actual: s,
                    };
                    self.teclas_global.push((codigo, g));
                    g
                }
            };
            let (c, a) = (g.actual, g.previa);
            let k = codigo as u32;
            let pulsa = |alt: bool| {
                if alt {
                    EventoRaton::TeclaAlt(k)
                } else {
                    EventoRaton::Tecla(k)
                }
            };
            let suelta = |alt: bool| {
                if alt {
                    EventoRaton::SueltaTeclaAlt(k)
                } else {
                    EventoRaton::SueltaTecla(k)
                }
            };
            if c.alterna != 0 && a.alterna == 0 && c.alterna_activa {
                eventos.push(pulsa(c.exploracion != 0));
            } else if c.alterna != 0 && a.alterna == 0 {
                // Se suelta con el tipo con que se pulsó.
                eventos.push(suelta(a.exploracion != 0));
            } else if c.normal + c.exploracion != 0 && a.normal + a.exploracion == 0 {
                self.antes_us = ahora;
                eventos.push(pulsa(c.exploracion != 0));
                self.repetir = false;
                self.mantenida = codigo;
            } else if self.repeticion_falsa
                && (c.alterna != 0
                    || a.alterna != 0
                    || c.repeticion != 0
                    || ((a.exploracion == 0) != (c.exploracion == 0)))
                && self.mantenida == codigo
            {
                // Pasado medio segundo, la tecla mantenida se repite cada 25 ms.
                if ahora >= self.antes_us + 500_000 && !self.repetir {
                    self.antes_us = ahora;
                    self.repetir = true;
                }
                if self.repetir && ahora >= self.antes_us + 25_000 {
                    self.antes_us = ahora;
                    eventos.push(pulsa(c.exploracion != 0));
                }
            }

            if c.alterna == 0
                && a.alterna == 0
                && c.normal + c.exploracion == 0
                && a.normal + a.exploracion != 0
            {
                eventos.push(suelta(a.exploracion != 0));
                self.repetir = false;
            }
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// Con `SendInput` la tecla mantenida se repite: a los 500 ms empieza y luego va cada 25 ms.
    #[test]
    fn la_tecla_mantenida_se_repite() {
        let mut s = Sintetico {
            repeticion_falsa: true,
            ..Default::default()
        };
        let mut pulsaciones = Vec::new();
        for ms in (0..=600).step_by(5) {
            s.ahora_us = ms * 1000;
            let p = s.tecla(65);
            p.normal += 1;
            p.repeticion += 1;
            let mut e = Vec::new();
            s.confirmar(&mut e);
            if e.contains(&EventoRaton::Tecla(65)) {
                pulsaciones.push(ms);
            }
        }
        assert_eq!(pulsaciones, vec![0, 525, 550, 575, 600]);
        s.ahora_us = 605_000;
        let mut e = Vec::new();
        s.confirmar(&mut e);
        assert_eq!(e, vec![EventoRaton::SueltaTecla(65)]);
    }
}
