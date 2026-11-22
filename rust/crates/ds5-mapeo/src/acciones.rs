//! Las acciones especiales (`Mapping.MapCustomAction` y `SpecialAction`): combinaciones de
//! controles que pulsan una tecla, lanzan una macro, encienden la luz de la batería o calibran el
//! volante. Lo que es de la app (cargar otro perfil, abrir un programa, desconectar el mando,
//! avisar de la batería o reiniciar la calibración del giroscopio) sale como una petición.
//!
//! El reloj de C# (`DateTime.UtcNow`) es aquí la hora del informe.

use ds5_informes::{Conexion, Estado};
use ds5_perfiles::mapeos::{Accion, AjusteControl};
use ds5_perfiles::otros::{AccionEspecial, Acciones, TipoAccion};

use crate::botones::control::*;
use crate::botones::{Campos, Entradas, LuzForzada, Tipo, TIPOS};
use crate::macros::{Macros, OpcionesMacro, REPETIR};
use crate::raton::{EventoRaton, Sintetico};
use crate::volante::Volante;

/// `DS5KeyType`.
const EXPLORACION: u8 = 1;
const ALTERNA: u8 = 2;

/// `DateTime.MinValue`, lejos de cualquier hora de informe.
const NUNCA: i64 = i64::MIN / 4;

/// Lo que la app tiene que hacer por una acción.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Peticion {
    /// Abrir un programa con sus argumentos. Si los argumentos llevan `$hidden`, C# lo abre con
    /// la ventana oculta (y los `.bat` y `.cmd` con `COMSPEC`).
    Programa {
        ruta: String,
        argumentos: Option<String>,
    },
    /// Cargar un perfil temporal (`LoadTempProfile`).
    CargarPerfil(String),
    /// Volver al perfil de antes: el normal del mando (`None`) o el temporal que había.
    VolverPerfil(Option<String>),
    /// Desconectar el mando por Bluetooth.
    DesconectarBt,
    /// Avisar en la bandeja del nivel de la batería.
    AvisoBateria,
    /// Empezar de nuevo la calibración continua del giroscopio.
    ReiniciarCalibracionGiro,
}

/// `getDS5ControlsByName` de `SpecialAction`: un nombre que no conoce da 0 (ningún control).
pub fn control_por_nombre(n: &str) -> usize {
    match n {
        "Share" => CREAR,
        "L3" => L3,
        "R3" => R3,
        "Options" => OPCIONES,
        "Up" => ARRIBA,
        "Right" => DERECHA,
        "Down" => ABAJO,
        "Left" => IZQUIERDA,
        "L1" => L1,
        "R1" => R1,
        "Triangle" => TRIANGULO,
        "Circle" => CIRCULO,
        "Cross" => EQUIS,
        "Square" => CUADRADO,
        "PS" => PS,
        "Mute" => MUTE,
        "Function Left" => FN_IZQ,
        "Function Right" => FN_DER,
        "Bottom Left Paddle" => PALETA_IZQ,
        "Bottom Right Paddle" => PALETA_DER,
        "Capture" => CAPTURA,
        "SideL" => LATERAL_IZQ,
        "SideR" => LATERAL_DER,
        "Left Stick Left" => LX_NEG,
        "Left Stick Up" => LY_NEG,
        "Right Stick Left" => RX_NEG,
        "Right Stick Up" => RY_NEG,
        "Left Stick Right" => LX_POS,
        "Left Stick Down" => LY_POS,
        "Right Stick Right" => RX_POS,
        "Right Stick Down" => RY_POS,
        "L2" => L2,
        "L2 Full Pull" => L2_A_FONDO,
        "R2" => R2,
        "R2 Full Pull" => R2_A_FONDO,
        "Left Touch" => PANEL_IZQUIERDA,
        "Multitouch" => PANEL_MULTI,
        "Upper Touch" => PANEL_ARRIBA,
        "Right Touch" => PANEL_DERECHA,
        "Swipe Up" => DESLIZAR_ARRIBA,
        "Swipe Down" => DESLIZAR_ABAJO,
        "Swipe Left" => DESLIZAR_IZQUIERDA,
        "Swipe Right" => DESLIZAR_DERECHA,
        "Tilt Up" => GIRO_Z_NEG,
        "Tilt Down" => GIRO_Z_POS,
        "Tilt Left" => GIRO_X_POS,
        "Tilt Right" => GIRO_X_NEG,
        "Touch Started" => TOQUE_EMPIEZA,
        "Touch Ended" => TOQUE_ACABA,
        _ => 0,
    }
}

fn controles(texto: &str) -> Vec<usize> {
    texto.split('/').map(control_por_nombre).collect()
}

/// `double.TryParse` con la cultura en-US: 0 si no es un número.
fn doble(texto: &str) -> f64 {
    texto.trim().parse().unwrap_or(0.0)
}

