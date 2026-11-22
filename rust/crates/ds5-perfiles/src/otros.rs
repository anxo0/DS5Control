//! Los demás archivos de ajustes: las acciones especiales (`Actions.xml`), los autoperfiles
//! (`Auto Profiles.xml`) y los perfiles fijos por mando (`LinkedProfiles.xml`).

use crate::numeros;
use crate::xml::{Elemento, Escritor};

fn empezar(raiz: &str) -> Escritor {
    let mut w = Escritor::new();
    w.crudo("<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n\r\n");
    w.abrir(raiz, &[]);
    w
}

fn leer_raiz(texto: &str, nombre: &str) -> Result<Elemento, String> {
    let raiz = Elemento::leer(texto).map_err(|e| e.0)?;
    if raiz.nombre != nombre {
        return Err(format!("el elemento raíz es {}", raiz.nombre));
    }
    Ok(raiz)
}

/// Los tipos de acción que conoce la app. Una acción de otro tipo se pierde al guardar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoAccion {
    Macro,
    Programa,
    Perfil,
    Tecla,
    DesconectarBt,
    Bateria,
    Multiple,
    CalibrarVolante,
    CalibrarGiroscopio,
}

impl TipoAccion {
    const NOMBRES: [(&'static str, TipoAccion); 9] = [
        ("Macro", TipoAccion::Macro),
        ("Program", TipoAccion::Programa),
        ("Profile", TipoAccion::Perfil),
        ("Key", TipoAccion::Tecla),
        ("DisconnectBT", TipoAccion::DesconectarBt),
        ("BatteryCheck", TipoAccion::Bateria),
        ("MultiAction", TipoAccion::Multiple),
        (
            "SASteeringWheelEmulationCalibrate",
            TipoAccion::CalibrarVolante,
        ),
        ("GyroCalibration", TipoAccion::CalibrarGiroscopio),
    ];

    fn desde(nombre: &str) -> Option<TipoAccion> {
        Self::NOMBRES
            .iter()
            .find(|(n, _)| *n == nombre)
            .map(|(_, t)| *t)
    }

    pub fn nombre(self) -> &'static str {
        Self::NOMBRES
            .iter()
            .find(|(_, t)| *t == self)
            .map(|(n, _)| *n)
            .unwrap()
    }
}

/// Una acción especial, con lo que guarda cada tipo.
#[derive(Debug, Clone, PartialEq)]
pub struct AccionEspecial {
    pub nombre: String,
    /// Los controles que la activan, separados por `/`.
    pub disparador: String,
    pub tipo: TipoAccion,
    /// Lo que hace: la macro, el programa, el perfil, la tecla... `None` en la calibración del
    /// volante, que no lo guarda. La tecla va tal cual la trae el archivo (`65 Scan Code`): C# lee
    /// de ahí si va por código de exploración, pero al guardar solo escribe el número.
    pub detalles: Option<String>,
    /// Las opciones de la macro, o los controles que deshacen la acción múltiple.
    pub extras: String,
    /// Los argumentos del programa.
    pub argumentos: String,
    /// El retardo (`Delay`) en segundos. Solo lo guarda el programa, pero C# también lo lee en la
    /// acción múltiple y en la calibración del volante.
    pub retardo: f64,
    /// Los controles que deshacen la acción de perfil o de tecla.
    pub deshacer: String,
    /// Cuándo se suelta la tecla: `Press` o `Release`.
    pub estilo_deshacer: String,
}

/// `Actions.xml`, como `ActionsDTO.cs`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Acciones {
    pub acciones: Vec<AccionEspecial>,
}

impl Acciones {
    pub fn leer(texto: &str) -> Result<Acciones, String> {
        let raiz = leer_raiz(texto, "Actions")?;
        let mut acciones = Vec::new();
        for e in raiz.elementos().filter(|e| e.nombre == "Action") {
            let campo = |n: &str| e.hijo(n).map(|h| h.texto_campo()).unwrap_or_default();
            let Some(tipo) = TipoAccion::desde(&campo("Type")) else {
                continue;
            };
            let mut accion = AccionEspecial {
                nombre: e.atributo("Name").unwrap_or_default().to_string(),
                disparador: campo("Trigger"),
                tipo,
                detalles: Some(campo("Details")),
                extras: String::new(),
                argumentos: String::new(),
                retardo: 0.0,
                deshacer: String::new(),
                estilo_deshacer: String::new(),
            };
            let retardo = e
                .hijo("Delay")
                .and_then(|h| numeros::doble_xml(&h.texto()))
                .unwrap_or(0.0);
            if matches!(tipo, TipoAccion::Multiple | TipoAccion::CalibrarVolante) {
                accion.retardo = retardo;
            }
            match tipo {
                TipoAccion::Macro | TipoAccion::Multiple => accion.extras = campo("Extras"),
                TipoAccion::Programa => {
                    accion.argumentos = campo("Arguements");
                    accion.retardo = retardo;
                }
                TipoAccion::Perfil => accion.deshacer = campo("UnloadTrigger"),
                TipoAccion::Tecla => {
                    accion.estilo_deshacer = campo("UnloadStyle");
                    accion.deshacer = campo("UnloadTrigger");
                }
                // Los colores se leen separados por `|` o por comas y se guardan con `|`.
                TipoAccion::Bateria => accion.detalles = Some(campo("Details").replace(',', "|")),
                TipoAccion::CalibrarVolante => accion.detalles = None,
                TipoAccion::DesconectarBt | TipoAccion::CalibrarGiroscopio => {}
            }
            acciones.push(accion);
        }
        Ok(Acciones { acciones })
    }

