//! La reasignación de los controles (`Mapping.MapCustom`): a otros controles del mando virtual, a
//! teclas, al ratón y a macros, con los extras, el flick stick y la capa shift; y el arreglo de
//! los gatillos del DS4 que hace `ControlService` después. Las acciones especiales van en el
//! bloque 5.

use crate::acciones::{AccionesEspeciales, Entorno};
use crate::flick::Flick;
use crate::luz::{AnimacionLuz, MacroLuz};
use crate::macros::Macros;
use crate::raton::{EventoRaton, Sintetico};
use crate::raton_botones::{AjustesRatonBotones, Angulos, RatonBotones};
use crate::sticks::SticksAccion;
use crate::volante::{AjustesVolante, Volante};
use ds5_informes::{boton, Estado};
use ds5_perfiles::mapeos::{Accion, AjusteControl};
use ds5_perfiles::perfil::{
    DosFases, EjeVolante, FlickStick, MandoVirtual, ModoPalanca, Perfil, SalidaGatillos,
    SalidaGiroscopio,
};

/// Los controles del mando, con su número en `DS5Controls` (el 0 es `None`).
pub mod control {
    pub const LX_NEG: usize = 1;
    pub const LX_POS: usize = 2;
    pub const LY_NEG: usize = 3;
    pub const LY_POS: usize = 4;
    pub const RX_NEG: usize = 5;
    pub const RX_POS: usize = 6;
    pub const RY_NEG: usize = 7;
    pub const RY_POS: usize = 8;
    pub const L1: usize = 9;
    pub const L2: usize = 10;
    pub const L3: usize = 11;
    pub const R1: usize = 12;
    pub const R2: usize = 13;
    pub const R3: usize = 14;
    pub const CUADRADO: usize = 15;
    pub const TRIANGULO: usize = 16;
    pub const CIRCULO: usize = 17;
    pub const EQUIS: usize = 18;
    pub const ARRIBA: usize = 19;
    pub const DERECHA: usize = 20;
    pub const ABAJO: usize = 21;
    pub const IZQUIERDA: usize = 22;
    pub const PS: usize = 23;
    pub const PANEL_IZQUIERDA: usize = 24;
    pub const PANEL_ARRIBA: usize = 25;
    pub const PANEL_MULTI: usize = 26;
    pub const PANEL_DERECHA: usize = 27;
    pub const CREAR: usize = 28;
    pub const OPCIONES: usize = 29;
    pub const MUTE: usize = 30;
    pub const FN_IZQ: usize = 31;
    pub const FN_DER: usize = 32;
    pub const PALETA_IZQ: usize = 33;
    pub const PALETA_DER: usize = 34;
    pub const GIRO_X_POS: usize = 35;
    pub const GIRO_X_NEG: usize = 36;
    pub const GIRO_Z_POS: usize = 37;
    pub const GIRO_Z_NEG: usize = 38;
    pub const DESLIZAR_IZQUIERDA: usize = 39;
    pub const DESLIZAR_DERECHA: usize = 40;
    pub const DESLIZAR_ARRIBA: usize = 41;
    pub const DESLIZAR_ABAJO: usize = 42;
    pub const L2_A_FONDO: usize = 43;
    pub const R2_A_FONDO: usize = 44;
    pub const GIRO_DESLIZAR_IZQUIERDA: usize = 45;
    pub const GIRO_DESLIZAR_DERECHA: usize = 46;
    pub const GIRO_DESLIZAR_ARRIBA: usize = 47;
    pub const GIRO_DESLIZAR_ABAJO: usize = 48;
    pub const CAPTURA: usize = 49;
    pub const LATERAL_IZQ: usize = 50;
    pub const LATERAL_DER: usize = 51;
    pub const EXTERIOR_LS: usize = 52;
    pub const EXTERIOR_RS: usize = 53;
    pub const TOQUE_EMPIEZA: usize = 54;
    pub const TOQUE_ACABA: usize = 55;
}
use control::*;

const N: usize = 56;

/// `DS5StateFieldMapping.ControlType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tipo {
    Desconocido,
    Boton,
    Eje,
    Gatillo,
    Toque,
    Giro,
    Deslizar,
}

/// `DS5StateFieldMapping.mappedType`.
pub(crate) const TIPOS: [Tipo; N] = {
    use Tipo::*;
    [
        Desconocido, // None
        Eje,
        Eje,
        Eje,
        Eje,
        Eje,
        Eje,
        Eje,
        Eje, // LXNeg a RYPos
        Boton,
        Gatillo,
        Boton,
        Boton,
        Gatillo,
        Boton, // L1 L2 L3 R1 R2 R3
        Boton,
        Boton,
        Boton,
        Boton, // cuadrado, triángulo, círculo, equis
        Boton,
        Boton,
        Boton,
        Boton,
        Boton, // cruceta y PS
        Toque,
        Toque,
        Toque,
        Toque, // zonas del panel
        Boton,
        Boton,
        Boton, // Share, Options, Mute
        Boton,
        Boton,
        Boton,
        Boton, // FnL, FnR, BLP, BRP
        Giro,
        Giro,
        Giro,
        Giro, // GyroXPos a GyroZNeg
        Deslizar,
        Deslizar,
        Deslizar,
        Deslizar, // SwipeLeft a SwipeDown
        Boton,
        Boton, // L2FullPull, R2FullPull
        Boton,
        Boton,
        Boton,
        Boton, // GyroSwipe
        Boton,
        Boton,
        Boton, // Capture, SideL, SideR
        Gatillo,
        Gatillo, // LSOuter, RSOuter
        Boton,
        Boton, // TouchStarted, TouchEnded
    ]
};

/// Los controles que pueden activar la capa shift, por su número en el perfil (`shiftTriggerMapping`).
/// El 26 es un dedo en el panel y no corresponde a ningún control.
const GATILLOS_SHIFT: [usize; 35] = [
    0,
    EQUIS,
    CIRCULO,
    CUADRADO,
    TRIANGULO,
    OPCIONES,
    CREAR,
    ARRIBA,
    ABAJO,
    IZQUIERDA,
    DERECHA,
    PS,
    L1,
    R1,
    L2,
    R2,
    L3,
    R3,
    PANEL_IZQUIERDA,
    PANEL_ARRIBA,
    PANEL_MULTI,
    PANEL_DERECHA,
    GIRO_Z_NEG,
    GIRO_Z_POS,
    GIRO_X_POS,
    GIRO_X_NEG,
    0,
    MUTE,
    FN_IZQ,
    FN_DER,
    PALETA_IZQ,
    PALETA_DER,
    CAPTURA,
    LATERAL_IZQ,
    LATERAL_DER,
];
const SHIFT_UN_DEDO: i32 = 26;

/// Los botones de salida que son controles del mando virtual (`X360Controls` de `LXNeg` a `Start`).
const SALIDA_ULTIMO_MANDO: u8 = 25;
const SALIDA_CLIC_PANEL: u8 = 26;
/// Los clics del ratón (`LeftMouse` a `FifthMouse`), la rueda y las direcciones del ratón, normal
/// (`MouseUp` a `MouseRight`) y absoluto.
const SALIDA_CLIC_IZQUIERDO: u8 = 27;
const SALIDA_CLIC_QUINTO: u8 = 31;
const SALIDA_RUEDA_ARRIBA: u8 = 32;
const SALIDA_RUEDA_ABAJO: u8 = 33;
const SALIDA_RATON_ARRIBA: u8 = 34;
const SALIDA_RATON_DERECHA: u8 = 37;
const SALIDA_ABSOLUTO_ARRIBA: u8 = 38;
const SALIDA_ABSOLUTO_DERECHA: u8 = 41;

/// `DS5KeyType`.
const TECLA_EXPLORACION: u8 = 1;
const TECLA_ALTERNA: u8 = 2;

/// El control que corresponde a un botón de salida del mando (`reverseX360ButtonMapping`): coinciden
/// hasta `Guide`, y luego `Back` es Share y `Start` es Options.
fn control_de_salida(b: u8) -> usize {
    match b {
        24 => CREAR,
        25 => OPCIONES,
        b => b as usize,
    }
}

/// El otro sentido del mismo eje: `LXNeg` con `LXPos` y así.
fn pareja(c: usize) -> usize {
    if c.is_multiple_of(2) {
        c - 1
    } else {
        c + 1
    }
}

