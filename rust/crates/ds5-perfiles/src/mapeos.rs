//! Las asignaciones de cada control del mando (`Control` y `ShiftControl`), como las leen y las
//! guardan `ProfileDTO.MapTo` y `MapFrom`.
//!
//! Al guardar, los controles van siempre en el orden del enum `DS5Controls`, no en el del archivo.

use crate::numeros;
use crate::xml::{Elemento, Escritor};

/// Los controles del mando, en el orden de `DS5Controls` (sin `None`).
pub const CONTROLES: [&str; 55] = [
    "LXNeg",
    "LXPos",
    "LYNeg",
    "LYPos",
    "RXNeg",
    "RXPos",
    "RYNeg",
    "RYPos",
    "L1",
    "L2",
    "L3",
    "R1",
    "R2",
    "R3",
    "Square",
    "Triangle",
    "Circle",
    "Cross",
    "DpadUp",
    "DpadRight",
    "DpadDown",
    "DpadLeft",
    "PS",
    "TouchLeft",
    "TouchUpper",
    "TouchMulti",
    "TouchRight",
    "Share",
    "Options",
    "Mute",
    "FnL",
    "FnR",
    "BLP",
    "BRP",
    "GyroXPos",
    "GyroXNeg",
    "GyroZPos",
    "GyroZNeg",
    "SwipeLeft",
    "SwipeRight",
    "SwipeUp",
    "SwipeDown",
    "L2FullPull",
    "R2FullPull",
    "GyroSwipeLeft",
    "GyroSwipeRight",
    "GyroSwipeUp",
    "GyroSwipeDown",
    "Capture",
    "SideL",
    "SideR",
    "LSOuter",
    "RSOuter",
    "TouchStarted",
    "TouchEnded",
];

/// Los botones de salida, en el orden de `X360Controls`.
const SALIDAS: [&str; 43] = [
    "None",
    "LXNeg",
    "LXPos",
    "LYNeg",
    "LYPos",
    "RXNeg",
    "RXPos",
    "RYNeg",
    "RYPos",
    "LB",
    "LT",
    "LS",
    "RB",
    "RT",
    "RS",
    "X",
    "Y",
    "B",
    "A",
    "DpadUp",
    "DpadRight",
    "DpadDown",
    "DpadLeft",
    "Guide",
    "Back",
    "Start",
    "TouchpadClick",
    "LeftMouse",
    "RightMouse",
    "MiddleMouse",
    "FourthMouse",
    "FifthMouse",
    "WUP",
    "WDOWN",
    "MouseUp",
    "MouseDown",
    "MouseLeft",
    "MouseRight",
    "AbsMouseUp",
    "AbsMouseDown",
    "AbsMouseLeft",
    "AbsMouseRight",
    "Unbound",
];

/// El texto con que se guarda cada botón de salida (`getX360ControlString`). `None` no tiene.
const TEXTOS_SALIDA: [&str; 43] = [
    "",
    "Left X-Axis-",
    "Left X-Axis+",
    "Left Y-Axis-",
    "Left Y-Axis+",
    "Right X-Axis-",
    "Right X-Axis+",
    "Right Y-Axis-",
    "Right Y-Axis+",
    "Left Bumper",
    "Left Trigger",
    "Left Stick",
    "Right Bumper",
    "Right Trigger",
    "Right Stick",
    "X Button",
    "Y Button",
    "B Button",
    "A Button",
    "Up Button",
    "Right Button",
    "Down Button",
    "Left Button",
    "Guide",
    "Back",
    "Start",
    "Touchpad Click",
    "Left Mouse Button",
    "Right Mouse Button",
    "Middle Mouse Button",
    "4th Mouse Button",
    "5th Mouse Button",
    "Mouse Wheel Up",
    "Mouse Wheel Down",
    "Mouse Up",
    "Mouse Down",
    "Mouse Left",
    "Mouse Right",
    "Abs Mouse Up",
    "Abs Mouse Down",
    "Abs Mouse Left",
    "Abs Mouse Right",
    "Unbound",
];

/// El botón de salida `Unbound`: el control no hace nada.
pub const SIN_ASIGNAR: u8 = 42;

/// Los indicadores de `DS5KeyType`, en orden de valor.
const TIPOS_TECLA: [(&str, u8); 6] = [
    ("ScanCode", 1),
    ("Toggle", 2),
    ("Unbound", 4),
    ("Macro", 8),
    ("HoldMacro", 16),
    ("RepeatMacro", 32),
];
pub const TIPO_SIN_ASIGNAR: u8 = 4;