    pub fn escribir(&self) -> String {
        let mut w = empezar("Actions");
        for a in &self.acciones {
            w.abrir("Action", &[("Name", &a.nombre)]);
            w.hoja("Trigger", &[], &a.disparador);
            w.hoja("Type", &[], a.tipo.nombre());
            if let Some(d) = &a.detalles {
                let d = if a.tipo == TipoAccion::Tecla {
                    d.split(' ').next().unwrap_or("")
                } else {
                    d
                };
                w.hoja("Details", &[], d);
            }
            let tipo = a.tipo;
            if matches!(tipo, TipoAccion::Macro | TipoAccion::Multiple) && !a.extras.is_empty() {
                w.hoja("Extras", &[], &a.extras);
            }
            if tipo == TipoAccion::Programa {
                if !a.argumentos.is_empty() {
                    w.hoja("Arguements", &[], &a.argumentos);
                }
                w.hoja("Delay", &[], &numeros::formato_doble(a.retardo));
            }
            if matches!(tipo, TipoAccion::Perfil | TipoAccion::Tecla) && !a.deshacer.is_empty() {
                w.hoja("UnloadTrigger", &[], &a.deshacer);
            }
            if tipo == TipoAccion::Tecla && !a.estilo_deshacer.is_empty() {
                w.hoja("UnloadStyle", &[], &a.estilo_deshacer);
            }
            w.cerrar();
        }
        w.cerrar();
        w.terminar()
    }
}

/// Un programa con autoperfil.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Autoperfil {
    pub ruta: String,
    pub titulo: String,
    /// El perfil de cada uno de los 8 mandos (`(none)` o vacío si no cambia).
    pub perfiles: [String; 8],
    /// Apaga DS5Control mientras el programa está abierto.
    pub apagar: bool,
}

/// `Auto Profiles.xml`, como `AutoProfilesDTO.cs`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Autoperfiles {
    pub programas: Vec<Autoperfil>,
}

impl Autoperfiles {
    pub fn leer(texto: &str) -> Result<Autoperfiles, String> {
        let raiz = leer_raiz(texto, "Programs")?;
        let mut programas = Vec::new();
        for e in raiz.elementos().filter(|e| e.nombre == "Program") {
            let mut p = Autoperfil {
                ruta: e.atributo("path").unwrap_or_default().to_string(),
                titulo: e.atributo("title").unwrap_or_default().to_string(),
                ..Default::default()
            };
            for (i, perfil) in p.perfiles.iter_mut().enumerate() {
                if let Some(h) = e.hijo(&format!("Controller{}", i + 1)) {
                    *perfil = h.texto_campo();
                }
            }
            if let Some(h) = e.hijo("TurnOff") {
                p.apagar = numeros::bool_net(&h.texto()).unwrap_or(false);
            }
            programas.push(p);
        }
        Ok(Autoperfiles { programas })
    }

    pub fn escribir(&self) -> String {
        let mut w = empezar("Programs");
        for p in &self.programas {
            w.abrir("Program", &[("path", &p.ruta), ("title", &p.titulo)]);
            for (i, perfil) in p.perfiles.iter().enumerate() {
                w.hoja(&format!("Controller{}", i + 1), &[], perfil);
            }
            w.hoja("TurnOff", &[], numeros::formato_bool(p.apagar));
            w.cerrar();
        }
        w.cerrar();
        w.terminar()
    }
}

/// `LinkedProfiles.xml`: el perfil fijo de cada mando por su dirección MAC, como
/// `LinkedProfilesDTO.cs`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Enlazados {
    /// Dirección sin `MAC` delante y perfil, en el orden del archivo.
    pub mandos: Vec<(String, String)>,
}

impl Enlazados {
    pub fn leer(texto: &str) -> Result<Enlazados, String> {
        let raiz = leer_raiz(texto, "LinkedControllers")?;
        let mut mandos: Vec<(String, String)> = Vec::new();
        for e in raiz.elementos().filter(|e| e.nombre.contains("MAC")) {
            let serie = e.nombre.replace("MAC", "");
            let perfil = e.texto();
            // Como en un diccionario: una dirección repetida se queda en su sitio con el último
            // perfil.
            match mandos.iter_mut().find(|(s, _)| *s == serie) {
                Some(m) => m.1 = perfil,
                None => mandos.push((serie, perfil)),
            }
        }
        Ok(Enlazados { mandos })
    }

    pub fn escribir(&self) -> String {
        let mut w = empezar("LinkedControllers");
        for (serie, perfil) in &self.mandos {
            w.hoja(&format!("MAC{}", serie.replace("MAC", "")), &[], perfil);
        }
        w.cerrar();
        w.terminar()
    }
}