fn negativo(c: usize) -> bool {
    matches!(c, LX_NEG | LY_NEG | RX_NEG | RY_NEG)
}

/// El `2.0078740157480315f` de C#: 255 entre 127.
const PASO_POSITIVO: f32 = 2.007_874_015_748_031_5_f64 as f32;

/// `(byte)` de C# sobre un `float`.
fn byte_f32(v: f32) -> u8 {
    v as i32 as u8
}

/// Lo que no sale del estado del mando: el panel táctil. Por defecto, nada pulsado.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Entradas {
    /// Las zonas del panel: izquierda, arriba, varios dedos y derecha (`Mouse.leftDown`...).
    pub zonas_panel: [bool; 4],
    /// Deslizar por el panel, a la izquierda, derecha, arriba y abajo: el valor y si cuenta.
    pub deslizar: [u8; 4],
    pub deslizar_activo: [bool; 4],
    /// Los gestos con el giroscopio (`gyroSwipe`): izquierda, derecha, arriba y abajo.
    pub giro_deslizar: [bool; 4],
    /// Los gestos del informe anterior, para soltar el que se acaba.
    pub giro_deslizar_antes: [bool; 4],
    pub toque_empieza: bool,
    pub toque_acaba: bool,
}

/// `DS5StateFieldMapping`: el valor de cada control, por su número.
#[derive(Debug, Clone)]
pub(crate) struct Campos {
    pub(crate) botones: [bool; N],
    pub(crate) ejes: [u8; N],
    pub(crate) gatillos: [u8; N],
    pub(crate) giro: [i32; N],
    pub(crate) deslizar: [u8; N],
    pub(crate) deslizar_activo: [bool; N],
    pub(crate) panel: bool,
    pub(crate) panel_salida: bool,
}

impl Campos {
    /// `PopulateFieldMapping`.
    pub(crate) fn desde(e: &Estado, o: &Entradas) -> Campos {
        let mut c = Campos {
            botones: [false; N],
            ejes: [0; N],
            gatillos: [0; N],
            giro: [0; N],
            deslizar: [0; N],
            deslizar_activo: [false; N],
            panel: e.pulsado(boton::PANEL),
            panel_salida: e.pulsado(boton::PANEL_SALIDA),
        };
        c.ejes[LX_NEG] = e.lx;
        c.ejes[LX_POS] = e.lx;
        c.ejes[LY_NEG] = e.ly;
        c.ejes[LY_POS] = e.ly;
        c.ejes[RX_NEG] = e.rx;
        c.ejes[RX_POS] = e.rx;
        c.ejes[RY_NEG] = e.ry;
        c.ejes[RY_POS] = e.ry;
        c.gatillos[EXTERIOR_LS] = e.exterior_ls;
        c.gatillos[EXTERIOR_RS] = e.exterior_rs;
        c.gatillos[L2] = e.l2;
        c.gatillos[R2] = e.r2;
        for (control, bit) in BOTONES {
            c.botones[control] = e.pulsado(bit);
        }
        c.botones[L2_A_FONDO] = e.l2_crudo == 255;
        c.botones[R2_A_FONDO] = e.r2_crudo == 255;
        c.botones[PANEL_IZQUIERDA] = o.zonas_panel[0];
        c.botones[PANEL_ARRIBA] = o.zonas_panel[1];
        c.botones[PANEL_MULTI] = o.zonas_panel[2];
        c.botones[PANEL_DERECHA] = o.zonas_panel[3];
        // La aceleración de salida que dejaron las curvas (`getOutputAccelX` y `Z`).
        let (ax, az) = e
            .movimiento
            .map_or((0, 0), |m| (m.salida_acel_x, m.salida_acel_z));
        let x = ax.wrapping_neg();
        c.giro[GIRO_X_POS] = x.max(0);
        c.giro[GIRO_X_NEG] = x.min(0);
        c.giro[GIRO_Z_POS] = az.max(0);
        c.giro[GIRO_Z_NEG] = az.min(0);
        for i in 0..4 {
            c.deslizar[DESLIZAR_IZQUIERDA + i] = o.deslizar[i];
            c.deslizar_activo[DESLIZAR_IZQUIERDA + i] = o.deslizar_activo[i];
            c.botones[GIRO_DESLIZAR_IZQUIERDA + i] = o.giro_deslizar[i];
        }
        c.botones[TOQUE_EMPIEZA] = o.toque_empieza;
        c.botones[TOQUE_ACABA] = o.toque_acaba;
        c
    }

    /// `PopulateState`: solo los campos que escribe; el resto se queda como estaba.
    fn volcar(&self, m: &mut Estado) {
        m.lx = self.ejes[LX_POS];
        m.ly = self.ejes[LY_POS];
        m.rx = self.ejes[RX_POS];
        m.ry = self.ejes[RY_POS];
        m.exterior_ls = self.gatillos[EXTERIOR_LS];
        m.exterior_rs = self.gatillos[EXTERIOR_RS];
        m.l2 = self.gatillos[L2];
        m.r2 = self.gatillos[R2];
        for (control, bit) in BOTONES {
            poner(m, bit, self.botones[control]);
        }
        poner(m, boton::PANEL, self.panel);
        poner(m, boton::PANEL_SALIDA, self.panel_salida);
    }
}

/// Los botones que pasan tal cual entre el estado y los campos.
const BOTONES: [(usize, u64); 23] = [
    (L1, boton::L1),
    (L3, boton::L3),
    (R1, boton::R1),
    (R3, boton::R3),
    (EQUIS, boton::EQUIS),
    (TRIANGULO, boton::TRIANGULO),
    (CIRCULO, boton::CIRCULO),
    (CUADRADO, boton::CUADRADO),
    (PS, boton::PS),
    (OPCIONES, boton::OPCIONES),
    (CREAR, boton::CREAR),
    (MUTE, boton::MUTE),
    (FN_IZQ, boton::FN_IZQ),
    (FN_DER, boton::FN_DER),
    (PALETA_IZQ, boton::PALETA_IZQ),
    (PALETA_DER, boton::PALETA_DER),
    (CAPTURA, boton::CAPTURA),
    (LATERAL_IZQ, boton::LATERAL_IZQ),
    (LATERAL_DER, boton::LATERAL_DER),
    (ARRIBA, boton::ARRIBA),
    (DERECHA, boton::DERECHA),
    (ABAJO, boton::ABAJO),
    (IZQUIERDA, boton::IZQUIERDA),
];

fn poner(e: &mut Estado, bit: u64, v: bool) {
    if v {
        e.botones |= bit;
    } else {
        e.botones &= !bit;
    }
}

/// Lo que hace falta del perfil para reasignar.
#[derive(Debug, Clone)]
pub struct AjustesBotones {
    /// La asignación de cada control, por su número menos uno.
    pub controles: Vec<AjusteControl>,
    /// Si el perfil cambia algo (`containsCustomAction`, `containsCustomExtras` o acciones
    /// especiales): si no, el estado pasa tal cual y no se reasigna nada.
    pub personalizado: bool,
    pub modo_ls: ModoPalanca,
    pub modo_rs: ModoPalanca,
    pub dos_fases_l2: DosFases,
    pub dos_fases_r2: DosFases,
    pub cadera_l2_ms: i64,
    pub cadera_r2_ms: i64,
    pub antirrebote_ms: i64,
    pub mando: MandoVirtual,
    pub salida_gatillos: SalidaGatillos,
    /// Los botones de más del DualSense Edge (Fn y palas traseras).
    pub edge: bool,
    /// Si el giroscopio va como controles (`IsUsingSAForControls`).
    pub giro_controles: bool,
    /// Si el giroscopio hace gestos (`DirectionalSwipe`).
    pub giro_gestos: bool,
    /// El ratón que mueven los controles.
    pub raton: AjustesRatonBotones,
    pub flick_ls: FlickStick,
    pub flick_rs: FlickStick,
    /// Por control: si tiene una macro de luz que no está activa. Solo entonces los extras
    /// cambian el color de la luz (`!dcs.LightbarMacro?.Active ?? false`).
    pub luz_extras: Vec<bool>,
    pub volante: AjustesVolante,
    /// Las acciones especiales del perfil, por su nombre.
    pub acciones: Vec<String>,
    /// La macro de luz activa de cada control.
    pub macros_luz: Vec<Option<MacroLuz>>,
}

