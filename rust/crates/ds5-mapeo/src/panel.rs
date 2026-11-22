//! El panel táctil: los sucesos que saca `DS5Touchpad` de cada informe y lo que hace con ellos el
//! ratón del panel (`Mouse.cs`, con `MouseCursor` y `MouseWheel`) en los modos ratón, controles,
//! ratón absoluto y directo. El panel como stick está en `panel_stick`.
//!
//! Los tiempos van en microsegundos de la grabación: C# usa la hora del informe para los toques y
//! la del reloj para el doble toque y el modo de soltar, que en la app son casi la misma.

use ds5_informes::dualsense::{PANEL_ALTO, PANEL_ANCHO};
use ds5_informes::{boton, Estado};
use ds5_perfiles::mapeos::Accion;
use ds5_perfiles::perfil::{ActivacionPanel, Perfil, SalidaPanel};

use crate::botones::{control, Entradas};
use crate::giroscopio::boton_activo;
use crate::panel_stick::{AjustesStickPanel, StickPanel};
use crate::raton::{Clic, EventoRaton, Sintetico};
use crate::sticks::SticksAccion;

/// Un dedo en un suceso del panel. `dx` y `dy` solo valen en los movimientos.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Dedo {
    pub x: i32,
    pub y: i32,
    pub id: u8,
    pub dx: i32,
    pub dy: i32,
    /// Dónde estaba en el informe anterior (solo en los movimientos).
    pub antes: (i32, i32),
}

/// `TouchpadEventArgs`. Sin dedos es el `null` de C#.
#[derive(Debug, Clone, PartialEq)]
pub struct Toques {
    pub dedos: Vec<Dedo>,
    /// El botón del panel pulsado.
    pub boton: bool,
    /// Algún dedo encima (`Touch1Finger`).
    pub activo: bool,
    pub tiempo_us: i64,
}

/// Lo que pasa en el panel en un informe, en el orden en que lo avisa `DS5Touchpad`.
#[derive(Debug, Clone, PartialEq)]
pub enum Suceso {
    Empiezan(Toques),
    Mueven(Toques),
    Acaban(Toques),
    Baja(Toques),
    Sube(Toques),
    SinCambios,
}

/// `DS5Touchpad`: compara cada informe con el anterior y avisa de lo que ha cambiado.
#[derive(Debug, Clone, Default)]
pub struct Panel {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    abajo: bool,
    activo1: bool,
    activo2: bool,
    id1: u8,
    id2: u8,
    paquete: [u8; 8],
}

/// Los 8 bytes de los toques del informe, como vienen del mando.
fn bytes_toques(e: &Estado) -> [u8; 8] {
    let mut b = [0u8; 8];
    for (i, t) in e.toques.iter().enumerate() {
        let (x, y) = (t.x as i32, t.y as i32);
        b[i * 4] = t.crudo;
        b[i * 4 + 1] = (x & 0xFF) as u8;
        b[i * 4 + 2] = (((x >> 8) & 0x0F) | ((y & 0x0F) << 4)) as u8;
        b[i * 4 + 3] = ((y >> 4) & 0xFF) as u8;
    }
    b
}

impl Panel {
    /// `handleTouchpad`.
    pub fn sucesos(&mut self, e: &Estado, tiempo_us: i64) -> Vec<Suceso> {
        let datos = bytes_toques(e);
        let abajo = e.pulsado(boton::PANEL);
        if datos == self.paquete && abajo == self.abajo {
            return vec![Suceso::SinCambios];
        }
        self.paquete = datos;
        let id1 = datos[0] & 0x7F;
        let id2 = datos[4] & 0x7F;
        let x1 = (((datos[2] & 0x0F) as i32) << 8) | datos[1] as i32;
        let y1 = ((datos[3] as i32) << 4) | ((datos[2] & 0xF0) >> 4) as i32;
        let x2 = (((datos[6] & 0x0F) as i32) << 8) | datos[5] as i32;
        let y2 = ((datos[7] as i32) << 4) | ((datos[6] & 0xF0) >> 4) as i32;
        let t1 = e.pulsado(boton::TOQUE1);
        let t2 = e.pulsado(boton::TOQUE2);
        let activo = e.pulsado(boton::UN_DEDO);
        let toques = |dedos: Vec<Dedo>| Toques {
            dedos,
            boton: abajo,
            activo,
            tiempo_us,
        };
        let quieto = |x, y, id| Dedo {
            x,
            y,
            id,
            ..Dedo::default()
        };
        let movido = |x: i32, y: i32, id, ax: i32, ay: i32| Dedo {
            x,
            y,
            id,
            dx: x - ax,
            dy: y - ay,
            antes: (ax, ay),
        };
        // Los dedos que hay ahora, sin movimiento.
        let ahora = || {
            if t1 && t2 {
                vec![quieto(x1, y1, id1), quieto(x2, y2, id2)]
            } else if t1 {
                vec![quieto(x1, y1, id1)]
            } else {
                vec![quieto(x2, y2, id2)]
            }
        };

        let mut s = Vec::new();
        if t1 || t2 {
            if (t1 && !self.activo1) || (t2 && !self.activo2) {
                s.push(Suceso::Empiezan(toques(ahora())));
            } else if t1 == self.activo1 && t2 == self.activo2 {
                let dedos = if t1 && t2 {
                    vec![
                        movido(x1, y1, id1, self.x1, self.y1),
                        movido(x2, y2, id2, self.x2, self.y2),
                    ]
                } else if t1 {
                    vec![movido(x1, y1, id1, self.x1, self.y1)]
                } else {
                    vec![movido(x2, y2, id2, self.x2, self.y2)]
                };
                s.push(Suceso::Mueven(toques(dedos)));
            }
            if !self.abajo && abajo {
                s.push(Suceso::Baja(toques(ahora())));
            } else if self.abajo && !abajo {
                s.push(Suceso::Sube(toques(ahora())));
            }
            if t1 {
                self.x1 = x1;
                self.y1 = y1;
            }
            if t2 {
                self.x2 = x2;
                self.y2 = y2;
            }
        } else {
            if abajo && !self.abajo {
                s.push(Suceso::Baja(toques(Vec::new())));
            } else if !abajo && self.abajo {
                s.push(Suceso::Sube(toques(Vec::new())));
            }
            if self.activo1 || self.activo2 {
                let dedos = if self.activo1 && self.activo2 {
                    vec![quieto(self.x1, self.y1, id1), quieto(self.x2, self.y2, id2)]
                } else if self.activo1 {
                    vec![quieto(self.x1, self.y1, id1)]
                } else {
                    vec![quieto(self.x2, self.y2, id2)]
                };
                s.push(Suceso::Acaban(toques(dedos)));
            }
        }
        self.activo1 = t1;
        self.activo2 = t2;
        self.id1 = id1;
        self.id2 = id2;
        self.abajo = abajo;
        s
    }
}