/// Lo que hace un control.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Accion {
    /// Lo que hace por defecto.
    #[default]
    Predeterminada,
    /// Un botón del mando virtual o del ratón, por su número en `X360Controls`.
    Boton(u8),
    /// Una tecla, por su código.
    Tecla(u16),
    /// Una macro: teclas y esperas.
    Macro(Vec<i32>),
}

/// La asignación de un control, normal y con la capa shift.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AjusteControl {
    pub accion: Accion,
    pub extras: Option<String>,
    pub tipo_tecla: u8,
    pub macro_luz: Option<String>,
    pub accion_shift: Accion,
    /// El botón que activa la capa shift; 0 si no hay.
    pub gatillo_shift: i32,
    pub tipo_tecla_shift: u8,
}

/// Las asignaciones de los 55 controles, en el orden de `CONTROLES`.
#[derive(Debug, Clone, PartialEq)]
pub struct Mapeos {
    pub controles: Vec<AjusteControl>,
}

impl Default for Mapeos {
    fn default() -> Self {
        Mapeos {
            controles: vec![AjusteControl::default(); CONTROLES.len()],
        }
    }
}

/// `Enum.TryParse` de .NET: nombres sin distinguir mayúsculas o números, y varios separados por
/// comas se combinan.
fn analizar_enum(texto: &str, nombres: &[&str]) -> Option<u8> {
    let mut valor = 0u8;
    for parte in texto.split(',') {
        let parte = parte.trim();
        let v = if parte.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '+') {
            numeros::entero_en::<u8>(parte)?
        } else {
            nombres.iter().position(|n| n.eq_ignore_ascii_case(parte))? as u8
        };
        valor |= v;
    }
    Some(valor)
}

/// El nombre de un botón de salida en el enum `X360Controls` (`ToString()`).
pub fn nombre_salida(v: u8) -> Option<&'static str> {
    SALIDAS.get(v as usize).copied()
}

/// El número de un botón de salida por su nombre en el enum (`Enum.TryParse`).
pub fn salida_por_nombre_enum(nombre: &str) -> Option<u8> {
    SALIDAS.iter().position(|s| *s == nombre).map(|i| i as u8)
}

/// `getX360ControlsByName`.
fn salida_por_nombre(texto: &str) -> u8 {
    if !texto.trim().is_empty() {
        if let Some(v) = analizar_enum(texto, &SALIDAS) {
            return v;
        }
    }
    TEXTOS_SALIDA
        .iter()
        .skip(1)
        .position(|t| *t == texto)
        .map_or(SIN_ASIGNAR, |i| i as u8 + 1)
}

/// `getX360ControlString`.
fn texto_salida(v: u8) -> &'static str {
    match TEXTOS_SALIDA.get(v as usize) {
        Some(t) if !t.is_empty() => t,
        _ => "Unbound",
    }
}

/// El índice de un control por el nombre de su elemento. Como en C#, `None` es un nombre
/// válido que no corresponde a ningún control.
fn control(nombre: &str) -> Option<Option<usize>> {
    if nombre == "None" {
        return Some(None);
    }
    CONTROLES.iter().position(|c| *c == nombre).map(Some)
}

/// Una de las secciones de `Control` o `ShiftControl` (`Button`, `Key`...), con sus entradas en
/// el orden del archivo y el atributo `Trigger` de cada una.
struct Seccion<T> {
    entradas: Vec<(Option<usize>, T)>,
    gatillos: Vec<(Option<usize>, i32)>,
}

impl<T> Seccion<T> {
    fn gatillo(&self, i: Option<usize>) -> i32 {
        self.gatillos
            .iter()
            .find(|(c, _)| *c == i)
            .map_or(0, |(_, t)| *t)
    }
}

/// Lee una sección. `valor` decide si la entrada vale y qué guarda; una entrada repetida hace
/// que C# rechace el perfil.
fn leer_seccion<T>(
    padre: Option<&Elemento>,
    nombre: &str,
    valor: impl Fn(&Elemento) -> Option<T>,
) -> Result<Seccion<T>, String> {
    let mut seccion = Seccion {
        entradas: Vec::new(),
        gatillos: Vec::new(),
    };
    let Some(e) = padre.and_then(|p| p.hijo(nombre)) else {
        return Ok(seccion);
    };
    for hijo in e.elementos() {
        let Some(v) = valor(hijo) else { continue };
        let Some(i) = control(&hijo.nombre) else {
            continue;
        };
        if let Some(t) = hijo.atributo("Trigger") {
            if !seccion.gatillos.iter().any(|(c, _)| *c == i) {
                seccion
                    .gatillos
                    .push((i, numeros::entero_en::<i32>(t).unwrap_or(0)));
            }
        }
        if seccion.entradas.iter().any(|(c, _)| *c == i) {
            return Err(format!("{nombre}/{} está repetido", hijo.nombre));
        }
        seccion.entradas.push((i, v));
    }
    Ok(seccion)
}