impl AjustesBotones {
    pub fn desde_perfil(p: &Perfil) -> AjustesBotones {
        let a = &p.ajustes;
        let controles = p.mapeos.controles.clone();
        // `CacheProfileCustomsFlags`: también cuentan el giroscopio como stick, el volante por
        // vJoy y los sticks que no van como controles.
        let personalizado = !p.acciones.is_empty()
            || controles.iter().any(|c| {
                c.accion != Accion::Predeterminada
                    || c.accion_shift != Accion::Predeterminada
                    || c.extras.is_some()
            })
            || a.salida_giroscopio == SalidaGiroscopio::MouseJoystick
            || !matches!(
                a.eje_volante,
                EjeVolante::None
                    | EjeVolante::LX
                    | EjeVolante::LY
                    | EjeVolante::RX
                    | EjeVolante::RY
                    | EjeVolante::L2R2
            )
            || a.ls_modo != ModoPalanca::Controls
            || a.rs_modo != ModoPalanca::Controls;
        let luz_extras = controles
            .iter()
            .map(|c| macro_luz_inactiva(c.macro_luz.as_deref()))
            .collect();
        AjustesBotones {
            volante: AjustesVolante::desde_perfil(a),
            acciones: p.acciones.clone(),
            macros_luz: controles
                .iter()
                .map(|c| c.macro_luz.as_deref().and_then(MacroLuz::leer))
                .collect(),
            controles,
            personalizado,
            modo_ls: a.ls_modo,
            modo_rs: a.rs_modo,
            dos_fases_l2: a.l2_dos_fases,
            dos_fases_r2: a.r2_dos_fases,
            cadera_l2_ms: a.l2_tiempo_cadera as i64,
            cadera_r2_ms: a.r2_tiempo_cadera as i64,
            antirrebote_ms: a.antirrebote_ms as i64,
            mando: a.mando_virtual,
            salida_gatillos: p.salida_gatillos.unwrap_or(SalidaGatillos::Default),
            edge: false,
            giro_controles: a.salida_giroscopio == SalidaGiroscopio::Controls,
            giro_gestos: a.salida_giroscopio == SalidaGiroscopio::DirectionalSwipe,
            raton: AjustesRatonBotones::desde_perfil(p),
            flick_ls: a.ls_salida.flick.clone(),
            flick_rs: a.rs_salida.flick.clone(),
            luz_extras,
        }
    }
}

/// Si la macro de luz del control existe y no está activa (`LightbarMacro.GetMacroFromString`).
/// Una macro que no se entiende se queda como una vacía, que no está activa.
fn macro_luz_inactiva(texto: Option<&str>) -> bool {
    let Some(texto) = texto.filter(|t| !t.is_empty()) else {
        return false;
    };
    let partes: Vec<&str> = texto.split('/').collect();
    if !(3..=4).contains(&partes.len()) {
        return true;
    }
    let activa = match partes[0].trim().to_ascii_lowercase().as_str() {
        "true" => true,
        "false" => false,
        _ => return true,
    };
    for elemento in partes[1].split(';') {
        if elemento.is_empty() {
            return true;
        }
        let mut e = elemento.split(':');
        let color = e.next().unwrap_or("");
        let Some(largo) = e.next() else {
            return true;
        };
        let rgb: Vec<&str> = color.split(',').collect();
        if rgb.len() != 3
            || rgb.iter().any(|v| v.trim().parse::<u8>().is_err())
            || largo.trim().parse::<u32>().is_err()
        {
            return true;
        }
    }
    if partes.len() == 4
        && !matches!(
            partes[3].trim().to_ascii_lowercase().as_str(),
            "true" | "false"
        )
    {
        return true;
    }
    !activa
}

/// La luz que fuerzan los extras y las macros (`DS5LightBar.forcelight`, `forcedColor` y
/// `forcedFlash`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LuzForzada {
    pub activa: bool,
    pub color: (u8, u8, u8),
    pub destello: u8,
}

/// Lo que piden los extras al mando además de reasignar: la vibración y la luz.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SalidaExtras {
    /// La última vibración pedida en este informe (`ctrl.setRumble(fuerte, suave)`), antes del
    /// refuerzo del perfil.
    pub vibracion: Option<(u8, u8)>,
    pub luz: LuzForzada,
}

/// Lo que recuerdan los extras entre informes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct EstadoExtras {
    /// `held`: el control cuyos extras están puestos; 0 si ninguno.
    mantenido: usize,
    /// `extrasRumbleActive`.
    vibrando: bool,
}

/// Lo que la reasignación necesita además del estado: el teclado y el ratón virtuales, el estado
/// del mando sin curvas (para el flick stick y la aceleración del stick) y el tiempo desde el
/// informe anterior.
pub struct Contexto<'a> {
    pub sintetico: &'a mut Sintetico,
    pub eventos: &'a mut Vec<EventoRaton>,
    pub crudo: &'a Estado,
    pub crudo_antes: &'a Estado,
    /// `lastTimeElapsedDouble`: milisegundos desde el informe anterior.
    pub transcurrido_ms: f64,
}

/// `TwoStageTriggerMappingData.EngageButtonsMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modo {
    SoloSuave,
    SoloFondo,
    Ambos,
}

const ZONA_SUAVE: u8 = 1;
const ZONA_FONDO: u8 = 2;

/// `TwoStageTriggerMappingData`: lo que recuerda un gatillo de dos fases.
#[derive(Debug, Clone)]
struct DatosDosFases {
    empezado: bool,
    momento_ms: i64,
    salida_activa: bool,
    suave_activo: bool,
    fondo_activo: bool,
    modo: Modo,
    anteriores: u8,
}

impl Default for DatosDosFases {
    fn default() -> Self {
        DatosDosFases {
            empezado: false,
            momento_ms: 0,
            salida_activa: false,
            suave_activo: false,
            fondo_activo: false,
            modo: Modo::Ambos,
            anteriores: 0,
        }
    }
}

impl DatosDosFases {
    fn empezar(&mut self, ahora_ms: i64) {
        *self = DatosDosFases {
            empezado: true,
            momento_ms: ahora_ms,
            ..Default::default()
        };
    }

    fn reiniciar(&mut self, ahora_ms: i64) {
        *self = DatosDosFases {
            momento_ms: ahora_ms,
            ..Default::default()
        };
    }

    /// `ProcessTwoStageTrigger`: si cuentan la asignación del gatillo (suave) y la de apretarlo a
    /// fondo. `espera_ms` es el tiempo de cadera más el del antirrebote.
    fn procesar(
        &mut self,
        modo: DosFases,
        valor: u8,
        crudo: u8,
        espera_ms: i64,
        ahora_ms: i64,
    ) -> (bool, bool) {
        let mut suave = true;
        let mut fondo = false;
        match modo {
            DosFases::Normal => fondo = crudo == 255,
            DosFases::ExclusiveButtons => {
                if crudo == 255 {
                    fondo = true;
                    suave = false;
                    self.modo = Modo::SoloFondo;
                    self.salida_activa = true;
                } else if valor != 0 && self.modo != Modo::SoloFondo {
                    self.modo = Modo::Ambos;
                    self.salida_activa = true;
                } else if valor == 0 && self.salida_activa {
                    self.reiniciar(ahora_ms);
                }
            }
            DosFases::HairTrigger => {
                suave = false;
                self.modo = Modo::Ambos;
                if crudo == 255 {
                    suave = true;
                    fondo = true;
                    self.suave_activo = true;
                    self.fondo_activo = true;
                    self.salida_activa = true;
                } else if valor != 0 && !self.fondo_activo {
                    suave = true;
                    self.suave_activo = true;
                    self.salida_activa = true;
                } else if valor == 0 && self.salida_activa {
                    self.reiniciar(ahora_ms);
                }
            }
            DosFases::HipFire | DosFases::HipFireExclusiveButtons => {
                let exclusivo = modo == DosFases::HipFireExclusiveButtons;
                suave = false;
                if valor != 0 && !self.empezado {
                    self.empezar(ahora_ms);
                } else if valor != 0 && !self.salida_activa {
                    if self.momento_ms + espera_ms < ahora_ms {
                        self.salida_activa = true;
                        if crudo == 255 {
                            fondo = true;
                            self.fondo_activo = true;
                            self.modo = Modo::SoloFondo;
                        } else if valor != 0 {
                            suave = true;
                            self.suave_activo = true;
                            self.modo = if exclusivo {
                                Modo::SoloSuave
                            } else {
                                Modo::Ambos
                            };
                        }
                    }
                } else if self.salida_activa {
                    if exclusivo {
                        if crudo == 255 && self.modo == Modo::SoloFondo {
                            fondo = true;
                        } else if valor != 0 && self.modo == Modo::SoloSuave {
                            suave = true;
                        } else if valor == 0 {
                            self.reiniciar(ahora_ms);
                        }
                    } else if crudo == 255 {
                        fondo = true;
                        self.fondo_activo = true;
                        if self.modo == Modo::Ambos {
                            suave = true;
                        }
                    } else if valor != 0 && self.modo == Modo::Ambos {
                        self.fondo_activo = false;
                        suave = true;
                        self.suave_activo = true;
                    } else if valor == 0 {
                        self.reiniciar(ahora_ms);
                    }
                } else if self.empezado {
                    self.reiniciar(ahora_ms);
                }
            }
            DosFases::Disabled => {}
        }
        (suave, fondo)
    }
}