/// Lo que hace falta del perfil para el panel.
#[derive(Debug, Clone, PartialEq)]
pub struct AjustesPanel {
    pub modo: SalidaPanel,
    /// `touchSensitivity`, en porcentaje.
    pub sensibilidad: u8,
    pub compensar_temblor: bool,
    /// `tapSensitivity`, en milisegundos (0 apaga el toque como clic).
    pub toque: u8,
    pub doble_toque: bool,
    pub desplazamiento: i32,
    pub invertir: i32,
    pub clic_directo: bool,
    pub esquina_derecha: bool,
    pub bola: bool,
    pub friccion: f64,
    /// En radianes, como la tiene C# en memoria.
    pub rotacion: f64,
    pub umbral: f64,
    pub absoluto_x: i32,
    pub absoluto_y: i32,
    pub absoluto_centrar: bool,
    pub activacion: ActivacionPanel,
    /// Los botones que, todos pulsados, quitan la inversión (`TouchDisInvTriggers`).
    pub sin_invertir: Vec<i32>,
    /// Si las zonas izquierda, arriba, derecha y multitoque hacen su clic de siempre.
    pub zonas_predeterminadas: [bool; 4],
    pub stick: AjustesStickPanel,
}

impl AjustesPanel {
    pub fn desde_perfil(p: &Perfil) -> AjustesPanel {
        let a = &p.ajustes;
        let predeterminada = |c: usize| p.mapeos.controles[c - 1].accion == Accion::Predeterminada;
        AjustesPanel {
            modo: a.salida_tactil,
            sensibilidad: a.sensibilidad_tactil,
            compensar_temblor: a.compensar_temblor_tactil,
            toque: a.sensibilidad_toque,
            doble_toque: a.doble_toque,
            desplazamiento: a.sensibilidad_desplazamiento,
            invertir: a.invertir_tactil,
            clic_directo: a.clic_tactil_directo,
            esquina_derecha: a.esquina_inferior_derecha,
            bola: a.bola,
            friccion: a.friccion_bola,
            // El perfil guarda lo que escribiría C#, que la pasa a radianes otra vez al guardar.
            rotacion: a.rotacion_raton_tactil * 180.0 / std::f64::consts::PI,
            umbral: a.umbral_raton_tactil,
            absoluto_x: a.raton_absoluto_tactil.zona_maxima_x,
            absoluto_y: a.raton_absoluto_tactil.zona_maxima_y,
            absoluto_centrar: a.raton_absoluto_tactil.volver_al_centro,
            activacion: a.activacion_tactil,
            sin_invertir: a.gatillos_desactivar_tactil.clone(),
            zonas_predeterminadas: [
                predeterminada(control::PANEL_IZQUIERDA),
                predeterminada(control::PANEL_ARRIBA),
                predeterminada(control::PANEL_DERECHA),
                predeterminada(control::PANEL_MULTI),
            ],
            stick: AjustesStickPanel::desde_perfil(p),
        }
    }
}

/// `TouchButtonModeCandidate`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Candidato {
    #[default]
    Ninguno,
    Izquierda,
    Derecha,
    Multi,
    Arriba,
}

const BOLA_LARGO: usize = 8;
const BOLA_RADIO: f64 = 0.0245;
const BOLA_ESCALA: f64 = 0.004;
const DESFASE_RATON: f64 = 0.015;

fn inercia_bola() -> f64 {
    2.0 * (45.0 * BOLA_RADIO * BOLA_RADIO) / 5.0
}

/// `Math.Sign` de un `double`, como entero.
fn signo(v: f64) -> i32 {
    if v > 0.0 {
        1
    } else if v < 0.0 {
        -1
    } else {
        0
    }
}

/// `Mapping.remainderCutoff(v * 100, 1.0) / 100` restado: deja dos decimales.
pub(crate) fn dos_decimales(v: f64) -> f64 {
    let d = v * 100.0;
    v - (d - (d as i32) as f64) / 100.0
}