/// Las seis secciones de `Control` o `ShiftControl`.
struct Secciones {
    botones: Seccion<u8>,
    teclas: Seccion<u16>,
    macros: Seccion<String>,
    extras: Seccion<String>,
    tipos: Seccion<u8>,
    macros_luz: Seccion<String>,
}

fn leer_secciones(e: Option<&Elemento>) -> Result<Secciones, String> {
    let texto_no_vacio = |h: &Elemento| Some(h.texto()).filter(|t| !t.is_empty());
    Ok(Secciones {
        botones: leer_seccion(e, "Button", |h| Some(salida_por_nombre(&h.texto())))?,
        teclas: leer_seccion(e, "Key", |h| numeros::entero_en::<u16>(&h.texto()))?,
        macros: leer_seccion(e, "Macro", |h| Some(h.texto()))?,
        extras: leer_seccion(e, "Extras", texto_no_vacio)?,
        tipos: leer_seccion(e, "KeyType", |h| {
            let texto = h.texto();
            let tipo = TIPOS_TECLA
                .iter()
                .filter(|(n, _)| texto.contains(n))
                .fold(0, |t, (_, v)| t | v);
            Some(tipo).filter(|t| *t != 0)
        })?,
        macros_luz: leer_seccion(e, "LightbarMacro", texto_no_vacio)?,
    })
}

/// Una macro `16/65/65/16`. Un número que no se entiende hace fallar la carga en C#.
fn analizar_macro(texto: &str) -> Result<Vec<i32>, String> {
    if texto.is_empty() {
        return Ok(Vec::new());
    }
    texto
        .split('/')
        .map(|p| numeros::entero_en::<i32>(p).ok_or_else(|| format!("macro no válida: {texto:?}")))
        .collect()
}

impl Mapeos {
    /// Aplica las secciones en el mismo orden que `MapTo`: botones, teclas y macros (que borran
    /// los extras y el tipo de tecla), luego extras, tipos de tecla y macros de luz.
    pub fn leer(control: Option<&Elemento>, shift: Option<&Elemento>) -> Result<Mapeos, String> {
        let mut mapeos = Mapeos::default();
        let normal = leer_secciones(control)?;
        let capa = leer_secciones(shift)?;
        let c = &mut mapeos.controles;

        let mut poner = |i: Option<usize>, accion: Accion| {
            if let Some(i) = i {
                c[i].accion = accion;
                c[i].extras = Some(String::new());
                c[i].tipo_tecla = 0;
            }
        };
        for (i, b) in &normal.botones.entradas {
            poner(*i, Accion::Boton(*b));
        }
        for (i, t) in &normal.teclas.entradas {
            poner(*i, Accion::Tecla(*t));
        }
        for (i, m) in &normal.macros.entradas {
            poner(*i, Accion::Macro(analizar_macro(m)?));
        }
        for (i, x) in normal
            .extras
            .entradas
            .iter()
            .filter_map(|(i, x)| i.map(|i| (i, x)))
        {
            c[i].extras = Some(x.clone());
        }
        for (i, t) in normal
            .tipos
            .entradas
            .iter()
            .filter_map(|(i, t)| i.map(|i| (i, t)))
        {
            c[i].tipo_tecla = *t;
        }
        for (i, m) in normal
            .macros_luz
            .entradas
            .iter()
            .filter_map(|(i, m)| i.map(|i| (i, m)))
        {
            c[i].macro_luz = Some(m.clone());
        }

        let mut poner_shift = |i: Option<usize>, accion: Accion, gatillo: i32| {
            if let (Some(i), true) = (i, gatillo > 0) {
                c[i].accion_shift = accion;
                c[i].tipo_tecla_shift = 0;
                c[i].gatillo_shift = gatillo;
            }
        };
        for (i, b) in &capa.botones.entradas {
            poner_shift(*i, Accion::Boton(*b), capa.botones.gatillo(*i));
        }
        for (i, t) in &capa.teclas.entradas {
            poner_shift(*i, Accion::Tecla(*t), capa.teclas.gatillo(*i));
        }
        for (i, m) in &capa.macros.entradas {
            let teclas = analizar_macro(m)?;
            poner_shift(*i, Accion::Macro(teclas), capa.macros.gatillo(*i));
        }
        // Un fallo heredado: los extras de la capa shift se guardan en los normales.
        for (i, x) in capa
            .extras
            .entradas
            .iter()
            .filter_map(|(i, x)| i.map(|i| (i, x)))
        {
            c[i].extras = Some(x.clone());
        }
        for (i, t) in capa
            .tipos
            .entradas
            .iter()
            .filter_map(|(i, t)| i.map(|i| (i, t)))
        {
            c[i].tipo_tecla_shift = *t;
        }
        Ok(mapeos)
    }