/// Una pasada de `MapCustom`: el estado de entrada, los campos que se leen y los que se escriben.
struct Pasada<'a, 'c> {
    e: &'a Estado,
    a: &'a AjustesBotones,
    giro_controles: bool,
    entrada: Campos,
    salida: Campos,
    /// `customMapQueue`: de qué control a qué control, en orden.
    cola: Vec<(usize, usize)>,
    ang: Angulos,
    ctx: &'a mut Contexto<'c>,
    raton: &'a mut RatonBotones,
    flick: &'a mut Flick,
    macros: &'a mut Macros,
    extras: &'a mut EstadoExtras,
    salida_extras: &'a mut SalidaExtras,
    /// `pressedonce`: lo comparten las teclas (por su código) y los clics (de 256 a 260; los
    /// demás botones, en el 0).
    una_vez: &'a mut [bool],
    animacion_luz: &'a mut AnimacionLuz,
    /// `tempMouseDeltaX` y `Y`.
    dx: f64,
    dy: f64,
}

impl Pasada<'_, '_> {
    /// La macro de luz del control: suena con el flanco del botón en el estado sin curvas.
    fn macro_luz(&mut self, c: usize) {
        let Some(m) = &self.a.macros_luz[c - 1] else {
            return;
        };
        let bit = match c {
            CUADRADO => boton::CUADRADO,
            TRIANGULO => boton::TRIANGULO,
            CIRCULO => boton::CIRCULO,
            EQUIS => boton::EQUIS,
            ARRIBA => boton::ARRIBA,
            ABAJO => boton::ABAJO,
            IZQUIERDA => boton::IZQUIERDA,
            DERECHA => boton::DERECHA,
            L1 => boton::L1,
            L3 => boton::L3,
            R1 => boton::R1,
            R3 => boton::R3,
            CREAR => boton::CREAR,
            OPCIONES => boton::OPCIONES,
            MUTE => boton::MUTE,
            PANEL_DERECHA => boton::PANEL_DERECHA,
            PANEL_IZQUIERDA => boton::PANEL_IZQUIERDA,
            CAPTURA => boton::CAPTURA,
            LATERAL_IZQ => boton::LATERAL_IZQ,
            LATERAL_DER => boton::LATERAL_DER,
            FN_IZQ => boton::FN_IZQ,
            FN_DER => boton::FN_DER,
            PALETA_IZQ => boton::PALETA_IZQ,
            PALETA_DER => boton::PALETA_DER,
            _ => return,
        };
        let ahora = self.ctx.crudo.pulsado(bit);
        let antes = self.ctx.crudo_antes.pulsado(bit);
        let flanco = if m.al_soltar {
            antes && !ahora
        } else {
            !antes && ahora
        };
        if flanco && (m.cortar || !self.animacion_luz.en_marcha) {
            self.animacion_luz
                .empezar(m, self.ctx.sintetico.ahora_us, &mut self.salida_extras.luz);
        }
    }

    /// `GetBoolActionMapping`: como `GetBoolMapping`, pero los sticks cuentan por el ángulo y los
    /// gatillos en cuanto se tocan.
    fn activo_accion(&self, c: usize) -> bool {
        let f = &self.entrada;
        let e = self.e;
        match TIPOS[c] {
            Tipo::Eje => {
                let (ls, rs) = (self.ang.ls, self.ang.rs);
                match c {
                    LX_NEG => e.lx < 128 && (112.5..=247.5).contains(&ls),
                    LY_NEG => e.ly < 128 && (22.5..=157.5).contains(&ls),
                    RX_NEG => e.rx < 128 && (112.5..=247.5).contains(&rs),
                    RY_NEG => e.ry < 128 && (22.5..=157.5).contains(&rs),
                    LX_POS => e.lx > 128 && (ls <= 67.5 || ls >= 292.5),
                    LY_POS => e.ly > 128 && (202.5..=337.5).contains(&ls),
                    RX_POS => e.rx > 128 && (rs <= 67.5 || rs >= 292.5),
                    _ => e.ry > 128 && (202.5..=337.5).contains(&rs),
                }
            }
            Tipo::Gatillo => f.gatillos[c] > 0,
            _ => self.activo(c),
        }
    }

    /// `pressedonce[i]`, que en C# es un vector de 2400.
    fn una_vez(&mut self, i: usize) -> Option<&mut bool> {
        self.una_vez.get_mut(i)
    }

    /// Los extras: mientras se mantiene el control, vibración, luz y otra sensibilidad del ratón.
    fn procesar_extras(&mut self, c: usize, extras: Option<&str>) {
        let texto = extras.filter(|x| !x.is_empty() && *x != "0,0,0,0,0,0,0,0,0");
        let Some(texto) = texto else {
            return;
        };
        if self.activo_accion(c) {
            self.extras.mantenido = c;
            // Lo que no es un número vale 0; si faltan valores, C# se para en el primero que falta.
            let x: Vec<i32> = texto
                .split(',')
                .map(|v| v.trim().parse().unwrap_or(0))
                .collect();
            let v = |i: usize| x.get(i).copied();
            let (Some(fuerte), Some(suave)) = (v(0), v(1)) else {
                return;
            };
            if !(fuerte == suave && suave == 0) {
                self.salida_extras.vibracion = Some((fuerte as u8, suave as u8));
                self.extras.vibrando = true;
            }
            let Some(luz) = v(2) else {
                return;
            };
            if luz == 1 && self.a.luz_extras[c - 1] {
                let (Some(r), Some(g), Some(b), Some(d)) = (v(3), v(4), v(5), v(6)) else {
                    return;
                };
                self.salida_extras.luz = LuzForzada {
                    activa: true,
                    color: (r as u8, g as u8, b as u8),
                    destello: d as u8,
                };
            }
            let Some(raton) = v(7) else {
                return;
            };
            if raton == 1 && self.raton.sensibilidad_temporal == -1 {
                let Some(s) = v(8) else {
                    return;
                };
                self.raton.sensibilidad_temporal = s;
                self.raton.sensibilidad = s;
            }
        } else if self.extras.mantenido == c {
            self.salida_extras.luz.activa = false;
            self.salida_extras.luz.destello = 0;
            if self.raton.sensibilidad_temporal != -1 {
                self.raton.sensibilidad = self.a.raton.sensibilidad;
                self.raton.sensibilidad_temporal = -1;
            }
            if self.extras.vibrando {
                self.salida_extras.vibracion = Some((0, 0));
                self.extras.vibrando = false;
            }
            self.extras.mantenido = 0;
        }
    }