/// El ratón del panel: `Mouse` con su `MouseCursor` y su `MouseWheel` (la parte del panel).
#[derive(Debug, Clone)]
pub struct RatonPanel {
    inicio_us: i64,
    primer_toque_us: i64,
    fin_us: i64,
    primero: (i32, i32),
    tocado_una_vez: bool,
    segundo_toque: bool,
    /// `swipeLeft`, `swipeRight`, `swipeUp` y `swipeDown`.
    pub deslizar: [bool; 4],
    /// Cuánto se ha deslizado en cada sentido, de 0 a 255.
    pub deslizar_b: [u8; 4],
    pub desliza_izquierda: bool,
    pub desliza_derecha: bool,
    /// `leftDown`, `rightDown`, `upperDown` y `multiDown`.
    pub zonas: [bool; 4],
    empieza: bool,
    acaba: bool,
    bola_x: [f64; BOLA_LARGO],
    bola_y: [f64; BOLA_LARGO],
    bola_cola: usize,
    bola_cabeza: usize,
    bola_aceleracion: f64,
    bola_vx: f64,
    bola_vy: f64,
    bola_activa: bool,
    bola_resto_x: f64,
    bola_resto_y: f64,
    candidato: Candidato,
    tocado: bool,
    clic_antes: bool,
    soltar_activo: bool,
    soltar_us: i64,
    pub arrastrando: bool,
    pub arrastrando2: bool,
    // MouseCursor
    resto_h: f64,
    resto_v: f64,
    ultimo_id: u8,
    // MouseWheel
    rueda_h: f64,
    rueda_v: f64,
    stick: StickPanel,
}

const IZQ: usize = 0;
const DER: usize = 1;
const ARR: usize = 2;
const MUL: usize = 3;

impl RatonPanel {
    /// Como queda tras cargar el perfil (`ResetTrackAccel` con su fricción).
    pub fn nuevo(a: &AjustesPanel) -> RatonPanel {
        RatonPanel {
            inicio_us: 0,
            primer_toque_us: i64::MIN / 4,
            fin_us: 0,
            primero: (0, 0),
            tocado_una_vez: false,
            segundo_toque: false,
            deslizar: [false; 4],
            deslizar_b: [0; 4],
            desliza_izquierda: false,
            desliza_derecha: false,
            zonas: [false; 4],
            empieza: false,
            acaba: false,
            bola_x: [0.0; BOLA_LARGO],
            bola_y: [0.0; BOLA_LARGO],
            bola_cola: 0,
            bola_cabeza: 0,
            bola_aceleracion: BOLA_RADIO * a.friccion / inercia_bola(),
            bola_vx: 0.0,
            bola_vy: 0.0,
            bola_activa: false,
            bola_resto_x: 0.0,
            bola_resto_y: 0.0,
            candidato: Candidato::Ninguno,
            tocado: false,
            clic_antes: false,
            soltar_activo: false,
            soltar_us: 0,
            arrastrando: false,
            arrastrando2: false,
            resto_h: 0.0,
            resto_v: 0.0,
            ultimo_id: 0,
            rueda_h: 0.0,
            rueda_v: 0.0,
            stick: StickPanel::default(),
        }
    }

    /// Si hay un toque que empieza o acaba sin leer (`TouchStarted` y `TouchEnded`, sin gastarlos).
    pub fn toque_pendiente(&self) -> (bool, bool) {
        (self.empieza, self.acaba)
    }

    /// Lo que lee la reasignación del panel. Como en C#, leer el toque que empieza o acaba lo gasta.
    pub fn entradas(&mut self) -> Entradas {
        let e = Entradas {
            zonas_panel: [
                self.zonas[IZQ],
                self.zonas[ARR],
                self.zonas[MUL],
                self.zonas[DER],
            ],
            deslizar: self.deslizar_b,
            deslizar_activo: self.deslizar,
            toque_empieza: self.empieza,
            toque_acaba: self.acaba,
            ..Entradas::default()
        };
        self.empieza = false;
        self.acaba = false;
        e
    }

    /// Todos los sucesos de un informe. `e` es el estado del mando, al que el panel le apaga el
    /// botón del panel de salida; `activo` es si el panel mueve el ratón (`TouchActive`). El panel
    /// como stick apunta en `sticks` lo que pide.
    #[allow(clippy::too_many_arguments)]
    pub fn procesar(
        &mut self,
        sucesos: &[Suceso],
        e: &mut Estado,
        a: &AjustesPanel,
        activo: bool,
        clics: &mut Sintetico,
        sticks: &mut SticksAccion,
        eventos: &mut Vec<EventoRaton>,
    ) {
        let mut r = Contexto {
            e,
            a,
            activo,
            clics,
            sticks,
            eventos,
        };
        for s in sucesos {
            match s {
                Suceso::Empiezan(t) => {
                    self.empiezan(t, &mut r);
                    self.empieza_o_acaba(t);
                }
                Suceso::Mueven(t) => self.mueven(t, &mut r),
                Suceso::Acaban(t) => {
                    self.acaban(t, &mut r);
                    self.empieza_o_acaba(t);
                }
                Suceso::Baja(t) => self.baja(t, &mut r),
                Suceso::Sube(t) => self.sube(t, &mut r),
                Suceso::SinCambios => self.sin_cambios(&mut r),
            }
        }
    }