/// `bool.Parse`.
fn logico(texto: &str) -> Option<bool> {
    match texto.trim().to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// `applyRatio` de `Global`: mezcla dos bytes con un porcentaje.
fn mezcla(a: u8, b: u8, r: f64) -> u8 {
    let r = r.clamp(0.0, 100.0) * 0.01;
    (a as f64 * (1.0 - r) + b as f64 * r).round_ties_even() as u8
}

/// `getTransitionedColor`.
pub fn transicion(a: (u8, u8, u8), b: (u8, u8, u8), r: f64) -> (u8, u8, u8) {
    (
        mezcla(a.0, b.0, r),
        mezcla(a.1, b.1, r),
        mezcla(a.2, b.2, r),
    )
}

/// Una acción especial como la arma C# (`SpecialAction`), con lo que recuerda entre informes.
#[derive(Debug, Clone)]
pub struct Especial {
    pub nombre: String,
    pub tipo: TipoAccion,
    /// `controls`: el texto de los controles que la activan.
    controles: String,
    disparo: Vec<usize>,
    /// `ucontrols`: el texto de los controles que la deshacen, o `None` si no tiene.
    controles_deshacer: Option<String>,
    deshacer: Vec<usize>,
    retardo: f64,
    detalles: String,
    extra: Option<String>,
    /// `pressRelease`.
    al_soltar: bool,
    tipo_tecla: u8,
    pasos: Vec<i32>,
    automatico: bool,
    sincronizada: bool,
    mantener_teclas: bool,
    una_vez: bool,
    primer_toque: bool,
    segundo_toque: bool,
    pasado_us: i64,
    primer_tap_us: i64,
    fin_us: i64,
    perfil_anterior: String,
}

/// Lo que la interfaz enseña de una acción (`ActionsPanel.Describe`).
#[derive(Debug, Clone, PartialEq)]
pub struct Descripcion {
    pub controles: String,
    pub detalles: String,
    pub extra: String,
    pub retardo: f64,
    pub pasos: Vec<i32>,
    pub repetir: bool,
}

impl Especial {
    pub fn descripcion(&self) -> Descripcion {
        Descripcion {
            controles: self.controles.clone(),
            detalles: self.detalles.clone(),
            extra: self.extra.clone().unwrap_or_default(),
            retardo: self.retardo,
            pasos: self.pasos.clone(),
            repetir: self.tipo_tecla & crate::macros::REPETIR != 0,
        }
    }

    /// `ActionsDTO.MapTo` y el constructor de `SpecialAction`.
    pub fn desde(a: &AccionEspecial) -> Especial {
        let crudo = a.detalles.clone().unwrap_or_default();
        let (detalles, retardo, extras) = match a.tipo {
            TipoAccion::Macro => (crudo.clone(), 0.0, a.extras.clone()),
            TipoAccion::Programa => (crudo.clone(), a.retardo, a.argumentos.clone()),
            TipoAccion::Perfil => (crudo.clone(), 0.0, a.deshacer.clone()),
            TipoAccion::Tecla => {
                let extras = if !a.estilo_deshacer.is_empty() || !a.deshacer.is_empty() {
                    format!("{}\n{}", a.estilo_deshacer, a.deshacer)
                } else {
                    String::new()
                };
                (
                    crudo.split(' ').next().unwrap_or("").to_string(),
                    0.0,
                    extras,
                )
            }
            TipoAccion::DesconectarBt => (crudo.clone(), doble(&crudo), String::new()),
            // El lector deja los colores separados por `|`; C# los guarda con comas.
            TipoAccion::Bateria => (
                crudo.replace('|', ","),
                doble(crudo.split('|').next().unwrap_or("")),
                String::new(),
            ),
            TipoAccion::Multiple => (crudo.clone(), a.retardo, a.extras.clone()),
            TipoAccion::CalibrarVolante => (String::new(), a.retardo, String::new()),
            TipoAccion::CalibrarGiroscopio => (crudo.clone(), doble(&crudo), String::new()),
        };
        let mut e = Especial {
            nombre: a.nombre.clone(),
            tipo: a.tipo,
            controles: a.disparador.clone(),
            disparo: controles(&a.disparador),
            controles_deshacer: None,
            deshacer: Vec::new(),
            retardo,
            detalles,
            extra: None,
            al_soltar: false,
            tipo_tecla: 0,
            pasos: Vec::new(),
            automatico: false,
            sincronizada: false,
            mantener_teclas: false,
            una_vez: false,
            primer_toque: false,
            segundo_toque: false,
            pasado_us: NUNCA,
            primer_tap_us: NUNCA,
            fin_us: NUNCA,
            perfil_anterior: String::new(),
        };
        match a.tipo {
            TipoAccion::Tecla => {
                if !extras.is_empty() {
                    e.extra = Some(extras.clone());
                    let partes: Vec<&str> = extras.split('\n').collect();
                    e.al_soltar = partes[0] == "Release";
                    if partes[0] == "Press" || partes[0] == "Release" {
                        e.tipo_tecla |= ALTERNA;
                    }
                    if let Some(u) = partes.get(1).filter(|u| !u.is_empty()) {
                        e.controles_deshacer = Some(u.to_string());
                        e.deshacer = controles(u);
                    }
                }
                if crudo.contains("Scan Code") {
                    e.tipo_tecla |= EXPLORACION;
                }
            }
            TipoAccion::Macro => {
                e.pasos = e
                    .detalles
                    .split('/')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if !extras.is_empty() {
                    e.extra = Some(extras.clone());
                    if extras.contains("Scan Code") {
                        e.tipo_tecla |= EXPLORACION;
                    }
                    e.al_soltar = extras.contains("RunOnRelease");
                    e.sincronizada = extras.contains("Sync");
                    e.mantener_teclas = extras.contains("KeepKeyState");
                    if extras.contains("Repeat") {
                        e.tipo_tecla |= REPETIR;
                    }
                }
            }
            TipoAccion::Programa | TipoAccion::Perfil if !extras.is_empty() => {
                e.extra = Some(extras.clone());
            }
            _ => {}
        }
        // Todo lo que no es una tecla toma sus extras como controles para deshacer: también los
        // argumentos del programa o las opciones de la macro, que no casan con ningún control.
        if a.tipo != TipoAccion::Tecla && !extras.is_empty() {
            e.controles_deshacer = Some(extras.clone());
            for s in extras.split('/') {
                if s == "AutomaticUntrigger" {
                    e.automatico = true;
                } else {
                    e.deshacer.push(control_por_nombre(s));
                }
            }
        }
        e
    }
}

/// Cómo acaba el recorrido de las acciones antes de tiempo.
enum Corte {
    /// C# se para con una excepción (`catch { return; }`).
    Excepcion,
    /// Un `return` después de cargar un perfil o desconectar el mando.
    Vuelta,
}

/// Lo que las acciones leen y tocan del resto de la reasignación.
pub(crate) struct Entorno<'a> {
    /// Los controles del estado ya pasado por las curvas (`fieldMapping`).
    pub entrada: &'a Campos,
    /// Lo que va a salir (`outputfieldMapping`): los controles que activan una acción se
    /// apagan.
    pub salida: &'a mut Campos,
    /// El estado ya pasado por las curvas.
    pub e: &'a Estado,
    /// El estado del mando sin curvas: de ahí salen la batería y si está cargando
    /// (`d.Battery`, `isCharging`).
    pub crudo: &'a Estado,
    /// El estado anterior del mando sin curvas, para la acción múltiple.
    pub crudo_antes: &'a Estado,
    pub controles: &'a [AjusteControl],
    /// Las acciones del perfil, por su nombre.
    pub perfil: &'a [String],
    /// El giroscopio sale como controles (`IsUsingSAForControls`).
    pub giro_controles: bool,
    pub antirrebote_ms: i64,
    pub sintetico: &'a mut Sintetico,
    pub eventos: &'a mut Vec<EventoRaton>,
    pub macros: &'a mut Macros,
    pub luz: &'a mut LuzForzada,
    pub volante: &'a mut Volante,
    pub ahora_us: i64,
}