    /// Un clic del ratón, la rueda o una dirección del ratón asignados a un control.
    fn procesar_raton(&mut self, c: usize, b: u8) {
        match b {
            SALIDA_CLIC_IZQUIERDO..=SALIDA_CLIC_QUINTO => {
                if self.activo_accion(c) {
                    let k = &mut self.ctx.sintetico.actual;
                    match b {
                        27 => k.izquierdo += 1,
                        28 => k.derecho += 1,
                        29 => k.central += 1,
                        30 => k.cuarto += 1,
                        _ => k.quinto += 1,
                    }
                }
            }
            SALIDA_RUEDA_ARRIBA | SALIDA_RUEDA_ABAJO => {
                if self.activo_accion(c) {
                    let abajo = b == SALIDA_RUEDA_ABAJO;
                    let analogico = (LX_NEG..=RY_POS).contains(&c)
                        || c == L2
                        || c == R2
                        || (GIRO_X_POS..=GIRO_Z_NEG).contains(&c);
                    if analogico {
                        self.raton.sentido_rueda(abajo);
                        let v = self.como_gatillo(c);
                        let ctx = &mut *self.ctx;
                        self.raton.rueda(v, abajo, ctx.sintetico, ctx.eventos);
                    } else if abajo {
                        self.ctx.sintetico.actual.rueda_abajo += 1;
                    } else {
                        self.ctx.sintetico.actual.rueda_arriba += 1;
                    }
                }
            }
            SALIDA_RATON_ARRIBA..=SALIDA_RATON_DERECHA => {
                let sentido = b - SALIDA_RATON_ARRIBA;
                let vertical = sentido <= 1;
                if (vertical && self.dy == 0.0) || (!vertical && self.dx == 0.0) {
                    let v = self.raton.mover(
                        &self.a.raton,
                        c,
                        &self.entrada,
                        self.e,
                        &self.ang,
                        sentido,
                        self.ctx.transcurrido_ms,
                    );
                    let v = if v == i32::MIN as f64 { 0.0 } else { v }.abs();
                    let v = if sentido.is_multiple_of(2) { -v } else { v };
                    if vertical {
                        self.dy = v;
                    } else {
                        self.dx = v;
                    }
                }
            }
            SALIDA_ABSOLUTO_ARRIBA..=SALIDA_ABSOLUTO_DERECHA => {
                let salida = b - SALIDA_ABSOLUTO_ARRIBA;
                let (v, movido) = RatonBotones::absoluto(
                    &self.a.raton,
                    c,
                    &self.entrada,
                    self.e,
                    &self.ang,
                    salida,
                );
                let s = &mut self.raton.absoluto;
                if salida <= 1 {
                    if movido && !s.sucio_y {
                        s.y = v;
                        s.sucio_y = true;
                    }
                } else if movido && !s.sucio_x {
                    s.x = v;
                    s.sucio_x = true;
                }
            }
            _ => {}
        }
    }
    /// `GetBoolMapping`.
    fn activo(&self, c: usize) -> bool {
        let f = &self.entrada;
        match TIPOS[c] {
            Tipo::Boton | Tipo::Toque => f.botones[c],
            Tipo::Eje => match c {
                LX_NEG => self.e.lx < 128 - 55,
                LY_NEG => self.e.ly < 128 - 55,
                RX_NEG => self.e.rx < 128 - 55,
                RY_NEG => self.e.ry < 128 - 55,
                _ => f.ejes[c] > 128 + 55,
            },
            Tipo::Gatillo => f.gatillos[c] > 100,
            Tipo::Deslizar => f.deslizar_activo[c],
            Tipo::Giro => {
                self.giro_controles
                    && match c {
                        GIRO_X_POS | GIRO_Z_POS => f.giro[c] > 0,
                        _ => f.giro[c] < 0,
                    }
            }
            Tipo::Desconocido => false,
        }
    }

    /// `GetByteMapping`: el control como gatillo, de 0 a 255.
    fn como_gatillo(&self, c: usize) -> u8 {
        let f = &self.entrada;
        match TIPOS[c] {
            Tipo::Boton | Tipo::Toque => {
                if f.botones[c] {
                    255
                } else {
                    0
                }
            }
            Tipo::Eje => {
                let v = f.ejes[c] as f32 - 128.0;
                if negativo(c) {
                    if v >= 0.0 {
                        0
                    } else {
                        byte_f32(-v * 1.9921875)
                    }
                } else if v < 0.0 {
                    0
                } else {
                    byte_f32(v * PASO_POSITIVO)
                }
            }
            Tipo::Gatillo => f.gatillos[c],
            Tipo::Deslizar => f.deslizar[c],
            Tipo::Giro => {
                if !self.giro_controles {
                    return 0;
                }
                let g = f.giro[c];
                let v = match c {
                    GIRO_X_POS | GIRO_Z_POS => g * 2,
                    _ => -g * 2,
                };
                v.min(255) as u8
            }
            Tipo::Desconocido => 0,
        }
    }

    /// `GetXYAxisMapping`: el control como eje, con el centro en 128. `alto` es que la salida es
    /// el sentido positivo.
    fn como_eje(&self, c: usize, alto: bool) -> u8 {
        const CENTRO: u8 = 128;
        let f = &self.entrada;
        let pulsado = if alto { 255 } else { 0 };
        match TIPOS[c] {
            Tipo::Boton | Tipo::Toque => {
                if f.botones[c] {
                    pulsado
                } else {
                    CENTRO
                }
            }
            Tipo::Eje => {
                let v = f.ejes[c];
                if negativo(c) {
                    match (v < CENTRO, alto) {
                        (true, false) => v,
                        (true, true) => 255 - v,
                        _ => CENTRO,
                    }
                } else {
                    match (v > CENTRO, alto) {
                        (true, false) => 255 - v,
                        (true, true) => v,
                        _ => CENTRO,
                    }
                }
            }
            Tipo::Gatillo => {
                let t = f.gatillos[c] as f32 / PASO_POSITIVO;
                byte_f32(if alto { 128.0 + t } else { 128.0 - t })
            }
            Tipo::Deslizar => {
                let s = f.deslizar[c] as f32 / 2.0;
                byte_f32(if alto { 128.0 + s } else { 128.0 - s })
            }
            Tipo::Giro => {
                let g = f.giro[c];
                let hay = match c {
                    GIRO_X_POS | GIRO_Z_POS => g > 0,
                    _ => g < 0,
                };
                if !(self.giro_controles && hay) {
                    return CENTRO;
                }
                let g = g.abs();
                if alto {
                    (128 + g).min(255) as u8
                } else {
                    (128 - g).max(0) as u8
                }
            }
            Tipo::Desconocido => 0,
        }
    }

    /// `ShiftTrigger`.
    fn shift(&self, gatillo: i32) -> bool {
        if gatillo == SHIFT_UN_DEDO {
            self.e.pulsado(boton::UN_DEDO)
        } else if gatillo > 0 && (gatillo as usize) < GATILLOS_SHIFT.len() {
            self.activo(GATILLOS_SHIFT[gatillo as usize])
        } else {
            false
        }
    }

    /// `ResetToDefaultValue`: lo que tenía asignado el control deja de salir por su sitio.
    fn soltar(&mut self, c: usize) {
        let s = &mut self.salida;
        match TIPOS[c] {
            Tipo::Boton | Tipo::Toque => s.botones[c] = false,
            Tipo::Eje => {
                s.ejes[c] = 128;
                s.ejes[pareja(c)] = 128;
            }
            Tipo::Gatillo => s.gatillos[c] = 0,
            _ => {}
        }
    }