    /// `Control` y `ShiftControl`, como `MapFrom`.
    pub fn escribir(&self, w: &mut Escritor) {
        let mut botones = Vec::new();
        let mut teclas = Vec::new();
        let mut macros = Vec::new();
        let mut extras = Vec::new();
        let mut tipos = Vec::new();
        let mut macros_luz = Vec::new();
        let mut botones_shift = Vec::new();
        let mut teclas_shift = Vec::new();
        let mut macros_shift = Vec::new();
        let mut tipos_shift = Vec::new();

        for (i, a) in self.controles.iter().enumerate() {
            let nombre = CONTROLES[i];
            if a.accion != Accion::Predeterminada {
                let mut tipo = a.tipo_tecla;
                if let Accion::Boton(SIN_ASIGNAR) = a.accion {
                    tipo |= TIPO_SIN_ASIGNAR;
                }
                if tipo != 0 {
                    tipos.push((nombre, tipo));
                }
                match &a.accion {
                    Accion::Boton(b) => botones.push((nombre, texto_salida(*b).to_string())),
                    Accion::Tecla(t) => teclas.push((nombre, t.to_string())),
                    Accion::Macro(m) => macros.push((nombre, unir_macro(m))),
                    Accion::Predeterminada => {}
                }
            }
            if let Some(x) = &a.extras {
                if !x.is_empty() && x.split(',').any(|p| p != "0") {
                    extras.push((nombre, x.clone()));
                }
            }
            if let Some(m) = a.macro_luz.as_ref().filter(|m| !m.is_empty()) {
                macros_luz.push((nombre, m.clone()));
            }
            if a.accion_shift != Accion::Predeterminada && a.gatillo_shift > 0 {
                let mut tipo = a.tipo_tecla_shift;
                if let Accion::Boton(SIN_ASIGNAR) = a.accion_shift {
                    tipo |= TIPO_SIN_ASIGNAR;
                }
                if tipo != 0 {
                    tipos_shift.push((nombre, tipo));
                }
                let g = a.gatillo_shift;
                match &a.accion_shift {
                    Accion::Boton(b) => {
                        botones_shift.push((nombre, g, texto_salida(*b).to_string()))
                    }
                    Accion::Tecla(t) => teclas_shift.push((nombre, g, t.to_string())),
                    Accion::Macro(m) => macros_shift.push((nombre, g, unir_macro(m))),
                    Accion::Predeterminada => {}
                }
            }
        }

        let sin_gatillo = |v: Vec<(&'static str, String)>| -> Vec<(&'static str, i32, String)> {
            v.into_iter().map(|(n, t)| (n, 0, t)).collect()
        };
        let tipos: Vec<_> = tipos.into_iter().map(|(n, t)| (n, texto_tipo(t))).collect();
        let tipos_shift: Vec<_> = tipos_shift
            .into_iter()
            .map(|(n, t)| (n, texto_tipo(t)))
            .collect();

        w.abrir("Control", &[]);
        escribir_seccion(w, "Button", &sin_gatillo(botones));
        escribir_seccion(w, "Key", &sin_gatillo(teclas));
        escribir_seccion(w, "Macro", &sin_gatillo(macros));
        escribir_seccion(w, "Extras", &sin_gatillo(extras));
        escribir_seccion(w, "KeyType", &sin_gatillo(tipos));
        escribir_seccion(w, "LightbarMacro", &sin_gatillo(macros_luz));
        w.cerrar();
        w.abrir("ShiftControl", &[]);
        escribir_seccion(w, "Button", &botones_shift);
        escribir_seccion(w, "Key", &teclas_shift);
        escribir_seccion(w, "Macro", &macros_shift);
        escribir_seccion(w, "KeyType", &sin_gatillo(tipos_shift));
        w.cerrar();
    }
}

fn unir_macro(m: &[i32]) -> String {
    m.iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join("/")
}

/// Los indicadores de tipo de tecla pegados, en orden de valor (`MacroRepeatMacro`).
fn texto_tipo(t: u8) -> String {
    TIPOS_TECLA
        .iter()
        .filter(|(_, v)| t & v != 0)
        .map(|(n, _)| *n)
        .collect()
}

fn escribir_seccion(w: &mut Escritor, nombre: &str, entradas: &[(&str, i32, String)]) {
    if entradas.is_empty() {
        return;
    }
    w.abrir(nombre, &[]);
    for (control, gatillo, texto) in entradas {
        if *gatillo > 0 {
            w.hoja(control, &[("Trigger", &gatillo.to_string())], texto);
        } else {
            w.hoja(control, &[], texto);
        }
    }
    w.cerrar();
}