/// `getBoolSpecialActionMapping`: si un control cuenta como pulsado para las acciones.
fn activo(c: usize, campos: &Campos, e: &Estado, giro_controles: bool) -> bool {
    match TIPOS[c] {
        Tipo::Boton | Tipo::Toque => campos.botones[c],
        Tipo::Eje => match c {
            LX_NEG => e.lx < 128 - 55,
            LY_NEG => e.ly < 128 - 55,
            RX_NEG => e.rx < 128 - 55,
            RY_NEG => e.ry < 128 - 55,
            _ => campos.ejes[c] > 128 + 55,
        },
        Tipo::Gatillo => campos.gatillos[c] > 100,
        Tipo::Deslizar => campos.deslizar_activo[c],
        Tipo::Giro => {
            giro_controles
                && match c {
                    GIRO_X_POS | GIRO_Z_POS => campos.giro[c] > 67,
                    GIRO_X_NEG | GIRO_Z_NEG => campos.giro[c] < -67,
                    _ => false,
                }
        }
        Tipo::Desconocido => false,
    }
}

/// `ResetToDefaultValue`: el control que activa una acción no sale.
fn apagar(c: usize, s: &mut Campos) {
    match TIPOS[c] {
        Tipo::Boton | Tipo::Toque => s.botones[c] = false,
        Tipo::Eje => {
            s.ejes[c] = 128;
            let pareja = if c.is_multiple_of(2) { c - 1 } else { c + 1 };
            s.ejes[pareja] = 128;
        }
        Tipo::Gatillo => s.gatillos[c] = 0,
        _ => {}
    }
}