    /// `ProcessControlSettingAction`, sin las macros de luz.
    fn procesar(&mut self, c: usize) {
        let a = self.a;
        let ajuste = &a.controles[c - 1];
        let (accion, tipo) =
            if ajuste.accion_shift != Accion::Predeterminada && self.shift(ajuste.gatillo_shift) {
                (&ajuste.accion_shift, ajuste.tipo_tecla_shift)
            } else {
                (&ajuste.accion, ajuste.tipo_tecla)
            };
        // Los extras de la capa shift no se cargan nunca: siempre son los normales.
        self.procesar_extras(c, ajuste.extras.as_deref());
        self.macro_luz(c);
        match accion {
            Accion::Predeterminada => {
                if TIPOS[c] == Tipo::Eje {
                    self.cola.push((c, c));
                }
            }
            Accion::Macro(m) => {
                let activo = self.activo(c);
                let ctx = &mut *self.ctx;
                self.macros.control(
                    c,
                    m,
                    tipo,
                    activo,
                    ctx.sintetico.ahora_us,
                    &mut self.salida_extras.luz,
                    ctx.eventos,
                );
                self.soltar(c);
            }
            Accion::Tecla(k) => {
                let k = *k;
                if self.activo_accion(c) {
                    let primera = self
                        .una_vez(k as usize)
                        .map(|v| !std::mem::replace(v, true));
                    let p = self.ctx.sintetico.tecla(k);
                    if tipo & TECLA_EXPLORACION != 0 {
                        p.exploracion += 1;
                    } else {
                        p.normal += 1;
                    }
                    if tipo & TECLA_ALTERNA != 0 {
                        if primera == Some(true) {
                            p.alterna_activa = !p.alterna_activa;
                        }
                        p.alterna += 1;
                    }
                    p.repeticion += 1;
                } else if let Some(v) = self.una_vez(k as usize) {
                    *v = false;
                }
                self.soltar(c);
            }
            Accion::Boton(b) => {
                let b = *b;
                if (1..=SALIDA_ULTIMO_MANDO).contains(&b) {
                    self.cola.push((c, control_de_salida(b)));
                } else if b == SALIDA_CLIC_PANEL {
                    if self.activo(c) {
                        self.salida.panel_salida = true;
                    }
                } else {
                    self.procesar_raton(c, b);
                }
                if tipo & TECLA_ALTERNA != 0 {
                    let indice = if (SALIDA_CLIC_IZQUIERDO..=SALIDA_CLIC_QUINTO).contains(&b) {
                        256 + (b - SALIDA_CLIC_IZQUIERDO) as usize
                    } else {
                        0
                    };
                    if self.activo_accion(c) {
                        if !std::mem::replace(&mut self.una_vez[indice], true) {
                            let k = &mut self.ctx.sintetico.actual;
                            k.alterna_activa = !k.alterna_activa;
                        }
                        self.ctx.sintetico.actual.alterna += 1;
                    } else {
                        self.una_vez[indice] = false;
                    }
                }
                self.soltar(c);
            }
        }
    }

    /// Un gatillo de dos fases: la asignación del gatillo y la de apretarlo a fondo.
    fn dos_fases(&mut self, datos: &mut DatosDosFases, izquierdo: bool, ahora_ms: i64) {
        let (gatillo, fondo, modo, cadera, valor, crudo) = if izquierdo {
            (
                L2,
                L2_A_FONDO,
                self.a.dos_fases_l2,
                self.a.cadera_l2_ms,
                self.e.l2,
                self.e.l2_crudo,
            )
        } else {
            (
                R2,
                R2_A_FONDO,
                self.a.dos_fases_r2,
                self.a.cadera_r2_ms,
                self.e.r2,
                self.e.r2_crudo,
            )
        };
        let espera = cadera + self.a.antirrebote_ms;
        let (suave, a_fondo) = datos.procesar(modo, valor, crudo, espera, ahora_ms);
        let mut activos = 0;
        if suave || datos.anteriores & ZONA_SUAVE != 0 {
            if suave {
                activos |= ZONA_SUAVE;
            } else {
                self.entrada.gatillos[gatillo] = 0;
                self.salida.gatillos[gatillo] = 0;
            }
            self.procesar(gatillo);
        } else {
            self.salida.gatillos[gatillo] = 0;
        }
        if a_fondo || datos.anteriores & ZONA_FONDO != 0 {
            if a_fondo {
                activos |= ZONA_FONDO;
            } else {
                self.entrada.botones[fondo] = false;
            }
            self.procesar(fondo);
        }
        datos.anteriores = activos;
    }

    /// Recorre la cola: cada control asignado a otro escribe su valor en la salida.
    fn vaciar_cola(&mut self) {
        for (de, a) in std::mem::take(&mut self.cola) {
            if (LX_NEG..=RY_POS).contains(&a) {
                let alto = a.is_multiple_of(2);
                let v = self.como_eje(de, alto);
                if v != 128 {
                    self.salida.ejes[a] = v;
                    self.salida.ejes[pareja(a)] = v;
                }
            } else if a == L2 || a == R2 {
                let v = self.como_gatillo(de);
                if v != 0 {
                    self.salida.gatillos[a] = v;
                }
            } else if self.activo(de) {
                self.salida.botones[a] = true;
            }
        }
    }
}

/// La reasignación de un mando, con lo que recuerda entre informes.
#[derive(Debug, Clone)]
pub struct Botones {
    /// `MappedState`: los campos que no escribe la reasignación se quedan como estaban.
    mapeado: Estado,
    l2: DatosDosFases,
    r2: DatosDosFases,
    /// Lo que piden el giroscopio y el panel como stick (`mapStickActionData`): se mezcla al
    /// final de la reasignación.
    pub sticks: SticksAccion,
    pub raton: RatonBotones,
    flick: Flick,
    pub macros: Macros,
    extras: EstadoExtras,
    /// Lo que han pedido los extras en el último informe.
    pub salida_extras: SalidaExtras,
    una_vez: Vec<bool>,
    /// El volante con el giroscopio y su calibración.
    pub volante: Volante,
    /// Las acciones especiales: las de `Actions.xml` las pone la app.
    pub acciones: AccionesEspeciales,
    /// La macro de luz que está sonando.
    pub animacion_luz: AnimacionLuz,
}

impl Default for Botones {
    fn default() -> Self {
        Botones {
            mapeado: Estado::default(),
            l2: DatosDosFases::default(),
            r2: DatosDosFases::default(),
            sticks: SticksAccion::default(),
            raton: RatonBotones::default(),
            flick: Flick::default(),
            macros: Macros::default(),
            extras: EstadoExtras::default(),
            salida_extras: SalidaExtras::default(),
            una_vez: vec![false; 2400],
            volante: Volante::nuevo(None),
            acciones: AccionesEspeciales::default(),
            animacion_luz: AnimacionLuz::default(),
        }
    }
}

impl Botones {
    /// Reasigna el estado ya pasado por las curvas, sin teclado ni ratón.
    pub fn aplicar(
        &mut self,
        e: &Estado,
        o: &Entradas,
        a: &AjustesBotones,
        mando: MandoVirtual,
        ahora_ms: i64,
    ) -> Estado {
        let mut sintetico = Sintetico::default();
        sintetico.ahora_us = ahora_ms * 1000;
        let mut eventos = Vec::new();
        let mut ctx = Contexto {
            sintetico: &mut sintetico,
            eventos: &mut eventos,
            crudo: e,
            crudo_antes: e,
            transcurrido_ms: 0.0,
        };
        self.aplicar_con(e, o, a, mando, &mut ctx)
    }

    /// Reasigna el estado ya pasado por las curvas y aplica el arreglo de los gatillos del mando
    /// virtual. `mando` es el mando virtual que está enchufado. Lo que se pide al teclado y al
    /// ratón queda en el contexto, para `Sintetico::confirmar`.
    pub fn aplicar_con(
        &mut self,
        e: &Estado,
        o: &Entradas,
        a: &AjustesBotones,
        mando: MandoVirtual,
        ctx: &mut Contexto,
    ) -> Estado {
        self.salida_extras.vibracion = None;
        self.macros.vibracion = None;
        // Con un perfil temporal cargado por una acción también se reasigna, para poder
        // deshacerlo.
        let personalizado = a.personalizado || self.acciones.temporal.is_some();
        let mut r = if personalizado {
            self.reasignar(e, o, a, ctx)
        } else {
            e.clone()
        };
        arreglar_gatillos(&mut r, mando, a.salida_gatillos);
        if personalizado {
            // El arreglo cambia el estado mapeado, que es el mismo objeto en C#.
            self.mapeado.botones = r.botones;
            self.mapeado.l2 = r.l2;
            self.mapeado.r2 = r.r2;
        }
        r
    }