    /// `TouchStartedOrEnded`.
    fn empieza_o_acaba(&mut self, t: &Toques) {
        if t.dedos.len() < 2 {
            if t.activo {
                self.empieza = true;
            } else {
                self.acaba = true;
            }
        }
    }

    /// Si los botones de `TouchDisInvTriggers` están todos pulsados.
    fn sin_invertir(r: &Contexto) -> bool {
        r.a.sin_invertir.iter().all(|&n| boton_activo(r.e, n))
    }

    fn mueven(&mut self, t: &Toques, r: &mut Contexto) {
        let d0 = t.dedos[0];
        match r.a.modo {
            SalidaPanel::Mouse => {
                if r.activo {
                    let sin_invertir = Self::sin_invertir(r);
                    if r.a.bola {
                        self.bola_apuntar(
                            d0.dx as f64 * BOLA_ESCALA / 0.004,
                            d0.dy as f64 * BOLA_ESCALA / 0.004,
                        );
                    }
                    let arrastra = self.arrastrando || self.arrastrando2;
                    self.cursor_mueven(t, arrastra, sin_invertir, r);
                    self.rueda_mueven(t, arrastra, r);
                } else if r.a.bola {
                    self.bola_apuntar(0.0, 0.0);
                }
            }
            SalidaPanel::Controls => {
                let (fx, fy) = self.primero;
                if !self.deslizar.iter().any(|&v| v) && t.dedos.len() == 1 {
                    if d0.x - fx > 300 {
                        self.deslizar[DER] = true;
                    }
                    if d0.x - fx < -300 {
                        self.deslizar[IZQ] = true;
                    }
                    if d0.y - fy > 300 {
                        self.deslizar[MUL] = true;
                    }
                    if d0.y - fy < -300 {
                        self.deslizar[ARR] = true;
                    }
                }
                let b = |v: f32| v.clamp(0.0, 255.0) as u8;
                self.deslizar_b[ARR] = b((fy - d0.y) as f32 * 1.5);
                self.deslizar_b[MUL] = b((d0.y - fy) as f32 * 1.5);
                self.deslizar_b[IZQ] = (fx - d0.x).clamp(0, 255) as u8;
                self.deslizar_b[DER] = (d0.x - fx).clamp(0, 255) as u8;
            }
            SalidaPanel::MouseJoystick => {
                let t = r.e.transcurrido;
                self.stick
                    .mueven(d0.dx, d0.dy, r.activo, t, &r.a.stick, r.sticks);
            }
            SalidaPanel::AbsoluteMouse if r.activo => self.absoluto(t, r),
            _ => {}
        }

        // Para cambiar de perfil deslizando dos dedos.
        let (fx, fy) = self.primero;
        if (fy - d0.y).abs() < 50 && t.dedos.len() == 2 {
            if d0.x - fx > 200 && !self.desliza_izquierda {
                self.desliza_derecha = true;
            } else if fx - d0.x > 200 && !self.desliza_derecha {
                self.desliza_izquierda = true;
            }
        }

        self.comprobar_zonas(t, r.a.activacion);
        self.clics_sinteticos(r);
    }

    fn bola_apuntar(&mut self, x: f64, y: f64) {
        let i = self.bola_cola;
        self.bola_x[i] = x;
        self.bola_y[i] = y;
        self.bola_cola = (i + 1) % BOLA_LARGO;
        if self.bola_cabeza == self.bola_cola {
            self.bola_cabeza = (self.bola_cabeza + 1) % BOLA_LARGO;
        }
    }

    fn bola_vaciar(&mut self) {
        self.bola_x = [0.0; BOLA_LARGO];
        self.bola_y = [0.0; BOLA_LARGO];
        self.bola_vx = 0.0;
        self.bola_vy = 0.0;
        self.bola_activa = false;
        self.bola_cola = 0;
        self.bola_cabeza = 0;
        self.bola_resto_x = 0.0;
        self.bola_resto_y = 0.0;
    }

    /// Un paso de la bola: frena y mueve el cursor lo que toque.
    fn bola_paso(&mut self, sin_invertir: bool, r: &mut Contexto) {
        let angulo = (-self.bola_vy).atan2(self.bola_vx);
        let nx = angulo.cos().abs();
        let ny = angulo.sin().abs();
        let sx = signo(self.bola_vx) as f64;
        let sy = signo(self.bola_vy) as f64;
        let t = r.e.transcurrido;
        let frena_x = self.bola_vx.abs().min(self.bola_aceleracion * t * nx);
        let frena_y = self.bola_vy.abs().min(self.bola_aceleracion * t * ny);
        let vx = self.bola_vx - (frena_x * sx);
        let vy = self.bola_vy - (frena_y * sy);
        let mut mx = (vx * t) / BOLA_ESCALA;
        let mut my = (vy * t) / BOLA_ESCALA;
        if mx != 0.0 {
            mx += self.bola_resto_x;
        } else {
            self.bola_resto_x = 0.0;
        }
        let dx = mx as i32;
        self.bola_resto_x = mx - dx as f64;
        if my != 0.0 {
            my += self.bola_resto_y;
        } else {
            self.bola_resto_y = 0.0;
        }
        let dy = my as i32;
        self.bola_resto_y = my - dy as f64;
        self.bola_vx = vx;
        self.bola_vy = vy;
        if dx == 0 && dy == 0 {
            self.bola_activa = false;
        } else {
            self.mover_cursor(dx, dy, sin_invertir, r);
        }
    }