/// Suelta las teclas que tienen asignadas los controles (`ReleaseActionKeys`). Un control que no
/// existe es una excepción en C#.
fn soltar_teclas(
    lista: &[usize],
    controles: &[AjusteControl],
    eventos: &mut Vec<EventoRaton>,
) -> Result<(), Corte> {
    for &c in lista {
        let ajuste = c
            .checked_sub(1)
            .and_then(|i| controles.get(i))
            .ok_or(Corte::Excepcion)?;
        match &ajuste.accion {
            Accion::Tecla(k) => eventos.push(EventoRaton::SueltaTecla(*k as u32)),
            Accion::Macro(m) => {
                for &k in m {
                    eventos.push(EventoRaton::SueltaTecla(k as u32));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// La macro de cada tipo de `XboxGameDVR`, que C# guarda ya como acción múltiple.
fn pasos_de(texto: &str) -> Option<Vec<i32>> {
    texto.split('/').map(|s| s.trim().parse().ok()).collect()
}

/// Las acciones especiales de un mando: la lista de `Actions.xml` y lo que C# guarda en
/// `Mapping` entre informes.
#[derive(Debug, Clone)]
pub struct AccionesEspeciales {
    /// Todas las acciones (`GetActions`), en su orden.
    pub todas: Vec<Especial>,
    /// `actionDone`: crece de una en una, como en C#.
    hechas: Vec<bool>,
    /// `untriggerindex`.
    indice_deshacer: i64,
    /// `untriggeraction`.
    accion_deshacer: Option<usize>,
    /// El perfil temporal cargado por una acción (`useTempProfile` y `tempprofilename`).
    pub temporal: Option<String>,
    /// `nowAction` y `oldnowAction`, para las acciones con retardo.
    ahora_accion_us: i64,
    antes_accion_us: i64,
    /// `fadetimer` y `lastColor`, para la luz de la batería.
    desvanecer: i32,
    color_antes: (u8, u8, u8),
    /// El color que tiene la luz del mando (`LightBarColor`). Lo pone quien maneja la luz.
    pub color_luz: (u8, u8, u8),
    pub conexion: Conexion,
    /// Los controles de la acción que acaba de cargar un perfil sin forma de deshacerlo: las
    /// acciones del perfil nuevo con los mismos controles esperan a que se suelten.
    marcar: Option<String>,
    /// Lo que han pedido las acciones en el último informe.
    pub peticiones: Vec<Peticion>,
}

impl Default for AccionesEspeciales {
    fn default() -> Self {
        AccionesEspeciales::nuevas(&Acciones::default())
    }
}

impl AccionesEspeciales {
    pub fn nuevas(acciones: &Acciones) -> AccionesEspeciales {
        AccionesEspeciales {
            todas: acciones.acciones.iter().map(Especial::desde).collect(),
            hechas: Vec::new(),
            indice_deshacer: -1,
            accion_deshacer: None,
            temporal: None,
            ahora_accion_us: NUNCA,
            antes_accion_us: NUNCA,
            desvanecer: 0,
            color_antes: (0, 0, 0),
            color_luz: (0, 0, 0),
            conexion: Conexion::Bt,
            marcar: None,
            peticiones: Vec::new(),
        }
    }

    fn hecha(&self, i: usize) -> Result<bool, Corte> {
        self.hechas.get(i).copied().ok_or(Corte::Excepcion)
    }

    fn poner(&mut self, i: usize, v: bool) -> Result<(), Corte> {
        let h = self.hechas.get_mut(i).ok_or(Corte::Excepcion)?;
        *h = v;
        Ok(())
    }

    /// `MapCustomAction`, una vez por informe.
    pub(crate) fn procesar(&mut self, en: &mut Entorno) {
        self.peticiones.clear();
        for i in en.macros.acciones_acabadas.drain(..) {
            if let Some(h) = self.hechas.get_mut(i) {
                *h = false;
            }
        }
        if self.recorrer(en).is_ok() {
            self.deshacer_perfil(en);
        }
    }

    /// El perfil temporal que pidió una acción ya está cargado, con estas acciones: las que
    /// comparten los controles de la que lo cargó no se lanzan hasta soltarlos.
    pub fn perfil_cargado(&mut self, perfil: &[String]) {
        let Some(controles) = self.marcar.take() else {
            return;
        };
        for nombre in perfil {
            if let Some(i) = self.todas.iter().position(|a| a.nombre == *nombre) {
                if self.todas[i].controles == controles {
                    if let Some(h) = self.hechas.get_mut(i) {
                        *h = true;
                    }
                }
            }
        }
    }

    fn recorrer(&mut self, en: &mut Entorno) -> Result<(), Corte> {
        let total = self.todas.len();
        let mut campos_antes: Option<Campos> = None;
        for nombre in en.perfil {
            let posicion = self.todas.iter().position(|a| a.nombre == *nombre);
            let indice = posicion.map_or(-1, |i| i as i64);
            let cuenta = self.hechas.len();
            if (cuenta as i64) < indice + 1 {
                self.hechas.push(false);
            } else if cuenta > total {
                self.hechas.pop();
            }
            let Some(i) = posicion else {
                continue;
            };
            self.accion(i, en, &mut campos_antes)?;
        }
        Ok(())
    }

    fn accion(
        &mut self,
        i: usize,
        en: &mut Entorno,
        campos_antes: &mut Option<Campos>,
    ) -> Result<(), Corte> {
        let ahora = en.ahora_us;
        let pulsado = |c: usize| activo(c, en.entrada, en.e, en.giro_controles);
        let a = &self.todas[i];
        let tipo = a.tipo;

        let activada = if a.retardo > 0.0 {
            let mut v = false;
            if a.disparo.iter().all(|&c| pulsado(c)) {
                self.ahora_accion_us = ahora;
                v = ahora >= self.antes_accion_us + (a.retardo * 1_000_000.0) as i64;
            } else if self.ahora_accion_us < ahora - 100_000 + en.antirrebote_ms * 1000 {
                self.antes_accion_us = ahora;
            }
            v
        } else {
            let v = a.disparo.iter().all(|&c| pulsado(c));
            // La macro que va al soltar cuenta al revés mientras espera.
            v != (tipo == TipoAccion::Macro && a.al_soltar && a.primer_toque)
        };

        let mut deshacer_activo = true;
        if tipo == TipoAccion::Tecla && !a.deshacer.is_empty() {
            deshacer_activo = a.deshacer.iter().all(|&c| pulsado(c));
            if a.al_soltar {
                deshacer_activo = !deshacer_activo;
            }
        }

        let mut hallada = false;
        if activada {
            for &c in &self.todas[i].disparo {
                apagar(c, en.salida);
            }
            match tipo {
                TipoAccion::Programa => {
                    hallada = true;
                    if !self.hecha(i)? {
                        self.poner(i, true)?;
                        let a = &self.todas[i];
                        self.peticiones.push(Peticion::Programa {
                            ruta: a.detalles.clone(),
                            argumentos: a.extra.clone(),
                        });
                    }
                }
                TipoAccion::Perfil => {
                    hallada = true;
                    let otra_deshace_perfil = self.temporal.is_some()
                        && self
                            .accion_deshacer
                            .is_some_and(|d| self.todas[d].tipo == TipoAccion::Perfil);
                    if !self.hecha(i)? && !otra_deshace_perfil {
                        self.poner(i, true)?;
                        let anterior = self.temporal.clone().unwrap_or_default();
                        let a = &mut self.todas[i];
                        if !a.deshacer.is_empty() || a.automatico {
                            a.perfil_anterior = anterior;
                            self.accion_deshacer = Some(i);
                            self.indice_deshacer = i as i64;
                        }
                        let a = &self.todas[i];
                        soltar_teclas(&a.disparo, en.controles, en.eventos)?;
                        let (detalles, marcar) = (
                            a.detalles.clone(),
                            (a.deshacer.is_empty() && !a.automatico).then(|| a.controles.clone()),
                        );
                        self.marcar = marcar;
                        self.temporal = Some(detalles.clone());
                        self.peticiones.push(Peticion::CargarPerfil(detalles));
                        return Err(Corte::Vuelta);
                    }
                }
                TipoAccion::Macro => {
                    hallada = true;
                    let a = &self.todas[i];
                    let (pasos, tipo_tecla) = (a.pasos.clone(), a.tipo_tecla);
                    let opciones = OpcionesMacro {
                        mantener_teclas: a.mantener_teclas,
                        accion: None,
                        cola: a.sincronizada.then(|| a.controles.clone()),
                    };
                    if !a.al_soltar {
                        if !self.hecha(i)? {
                            self.poner(i, true)?;
                            let opciones = OpcionesMacro {
                                accion: Some(i),
                                ..opciones
                            };
                            en.macros
                                .lanzar(&pasos, tipo_tecla, opciones, ahora, en.luz, en.eventos);
                        } else if tipo_tecla & REPETIR == 0 {
                            en.macros.terminar(&pasos, ahora, en.eventos);
                        }
                    } else if a.primer_toque {
                        self.todas[i].primer_toque = false;
                        if !self.hecha(i)? {
                            self.poner(i, true)?;
                            en.macros
                                .lanzar(&pasos, tipo_tecla, opciones, ahora, en.luz, en.eventos);
                        }
                    } else {
                        self.todas[i].primer_toque = true;
                    }
                }
                TipoAccion::Tecla => {
                    hallada = true;
                    let a = &self.todas[i];
                    let sin_deshacer = a.deshacer.is_empty();
                    if sin_deshacer || (self.indice_deshacer == -1 && !self.hecha(i)?) {
                        self.poner(i, true)?;
                        self.indice_deshacer = i as i64;
                        let a = &self.todas[i];
                        // `ushort.TryParse`: 0 si no es un número.
                        let tecla: u16 = a.detalles.trim().parse().unwrap_or(0);
                        let exploracion = a.tipo_tecla & EXPLORACION != 0;
                        if sin_deshacer {
                            let p = en.sintetico.tecla(tecla);
                            if exploracion {
                                p.exploracion += 1;
                            } else {
                                p.normal += 1;
                            }
                            p.repeticion += 1;
                        } else if exploracion {
                            en.eventos.push(EventoRaton::TeclaAlt(tecla as u32));
                        } else {
                            en.eventos.push(EventoRaton::Tecla(tecla as u32));
                        }
                    }
                }
                TipoAccion::DesconectarBt => {
                    hallada = true;
                    // `isSynced` y que no esté cargando; por cable no hace nada.
                    if !en.crudo.cargando && self.conexion == Conexion::Bt {
                        self.peticiones.push(Peticion::DesconectarBt);
                        soltar_teclas(&self.todas[i].disparo, en.controles, en.eventos)?;
                        return Err(Corte::Vuelta);
                    }
                }
                TipoAccion::Bateria => {
                    hallada = true;
                    let detalles = self.todas[i].detalles.clone();
                    let mut d: Vec<&str> = detalles.split('|').collect();
                    if d.len() == 1 {
                        d = detalles.split(',').collect();
                    }
                    let parte = |n: usize| d.get(n).copied().ok_or(Corte::Excepcion);
                    let aviso = logico(parte(1)?).ok_or(Corte::Excepcion)?;
                    if aviso && !self.hecha(i)? {
                        self.peticiones.push(Peticion::AvisoBateria);
                    }
                    if logico(parte(2)?).ok_or(Corte::Excepcion)? {
                        if !self.hecha(i)? {
                            self.color_antes = self.color_luz;
                            en.luz.activa = true;
                        }
                        let byte = |n: usize| -> Result<u8, Corte> {
                            parte(n)?.trim().parse().map_err(|_| Corte::Excepcion)
                        };
                        let vacia = (byte(3)?, byte(4)?, byte(5)?);
                        let llena = (byte(6)?, byte(7)?, byte(8)?);
                        let color = transicion(vacia, llena, en.crudo.bateria as f64);
                        if self.desvanecer < 100 {
                            self.desvanecer += 2;
                            en.luz.color =
                                transicion(self.color_antes, color, self.desvanecer as f64);
                        }
                    }
                    self.poner(i, true)?;
                }
                TipoAccion::CalibrarVolante => {
                    hallada = true;
                    let a = &mut self.todas[i];
                    let pasado = ahora > a.primer_tap_us.saturating_add(3_000_000);
                    if en.volante.calibrando == 0 && pasado {
                        a.primer_tap_us = ahora;
                        en.volante.calibrando = 1;
                    } else if en.volante.calibrando == 2 && pasado {
                        a.primer_tap_us = ahora;
                        en.volante.calibrando = 3;
                    }
                    self.poner(i, true)?;
                }
                TipoAccion::CalibrarGiroscopio => {
                    hallada = true;
                    if !self.hecha(i)? {
                        self.peticiones.push(Peticion::ReiniciarCalibracionGiro);
                        self.poner(i, true)?;
                    }
                }
                TipoAccion::Multiple => {}
            }
        } else {
            match tipo {
                TipoAccion::Bateria => {
                    hallada = true;
                    if self.hecha(i)? {
                        self.desvanecer = 0;
                        en.luz.activa = false;
                        self.poner(i, false)?;
                    }
                }
                TipoAccion::Tecla | TipoAccion::Multiple => {}
                _ => {
                    hallada = true;
                    self.poner(i, false)?;
                }
            }
        }

        if hallada {
            return Ok(());
        }
        let a = &self.todas[i];
        if !a.deshacer.is_empty() && deshacer_activo && tipo == TipoAccion::Tecla {
            if self.indice_deshacer > -1 && !self.hecha(i)? {
                self.poner(i, true)?;
                self.indice_deshacer = -1;
                let a = &self.todas[i];
                let tecla: u16 = a.detalles.trim().parse().unwrap_or(0);
                en.eventos.push(if a.tipo_tecla & EXPLORACION != 0 {
                    EventoRaton::SueltaTeclaAlt(tecla as u32)
                } else {
                    EventoRaton::SueltaTecla(tecla as u32)
                });
            }
        } else if tipo == TipoAccion::Multiple {
            self.multiple(i, en, campos_antes)?;
        } else {
            self.poner(i, false)?;
        }
        Ok(())
    }

    /// La acción múltiple: un toque, mantener o dos toques, cada uno con su macro.
    fn multiple(
        &mut self,
        i: usize,
        en: &mut Entorno,
        campos_antes: &mut Option<Campos>,
    ) -> Result<(), Corte> {
        let ahora = en.ahora_us;
        let rebote = en.antirrebote_ms * 1000;
        let antes =
            campos_antes.get_or_insert_with(|| Campos::desde(en.crudo_antes, &Entradas::default()));
        let a = &mut self.todas[i];
        let d: Vec<String> = a.detalles.split(',').map(str::to_string).collect();
        let c = *a.disparo.first().ok_or(Corte::Excepcion)?;
        let ahora_activo = activo(c, en.entrada, en.e, en.giro_controles);
        let antes_activo = activo(c, antes, en.crudo_antes, en.giro_controles);
        if ahora_activo && !antes_activo {
            a.pasado_us = ahora;
            if a.pasado_us <= a.primer_tap_us.saturating_add(150_000 + rebote) {
                a.una_vez = false;
                a.segundo_toque = true;
            } else {
                a.primer_toque = true;
            }
        } else if !ahora_activo && antes_activo {
            if a.segundo_toque {
                a.primer_toque = false;
                a.segundo_toque = false;
            } else if a.primer_toque {
                a.primer_toque = false;
                if ahora <= a.pasado_us.saturating_add(150_000 + rebote) && !a.una_vez {
                    a.una_vez = true;
                    a.primer_tap_us = ahora;
                    a.fin_us = ahora;
                }
            }
        }

        let parte = |n: usize| d.get(n).cloned().ok_or(Corte::Excepcion);
        let mut lanzar = None;
        if a.una_vez {
            let m = parte(0)?;
            if ahora - a.fin_us > 150_000 + rebote {
                lanzar = Some(m);
                a.una_vez = false;
            }
        } else if a.primer_toque && ahora - a.pasado_us > 500_000 + rebote {
            lanzar = Some(parte(1)?);
            a.primer_toque = false;
        } else if a.segundo_toque {
            lanzar = Some(parte(2)?);
            a.segundo_toque = false;
        }
        // Una macro que no se puede leer falla en su hilo, sin hacer nada.
        if let Some(pasos) = lanzar.filter(|m| !m.is_empty()).and_then(|m| pasos_de(&m)) {
            en.macros.lanzar(
                &pasos,
                0,
                OpcionesMacro::default(),
                ahora,
                en.luz,
                en.eventos,
            );
        }
        Ok(())
    }

    /// El final de `MapCustomAction`: deshacer el perfil temporal.
    fn deshacer_perfil(&mut self, en: &mut Entorno) {
        let Some(d) = self.accion_deshacer else {
            return;
        };
        let pulsado = |c: usize| activo(c, en.entrada, en.e, en.giro_controles);
        let a = &self.todas[d];
        let deshacer = if !a.automatico {
            a.deshacer.iter().all(|&c| pulsado(c))
        } else {
            a.disparo.iter().any(|&c| !pulsado(c))
        };
        // En C# un índice que no vale aquí es una excepción fuera del `try`: Rust la ignora.
        let Ok(indice) = usize::try_from(self.indice_deshacer) else {
            return;
        };
        if deshacer && a.tipo == TipoAccion::Perfil {
            let mismos = a.controles_deshacer.as_deref() == Some(a.controles.as_str());
            let libre = !mismos || !self.hechas.get(indice).copied().unwrap_or(true);
            if libre && self.temporal.is_some() {
                let lista = a.deshacer.clone();
                let anterior = a.perfil_anterior.clone();
                if !lista.is_empty() {
                    if let Some(h) = self.hechas.get_mut(indice) {
                        *h = true;
                    }
                }
                // Un control que no existe es una excepción fuera del `try` en C#.
                let _ = soltar_teclas(&lista, en.controles, en.eventos);
                self.accion_deshacer = None;
                let vuelta = (!anterior.is_empty()).then_some(anterior);
                self.temporal = vuelta.clone();
                self.peticiones.push(Peticion::VolverPerfil(vuelta));
            }
        } else if let Some(h) = self.hechas.get_mut(indice) {
            *h = false;
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use ds5_informes::boton;

    fn accion(nombre: &str, disparo: &str, tipo: TipoAccion, detalles: &str) -> AccionEspecial {
        AccionEspecial {
            nombre: nombre.into(),
            disparador: disparo.into(),
            tipo,
            detalles: Some(detalles.into()),
            extras: String::new(),
            argumentos: String::new(),
            retardo: 0.0,
            deshacer: String::new(),
            estilo_deshacer: String::new(),
        }
    }

    struct Banco {
        acciones: AccionesEspeciales,
        perfil: Vec<String>,
        sintetico: Sintetico,
        macros: Macros,
        luz: LuzForzada,
        volante: Volante,
        antes: Estado,
    }

    impl Banco {
        fn nuevo(lista: Vec<AccionEspecial>) -> Banco {
            let perfil = lista.iter().map(|a| a.nombre.clone()).collect();
            Banco {
                acciones: AccionesEspeciales::nuevas(&Acciones { acciones: lista }),
                perfil,
                sintetico: Sintetico::default(),
                macros: Macros::default(),
                luz: LuzForzada::default(),
                volante: Volante::nuevo(None),
                antes: Estado::default(),
            }
        }

        /// Un informe con estos botones a la hora dada; devuelve los eventos y lo que sale.
        fn paso(&mut self, botones: u64, ms: i64) -> (Vec<EventoRaton>, Campos) {
            let e = Estado {
                botones,
                lx: 128,
                ly: 128,
                rx: 128,
                ry: 128,
                bateria: 50,
                ..Estado::default()
            };
            let entrada = Campos::desde(&e, &Entradas::default());
            let mut salida = entrada.clone();
            let mut eventos = Vec::new();
            let controles = vec![AjusteControl::default(); 55];
            let ahora_us = ms * 1000;
            self.macros.avanzar(ahora_us, &mut self.luz, &mut eventos);
            let mut en = Entorno {
                entrada: &entrada,
                salida: &mut salida,
                e: &e,
                crudo: &e,
                crudo_antes: &self.antes,
                controles: &controles,
                perfil: &self.perfil,
                giro_controles: false,
                antirrebote_ms: 0,
                sintetico: &mut self.sintetico,
                eventos: &mut eventos,
                macros: &mut self.macros,
                luz: &mut self.luz,
                volante: &mut self.volante,
                ahora_us,
            };
            self.acciones.procesar(&mut en);
            self.sintetico.ahora_us = ahora_us;
            self.sintetico.confirmar(&mut eventos);
            self.antes = e;
            (eventos, salida)
        }
    }

    /// `actionDone` crece de uno en uno: una acción que no es la primera de la lista no
    /// funciona hasta que crece lo bastante.
    #[test]
    fn la_lista_de_hechas_crece_de_una_en_una() {
        let mut lista = vec![
            accion("a", "Cross", TipoAccion::Tecla, "65"),
            accion("b", "Circle", TipoAccion::Tecla, "66"),
            accion("c", "Square", TipoAccion::Tecla, "67"),
        ];
        let mut b = Banco::nuevo(lista.clone());
        // Solo la tercera está en el perfil: necesita tres informes.
        b.perfil = vec!["c".into()];
        let mut pulsada = None;
        for n in 0..5 {
            let (e, _) = b.paso(boton::CUADRADO, n * 10);
            if pulsada.is_none() && e.contains(&EventoRaton::Tecla(67)) {
                pulsada = Some(n);
            }
        }
        assert_eq!(pulsada, Some(2));
        lista.truncate(1);
    }

    /// La tecla sin control para deshacer va por el teclado sintético; el control que la activa
    /// deja de salir.
    #[test]
    fn la_tecla_se_mantiene_mientras_se_pulsa() {
        let mut b = Banco::nuevo(vec![accion("a", "Cross/Circle", TipoAccion::Tecla, "65")]);
        let (e, s) = b.paso(boton::EQUIS | boton::CIRCULO, 0);
        assert_eq!(e, vec![EventoRaton::Tecla(65)]);
        assert!(!s.botones[EQUIS] && !s.botones[CIRCULO]);
        let (e, _) = b.paso(boton::EQUIS | boton::CIRCULO, 10);
        assert!(e.is_empty());
        let (e, s) = b.paso(boton::EQUIS, 20);
        assert_eq!(e, vec![EventoRaton::SueltaTecla(65)]);
        assert!(s.botones[EQUIS]);
    }

    /// Con un control para deshacer, la tecla se pulsa con los de activar y se suelta con los
    /// de deshacer.
    #[test]
    fn la_tecla_con_deshacer_espera_a_su_control() {
        let mut a = accion("a", "Cross", TipoAccion::Tecla, "65 Scan Code");
        a.estilo_deshacer = "Press".into();
        a.deshacer = "Circle".into();
        let mut b = Banco::nuevo(vec![a]);
        assert_eq!(b.paso(boton::EQUIS, 0).0, vec![EventoRaton::TeclaAlt(65)]);
        assert!(b.paso(0, 10).0.is_empty());
        assert!(b.paso(0, 20).0.is_empty());
        assert_eq!(
            b.paso(boton::CIRCULO, 30).0,
            vec![EventoRaton::SueltaTeclaAlt(65)]
        );
        assert!(b.paso(boton::CIRCULO, 40).0.is_empty());
    }

    /// Apagar un eje que activa una acción apaga también su otra mitad (`ResetToDefaultValue`).
    #[test]
    fn apagar_un_eje_apaga_su_pareja() {
        let e = Estado {
            lx: 0,
            ..Estado::default()
        };
        let mut s = Campos::desde(&e, &Entradas::default());
        apagar(LX_NEG, &mut s);
        assert_eq!((s.ejes[LX_NEG], s.ejes[LX_POS]), (128, 128));
        let mut s = Campos::desde(&e, &Entradas::default());
        apagar(RY_POS, &mut s);
        assert_eq!((s.ejes[RY_NEG], s.ejes[RY_POS]), (128, 128));
    }

    /// La tecla de la acción mantenida se repite como las demás (a los 500 ms y luego cada 25).
    #[test]
    fn la_tecla_de_la_accion_se_repite() {
        let mut b = Banco::nuevo(vec![accion("a", "Cross", TipoAccion::Tecla, "65")]);
        b.sintetico.repeticion_falsa = true;
        let mut veces = 0;
        for ms in (0..=700).step_by(10) {
            veces += b
                .paso(boton::EQUIS, ms)
                .0
                .iter()
                .filter(|e| **e == EventoRaton::Tecla(65))
                .count();
        }
        assert!(veces > 1, "{veces}");
    }

    /// La luz de la batería va del color que tenía al de la batería, de 2 en 2 por ciento por
    /// informe, y se apaga al soltar.
    #[test]
    fn la_luz_de_la_bateria_se_funde() {
        let mut b = Banco::nuevo(vec![accion(
            "a",
            "PS",
            TipoAccion::Bateria,
            "0|false|true|255|0|0|0|255|0",
        )]);
        b.acciones.color_luz = (0, 0, 200);
        b.paso(boton::PS, 0);
        assert!(b.luz.activa);
        assert_eq!(b.luz.color, transicion((0, 0, 200), (128, 128, 0), 2.0));
        for n in 1..60 {
            b.paso(boton::PS, n * 10);
        }
        assert_eq!(b.luz.color, (128, 128, 0));
        b.paso(0, 1000);
        assert!(!b.luz.activa);
    }

    /// El perfil temporal se carga al pulsar y se deshace al soltar con `AutomaticUntrigger`.
    #[test]
    fn el_perfil_temporal_vuelve_al_soltar() {
        let mut a = accion("a", "L1/R1", TipoAccion::Perfil, "Carreras");
        a.deshacer = "AutomaticUntrigger".into();
        let mut b = Banco::nuevo(vec![a]);
        b.paso(boton::L1 | boton::R1, 0);
        assert_eq!(
            b.acciones.peticiones,
            vec![Peticion::CargarPerfil("Carreras".into())]
        );
        b.paso(boton::L1 | boton::R1, 10);
        assert!(b.acciones.peticiones.is_empty());
        b.paso(boton::L1, 20);
        assert_eq!(b.acciones.peticiones, vec![Peticion::VolverPerfil(None)]);
        assert_eq!(b.acciones.temporal, None);
    }

    /// La macro de la acción suena una vez por pulsación; con `Repeat` vuelve a sonar mientras
    /// se mantiene.
    #[test]
    fn la_macro_de_la_accion_se_repite_si_lo_pide() {
        let mut a = accion("a", "Cross", TipoAccion::Macro, "65/65");
        let mut b = Banco::nuevo(vec![a.clone()]);
        let mut veces = 0;
        for n in 0..5 {
            veces += b
                .paso(boton::EQUIS, n * 10)
                .0
                .iter()
                .filter(|e| **e == EventoRaton::Tecla(65))
                .count();
        }
        assert_eq!(veces, 1);
        a.extras = "Repeat".into();
        let mut b = Banco::nuevo(vec![a]);
        let mut veces = 0;
        for n in 0..5 {
            veces += b
                .paso(boton::EQUIS, n * 10)
                .0
                .iter()
                .filter(|e| **e == EventoRaton::Tecla(65))
                .count();
        }
        assert!(veces >= 2, "{veces}");
    }

    /// La acción múltiple: un toque, mantener medio segundo y dos toques lanzan cada uno su
    /// macro.
    #[test]
    fn la_accion_multiple_distingue_los_toques() {
        let a = accion("a", "Triangle", TipoAccion::Multiple, "65/65,66/66,67/67");
        let mut b = Banco::nuevo(vec![a]);
        let mut teclas = Vec::new();
        let mut paso = |b: &mut Banco, pulsado: bool, ms: i64| {
            let (e, _) = b.paso(if pulsado { boton::TRIANGULO } else { 0 }, ms);
            for x in e {
                if let EventoRaton::Tecla(k) = x {
                    teclas.push(k);
                }
            }
        };
        // Un toque corto.
        paso(&mut b, true, 0);
        paso(&mut b, false, 50);
        for ms in (60..=400).step_by(20) {
            paso(&mut b, false, ms);
        }
        // Mantener.
        for ms in (1000..=1600).step_by(20) {
            paso(&mut b, true, ms);
        }
        paso(&mut b, false, 1620);
        // Dos toques.
        paso(&mut b, true, 3000);
        paso(&mut b, false, 3040);
        paso(&mut b, true, 3080);
        paso(&mut b, false, 3120);
        for ms in (3140..=3500).step_by(20) {
            paso(&mut b, false, ms);
        }
        assert_eq!(teclas, vec![65, 66, 67]);
    }

    /// La calibración del volante: la acción la empieza y, otra vez pasados 3 s, la acaba.
    #[test]
    fn la_accion_calibra_el_volante() {
        let mut b = Banco::nuevo(vec![accion(
            "a",
            "PS/Cross",
            TipoAccion::CalibrarVolante,
            "",
        )]);
        b.paso(boton::PS | boton::EQUIS, 0);
        assert_eq!(b.volante.calibrando, 1);
        b.volante.calibrando = 2;
        b.paso(boton::PS | boton::EQUIS, 1000);
        assert_eq!(b.volante.calibrando, 2);
        b.paso(boton::PS | boton::EQUIS, 3100);
        assert_eq!(b.volante.calibrando, 3);
    }

    /// Con retardo, la acción espera a que los controles sigan pulsados ese tiempo.
    #[test]
    fn la_accion_con_retardo_espera() {
        let mut a = accion("a", "PS", TipoAccion::DesconectarBt, "0.5");
        a.retardo = 0.0;
        let mut b = Banco::nuevo(vec![a]);
        b.paso(0, 0);
        b.paso(boton::PS, 100);
        assert!(b.acciones.peticiones.is_empty());
        b.paso(boton::PS, 400);
        assert!(b.acciones.peticiones.is_empty());
        b.paso(boton::PS, 600);
        assert_eq!(b.acciones.peticiones, vec![Peticion::DesconectarBt]);
    }
}