    fn reasignar(
        &mut self,
        e: &Estado,
        o: &Entradas,
        a: &AjustesBotones,
        ctx: &mut Contexto,
    ) -> Estado {
        let ahora_ms = ctx.sintetico.ahora_us.div_euclid(1000);
        if self.raton.sensibilidad_temporal == -1 {
            self.raton.sensibilidad = a.raton.sensibilidad;
        }
        self.macros.avanzar(
            ctx.sintetico.ahora_us,
            &mut self.salida_extras.luz,
            ctx.eventos,
        );
        self.animacion_luz
            .avanzar(ctx.sintetico.ahora_us, &mut self.salida_extras.luz);
        let entrada = Campos::desde(e, o);
        let mut salida = entrada.clone();
        if !a.acciones.is_empty() || self.acciones.temporal.is_some() {
            let ahora_us = ctx.sintetico.ahora_us;
            let mut en = Entorno {
                entrada: &entrada,
                salida: &mut salida,
                e,
                crudo: ctx.crudo,
                crudo_antes: ctx.crudo_antes,
                controles: &a.controles,
                perfil: &a.acciones,
                giro_controles: a.giro_controles,
                antirrebote_ms: a.antirrebote_ms,
                sintetico: &mut *ctx.sintetico,
                eventos: &mut *ctx.eventos,
                macros: &mut self.macros,
                luz: &mut self.salida_extras.luz,
                volante: &mut self.volante,
                ahora_us,
            };
            self.acciones.procesar(&mut en);
        }
        let mut p = Pasada {
            e,
            a,
            giro_controles: a.giro_controles,
            salida,
            entrada,
            cola: Vec::new(),
            ang: Angulos::de(e),
            ctx,
            raton: &mut self.raton,
            flick: &mut self.flick,
            macros: &mut self.macros,
            extras: &mut self.extras,
            salida_extras: &mut self.salida_extras,
            una_vez: &mut self.una_vez,
            animacion_luz: &mut self.animacion_luz,
            dx: 0.0,
            dy: 0.0,
        };
        p.raton.absoluto.sucio_x = false;
        p.raton.absoluto.sucio_y = false;

        for (izquierdo, modo, exterior, ejes) in [
            (true, a.modo_ls, EXTERIOR_LS, LX_NEG..=LY_POS),
            (false, a.modo_rs, EXTERIOR_RS, RX_NEG..=RY_POS),
        ] {
            if modo == ModoPalanca::Controls {
                if let (false, Some(d)) = (izquierdo, &a.raton.delta_rs) {
                    // La aceleración por velocidad del stick derecho, antes de sus controles.
                    let (c, crudo) = (p.e, p.ctx.crudo);
                    let max = |v: u8| if v >= 128 { 127.0 } else { -128.0 };
                    p.raton.delta_rs.procesar(
                        c.rx as f64 - 128.0,
                        c.ry as f64 - 128.0,
                        crudo.rx as f64 - 128.0,
                        crudo.ry as f64 - 128.0,
                        max(c.rx),
                        max(c.ry),
                        d,
                        p.ctx.sintetico.ahora_us,
                    );
                }
                p.procesar(exterior);
                for c in ejes {
                    p.procesar(c);
                }
            } else {
                // Sin controles (o con flick stick, que mueve el ratón) el stick no sale.
                for c in ejes {
                    p.salida.ejes[c] = 128;
                }
                if modo == ModoPalanca::FlickStick {
                    let (cr, an) = (p.ctx.crudo, p.ctx.crudo_antes);
                    let (x, y, px, py, ajustes) = if izquierdo {
                        (cr.lx, cr.ly, an.lx, an.ly, &a.flick_ls)
                    } else {
                        (cr.rx, cr.ry, an.rx, an.ry, &a.flick_rs)
                    };
                    p.dx += p.flick.procesar(x, y, px, py, ajustes, cr.transcurrido);
                }
            }
        }

        if a.dos_fases_l2 == DosFases::Disabled {
            p.procesar(L2);
        } else {
            p.dos_fases(&mut self.l2, true, ahora_ms);
        }
        if a.dos_fases_r2 == DosFases::Disabled {
            p.procesar(R2);
        } else {
            p.dos_fases(&mut self.r2, false, ahora_ms);
        }

        for c in [L1, L3, R1, R3]
            .into_iter()
            .chain(CUADRADO..=MUTE)
            .chain(GIRO_X_POS..=DESLIZAR_ABAJO)
        {
            p.procesar(c);
        }
        if a.edge {
            for c in FN_IZQ..=PALETA_DER {
                p.procesar(c);
            }
        }

        // Los gestos del giroscopio van aparte: en cada eje, primero se suelta el que acaba y
        // después el que sigue.
        if a.giro_gestos {
            let (g, antes) = (o.giro_deslizar, o.giro_deslizar_antes);
            let eje = |i: usize| {
                let acaba = if antes[i] && !g[i] {
                    Some(GIRO_DESLIZAR_IZQUIERDA + i)
                } else if antes[i + 1] && !g[i + 1] {
                    Some(GIRO_DESLIZAR_IZQUIERDA + i + 1)
                } else {
                    None
                };
                let sigue = if g[i] {
                    Some(GIRO_DESLIZAR_IZQUIERDA + i)
                } else if g[i + 1] {
                    Some(GIRO_DESLIZAR_IZQUIERDA + i + 1)
                } else {
                    None
                };
                [acaba, sigue]
            };
            for c in eje(0).into_iter().chain(eje(2)).flatten() {
                p.procesar(c);
            }
        }

        p.vaciar_cola();
        let (dx, dy) = (p.dx, p.dy);
        p.salida.volcar(&mut self.mapeado);
        self.macros.volcar(&mut self.mapeado);
        if a.volante.eje != EjeVolante::None {
            self.mapeado.volante = self.volante.escalar(
                ctx.crudo,
                &a.volante,
                ctx.sintetico.ahora_us,
                &mut self.salida_extras.luz,
            );
        }
        self.sticks.mezclar(&mut self.mapeado);

        // El ratón que han movido los controles.
        let (mx, my) = self.raton.final_movimiento(dx, dy);
        if mx != 0 || my != 0 {
            ctx.eventos.push(EventoRaton::Mover(mx, my));
        }
        self.raton.final_absoluto(&a.raton, ctx.eventos);

        // Lo que copia ControlService del estado de entrada.
        let m = &mut self.mapeado;
        m.movimiento = e.movimiento;
        m.marca_ds5 = e.marca_ds5;
        m.contador_tramas = e.contador_tramas;
        m.contador_panel = e.contador_panel;
        m.toques = e.toques;
        m.clone()
    }
}