    fn empiezan(&mut self, t: &Toques, r: &mut Contexto) {
        let raton = r.a.modo == SalidaPanel::Mouse;
        if raton {
            self.bola_vaciar();
            if t.dedos.len() == 1 {
                self.resto_h = 0.0;
                self.resto_v = 0.0;
            }
            if t.dedos.len() == 2 {
                self.rueda_h = 0.0;
                self.rueda_v = 0.0;
            }
        } else if r.a.modo == SalidaPanel::MouseJoystick {
            self.stick.empiezan();
        }
        self.stick.olvidar();
        self.inicio_us = t.tiempo_us;
        self.primero = (t.dedos[0].x, t.dedos[0].y);
        if raton
            && r.a.doble_toque
            && (t.tiempo_us as f64) <= self.primer_toque_us as f64 + r.a.toque as f64 * 1.5 * 1000.0
            && !t.boton
        {
            self.segundo_toque = true;
        }
        self.tocado = false;
        self.clic_antes = false;
        self.soltar_activo = false;
        self.candidato = Candidato::Ninguno;
        self.zonas_arriba();
        self.comprobar_zonas(t, r.a.activacion);
        self.clics_sinteticos(r);
    }

    fn acaban(&mut self, t: &Toques, r: &mut Contexto) {
        self.tocado = true;
        self.desliza_derecha = false;
        self.desliza_izquierda = false;
        self.deslizar = [false; 4];
        self.deslizar_b = [0; 4];
        self.stick.olvidar();
        if r.a.toque != 0 && r.a.modo == SalidaPanel::Mouse {
            if self.segundo_toque {
                self.tocado_una_vez = false;
                self.segundo_toque = false;
            }
            let d0 = t.dedos[0];
            if (t.tiempo_us as f64) <= self.inicio_us as f64 + r.a.toque as f64 * 2.0 * 1000.0
                && !t.boton
                && !self.tocado_una_vez
                && (self.primero.0 - d0.x).abs() < 10
                && (self.primero.1 - d0.y).abs() < 10
            {
                if r.a.doble_toque {
                    self.tocado_una_vez = true;
                    self.primer_toque_us = t.tiempo_us;
                    self.fin_us = t.tiempo_us;
                } else {
                    r.clics.clic(Clic::Izquierdo);
                }
            }
        } else {
            match r.a.modo {
                SalidaPanel::Mouse => {
                    let sin_invertir = Self::sin_invertir(r);
                    if r.a.bola {
                        if !self.bola_activa {
                            let (mut x, mut y, mut peso) = (0.0, 0.0, 0.0);
                            let mut idx = usize::MAX;
                            let mut i = 0;
                            while i < BOLA_LARGO && idx != self.bola_cabeza {
                                idx = (self.bola_cola + BOLA_LARGO - i - 1) % BOLA_LARGO;
                                x += self.bola_x[idx];
                                y += self.bola_y[idx];
                                peso += 1.0;
                                i += 1;
                            }
                            self.bola_vx = x / peso;
                            self.bola_vy = y / peso;
                            self.bola_activa = true;
                        }
                        self.bola_paso(sin_invertir, r);
                    }
                }
                SalidaPanel::MouseJoystick => {
                    let tiempo = r.e.transcurrido;
                    self.stick.acaban(tiempo, &r.a.stick, r.sticks);
                }
                SalidaPanel::AbsoluteMouse if r.activo && r.a.absoluto_centrar => {
                    r.eventos.push(EventoRaton::Absoluto(0.5, 0.5));
                }
                _ => {}
            }
        }
        self.comprobar_zonas(t, r.a.activacion);
        self.clics_sinteticos(r);
    }

    fn sin_cambios(&mut self, r: &mut Contexto) {
        if r.e.pulsado(boton::UN_DEDO) {
            self.tocado = false;
        }
        if r.a.modo == SalidaPanel::Mouse && self.bola_activa {
            let sin_invertir = Self::sin_invertir(r);
            self.bola_paso(sin_invertir, r);
        } else if r.a.modo == SalidaPanel::MouseJoystick {
            let dedo = r.e.pulsado(boton::TOQUE1);
            let tiempo = r.e.transcurrido;
            self.stick.sin_cambios(dedo, tiempo, &r.a.stick, r.sticks);
        }
        let mut soltado = false;
        // C# compara al revés (`<`): casi siempre suelta en el informe siguiente.
        if self.soltar_activo && (r.ahora() as f64) < self.soltar_us as f64 + 100_000.0 {
            self.soltar_activo = false;
            soltado = true;
            self.zonas_arriba();
        }
        if r.e.pulsado(boton::UN_DEDO) || r.e.pulsado(boton::PANEL) || self.soltar_activo || soltado
        {
            self.clics_sinteticos(r);
        }
    }

    fn sube(&mut self, t: &Toques, r: &mut Contexto) {
        self.zonas = [false; 4];
        self.clic_antes = true;
        if r.e.pulsado(boton::TOQUE1)
            || r.e.pulsado(boton::TOQUE2)
            || r.a.activacion == ActivacionPanel::Release
        {
            self.comprobar_zonas(t, r.a.activacion);
            self.clics_sinteticos(r);
        }
    }

    fn baja(&mut self, t: &Toques, r: &mut Contexto) {
        if t.dedos.len() == 1
            && r.a.esquina_derecha
            && t.dedos[0].x > (1920 * 3) / 4
            && t.dedos[0].y > (960 * 3) / 4
        {
            r.clics.clic(Clic::Derecho);
        } else if t.dedos.is_empty() {
            self.candidato = Candidato::Ninguno;
        }
        self.clic_antes = false;
        self.comprobar_zonas(t, r.a.activacion);
        self.clics_sinteticos(r);
    }

    fn zonas_arriba(&mut self) {
        self.zonas = [false; 4];
        self.candidato = Candidato::Ninguno;
    }

    /// El candidato según dónde está el dedo (`isLeft` e `isRight`).
    fn por_lado(d: &Dedo) -> Candidato {
        if d.x < 1920 * 2 / 5 {
            Candidato::Izquierda
        } else {
            Candidato::Derecha
        }
    }

    /// `TouchButtonCheckProcess`.
    fn comprobar_zonas(&mut self, t: &Toques, modo: ActivacionPanel) {
        let mut activar = false;
        match Self::activacion(t, modo) {
            Activacion::Clic | Activacion::Toque => {
                if t.dedos.is_empty() {
                    self.candidato = Candidato::Arriba;
                } else if t.dedos.len() > 1 {
                    self.candidato = Candidato::Multi;
                } else {
                    self.candidato = Self::por_lado(&t.dedos[0]);
                }
                activar = true;
            }
            Activacion::Suelta => {
                self.zonas = [false; 4];
            }
            Activacion::Soltar => {
                if self.candidato == Candidato::Ninguno {
                    if !t.activo && !t.boton && self.clic_antes {
                        self.candidato = Candidato::Arriba;
                    } else if t.activo {
                        if t.dedos.len() > 1 {
                            self.candidato = Candidato::Multi;
                        }
                    } else if !t.activo && !t.dedos.is_empty() {
                        self.candidato = Self::por_lado(&t.dedos[0]);
                    }
                }
                if self.candidato != Candidato::Ninguno {
                    let suelta = if self.candidato == Candidato::Multi {
                        self.tocado && !t.activo
                    } else {
                        (self.tocado && !t.activo) || (self.clic_antes && !t.boton && !t.activo)
                    };
                    if suelta {
                        activar = true;
                        self.soltar_activo = true;
                        self.soltar_us = t.tiempo_us;
                    }
                }
            }
            Activacion::Nada => {}
        }
        if activar {
            match self.candidato {
                Candidato::Izquierda => self.zonas[IZQ] = true,
                Candidato::Derecha => self.zonas[DER] = true,
                Candidato::Multi => self.zonas[MUL] = true,
                Candidato::Arriba => self.zonas[ARR] = true,
                Candidato::Ninguno => {}
            }
        }
    }

    fn activacion(t: &Toques, modo: ActivacionPanel) -> Activacion {
        match modo {
            ActivacionPanel::Click if t.boton => Activacion::Clic,
            ActivacionPanel::Click => Activacion::Nada,
            ActivacionPanel::Touch if t.activo || t.boton => Activacion::Toque,
            ActivacionPanel::Touch => Activacion::Suelta,
            ActivacionPanel::Release => Activacion::Soltar,
        }
    }

    /// `synthesizeMouseButtons`.
    fn clics_sinteticos(&mut self, r: &mut Contexto) {
        if r.a.modo == SalidaPanel::Passthru {
            return;
        }
        if !r.a.clic_directo {
            r.e.botones &= !boton::PANEL_SALIDA;
        }
        let [izq, arriba, der, multi] = r.a.zonas_predeterminadas;
        if izq && self.zonas[IZQ] {
            r.clics.clic(Clic::Izquierdo);
            self.arrastrando2 = true;
        } else {
            self.arrastrando2 = false;
        }
        if arriba && self.zonas[ARR] {
            r.clics.clic(Clic::Central);
        }
        if der && self.zonas[DER] {
            r.clics.clic(Clic::Izquierdo);
        }
        if multi && self.zonas[MUL] {
            r.clics.clic(Clic::Derecho);
        }
        if r.a.modo == SalidaPanel::Mouse {
            if self.tocado_una_vez
                && r.ahora() as f64 > self.fin_us as f64 + r.a.toque as f64 * 1.5 * 1000.0
            {
                r.clics.clic(Clic::Izquierdo);
                self.tocado_una_vez = false;
            }
            if self.segundo_toque {
                r.clics.clic(Clic::Izquierdo);
                self.arrastrando = true;
            } else {
                self.arrastrando = false;
            }
        }
    }

    /// `MouseCursor.touchesMoved`.
    fn cursor_mueven(&mut self, t: &Toques, arrastra: bool, sin_invertir: bool, r: &mut Contexto) {
        let n = t.dedos.len();
        if (!arrastra && n != 1) || (arrastra && n < 1) {
            return;
        }
        let (mut dx, mut dy) = (0, 0);
        if t.dedos[0].id != self.ultimo_id {
            self.resto_h = 0.0;
            self.resto_v = 0.0;
            self.ultimo_id = t.dedos[0].id;
        } else if arrastra && n > 1 {
            dx = t.dedos[1].dx;
            dy = t.dedos[1].dy;
        } else {
            dx = t.dedos[0].dx;
            dy = t.dedos[0].dy;
        }
        self.mover_cursor(dx, dy, sin_invertir, r);
    }