/// El arreglo de `ControlService` para el DS4 virtual: el botón de cada gatillo se enciende en
/// cuanto se aprieta y, en modo botones, el gatillo analógico no sale.
pub fn arreglar_gatillos(e: &mut Estado, mando: MandoVirtual, modo: SalidaGatillos) {
    if mando != MandoVirtual::DS4 || modo == SalidaGatillos::Analog {
        return;
    }
    poner(e, boton::L2, e.l2 > 0);
    poner(e, boton::R2, e.r2 > 0);
    if modo == SalidaGatillos::Buttons {
        e.l2 = 0;
        e.r2 = 0;
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use ds5_perfiles::perfil::Ajustes;

    fn ajustes() -> AjustesBotones {
        let p = Perfil {
            ajustes: Ajustes {
                mando_virtual: MandoVirtual::X360,
                ..Default::default()
            },
            ..Default::default()
        };
        AjustesBotones::desde_perfil(&p)
    }

    fn reposo() -> Estado {
        Estado {
            lx: 128,
            ly: 128,
            rx: 128,
            ry: 128,
            ..Default::default()
        }
    }

    /// Con un dedo en el panel (el gatillo shift 26), el triángulo hace de B.
    #[test]
    fn el_shift_con_un_dedo_en_el_panel() {
        let mut a = ajustes();
        a.controles[TRIANGULO - 1].accion_shift = Accion::Boton(17);
        a.controles[TRIANGULO - 1].gatillo_shift = SHIFT_UN_DEDO;
        a.personalizado = true;
        let mut b = Botones::default();
        let mut e = reposo();
        e.botones = boton::TRIANGULO;
        let r = b.aplicar(&e, &Entradas::default(), &a, MandoVirtual::X360, 0);
        assert!(r.pulsado(boton::TRIANGULO) && !r.pulsado(boton::CIRCULO));
        e.botones |= boton::UN_DEDO;
        let r = b.aplicar(&e, &Entradas::default(), &a, MandoVirtual::X360, 0);
        assert!(!r.pulsado(boton::TRIANGULO) && r.pulsado(boton::CIRCULO));
    }

    /// Con el modo cadera, el gatillo no cuenta hasta pasado el tiempo; si se aprieta a fondo
    /// antes, solo sale la asignación de apretar a fondo.
    #[test]
    fn el_gatillo_de_cadera_espera_su_tiempo() {
        let mut a = ajustes();
        a.dos_fases_l2 = DosFases::HipFire;
        a.cadera_l2_ms = 100;
        a.controles[L2_A_FONDO - 1].accion = Accion::Boton(18);
        a.personalizado = true;
        let mut b = Botones::default();
        let mut e = reposo();
        e.l2 = 80;
        e.l2_crudo = 80;
        let paso = |b: &mut Botones, e: &Estado, t| {
            b.aplicar(e, &Entradas::default(), &a, MandoVirtual::X360, t)
        };
        assert_eq!(paso(&mut b, &e, 1000).l2, 0);
        assert_eq!(paso(&mut b, &e, 1050).l2, 0);
        assert_eq!(paso(&mut b, &e, 1101).l2, 80);
        // Suelto y a fondo de golpe: se espera otra vez y luego sale A, no el gatillo.
        e.l2 = 0;
        e.l2_crudo = 0;
        paso(&mut b, &e, 1200);
        e.l2 = 255;
        e.l2_crudo = 255;
        assert!(!paso(&mut b, &e, 1300).pulsado(boton::EQUIS));
        let r = paso(&mut b, &e, 1401);
        assert!(r.pulsado(boton::EQUIS));
        assert_eq!(r.l2, 0);
    }

    /// Los extras solo cambian la luz si el control tiene una macro de luz y no está activa.
    #[test]
    fn la_luz_de_los_extras_necesita_una_macro_de_luz_apagada() {
        assert!(!macro_luz_inactiva(None));
        assert!(!macro_luz_inactiva(Some("True/255,0,0:100/Press/False")));
        assert!(macro_luz_inactiva(Some("False/255,0,0:100/Press/False")));
        assert!(macro_luz_inactiva(Some("True//Press/False")));
        assert!(macro_luz_inactiva(Some("basura")));
    }

    /// Una tecla en alterna: se pulsa con la primera pulsación y se suelta con la segunda.
    #[test]
    fn la_tecla_en_alterna() {
        let mut a = ajustes();
        a.controles[EQUIS - 1].accion = Accion::Tecla(65);
        a.controles[EQUIS - 1].tipo_tecla = TECLA_ALTERNA;
        a.personalizado = true;
        let mut b = Botones::default();
        let mut s = Sintetico::default();
        let mut pulsado = reposo();
        pulsado.botones = boton::EQUIS;
        let mut todo = Vec::new();
        for e in [&pulsado, &reposo(), &pulsado, &reposo()] {
            let mut eventos = Vec::new();
            let mut ctx = Contexto {
                sintetico: &mut s,
                eventos: &mut eventos,
                crudo: e,
                crudo_antes: e,
                transcurrido_ms: 0.0,
            };
            b.aplicar_con(e, &Entradas::default(), &a, MandoVirtual::X360, &mut ctx);
            s.confirmar(&mut eventos);
            todo.push(eventos);
        }
        assert_eq!(
            todo,
            vec![
                vec![EventoRaton::Tecla(65)],
                vec![],
                vec![EventoRaton::SueltaTecla(65)],
                vec![]
            ]
        );
    }

    /// Pasa una serie de estados por la reasignación y el teclado, y da los eventos de cada uno.
    fn pasar(
        b: &mut Botones,
        a: &AjustesBotones,
        pasos: &[(Estado, Entradas)],
    ) -> Vec<Vec<EventoRaton>> {
        let mut s = Sintetico::default();
        let mut todo = Vec::new();
        for (e, o) in pasos {
            let mut eventos = Vec::new();
            let mut ctx = Contexto {
                sintetico: &mut s,
                eventos: &mut eventos,
                crudo: e,
                crudo_antes: e,
                transcurrido_ms: 4.0,
            };
            b.aplicar_con(e, o, a, MandoVirtual::X360, &mut ctx);
            s.confirmar(&mut eventos);
            todo.push(eventos);
        }
        todo
    }

    /// Si dos controles mueven el ratón absoluto en el mismo eje, manda el primero que se procesa
    /// (el círculo va antes que la equis).
    #[test]
    fn el_raton_absoluto_lo_pone_el_primer_control() {
        let mut a = ajustes();
        a.controles[CIRCULO - 1].accion = Accion::Boton(SALIDA_ABSOLUTO_ARRIBA);
        a.controles[EQUIS - 1].accion = Accion::Boton(SALIDA_ABSOLUTO_ARRIBA + 1);
        a.personalizado = true;
        let mut e = reposo();
        e.botones = boton::CIRCULO | boton::EQUIS;
        let r = pasar(&mut Botones::default(), &a, &[(e, Entradas::default())]);
        assert_eq!(r[0], vec![EventoRaton::Absoluto(0.5, 0.0)]);
    }

    /// Los extras cambian la sensibilidad del ratón mientras se mantiene el control y la
    /// devuelven al soltarlo.
    #[test]
    fn los_extras_cambian_la_sensibilidad_del_raton() {
        let mut a = ajustes();
        a.controles[CIRCULO - 1].extras = Some("0,0,0,0,0,0,0,1,80".into());
        a.controles[EQUIS - 1].accion = Accion::Boton(SALIDA_RATON_DERECHA);
        a.personalizado = true;
        let mut normal = reposo();
        normal.botones = boton::EQUIS;
        let mut rapido = normal.clone();
        rapido.botones |= boton::CIRCULO;
        let mut pasos = Vec::new();
        for e in [&normal, &rapido, &normal] {
            pasos.extend(std::iter::repeat_n((e.clone(), Entradas::default()), 10));
        }
        let r = pasar(&mut Botones::default(), &a, &pasos);
        let x: Vec<i32> = r
            .chunks(10)
            .map(|t| {
                t.iter()
                    .flatten()
                    .map(|e| match e {
                        EventoRaton::Mover(x, _) => *x,
                        _ => 0,
                    })
                    .sum()
            })
            .collect();
        assert!(x[1] > 2 * x[0] && x[2] < x[1], "{x:?}");
    }

    /// Con la misma tecla en alterna para los dos lados, pasar del gesto a la izquierda al de la
    /// derecha en un informe suelta primero el que acaba, así que el otro vuelve a alternarla.
    #[test]
    fn el_gesto_que_acaba_se_suelta_antes_del_que_sigue() {
        let mut a = ajustes();
        for c in [GIRO_DESLIZAR_IZQUIERDA, GIRO_DESLIZAR_DERECHA] {
            a.controles[c - 1].accion = Accion::Tecla(65);
            a.controles[c - 1].tipo_tecla = TECLA_ALTERNA;
        }
        a.giro_gestos = true;
        a.personalizado = true;
        let gesto = |ahora: [bool; 4], antes: [bool; 4]| Entradas {
            giro_deslizar: ahora,
            giro_deslizar_antes: antes,
            ..Default::default()
        };
        let (n, i, d) = (
            [false; 4],
            [true, false, false, false],
            [false, true, false, false],
        );
        let r = pasar(
            &mut Botones::default(),
            &a,
            &[
                (reposo(), gesto(i, n)),
                (reposo(), gesto(d, i)),
                (reposo(), gesto(n, d)),
                (reposo(), gesto(i, n)),
            ],
        );
        // La tecla no se suelta entre medias, así que C# no vuelve a pulsarla ni a soltarla
        // hasta el siguiente gesto; entonces se ve que el de la derecha la ha alternado.
        assert_eq!(
            r,
            vec![
                vec![EventoRaton::Tecla(65)],
                vec![],
                vec![],
                vec![EventoRaton::Tecla(65)]
            ]
        );
    }

    /// Para el DS4 virtual el botón del gatillo se enciende en cuanto se aprieta; en modo
    /// botones, además, el gatillo analógico no sale.
    #[test]
    fn el_arreglo_de_los_gatillos_del_ds4() {
        let mut e = reposo();
        e.l2 = 3;
        arreglar_gatillos(&mut e, MandoVirtual::DS4, SalidaGatillos::Default);
        assert!(e.pulsado(boton::L2) && e.l2 == 3);
        arreglar_gatillos(&mut e, MandoVirtual::DS4, SalidaGatillos::Buttons);
        assert!(e.pulsado(boton::L2) && e.l2 == 0);
        let mut x = reposo();
        x.l2 = 3;
        arreglar_gatillos(&mut x, MandoVirtual::X360, SalidaGatillos::Default);
        assert!(!x.pulsado(boton::L2));
    }
}