    /// `MouseCursor.TouchMoveCursor`.
    fn mover_cursor(&mut self, mut dx: i32, mut dy: i32, sin_invertir: bool, r: &mut Contexto) {
        let a = r.a;
        if a.rotacion != 0.0 {
            let (s, c) = (a.rotacion.sin(), a.rotacion.cos());
            let (x, y) = (dx as f64, dy as f64);
            dx = (x * c - y * s).clamp(-PANEL_ANCHO as f64, PANEL_ANCHO as f64) as i32;
            dy = (x * s + y * c).clamp(-PANEL_ALTO as f64, PANEL_ALTO as f64) as i32;
        }
        let angulo = (-dy as f64).atan2(dx as f64);
        let nx = angulo.cos().abs();
        let ny = angulo.sin().abs();
        let sx = signo(dx as f64) as f64;
        let sy = signo(dy as f64) as f64;
        let k = a.sensibilidad as f64 * 0.01;
        let mut mx = if dx != 0 {
            k * dx as f64 + (nx * (DESFASE_RATON * sx))
        } else {
            0.0
        };
        let mut my = if dy != 0 {
            k * dy as f64 + (ny * (DESFASE_RATON * sy))
        } else {
            0.0
        };
        if a.compensar_temblor {
            let umbral = 0.15f32 as f64;
            let ax = mx.abs();
            if ax <= nx * 0.15 {
                mx = sx * (ax / umbral).powf(1.408) * 0.15;
            }
            let ay = my.abs();
            if ay <= ny * 0.15 {
                my = sy * (ay / umbral).powf(1.408) * 0.15;
            }
        }
        if (mx > 0.0 && self.resto_h > 0.0) || (mx < 0.0 && self.resto_h < 0.0) {
            mx += self.resto_h;
        }
        if (my > 0.0 && self.resto_v > 0.0) || (my < 0.0 && self.resto_v < 0.0) {
            my += self.resto_v;
        }
        mx = dos_decimales(mx);
        my = dos_decimales(my);
        let distancia = mx * mx + my * my;
        let mut x = mx as i32;
        let mut y = my as i32;
        if a.umbral == 1.0 || distancia >= a.umbral * a.umbral {
            self.resto_h = mx - x as f64;
            self.resto_v = my - y as f64;
        } else {
            self.resto_h = mx;
            x = 0;
            self.resto_v = my;
            y = 0;
        }
        if !sin_invertir {
            if (a.invertir & 2) == 2 {
                x = -x;
            }
            if (a.invertir & 1) == 1 {
                y = -y;
            }
        }
        if x != 0 || y != 0 {
            r.eventos.push(EventoRaton::Mover(x, y));
        }
    }

    /// `MouseWheel.touchesMoved`.
    fn rueda_mueven(&mut self, t: &Toques, arrastra: bool, r: &mut Contexto) {
        if t.dedos.len() != 2 || arrastra {
            return;
        }
        let (d0, d1) = (t.dedos[0], t.dedos[1]);
        let antes_x = (d0.antes.0 + d1.antes.0) as f64 / 2.0;
        let antes_y = (d0.antes.1 + d1.antes.1) as f64 / 2.0;
        let ahora_x = (d0.x + d1.x) as f64 / 2.0;
        let ahora_y = (d0.y + d1.y) as f64 / 2.0;
        let mut k = r.a.desplazamiento as f64 / 100.0;
        let (ex, ey) = ((d1.x - d0.x) as f64, (d1.y - d0.y) as f64);
        k *= (ex * ex + ey * ey).sqrt() / 960.0;
        let mut mx = k * (ahora_x - antes_x);
        if (mx > 0.0 && self.rueda_h > 0.0) || (mx < 0.0 && self.rueda_h < 0.0) {
            mx += self.rueda_h;
        }
        let mut x = mx as i32;
        self.rueda_h = mx - x as f64;
        let mut my = k * (antes_y - ahora_y);
        if (my > 0.0 && self.rueda_v > 0.0) || (my < 0.0 && self.rueda_v < 0.0) {
            my += self.rueda_v;
        }
        let mut y = my as i32;
        self.rueda_v = my - y as f64;
        if y != 0 || x != 0 {
            // C# multiplica los negativos por -1 y por la muesca hacia abajo (-120): da lo mismo.
            y *= 120;
            x *= 120;
            r.eventos.push(EventoRaton::Rueda(y, x));
        }
    }

    /// `MouseCursor.TouchesMovedAbsolute`.
    fn absoluto(&mut self, t: &Toques, r: &mut Contexto) {
        if t.dedos.len() != 1 {
            return;
        }
        let (mut x, mut y) = (t.dedos[0].x, t.dedos[0].y);
        let mitad_x = PANEL_ANCHO / 2;
        let mitad_y = PANEL_ALTO / 2;
        let zx = r.a.absoluto_x as f64 * 0.01 * mitad_x as f64;
        let zy = r.a.absoluto_y as f64 * 0.01 * mitad_y as f64;
        let min_x = (mitad_x as f64 - zx) as i32;
        let min_y = (mitad_y as f64 - zy) as i32;
        let max_x = (mitad_x as f64 + zx) as i32;
        let max_y = (mitad_y as f64 + zy) as i32;
        let mx = PANEL_ANCHO as f64 / (max_x - min_x) as f64;
        let bx = min_x as f64 * mx;
        let my = PANEL_ALTO as f64 / (max_y - min_y) as f64;
        let by = min_y as f64 * my;
        x = if x > max_x {
            max_x
        } else if x < min_x {
            min_x
        } else {
            x
        };
        // C# compara la x con el mínimo de la y.
        y = if y > max_y {
            max_y
        } else if x < min_y {
            min_y
        } else {
            y
        };
        let ax = ((x as f64 * mx - bx) / PANEL_ANCHO as f64).clamp(0.0, 1.0);
        let ay = ((y as f64 * my - by) / PANEL_ALTO as f64).clamp(0.0, 1.0);
        r.eventos.push(EventoRaton::Absoluto(ax, ay));
    }
}

/// Cómo comprueba las zonas cada modo de activación en este suceso.
enum Activacion {
    Clic,
    Toque,
    /// El modo toque sin dedos ni botón: suelta las zonas.
    Suelta,
    Soltar,
    Nada,
}

/// Lo que comparten todos los sucesos de un informe.
struct Contexto<'a> {
    e: &'a mut Estado,
    a: &'a AjustesPanel,
    activo: bool,
    clics: &'a mut Sintetico,
    sticks: &'a mut SticksAccion,
    eventos: &'a mut Vec<EventoRaton>,
}

impl Contexto<'_> {
    fn ahora(&self) -> i64 {
        self.clics.ahora_us
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use ds5_informes::Toque;

    /// Un estado con un dedo en (x, y), o sin dedos, y el botón del panel.
    fn estado(dedo: Option<(i16, i16)>, pulsado: bool) -> Estado {
        let mut e = Estado::default();
        e.toques[1].crudo = 0x80;
        match dedo {
            Some((x, y)) => {
                e.toques[0] = Toque {
                    activo: true,
                    id: 1,
                    x,
                    y,
                    crudo: 1,
                };
                e.botones |= boton::TOQUE1 | boton::UN_DEDO;
            }
            None => e.toques[0].crudo = 0x81,
        }
        if pulsado {
            e.botones |= boton::PANEL | boton::PANEL_SALIDA;
        }
        e
    }

    fn ajustes() -> AjustesPanel {
        AjustesPanel::desde_perfil(&Perfil::default())
    }

    struct Banco {
        panel: Panel,
        raton: RatonPanel,
        clics: Sintetico,
        eventos: Vec<EventoRaton>,
        a: AjustesPanel,
    }

    impl Banco {
        fn nuevo(a: AjustesPanel) -> Banco {
            Banco {
                panel: Panel::default(),
                raton: RatonPanel::nuevo(&a),
                clics: Sintetico::default(),
                eventos: Vec::new(),
                a,
            }
        }

        fn informe(&mut self, mut e: Estado, ms: i64) {
            let us = ms * 1000;
            self.clics.ahora_us = us;
            let s = self.panel.sucesos(&e, us);
            self.raton.procesar(
                &s,
                &mut e,
                &self.a,
                true,
                &mut self.clics,
                &mut SticksAccion::default(),
                &mut self.eventos,
            );
        }
    }

    #[test]
    fn un_toque_corto_es_un_clic() {
        let mut a = ajustes();
        a.toque = 20;
        let mut b = Banco::nuevo(a);
        b.informe(estado(Some((800, 400)), false), 0);
        b.informe(estado(None, false), 30);
        assert_eq!(b.clics.actual.izquierdo, 1);
    }

    #[test]
    fn el_doble_toque_arrastra() {
        let mut a = ajustes();
        a.toque = 100;
        a.doble_toque = true;
        let mut b = Banco::nuevo(a);
        b.informe(estado(Some((800, 400)), false), 0);
        b.informe(estado(None, false), 50);
        assert_eq!(
            b.clics.actual.izquierdo, 0,
            "el primer toque espera al segundo"
        );
        b.informe(estado(Some((800, 400)), false), 100);
        assert_eq!(b.clics.actual.izquierdo, 1);
        assert!(b.raton.arrastrando);
    }

    #[test]
    fn la_esquina_inferior_derecha_es_el_clic_derecho() {
        let mut a = ajustes();
        a.esquina_derecha = true;
        let mut b = Banco::nuevo(a);
        b.informe(estado(Some((1500, 730)), false), 0);
        b.informe(estado(Some((1500, 730)), true), 4);
        assert_eq!(b.clics.actual.derecho, 1);
        let mut b = Banco::nuevo(b.a.clone());
        b.informe(estado(Some((1500, 715)), false), 0);
        b.informe(estado(Some((1500, 715)), true), 4);
        assert_eq!(
            b.clics.actual.derecho, 0,
            "C# mide la esquina con 960 de alto"
        );
    }

    #[test]
    fn el_raton_absoluto_sube_la_y_con_el_dedo_a_la_izquierda() {
        let mut a = ajustes();
        a.modo = SalidaPanel::AbsoluteMouse;
        a.absoluto_y = 50;
        let y_de = |x: i16| {
            let mut b = Banco::nuevo(a.clone());
            b.informe(estado(Some((x, 400)), false), 0);
            b.informe(estado(Some((x, 402)), false), 4);
            match b.eventos.last() {
                Some(EventoRaton::Absoluto(_, y)) => *y,
                otro => panic!("sin movimiento absoluto: {otro:?}"),
            }
        };
        // C# compara la x con el mínimo de la y: con la x por debajo (236), la y se va arriba.
        assert_eq!(y_de(100), 0.0);
        let y = y_de(960);
        assert!(y > 0.3 && y < 0.5, "{y}");
    }
}
