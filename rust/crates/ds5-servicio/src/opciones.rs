//! Los ajustes avanzados del perfil, como `ProfileOptions.cs`: todos descritos como datos para
//! que una sola pantalla genérica de la interfaz los enseñe (`pedirOpciones` y el mensaje
//! `opciones` con sus `grupos`) y los cambie (`opcion`).
//!
//! C# lee y escribe cada valor a través de `ProfileSettingsViewModel`, que trabaja con los
//! valores en memoria; aquí los valores son los del archivo (`Perfil`). Para cada ajuste se pasa
//! del archivo a la memoria como al cargar el perfil, de la memoria a lo que enseña la ventana
//! como el modelo de vista, y al cambiarlo se hace el camino de vuelta igual que al guardar
//! (`ProfileDTO.MapFrom`), con los mismos límites, redondeos y rarezas.

use std::f64::consts::PI;

use serde_json::{json, Value};

use ds5_perfiles::numeros;
use ds5_perfiles::perfil::{
    AceleracionPalanca, Ajustes, DosFases, EjeDeslizar, EjeVolante, EjesSalida, EmulacionVibracion,
    FlickStick, ModoLuz, ModoPalanca, PalancaSalida, SalidaGiroscopio, SalidaPanel,
    SuavizadoGiroscopio, SuavizadoPanel, TipoZonaMuerta,
};
use ds5_perfiles::{Curva, MetodoSuavizado, Perfil};

use crate::idioma::Idioma;

const CURVAS: &[&str] = &[
    "Lineal",
    "Más precisión",
    "Cuadrática",
    "Cúbica",
    "Suave cuadrática",
    "Suave cúbica",
    "Personalizada",
];
const CONDICIONES: &[&str] = &["Todos a la vez", "Cualquiera"];
const SUAVIZADOS: &[&str] = &["Filtro 1€", "Media ponderada"];
const GIROS: &[&str] = &["Girar", "Inclinar"];
const STICKS_SALIDA: &[&str] = &["Ninguno", "Izquierdo", "Derecho"];
const EJES_SALIDA: &[&str] = &["Ninguno", "Ambos", "Solo horizontal", "Solo vertical"];

/// Los botones que activan el giroscopio, por su índice en la lista de gatillos.
const BOTONES: &[&str] = &[
    "Cruz",
    "Círculo",
    "Cuadrado",
    "Triángulo",
    "L1",
    "L2",
    "R1",
    "R2",
    "Arriba",
    "Abajo",
    "Izquierda",
    "Derecha",
    "L3",
    "R3",
    "Dedo en el panel",
    "Dos dedos en el panel",
    "Options",
    "Crear",
    "PS",
    "Clic del panel",
    "Mute",
    "Lateral izquierdo",
    "Lateral derecho",
    "Función izquierda",
    "Función derecha",
    "Palanca izquierda",
    "Palanca derecha",
];

/// Las curvas en el orden de su índice (`stickOutputCurveString`).
const ORDEN_CURVAS: [Curva; 7] = [
    Curva::Lineal,
    Curva::PrecisionMejorada,
    Curva::Cuadratica,
    Curva::Cubica,
    Curva::SalidaCuadratica,
    Curva::SalidaCubica,
    Curva::Personalizada,
];

/// Los grados de giro del volante (`saSteeringRangeValues`).
const RANGOS_VOLANTE: [i32; 9] = [90, 180, 270, 360, 450, 720, 900, 1080, 1440];

/// `touchpadInvertToValue`: el índice de la lista y el valor que se guarda.
const INVERTIR_PANEL: [i32; 4] = [0, 2, 1, 3];

/// Los valores que C# tiene en memoria pero no guarda en el perfil.
const BOTONES_VIRTUALES_GATILLO: bool = true;
const INERCIA_STICK_PANEL: bool = true;
const FRICCION_STICK_PANEL: f64 = 10.0;

#[derive(Debug, Clone, Copy)]
enum Tipo {
    Interruptor,
    Numero {
        min: f64,
        max: f64,
        paso: f64,
        porcentaje: bool,
    },
    Opciones(&'static [&'static str]),
    Botones,
    Curva,
}

struct Ajuste {
    id: String,
    nombre: &'static str,
    tipo: Tipo,
}

struct Seccion {
    nombre: String,
    ajustes: Vec<Ajuste>,
    /// La sección solo se ve mientras ese valor del modelo de vista vale eso.
    cuando: Option<(String, i64)>,
}

struct Grupo {
    id: &'static str,
    nombre: &'static str,
    secciones: Vec<Seccion>,
}

fn interruptor(id: impl Into<String>, nombre: &'static str) -> Ajuste {
    Ajuste {
        id: id.into(),
        nombre,
        tipo: Tipo::Interruptor,
    }
}

fn numero(id: impl Into<String>, nombre: &'static str, min: f64, max: f64, paso: f64) -> Ajuste {
    Ajuste {
        id: id.into(),
        nombre,
        tipo: Tipo::Numero {
            min,
            max,
            paso,
            porcentaje: false,
        },
    }
}

fn porcentaje(
    id: impl Into<String>,
    nombre: &'static str,
    min: f64,
    max: f64,
    paso: f64,
) -> Ajuste {
    Ajuste {
        id: id.into(),
        nombre,
        tipo: Tipo::Numero {
            min,
            max,
            paso,
            porcentaje: true,
        },
    }
}

fn eleccion(
    id: impl Into<String>,
    nombre: &'static str,
    opciones: &'static [&'static str],
) -> Ajuste {
    Ajuste {
        id: id.into(),
        nombre,
        tipo: Tipo::Opciones(opciones),
    }
}

fn curva(id: impl Into<String>) -> Ajuste {
    Ajuste {
        id: id.into(),
        nombre: "Curva",
        tipo: Tipo::Curva,
    }
}

fn botones(id: &str) -> Ajuste {
    Ajuste {
        id: id.into(),
        nombre: "Se activa con",
        tipo: Tipo::Botones,
    }
}

fn seccion(nombre: impl Into<String>, ajustes: Vec<Ajuste>) -> Seccion {
    Seccion {
        nombre: nombre.into(),
        ajustes,
        cuando: None,
    }
}

fn seccion_si(
    nombre: impl Into<String>,
    cuando: impl Into<String>,
    valor: i64,
    ajustes: Vec<Ajuste>,
) -> Seccion {
    Seccion {
        nombre: nombre.into(),
        ajustes,
        cuando: Some((cuando.into(), valor)),
    }
}

/// `BuildGroups`.
fn grupos() -> Vec<Grupo> {
    let mut sticks = palanca("Stick izquierdo", "LS", "Left");
    sticks.extend(palanca("Stick derecho", "RS", "Right"));
    let mut gatillos = gatillo("L2");
    gatillos.extend(gatillo("R2"));
    vec![
        Grupo {
            id: "sticks",
            nombre: "Sticks",
            secciones: sticks,
        },
        Grupo {
            id: "gatillos",
            nombre: "Gatillos",
            secciones: gatillos,
        },
        Grupo {
            id: "giroscopio",
            nombre: "Giroscopio",
            secciones: giroscopio(),
        },
        Grupo {
            id: "panel",
            nombre: "Panel táctil",
            secciones: panel(),
        },
        Grupo {
            id: "luz",
            nombre: "Luz",
            secciones: luz(),
        },
        Grupo {
            id: "vibracion",
            nombre: "Vibración",
            secciones: vec![seccion(
                "Motores",
                vec![
                    numero("RumbleBoost", "Refuerzo de vibración", 0.0, 200.0, 10.0),
                    numero(
                        "RumbleAutostopTime",
                        "Parar tras (milisegundos)",
                        0.0,
                        3600.0,
                        10.0,
                    ),
                    interruptor("InverseRumbleMotors", "Intercambiar motores"),
                    eleccion(
                        "DualSenseRumbleEmulationPerIndex",
                        "Modo de vibración del DualSense",
                        &["Preciso", "Clásico", "Desactivado"],
                    ),
                    eleccion(
                        "DualSenseHapticPowerLevelPerIndex",
                        "Fuerza háptica",
                        &["100%", "87%", "75%", "62%", "50%", "37%", "25%", "12%"],
                    ),
                ],
            )],
        },
        Grupo {
            id: "raton",
            nombre: "Ratón",
            secciones: vec![seccion(
                "Botones que mueven el ratón",
                vec![
                    numero("ButtonMouseSensitivity", "Sensibilidad", 0.0, 255.0, 1.0),
                    numero(
                        "ButtonMouseVerticalScale",
                        "Escala vertical",
                        0.0,
                        500.0,
                        10.0,
                    ),
                    numero("ButtonMouseOffset", "Desplazamiento", 0.0, 100.0, 1.0),
                    interruptor("MouseAcceleration", "Aceleración"),
                ],
            )],
        },
        Grupo {
            id: "volante",
            nombre: "Volante",
            secciones: vec![seccion(
                "Girar el mando como un volante",
                vec![
                    eleccion(
                        "SASteeringWheelEmulationAxisIndex",
                        "Eje que usa",
                        &[
                            "Ninguno",
                            "Stick izq. X",
                            "Stick izq. Y",
                            "Stick der. X",
                            "Stick der. Y",
                            "L2 y R2",
                        ],
                    ),
                    eleccion(
                        "SASteeringWheelEmulationRangeIndex",
                        "Giro total (grados)",
                        &[
                            "90", "180", "270", "360", "450", "720", "900", "1080", "1440",
                        ],
                    ),
                    numero("SASteeringWheelFuzz", "Filtro de temblor", 0.0, 100.0, 1.0),
                    interruptor("SASteeringWheelUseSmoothing", "Suavizado"),
                    numero(
                        "SASteeringWheelSmoothMinCutoff",
                        "Corte mínimo",
                        0.00001,
                        1000.0,
                        0.1,
                    ),
                    numero("SASteeringWheelSmoothBeta", "Beta", 0.0, 1.0, 0.1),
                ],
            )],
        },
        Grupo {
            id: "otros",
            nombre: "Otros",
            secciones: vec![seccion(
                "Conexión",
                vec![
                    numero(
                        "IdleDisconnect",
                        "Apagar el mando tras (minutos sin usar)",
                        0.0,
                        60.0,
                        1.0,
                    ),
                    numero(
                        "DebouncingMs",
                        "Antirrebote de botones (milisegundos)",
                        0.0,
                        1000.0,
                        1.0,
                    ),
                    interruptor("EnableOutputDataToDS5", "Enviar datos al mando"),
                    interruptor("DInputOnly", "Sin mando virtual"),
                    interruptor("VirtualTriggerButtons", "Botones virtuales de gatillo"),
                    interruptor("UseDs3PitchRollSim", "Simular inclinación en DS3"),
                ],
            )],
        },
    ]
}

fn palanca(nombre: &str, p: &str, lado: &str) -> Vec<Seccion> {
    vec![
        seccion(
            nombre,
            vec![
                porcentaje(format!("{p}DeadZone"), "Zona muerta", 0.0, 1.0, 0.01),
                eleccion(
                    format!("{p}DeadTypeIndex"),
                    "Forma de la zona muerta",
                    &["Circular", "Por ejes"],
                ),
                porcentaje(
                    format!("{p}AntiDeadZone"),
                    "Compensar zona muerta del juego",
                    0.0,
                    1.0,
                    0.01,
                ),
                porcentaje(format!("{p}MaxZone"), "Recorrido máximo", 0.0, 1.0, 0.01),
                porcentaje(format!("{p}MaxOutput"), "Salida máxima", 0.0, 1.0, 0.01),
                interruptor(format!("{p}MaxOutputForce"), "Forzar salida máxima"),
                numero(format!("{p}Sens"), "Sensibilidad", 0.1, 5.0, 0.1),
                numero(
                    format!("{p}VerticalScale"),
                    "Escala vertical",
                    0.0,
                    4.0,
                    0.1,
                ),
                numero(
                    format!("{p}Rotation"),
                    "Rotación (grados)",
                    -180.0,
                    180.0,
                    1.0,
                ),
                numero(format!("{p}Fuzz"), "Filtro de temblor", 0.0, 100.0, 1.0),
                eleccion(format!("{p}OutputCurveIndex"), "Curva de respuesta", CURVAS),
                eleccion(
                    format!("{p}OutputIndex"),
                    "Función",
                    &["Desactivado", "Normal", "Flick stick"],
                ),
            ],
        ),
        seccion_si(
            format!("{nombre}: curva personalizada"),
            format!("{p}CustomCurveSelected"),
            1,
            vec![curva(format!("{p}CustomCurve"))],
        ),
        seccion_si(
            format!("{nombre}: flick stick"),
            format!("{p}OutputIndex"),
            2,
            vec![
                numero(
                    format!("{p}FlickRWC"),
                    "Giro completo (centímetros)",
                    1.0,
                    200.0,
                    1.0,
                ),
                porcentaje(format!("{p}FlickThreshold"), "Umbral", 0.0, 1.0, 0.05),
                numero(
                    format!("{p}FlickTime"),
                    "Duración (segundos)",
                    0.01,
                    10.0,
                    0.05,
                ),
                numero(
                    format!("{p}MinAngleThreshold"),
                    "Ángulo mínimo",
                    0.0,
                    180.0,
                    0.5,
                ),
            ],
        ),
        seccion(
            format!("{nombre}: precisión"),
            vec![
                interruptor(format!("{p}SquareStick"), "Recorrido cuadrado"),
                numero(format!("{p}SquareRoundness"), "Redondeo", 0.0, 1000.0, 1.0),
                interruptor(format!("{p}AntiSnapback"), "Evitar rebote al soltar"),
                numero(
                    format!("{p}AntiSnapbackDelta"),
                    "Margen de rebote",
                    0.0,
                    256.0,
                    1.0,
                ),
                numero(
                    format!("{p}AntiSnapbackTimeout"),
                    "Tiempo de rebote (milisegundos)",
                    0.0,
                    1000.0,
                    1.0,
                ),
                interruptor(format!("{p}OuterBindInvert"), "Invertir zona exterior"),
                porcentaje(
                    format!("{p}OuterBindDead"),
                    "Inicio de la zona exterior",
                    0.0,
                    1.0,
                    0.01,
                ),
                interruptor(format!("{p}DeltaAccelEnabled"), "Aceleración por velocidad"),
                numero(
                    format!("{p}DeltaMultiplier"),
                    "Multiplicador",
                    0.0,
                    10.0,
                    0.1,
                ),
                numero(
                    format!("{p}DeltaMinFactor"),
                    "Factor mínimo",
                    1.0,
                    10.0,
                    0.1,
                ),
                porcentaje(
                    format!("{p}DeltaMinTravel"),
                    "Recorrido mínimo",
                    0.0,
                    1.0,
                    0.05,
                ),
                porcentaje(
                    format!("{p}DeltaMaxTravel"),
                    "Recorrido máximo",
                    0.0,
                    1.0,
                    0.05,
                ),
                numero(
                    format!("{p}DeltaEasingDuration"),
                    "Suavizado (segundos)",
                    0.0,
                    600.0,
                    0.1,
                ),
            ],
        ),
        seccion(
            format!("{nombre}: corrección de deriva"),
            vec![
                numero(
                    format!("{lado}StickDriftXAxis"),
                    "Horizontal",
                    -127.0,
                    127.0,
                    1.0,
                ),
                numero(
                    format!("{lado}StickDriftYAxis"),
                    "Vertical",
                    -127.0,
                    127.0,
                    1.0,
                ),
            ],
        ),
    ]
}

fn gatillo(t: &str) -> Vec<Seccion> {
    vec![
        seccion(
            t,
            vec![
                porcentaje(format!("{t}DeadZone"), "Zona muerta", 0.0, 1.0, 0.01),
                porcentaje(
                    format!("{t}AntiDeadZone"),
                    "Compensar zona muerta del juego",
                    0.0,
                    1.0,
                    0.01,
                ),
                porcentaje(format!("{t}MaxZone"), "Recorrido máximo", 0.0, 1.0, 0.01),
                porcentaje(format!("{t}MaxOutput"), "Salida máxima", 0.0, 1.0, 0.01),
                numero(format!("{t}Sens"), "Sensibilidad", 0.1, 10.0, 0.1),
                eleccion(format!("{t}OutputCurveIndex"), "Curva de respuesta", CURVAS),
                eleccion(
                    format!("{t}TriggerMode"),
                    "Dos fases",
                    &[
                        "Desactivado",
                        "Normal",
                        "Solo botones",
                        "Gatillo rápido",
                        "Disparo de cadera",
                        "Cadera y botones",
                    ],
                ),
                numero(
                    format!("{t}HipFireTime"),
                    "Tiempo de cadera (milisegundos)",
                    0.0,
                    5000.0,
                    10.0,
                ),
            ],
        ),
        seccion_si(
            format!("{t}: curva personalizada"),
            format!("{t}CustomCurveSelected"),
            1,
            vec![curva(format!("{t}CustomCurve"))],
        ),
    ]
}

fn giroscopio() -> Vec<Seccion> {
    vec![
        seccion(
            "Uso del giroscopio",
            vec![eleccion(
                "GyroOutModeIndex",
                "Función",
                &["Sensores al juego", "Ratón", "Stick", "Gestos", "Directo"],
            )],
        ),
        seccion_si(
            "Sensores al juego",
            "GyroOutModeIndex",
            0,
            vec![
                botones("gyroControles"),
                eleccion(
                    "GyroControlsEvalCondIndex",
                    "Botones necesarios",
                    CONDICIONES,
                ),
                interruptor("GyroControlsTurns", "Encender o apagar al pulsar"),
                interruptor("GyroControlsToggle", "Alternar"),
            ],
        ),
        seccion_si(
            "Ratón con giroscopio",
            "GyroOutModeIndex",
            1,
            vec![
                botones("gyroRaton"),
                eleccion("GyroMouseEvalCondIndex", "Botones necesarios", CONDICIONES),
                interruptor("GyroMouseTurns", "Encender o apagar al pulsar"),
                interruptor("GyroMouseToggle", "Alternar"),
                numero("GyroSensitivity", "Sensibilidad", 0.0, 500.0, 10.0),
                numero("GyroVertScale", "Escala vertical", 0.0, 400.0, 10.0),
                numero("GyroMouseDeadZone", "Zona muerta", 0.0, 1000.0, 1.0),
                numero("GyroMouseMinThreshold", "Umbral mínimo", 1.0, 40.0, 1.0),
                eleccion("GyroMouseXAxis", "Eje horizontal", GIROS),
                interruptor("GyroMouseInvertX", "Invertir horizontal"),
                interruptor("GyroMouseInvertY", "Invertir vertical"),
                interruptor("GyroMouseJitterCompensation", "Compensar temblor"),
                interruptor("GyroMouseSmooth", "Suavizado"),
                eleccion(
                    "GyroMouseSmoothMethodIndex",
                    "Método de suavizado",
                    SUAVIZADOS,
                ),
                numero("GyroMouseSmoothWeight", "Peso", 0.0, 1.0, 0.1),
                numero(
                    "GyroMouseOneEuroMinCutoff",
                    "Corte mínimo",
                    0.00001,
                    100.0,
                    0.1,
                ),
                numero("GyroMouseOneEuroBeta", "Beta", 0.0, 1.0, 0.1),
            ],
        ),
        seccion_si(
            "Stick con giroscopio",
            "GyroOutModeIndex",
            2,
            vec![
                botones("gyroStick"),
                eleccion(
                    "GyroMouseStickEvalCondIndex",
                    "Botones necesarios",
                    CONDICIONES,
                ),
                interruptor("GyroMouseStickTurns", "Encender o apagar al pulsar"),
                interruptor("GyroMouseStickToggle", "Alternar"),
                eleccion(
                    "GyroMouseStickOutputStick",
                    "Stick de salida",
                    STICKS_SALIDA,
                ),
                eleccion("GyroMouseStickOutputAxes", "Ejes", EJES_SALIDA),
                numero("GyroMouseStickDeadZone", "Zona muerta", 0.0, 1000.0, 1.0),
                numero(
                    "GyroMouseStickMaxZone",
                    "Recorrido máximo",
                    1.0,
                    1000.0,
                    1.0,
                ),
                numero(
                    "GyroMouseStickAntiDeadX",
                    "Compensar zona muerta horizontal",
                    0.0,
                    100.0,
                    1.0,
                ),
                numero(
                    "GyroMouseStickAntiDeadY",
                    "Compensar zona muerta vertical",
                    0.0,
                    100.0,
                    1.0,
                ),
                numero(
                    "GyroMouseStickVertScale",
                    "Escala vertical",
                    0.0,
                    400.0,
                    10.0,
                ),
                interruptor("GyroMouseStickMaxOutputEnabled", "Limitar salida"),
                numero("GyroMouseStickMaxOutput", "Salida máxima", 0.0, 100.0, 1.0),
                eleccion("GyroMouseStickXAxis", "Eje horizontal", GIROS),
                interruptor("GyroMouseStickInvertX", "Invertir horizontal"),
                interruptor("GyroMouseStickInvertY", "Invertir vertical"),
                interruptor("GyroMouseStickJitterCompensation", "Compensar temblor"),
                interruptor("GyroMouseStickSmooth", "Suavizado"),
                eleccion(
                    "GyroMouseStickSmoothMethodIndex",
                    "Método de suavizado",
                    SUAVIZADOS,
                ),
                numero(
                    "GyroMouseStickOneEuroMinCutoff",
                    "Corte mínimo",
                    0.00001,
                    100.0,
                    0.1,
                ),
                numero("GyroMouseStickOneEuroBeta", "Beta", 0.0, 1.0, 0.1),
            ],
        ),
        seccion_si(
            "Gestos con giroscopio",
            "GyroOutModeIndex",
            3,
            vec![
                botones("gyroGestos"),
                eleccion("GyroSwipeEvalCondIndex", "Botones necesarios", CONDICIONES),
                interruptor("GyroSwipeTurns", "Encender o apagar al pulsar"),
                eleccion("GyroSwipeXAxis", "Eje horizontal", GIROS),
                numero(
                    "GyroSwipeDeadZoneX",
                    "Zona muerta horizontal",
                    0.0,
                    2000.0,
                    10.0,
                ),
                numero(
                    "GyroSwipeDeadZoneY",
                    "Zona muerta vertical",
                    0.0,
                    2000.0,
                    10.0,
                ),
                numero(
                    "GyroSwipeDelayTime",
                    "Retardo (milisegundos)",
                    0.0,
                    5000.0,
                    10.0,
                ),
            ],
        ),
        seccion(
            "Sensibilidad de sensores",
            vec![
                porcentaje("SXDeadZone", "Zona muerta lateral", 0.0, 1.0, 0.01),
                porcentaje("SZDeadZone", "Zona muerta frontal", 0.0, 1.0, 0.01),
                porcentaje("SXMaxZone", "Recorrido lateral", 0.0, 1.0, 0.01),
                porcentaje("SZMaxZone", "Recorrido frontal", 0.0, 1.0, 0.01),
                porcentaje("SXAntiDeadZone", "Compensación lateral", 0.0, 1.0, 0.01),
                porcentaje("SZAntiDeadZone", "Compensación frontal", 0.0, 1.0, 0.01),
                numero("SXSens", "Sensibilidad lateral", 0.0, 5.0, 0.1),
                numero("SZSens", "Sensibilidad frontal", 0.0, 5.0, 0.1),
                eleccion("SXOutputCurveIndex", "Curva lateral", CURVAS),
                eleccion("SZOutputCurveIndex", "Curva frontal", CURVAS),
            ],
        ),
        seccion_si(
            "Curva lateral personalizada",
            "SXCustomCurveSelected",
            1,
            vec![curva("SXCustomCurve")],
        ),
        seccion_si(
            "Curva frontal personalizada",
            "SZCustomCurveSelected",
            1,
            vec![curva("SZCustomCurve")],
        ),
    ]
}

fn panel() -> Vec<Seccion> {
    vec![
        seccion(
            "Uso del panel",
            vec![
                eleccion(
                    "TouchpadOutputIndex",
                    "Función",
                    &["Ratón", "Controles", "Stick", "Ratón absoluto", "Directo"],
                ),
                interruptor("EnableTouchpadToggle", "Encender o apagar con PS + panel"),
                interruptor("StartTouchpadOff", "Empezar apagado"),
                interruptor("TouchpadClickPassthru", "Pasar el clic al juego"),
                interruptor(
                    "LowerRightTouchRMB",
                    "Esquina inferior derecha: clic derecho",
                ),
            ],
        ),
        seccion_si(
            "Ratón con el panel",
            "TouchpadOutputIndex",
            0,
            vec![
                numero("TouchSens", "Sensibilidad", 0.0, 500.0, 1.0),
                numero(
                    "TouchScroll",
                    "Desplazamiento con dos dedos",
                    0.0,
                    255.0,
                    1.0,
                ),
                numero("TouchTap", "Toque para clic", 0.0, 500.0, 1.0),
                interruptor("TouchDoubleTap", "Doble toque"),
                interruptor("TouchJitter", "Compensar temblor"),
                eleccion(
                    "TouchInvertIndex",
                    "Invertir",
                    &["No", "Horizontal", "Vertical", "Ambos"],
                ),
                numero(
                    "TouchRelMouseRotation",
                    "Rotación (grados)",
                    -180.0,
                    180.0,
                    1.0,
                ),
                numero("TouchRelMouseMinThreshold", "Umbral mínimo", 1.0, 40.0, 1.0),
                interruptor("TouchTrackball", "Inercia"),
                numero(
                    "TouchTrackballFriction",
                    "Fricción de la inercia",
                    0.1,
                    20.0,
                    0.1,
                ),
            ],
        ),
        seccion_si(
            "Stick con el panel",
            "TouchpadOutputIndex",
            2,
            vec![
                eleccion(
                    "TouchMouseStickOutputStick",
                    "Stick de salida",
                    STICKS_SALIDA,
                ),
                eleccion("TouchMouseStickOutputAxes", "Ejes", EJES_SALIDA),
                numero("TouchMouseStickDeadZone", "Zona muerta", 0.0, 1000.0, 1.0),
                numero(
                    "TouchMouseStickMaxZone",
                    "Recorrido máximo",
                    1.0,
                    1000.0,
                    1.0,
                ),
                numero(
                    "TouchMouseStickAntiDeadX",
                    "Compensar zona muerta horizontal",
                    0.0,
                    100.0,
                    1.0,
                ),
                numero(
                    "TouchMouseStickAntiDeadY",
                    "Compensar zona muerta vertical",
                    0.0,
                    100.0,
                    1.0,
                ),
                numero(
                    "TouchMouseStickVertScale",
                    "Escala vertical",
                    0.0,
                    400.0,
                    10.0,
                ),
                interruptor("TouchMouseStickMaxOutputEnabled", "Limitar salida"),
                numero("TouchMouseStickMaxOutput", "Salida máxima", 0.0, 100.0, 1.0),
                interruptor("TouchMouseStickInvertX", "Invertir horizontal"),
                interruptor("TouchMouseStickInvertY", "Invertir vertical"),
                numero(
                    "TouchMouseStickRotation",
                    "Rotación (grados)",
                    -180.0,
                    180.0,
                    1.0,
                ),
                interruptor("TouchMouseStickSmooth", "Suavizado"),
                interruptor("TouchMouseStickTrackball", "Inercia"),
                numero(
                    "TouchMouseStickTrackballFriction",
                    "Fricción de la inercia",
                    0.1,
                    20.0,
                    0.1,
                ),
            ],
        ),
        seccion_si(
            "Ratón absoluto",
            "TouchpadOutputIndex",
            3,
            vec![
                numero(
                    "TouchAbsMouseMaxZoneX",
                    "Recorrido horizontal",
                    0.0,
                    100.0,
                    1.0,
                ),
                numero(
                    "TouchAbsMouseMaxZoneY",
                    "Recorrido vertical",
                    0.0,
                    100.0,
                    1.0,
                ),
                interruptor("TouchAbsMouseSnapCenter", "Volver al centro"),
            ],
        ),
    ]
}

fn luz() -> Vec<Seccion> {
    vec![
        seccion(
            "Avisos",
            vec![
                eleccion(
                    "LightbarModeIndex",
                    "Quién controla la luz",
                    &["DS5Control", "El juego"],
                ),
                numero(
                    "FlashAt",
                    "Parpadear con batería por debajo de (%)",
                    0.0,
                    100.0,
                    5.0,
                ),
                eleccion("FlashTypeIndex", "Forma de avisar", &["Parpadeo", "Pulso"]),
                eleccion(
                    "ChargingType",
                    "Luz mientras carga",
                    &["Normal", "Pulso", "Arcoíris", "Color"],
                ),
            ],
        ),
        seccion(
            "Color con batería baja",
            vec![
                numero("LowColorR", "Rojo", 0.0, 255.0, 1.0),
                numero("LowColorG", "Verde", 0.0, 255.0, 1.0),
                numero("LowColorB", "Azul", 0.0, 255.0, 1.0),
            ],
        ),
        seccion(
            "Arcoíris",
            vec![numero(
                "MaxSatRainbow",
                "Saturación máxima",
                0.0,
                100.0,
                1.0,
            )],
        ),
    ]
}

/// El nombre de una sección en `idioma`. Las de cada stick y gatillo son "Stick izquierdo:
/// precisión" y parecidas: se traduce cada lado de los dos puntos.
fn nombre_seccion(nombre: &str, idioma: Idioma) -> String {
    let entero = idioma.texto(nombre);
    if entero != nombre {
        return entero.to_string();
    }
    match nombre.split_once(": ") {
        Some((de, que)) => format!("{}: {}", idioma.texto(de), idioma.texto(que)),
        None => entero.to_string(),
    }
}

/// `Describe`: el array `grupos` del mensaje `opciones`, con los textos en `idioma`. Cada
/// sección lleva en `id` su nombre en español, el de siempre, para que la interfaz la reconozca
/// en cualquier idioma.
pub fn describir(p: &Perfil, idioma: Idioma) -> Value {
    // Los accesos son de escritura y lectura a la vez; se lee sobre una copia.
    let mut a = p.ajustes.clone();
    let mut lista = Vec::new();
    for g in grupos() {
        let mut secciones = Vec::new();
        for s in &g.secciones {
            if let Some((id, valor)) = &s.cuando {
                let actual = acceder(&mut a, id, None).as_ref().and_then(como_entero);
                if actual != Some(*valor) {
                    continue;
                }
            }
            let ajustes: Vec<Value> = s
                .ajustes
                .iter()
                .map(|o| describir_ajuste(o, acceder(&mut a, &o.id, None), idioma))
                .collect();
            secciones.push(json!({
                "id": s.nombre,
                "nombre": nombre_seccion(&s.nombre, idioma),
                "ajustes": ajustes,
            }));
        }
        lista.push(json!({
            "id": g.id,
            "nombre": idioma.texto(g.nombre),
            "secciones": secciones,
        }));
    }
    Value::Array(lista)
}

fn describir_ajuste(o: &Ajuste, valor: Option<Value>, idioma: Idioma) -> Value {
    let (tipo, min, max, paso, en_porcentaje, opciones) = match o.tipo {
        Tipo::Interruptor => ("interruptor", 0.0, 0.0, 0.0, false, None),
        Tipo::Numero {
            min,
            max,
            paso,
            porcentaje,
        } => ("numero", min, max, paso, porcentaje, None),
        Tipo::Opciones(l) => ("opciones", 0.0, 0.0, 0.0, false, Some(l)),
        Tipo::Botones => ("botones", 0.0, 0.0, 0.0, false, Some(BOTONES)),
        Tipo::Curva => ("curva", 0.0, 0.0, 0.0, false, None),
    };
    let opciones: Option<Vec<&str>> = opciones.map(|l| l.iter().map(|t| idioma.texto(t)).collect());
    json!({
        "id": o.id,
        "nombre": idioma.texto(o.nombre),
        "tipo": tipo,
        "min": real(min),
        "max": real(max),
        "paso": real(paso),
        "porcentaje": en_porcentaje,
        "opciones": opciones,
        "valor": valor.unwrap_or(Value::Null),
    })
}

/// `Apply`: cambia el ajuste `id` del perfil. Devuelve `true` si ha cambiado algo; `false` si el
/// ajuste no existe, el valor no vale (donde C# lanzaría una excepción) o ya estaba así.
pub fn aplicar(p: &mut Perfil, id: &str, valor: &Value) -> bool {
    let existe = grupos()
        .iter()
        .flat_map(|g| &g.secciones)
        .flat_map(|s| &s.ajustes)
        .any(|o| o.id == id);
    if !existe {
        return false;
    }
    let antes = p.ajustes.clone();
    if acceder(&mut p.ajustes, id, Some(valor)).is_none() {
        p.ajustes = antes;
        return false;
    }
    p.ajustes != antes
}

// Conversiones de valores, como las de C#.

/// Un número de .NET pasado a JSON: los enteros sin decimales, como `System.Text.Json`.
fn real(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        json!(v as i64)
    } else {
        json!(v)
    }
}

/// El valor de la sección que decide si se ve: los booleanos cuentan como 1 y 0.
fn como_entero(v: &Value) -> Option<i64> {
    match v {
        Value::Bool(b) => Some(*b as i64),
        _ => a_entero(v).map(i64::from),
    }
}

/// `Convert.ToInt32(double)`: redondeo al par y error si se sale de rango.
fn a_entero(v: &Value) -> Option<i32> {
    let d = v.as_f64()?;
    if !(-2_147_483_648.5..2_147_483_647.5).contains(&d) {
        return None;
    }
    Some(d.round_ties_even() as i32)
}

/// `JsonElement.GetInt32`: solo un entero escrito como tal.
fn a_entero_exacto(v: &Value) -> Option<i32> {
    v.as_i64().and_then(|i| i32::try_from(i).ok())
}

/// `Convert.ToBoolean(double)` si llega un número.
fn a_bool(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        Value::Number(_) => v.as_f64().map(|d| d != 0.0),
        _ => None,
    }
}

fn a_doble(v: &Value) -> Option<f64> {
    v.as_f64()
}

/// `Math.Round(x, 2)`.
fn redondear_2(x: f64) -> f64 {
    (x * 100.0).round_ties_even() / 100.0
}

/// Lo que devuelve un acceso que ha escrito bien.
const HECHO: Option<Value> = Some(Value::Null);
type Nuevo<'v> = Option<&'v Value>;

fn si(c: &mut bool, n: Nuevo) -> Option<Value> {
    match n {
        None => Some(json!(*c)),
        Some(v) => {
            *c = a_bool(v)?;
            HECHO
        }
    }
}

fn doble(c: &mut f64, n: Nuevo) -> Option<Value> {
    doble_limitado(c, n, f64::NEG_INFINITY, f64::INFINITY)
}

/// Un `double` que pasa tal cual y al guardarse se limita (`Math.Clamp` del DTO).
fn doble_limitado(c: &mut f64, n: Nuevo, min: f64, max: f64) -> Option<Value> {
    match n {
        None => Some(real(*c)),
        Some(v) => {
            *c = a_doble(v)?.clamp(min, max);
            HECHO
        }
    }
}

fn entero(c: &mut i32, n: Nuevo) -> Option<Value> {
    entero_limitado(c, n, i32::MIN, i32::MAX)
}

fn entero_limitado(c: &mut i32, n: Nuevo, min: i32, max: i32) -> Option<Value> {
    match n {
        None => Some(json!(*c)),
        Some(v) => {
            *c = a_entero(v)?.clamp(min, max);
            HECHO
        }
    }
}

/// Una propiedad `int` que se guarda como `byte` con `(byte)value`, que se queda con los 8 bits
/// de abajo.
fn byte(c: &mut u8, n: Nuevo) -> Option<Value> {
    match n {
        None => Some(json!(*c)),
        Some(v) => {
            *c = a_entero(v)? as u8;
            HECHO
        }
    }
}

/// Las derivas son `sbyte`: `Convert.ToSByte` falla fuera de rango.
fn con_signo(c: &mut i8, n: Nuevo) -> Option<Value> {
    match n {
        None => Some(json!(*c)),
        Some(v) => {
            *c = i8::try_from(a_entero(v)?).ok()?;
            HECHO
        }
    }
}

/// Un `double` que el modelo de vista enseña de otra forma: `ver` pasa del archivo a lo que se
/// ve y `guardar` de lo que llega a lo que queda en el archivo.
fn doble_convertido(
    c: &mut f64,
    n: Nuevo,
    ver: impl Fn(f64) -> f64,
    guardar: impl Fn(f64) -> f64,
) -> Option<Value> {
    match n {
        None => Some(real(ver(*c))),
        Some(v) => {
            *c = guardar(a_doble(v)?);
            HECHO
        }
    }
}

/// Un entero del archivo que se enseña como `double` (`/ 100.0`) y vuelve con `(int)(v * 100)`.
fn centesimas(c: &mut i32, n: Nuevo, min: i32, max: i32) -> Option<Value> {
    match n {
        None => Some(real(*c as f64 / 100.0)),
        Some(v) => {
            *c = ((a_doble(v)? * 100.0) as i32).clamp(min, max);
            HECHO
        }
    }
}

/// Una lista de opciones del modelo de vista hecha con un `switch`: lo que no está en la lista
/// se lee como 0 y se escribe como `otro`.
fn indice<E: Copy + PartialEq>(c: &mut E, valores: &[E], otro: E, n: Nuevo) -> Option<Value> {
    match n {
        None => Some(json!(valores.iter().position(|e| e == c).unwrap_or(0))),
        Some(v) => {
            let i = a_entero(v)?;
            *c = usize::try_from(i)
                .ok()
                .and_then(|i| valores.get(i).copied())
                .unwrap_or(otro);
            HECHO
        }
    }
}

/// Una enumeración leída y escrita con `(int)` y `(Enum)value`: la conversión se queda con los
/// bits del tipo (`mascara`) y un valor que no es de la enumeración no se puede guardar.
fn enumeracion<E: Copy + PartialEq>(
    c: &mut E,
    valores: &[E],
    mascara: u32,
    n: Nuevo,
) -> Option<Value> {
    match n {
        None => Some(json!(valores.iter().position(|e| e == c).unwrap_or(0))),
        Some(v) => {
            let i = (a_entero(v)? as u32 & mascara) as usize;
            *c = *valores.get(i)?;
            HECHO
        }
    }
}

fn curva_indice(c: &mut Curva, n: Nuevo) -> Option<Value> {
    // Cualquier índice se guarda; los que no son una curva se escriben como lineal.
    indice(c, &ORDEN_CURVAS, Curva::Lineal, n)
}

fn curva_elegida(c: &Curva, n: Nuevo) -> Option<Value> {
    match n {
        None => Some(json!(*c == Curva::Personalizada)),
        Some(_) => None,
    }
}

/// `CustomDefinition`: el texto se guarda tal cual, valga o no como curva.
fn curva_texto(c: &mut Option<String>, n: Nuevo) -> Option<Value> {
    match n {
        None => Some(c.as_ref().map_or(Value::Null, |t| json!(t))),
        Some(v) => {
            *c = Some(v.as_str()?.to_string());
            HECHO
        }
    }
}

/// `TriggerCond`: `true` es "todos a la vez" (índice 0).
fn condicion(c: &mut bool, n: Nuevo) -> Option<Value> {
    match n {
        None => Some(json!(if *c { 0 } else { 1 })),
        Some(v) => {
            *c = a_entero(v)? == 0;
            HECHO
        }
    }
}

fn bit_entero(c: &mut i32, bit: i32, n: Nuevo) -> Option<Value> {
    match n {
        None => Some(json!(*c & bit == bit)),
        Some(v) => {
            if a_bool(v)? {
                *c |= bit;
            } else {
                *c &= !bit;
            }
            HECHO
        }
    }
}

fn bit_natural(c: &mut u32, bit: u32, n: Nuevo) -> Option<Value> {
    match n {
        None => Some(json!(*c & bit == bit)),
        Some(v) => {
            if a_bool(v)? {
                *c |= bit;
            } else {
                *c &= !bit;
            }
            HECHO
        }
    }
}

/// Los botones que activan el giroscopio: índices separados por comas, `-1` es siempre.
fn botones_valor(c: &mut String, n: Nuevo) -> Option<Value> {
    match n {
        None => {
            let indices: Vec<i32> = c
                .split(',')
                .filter(|t| !t.is_empty())
                .map(|t| numeros::entero_en::<i32>(t).unwrap_or(-1))
                .filter(|i| *i >= 0)
                .collect();
            Some(json!(indices))
        }
        Some(v) => {
            let mut indices = v
                .as_array()?
                .iter()
                .map(a_entero_exacto)
                .collect::<Option<Vec<i32>>>()?;
            indices.retain(|i| *i >= 0 && (*i as usize) < BOTONES.len());
            indices.sort();
            *c = if indices.is_empty() {
                "-1".to_string()
            } else {
                indices
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            };
            HECHO
        }
    }
}

/// El método de suavizado del giroscopio. El modelo de vista recuerda el índice; aquí sale del
/// perfil, igual que cuando se crea (`FindGyroMouseSmoothMethodIndex`).
fn metodo_suavizado(c: &mut MetodoSuavizado, n: Nuevo) -> Option<Value> {
    let actual = i32::from(*c == MetodoSuavizado::MediaPonderada);
    match n {
        None => Some(json!(actual)),
        Some(v) => {
            let i = a_entero(v)?;
            if i != actual {
                match i {
                    0 => *c = MetodoSuavizado::UnEuro,
                    1 => *c = MetodoSuavizado::MediaPonderada,
                    _ => {}
                }
            }
            HECHO
        }
    }
}

/// `SmoothingWeight` se guarda en porcentaje y se carga limitado a 0..1.
fn peso_suavizado(c: &mut f64, n: Nuevo) -> Option<Value> {
    doble_convertido(c, n, |v| (v * 0.01).clamp(0.0, 1.0), |v| v * 100.0)
}

fn suavizado_giroscopio(s: &mut SuavizadoGiroscopio, clave: &str, n: Nuevo) -> Option<Value> {
    match clave {
        "Smooth" => si(&mut s.activo, n),
        "SmoothMethodIndex" => metodo_suavizado(&mut s.metodo, n),
        "SmoothWeight" => peso_suavizado(&mut s.peso, n),
        "OneEuroMinCutoff" => doble_limitado(&mut s.corte_minimo, n, 0.0, 100.0),
        "OneEuroBeta" => doble_limitado(&mut s.beta, n, 0.0, 1.0),
        _ => None,
    }
}

/// Un stick: los campos de `LS...` o de `RS...`.
struct Palanca<'a> {
    zona_muerta: &'a mut i32,
    tipo_zona_muerta: &'a mut TipoZonaMuerta,
    antizona: &'a mut i32,
    zona_maxima: &'a mut i32,
    salida_maxima: &'a mut f64,
    forzar_salida_maxima: &'a mut bool,
    sensibilidad: &'a mut f64,
    escala_vertical: &'a mut f64,
    rotacion: &'a mut f64,
    difuminado: &'a mut i32,
    curva: &'a mut Curva,
    curva_personalizada: &'a mut Option<String>,
    modo: &'a mut ModoPalanca,
    flick: &'a mut FlickStick,
    cuadrado: &'a mut bool,
    redondez: &'a mut f64,
    antirrebote: &'a mut bool,
    antirrebote_delta: &'a mut f64,
    antirrebote_tiempo: &'a mut i32,
    invertir_exterior: &'a mut bool,
    zona_exterior: &'a mut f64,
    aceleracion: &'a mut AceleracionPalanca,
    deriva_x: &'a mut i8,
    deriva_y: &'a mut i8,
}

fn palanca_izquierda(a: &mut Ajustes) -> Palanca<'_> {
    Palanca {
        zona_muerta: &mut a.ls_zona_muerta,
        tipo_zona_muerta: &mut a.ls_tipo_zona_muerta,
        antizona: &mut a.ls_antizona,
        zona_maxima: &mut a.ls_zona_maxima,
        salida_maxima: &mut a.ls_salida_maxima,
        forzar_salida_maxima: &mut a.ls_forzar_salida_maxima,
        sensibilidad: &mut a.sensibilidades[0],
        escala_vertical: &mut a.ls_escala_vertical,
        rotacion: &mut a.ls_rotacion,
        difuminado: &mut a.ls_difuminado,
        curva: &mut a.ls_curva,
        curva_personalizada: &mut a.ls_curva_personalizada,
        modo: &mut a.ls_modo,
        flick: &mut a.ls_salida.flick,
        cuadrado: &mut a.ls_cuadrado,
        redondez: &mut a.ls_redondez,
        antirrebote: &mut a.ls_antirrebote,
        antirrebote_delta: &mut a.ls_antirrebote_delta,
        antirrebote_tiempo: &mut a.ls_antirrebote_tiempo,
        invertir_exterior: &mut a.ls_invertir_exterior,
        zona_exterior: &mut a.ls_zona_exterior,
        aceleracion: &mut a.ls_aceleracion,
        deriva_x: &mut a.deriva_izq_x,
        deriva_y: &mut a.deriva_izq_y,
    }
}

fn palanca_derecha(a: &mut Ajustes) -> Palanca<'_> {
    Palanca {
        zona_muerta: &mut a.rs_zona_muerta,
        tipo_zona_muerta: &mut a.rs_tipo_zona_muerta,
        antizona: &mut a.rs_antizona,
        zona_maxima: &mut a.rs_zona_maxima,
        salida_maxima: &mut a.rs_salida_maxima,
        forzar_salida_maxima: &mut a.rs_forzar_salida_maxima,
        sensibilidad: &mut a.sensibilidades[1],
        escala_vertical: &mut a.rs_escala_vertical,
        rotacion: &mut a.rs_rotacion,
        difuminado: &mut a.rs_difuminado,
        curva: &mut a.rs_curva,
        curva_personalizada: &mut a.rs_curva_personalizada,
        modo: &mut a.rs_modo,
        flick: &mut a.rs_salida.flick,
        cuadrado: &mut a.rs_cuadrado,
        redondez: &mut a.rs_redondez,
        antirrebote: &mut a.rs_antirrebote,
        antirrebote_delta: &mut a.rs_antirrebote_delta,
        antirrebote_tiempo: &mut a.rs_antirrebote_tiempo,
        invertir_exterior: &mut a.rs_invertir_exterior,
        zona_exterior: &mut a.rs_zona_exterior,
        aceleracion: &mut a.rs_aceleracion,
        deriva_x: &mut a.deriva_der_x,
        deriva_y: &mut a.deriva_der_y,
    }
}

fn acceder_palanca(s: Palanca, clave: &str, n: Nuevo) -> Option<Value> {
    match clave {
        // `Math.Round(deadZone / 127d, 2)` y vuelta con `(int)Math.Round(value * 127d)`.
        "DeadZone" => {
            let actual = redondear_2(*s.zona_muerta as f64 / 127.0);
            match n {
                None => Some(real(actual)),
                Some(v) => {
                    let v = a_doble(v)?;
                    if actual != v {
                        *s.zona_muerta = ((v * 127.0).round_ties_even() as i32).clamp(0, 127);
                    }
                    HECHO
                }
            }
        }
        "DeadTypeIndex" => indice(
            s.tipo_zona_muerta,
            &[TipoZonaMuerta::Radial, TipoZonaMuerta::Axial],
            TipoZonaMuerta::Radial,
            n,
        ),
        "AntiDeadZone" => centesimas(s.antizona, n, i32::MIN, i32::MAX),
        "MaxZone" => centesimas(s.zona_maxima, n, 0, 100),
        "MaxOutput" => doble_convertido(
            s.salida_maxima,
            n,
            |v| v / 100.0,
            |v| (v * 100.0).clamp(0.0, 100.0),
        ),
        "MaxOutputForce" => si(s.forzar_salida_maxima, n),
        "Sens" => doble(s.sensibilidad, n),
        "VerticalScale" => doble_convertido(
            s.escala_vertical,
            n,
            |v| v / 100.0,
            |v| (v * 100.0).clamp(0.0, 400.0),
        ),
        // En memoria va en radianes; el archivo y la ventana, en grados.
        "Rotation" => doble_convertido(
            s.rotacion,
            n,
            |v| v.clamp(-180.0, 180.0) * PI / 180.0 * 180.0 / PI,
            |v| v * PI / 180.0 * 180.0 / PI,
        ),
        "Fuzz" => entero_limitado(s.difuminado, n, 0, 100),
        "OutputCurveIndex" => curva_indice(s.curva, n),
        "CustomCurveSelected" => curva_elegida(s.curva, n),
        "CustomCurve" => curva_texto(s.curva_personalizada, n),
        "OutputIndex" => indice(
            s.modo,
            &[
                ModoPalanca::None,
                ModoPalanca::Controls,
                ModoPalanca::FlickStick,
            ],
            ModoPalanca::None,
            n,
        ),
        "FlickRWC" => doble(&mut s.flick.calibracion, n),
        "FlickThreshold" => doble(&mut s.flick.umbral, n),
        "FlickTime" => doble(&mut s.flick.tiempo, n),
        "MinAngleThreshold" => doble(&mut s.flick.angulo_minimo, n),
        "SquareStick" => si(s.cuadrado, n),
        "SquareRoundness" => doble(s.redondez, n),
        "AntiSnapback" => si(s.antirrebote, n),
        "AntiSnapbackDelta" => doble(s.antirrebote_delta, n),
        "AntiSnapbackTimeout" => entero(s.antirrebote_tiempo, n),
        "OuterBindInvert" => si(s.invertir_exterior, n),
        "OuterBindDead" => doble_convertido(
            s.zona_exterior,
            n,
            |v| v.clamp(0.0, 100.0) / 100.0,
            |v| v * 100.0,
        ),
        "DeltaAccelEnabled" => si(&mut s.aceleracion.activa, n),
        "DeltaMultiplier" => doble_limitado(&mut s.aceleracion.multiplicador, n, 0.0, 10.0),
        "DeltaMinFactor" => doble_limitado(&mut s.aceleracion.factor_minimo, n, 1.0, 10.0),
        "DeltaMinTravel" => doble_limitado(&mut s.aceleracion.recorrido_minimo, n, 0.0, 1.0),
        "DeltaMaxTravel" => doble_limitado(&mut s.aceleracion.recorrido_maximo, n, 0.0, 1.0),
        "DeltaEasingDuration" => {
            doble_limitado(&mut s.aceleracion.duracion_suavizado, n, 0.0, 600.0)
        }
        "StickDriftXAxis" => con_signo(s.deriva_x, n),
        "StickDriftYAxis" => con_signo(s.deriva_y, n),
        _ => None,
    }
}

/// Un gatillo: los campos de `L2...` o de `R2...`.
struct Gatillo<'a> {
    zona_muerta: &'a mut u8,
    antizona: &'a mut i32,
    zona_maxima: &'a mut i32,
    salida_maxima: &'a mut f64,
    sensibilidad: &'a mut f64,
    curva: &'a mut Curva,
    curva_personalizada: &'a mut Option<String>,
    dos_fases: &'a mut DosFases,
    tiempo_cadera: &'a mut i32,
}

fn gatillo_de(a: &mut Ajustes, izquierdo: bool) -> Gatillo<'_> {
    let [_, _, l2, r2, _, _] = &mut a.sensibilidades;
    if izquierdo {
        Gatillo {
            zona_muerta: &mut a.l2_zona_muerta,
            antizona: &mut a.l2_antizona,
            zona_maxima: &mut a.l2_zona_maxima,
            salida_maxima: &mut a.l2_salida_maxima,
            sensibilidad: l2,
            curva: &mut a.l2_curva,
            curva_personalizada: &mut a.l2_curva_personalizada,
            dos_fases: &mut a.l2_dos_fases,
            tiempo_cadera: &mut a.l2_tiempo_cadera,
        }
    } else {
        Gatillo {
            zona_muerta: &mut a.r2_zona_muerta,
            antizona: &mut a.r2_antizona,
            zona_maxima: &mut a.r2_zona_maxima,
            salida_maxima: &mut a.r2_salida_maxima,
            sensibilidad: r2,
            curva: &mut a.r2_curva,
            curva_personalizada: &mut a.r2_curva_personalizada,
            dos_fases: &mut a.r2_dos_fases,
            tiempo_cadera: &mut a.r2_tiempo_cadera,
        }
    }
}

fn acceder_gatillo(g: Gatillo, clave: &str, n: Nuevo) -> Option<Value> {
    match clave {
        // `deadZone / 255.0` y vuelta con `(byte)(value * 255.0)`.
        "DeadZone" => {
            let actual = *g.zona_muerta as f64 / 255.0;
            match n {
                None => Some(real(actual)),
                Some(v) => {
                    let v = a_doble(v)?;
                    if actual != v {
                        *g.zona_muerta = (v * 255.0) as u8;
                    }
                    HECHO
                }
            }
        }
        "AntiDeadZone" => centesimas(g.antizona, n, i32::MIN, i32::MAX),
        "MaxZone" => centesimas(g.zona_maxima, n, 0, 100),
        "MaxOutput" => doble_convertido(
            g.salida_maxima,
            n,
            |v| v / 100.0,
            |v| (v * 100.0).clamp(0.0, 100.0),
        ),
        "Sens" => doble(g.sensibilidad, n),
        "OutputCurveIndex" => curva_indice(g.curva, n),
        "CustomCurveSelected" => curva_elegida(g.curva, n),
        "CustomCurve" => curva_texto(g.curva_personalizada, n),
        // Propiedad de tipo enumeración: `Enum.ToObject(type, value.GetInt32())`.
        "TriggerMode" => {
            if let Some(v) = n {
                a_entero_exacto(v)?;
            }
            enumeracion(
                g.dos_fases,
                &[
                    DosFases::Disabled,
                    DosFases::Normal,
                    DosFases::ExclusiveButtons,
                    DosFases::HairTrigger,
                    DosFases::HipFire,
                    DosFases::HipFireExclusiveButtons,
                ],
                u32::MAX,
                n,
            )
        }
        "HipFireTime" => entero_limitado(g.tiempo_cadera, n, 0, 5000),
        _ => None,
    }
}

/// Un eje de los sensores: los campos de `SX...` o de `SZ...`.
fn acceder_sensor(a: &mut Ajustes, lateral: bool, clave: &str, n: Nuevo) -> Option<Value> {
    let [_, _, _, _, sx, sz] = &mut a.sensibilidades;
    let (zona_muerta, zona_maxima, antizona, sensibilidad, curva, personalizada) = if lateral {
        (
            &mut a.sx_zona_muerta,
            &mut a.sx_zona_maxima,
            &mut a.sx_antizona,
            sx,
            &mut a.sx_curva,
            &mut a.sx_curva_personalizada,
        )
    } else {
        (
            &mut a.sz_zona_muerta,
            &mut a.sz_zona_maxima,
            &mut a.sz_antizona,
            sz,
            &mut a.sz_curva,
            &mut a.sz_curva_personalizada,
        )
    };
    // El recorrido y la compensación se guardan en porcentaje y se cargan limitados a 0..1.
    let ver = |v: f64| (v * 0.01).clamp(0.0, 1.0);
    let guardar = |v: f64| v * 100.0;
    match clave {
        "DeadZone" => doble(zona_muerta, n),
        "MaxZone" => doble_convertido(zona_maxima, n, ver, guardar),
        "AntiDeadZone" => doble_convertido(antizona, n, ver, guardar),
        "Sens" => doble(sensibilidad, n),
        "OutputCurveIndex" => curva_indice(curva, n),
        "CustomCurveSelected" => curva_elegida(curva, n),
        "CustomCurve" => curva_texto(personalizada, n),
        _ => None,
    }
}

/// Lee (`n` vacío) o escribe un ajuste por su id. Al leer devuelve el valor que enseña el
/// modelo de vista; al escribir, `None` si el id no existe o el valor no vale.
fn acceder(a: &mut Ajustes, id: &str, n: Nuevo) -> Option<Value> {
    if let Some(clave) = id.strip_prefix("LS") {
        return acceder_palanca(palanca_izquierda(a), clave, n);
    }
    if let Some(clave) = id.strip_prefix("RS") {
        return acceder_palanca(palanca_derecha(a), clave, n);
    }
    if let Some(clave) = id.strip_prefix("Left") {
        return acceder_palanca(palanca_izquierda(a), clave, n);
    }
    if let Some(clave) = id.strip_prefix("Right") {
        return acceder_palanca(palanca_derecha(a), clave, n);
    }
    if let Some(clave) = id.strip_prefix("L2") {
        return acceder_gatillo(gatillo_de(a, true), clave, n);
    }
    if let Some(clave) = id.strip_prefix("R2") {
        return acceder_gatillo(gatillo_de(a, false), clave, n);
    }
    if let Some(clave) = id.strip_prefix("SX") {
        return acceder_sensor(a, true, clave, n);
    }
    if let Some(clave) = id.strip_prefix("SZ") {
        return acceder_sensor(a, false, clave, n);
    }
    if let Some(clave) = id.strip_prefix("GyroMouseStick") {
        if let Some(v) = suavizado_giroscopio(&mut a.suavizado_giroscopio_stick, clave, n) {
            return Some(v);
        }
    } else if let Some(clave) = id.strip_prefix("GyroMouse") {
        if let Some(v) = suavizado_giroscopio(&mut a.suavizado_giroscopio_raton, clave, n) {
            return Some(v);
        }
    }

    let panel = &mut a.tactil_como_stick;
    let gestos = &mut a.deslizar_giroscopio;
    let controles = &mut a.controles_giroscopio;
    match id {
        // Vibración.
        "RumbleBoost" => byte(&mut a.vibracion, n),
        // El archivo va en milisegundos y el modelo de vista en segundos.
        "RumbleAutostopTime" => match n {
            None => Some(json!(a.parada_vibracion / 1000)),
            Some(v) => {
                a.parada_vibracion = a_entero(v)?.wrapping_mul(1000);
                HECHO
            }
        },
        "InverseRumbleMotors" => si(&mut a.invertir_motores, n),
        "DualSenseRumbleEmulationPerIndex" => indice(
            &mut a.dualsense.vibracion.emulacion,
            &[
                EmulacionVibracion::Accurate,
                EmulacionVibracion::Legacy,
                EmulacionVibracion::Disabled,
            ],
            EmulacionVibracion::Accurate,
            n,
        ),
        "DualSenseHapticPowerLevelPerIndex" => byte(&mut a.dualsense.vibracion.potencia_haptica, n),

        // Ratón con botones.
        "ButtonMouseSensitivity" => entero(&mut a.sensibilidad_raton_botones, n),
        // `Convert.ToInt32(buttonVerticalScale * 100.0)` y vuelta con `value * 0.01`.
        "ButtonMouseVerticalScale" => match n {
            None => {
                let memoria = a.escala_vertical_raton_botones.clamp(0.0, 500.0) * 0.01;
                Some(json!((memoria * 100.0).round_ties_even() as i32))
            }
            Some(v) => {
                a.escala_vertical_raton_botones = a_entero(v)? as f64 * 0.01 * 100.0;
                HECHO
            }
        },
        "ButtonMouseOffset" => {
            doble_convertido(&mut a.desfase_raton_botones, n, |v| v * 100.0, |v| v * 0.01)
        }
        "MouseAcceleration" => si(&mut a.aceleracion_raton, n),

        // Volante.
        "SASteeringWheelEmulationAxisIndex" => enumeracion(
            &mut a.eje_volante,
            &[
                EjeVolante::None,
                EjeVolante::LX,
                EjeVolante::LY,
                EjeVolante::RX,
                EjeVolante::RY,
                EjeVolante::L2R2,
                EjeVolante::VJoy1X,
                EjeVolante::VJoy1Y,
                EjeVolante::VJoy1Z,
                EjeVolante::VJoy2X,
                EjeVolante::VJoy2Y,
                EjeVolante::VJoy2Z,
            ],
            0xff,
            n,
        ),
        // Un rango que no está en la lista se lee como 360, igual que en C#.
        "SASteeringWheelEmulationRangeIndex" => match n {
            None => Some(json!(RANGOS_VOLANTE
                .iter()
                .position(|r| *r == a.rango_volante)
                .unwrap_or(360))),
            Some(v) => {
                let i = usize::try_from(a_entero(v)?).ok()?;
                a.rango_volante = *RANGOS_VOLANTE.get(i)?;
                HECHO
            }
        },
        "SASteeringWheelFuzz" => entero_limitado(&mut a.difuminado_volante, n, 0, 100),
        "SASteeringWheelUseSmoothing" => si(&mut a.suavizado_volante.activo, n),
        "SASteeringWheelSmoothMinCutoff" => doble(&mut a.suavizado_volante.corte_minimo, n),
        "SASteeringWheelSmoothBeta" => doble(&mut a.suavizado_volante.beta, n),

        // Otros.
        // El archivo va en segundos y el modelo de vista en minutos.
        "IdleDisconnect" => match n {
            None => Some(json!(a.desconexion_inactivo / 60)),
            Some(v) => {
                let v = a_entero(v)?;
                if a.desconexion_inactivo / 60 != v {
                    a.desconexion_inactivo = v.wrapping_mul(60);
                }
                HECHO
            }
        },
        "DebouncingMs" => entero(&mut a.antirrebote_ms, n),
        "EnableOutputDataToDS5" => si(&mut a.enviar_al_mando, n),
        "DInputOnly" => si(&mut a.solo_dinput, n),
        "UseDs3PitchRollSim" => si(&mut a.simular_ds3, n),
        // No se guarda en el perfil: C# solo lo tiene en memoria.
        "VirtualTriggerButtons" => solo_memoria(json!(BOTONES_VIRTUALES_GATILLO), n),

        // Giroscopio.
        "GyroOutModeIndex" => indice(
            &mut a.salida_giroscopio,
            &[
                SalidaGiroscopio::Controls,
                SalidaGiroscopio::Mouse,
                SalidaGiroscopio::MouseJoystick,
                SalidaGiroscopio::DirectionalSwipe,
                SalidaGiroscopio::Passthru,
            ],
            SalidaGiroscopio::Controls,
            n,
        ),
        "gyroControles" => botones_valor(&mut controles.gatillos, n),
        "GyroControlsEvalCondIndex" => condicion(&mut controles.condicion, n),
        "GyroControlsTurns" => si(&mut controles.gatillo_alterna, n),
        "GyroControlsToggle" => si(&mut controles.alterna, n),
        "gyroRaton" => botones_valor(&mut a.gatillos_giroscopio, n),
        "GyroMouseEvalCondIndex" => condicion(&mut a.condicion_giroscopio, n),
        "GyroMouseTurns" => si(&mut a.gatillo_alterna_giroscopio, n),
        "GyroMouseToggle" => si(&mut a.alterna_giroscopio_raton, n),
        "GyroSensitivity" => entero(&mut a.sensibilidad_giroscopio, n),
        "GyroVertScale" => entero(&mut a.escala_vertical_giroscopio, n),
        "GyroMouseDeadZone" => entero(&mut a.zona_muerta_giroscopio_raton, n),
        "GyroMouseMinThreshold" => doble_limitado(&mut a.umbral_giroscopio_raton, n, 1.0, 40.0),
        "GyroMouseXAxis" => entero_limitado(&mut a.eje_horizontal_giroscopio_raton, n, 0, 1),
        "GyroMouseInvertX" => bit_entero(&mut a.invertir_giroscopio, 2, n),
        "GyroMouseInvertY" => bit_entero(&mut a.invertir_giroscopio, 1, n),
        "GyroMouseJitterCompensation" => si(&mut a.compensar_temblor_giroscopio_raton, n),
        "gyroStick" => botones_valor(&mut a.gatillos_giroscopio_stick, n),
        "GyroMouseStickEvalCondIndex" => condicion(&mut a.condicion_giroscopio_stick, n),
        "GyroMouseStickTurns" => si(&mut a.gatillo_alterna_giroscopio_stick, n),
        "GyroMouseStickToggle" => si(&mut a.alterna_giroscopio_stick, n),
        "GyroMouseStickOutputStick" => enumeracion(
            &mut a.stick_giroscopio,
            &[
                PalancaSalida::None,
                PalancaSalida::LeftStick,
                PalancaSalida::RightStick,
            ],
            0xff,
            n,
        ),
        "GyroMouseStickOutputAxes" => enumeracion(
            &mut a.ejes_giroscopio_stick,
            &[
                EjesSalida::None,
                EjesSalida::XY,
                EjesSalida::X,
                EjesSalida::Y,
            ],
            0xff,
            n,
        ),
        "GyroMouseStickDeadZone" => entero(&mut a.zona_muerta_giroscopio_stick, n),
        "GyroMouseStickMaxZone" => {
            entero_limitado(&mut a.zona_maxima_giroscopio_stick, n, 1, i32::MAX)
        }
        "GyroMouseStickAntiDeadX" => doble_convertido(
            &mut a.antizona_giroscopio_stick_x,
            n,
            |v| v * 100.0,
            |v| (v * 0.01).clamp(0.0, 1.0),
        ),
        "GyroMouseStickAntiDeadY" => doble_convertido(
            &mut a.antizona_giroscopio_stick_y,
            n,
            |v| v * 100.0,
            |v| (v * 0.01).clamp(0.0, 1.0),
        ),
        "GyroMouseStickVertScale" => entero(&mut a.escala_vertical_giroscopio_stick, n),
        "GyroMouseStickMaxOutputEnabled" => si(&mut a.salida_maxima_giroscopio_stick_activa, n),
        "GyroMouseStickMaxOutput" => {
            doble_limitado(&mut a.salida_maxima_giroscopio_stick, n, 0.0, 100.0)
        }
        "GyroMouseStickXAxis" => entero_limitado(&mut a.eje_horizontal_giroscopio_stick, n, 0, 1),
        "GyroMouseStickInvertX" => bit_natural(&mut a.invertir_giroscopio_stick, 1, n),
        "GyroMouseStickInvertY" => bit_natural(&mut a.invertir_giroscopio_stick, 2, n),
        "GyroMouseStickJitterCompensation" => si(&mut a.compensar_temblor_giroscopio_stick, n),
        "gyroGestos" => botones_valor(&mut gestos.gatillos, n),
        "GyroSwipeEvalCondIndex" => condicion(&mut gestos.condicion, n),
        "GyroSwipeTurns" => si(&mut gestos.gatillo_alterna, n),
        "GyroSwipeXAxis" => enumeracion(
            &mut gestos.eje_x,
            &[EjeDeslizar::Yaw, EjeDeslizar::Roll],
            0xffff,
            n,
        ),
        "GyroSwipeDeadZoneX" => entero(&mut gestos.zona_muerta_x, n),
        "GyroSwipeDeadZoneY" => entero(&mut gestos.zona_muerta_y, n),
        "GyroSwipeDelayTime" => entero(&mut gestos.retardo, n),

        // Panel táctil.
        "TouchpadOutputIndex" => indice(
            &mut a.salida_tactil,
            &[
                SalidaPanel::Mouse,
                SalidaPanel::Controls,
                SalidaPanel::MouseJoystick,
                SalidaPanel::AbsoluteMouse,
                SalidaPanel::Passthru,
            ],
            SalidaPanel::Mouse,
            n,
        ),
        "EnableTouchpadToggle" => si(&mut a.tactil_alterna, n),
        "StartTouchpadOff" => si(&mut a.empezar_tactil_apagado, n),
        "TouchpadClickPassthru" => si(&mut a.clic_tactil_directo, n),
        "LowerRightTouchRMB" => si(&mut a.esquina_inferior_derecha, n),
        "TouchSens" => byte(&mut a.sensibilidad_tactil, n),
        "TouchScroll" => entero(&mut a.sensibilidad_desplazamiento, n),
        "TouchTap" => byte(&mut a.sensibilidad_toque, n),
        "TouchDoubleTap" => si(&mut a.doble_toque, n),
        "TouchJitter" => si(&mut a.compensar_temblor_tactil, n),
        "TouchInvertIndex" => match n {
            None => Some(json!(INVERTIR_PANEL
                .iter()
                .position(|v| *v == a.invertir_tactil)
                .map_or(-1, |i| i as i64))),
            Some(v) => {
                let i = usize::try_from(a_entero(v)?).ok()?;
                a.invertir_tactil = (*INVERTIR_PANEL.get(i)?).clamp(0, 3);
                HECHO
            }
        },
        // En memoria va en radianes; al guardar, C# lo vuelve a pasar a radianes.
        "TouchRelMouseRotation" => doble_convertido(
            &mut a.rotacion_raton_tactil,
            n,
            |v| v.clamp(-180.0, 180.0) * PI / 180.0 * 180.0 / PI,
            |v| (v * PI / 180.0).clamp(-180.0, 180.0) * PI / 180.0,
        ),
        "TouchRelMouseMinThreshold" => doble_limitado(&mut a.umbral_raton_tactil, n, 1.0, 40.0),
        "TouchTrackball" => si(&mut a.bola, n),
        "TouchTrackballFriction" => doble(&mut a.friccion_bola, n),
        "TouchMouseStickOutputStick" => enumeracion(
            &mut panel.stick,
            &[
                PalancaSalida::None,
                PalancaSalida::LeftStick,
                PalancaSalida::RightStick,
            ],
            0xff,
            n,
        ),
        "TouchMouseStickOutputAxes" => enumeracion(
            &mut panel.ejes,
            &[
                EjesSalida::None,
                EjesSalida::XY,
                EjesSalida::X,
                EjesSalida::Y,
            ],
            0xff,
            n,
        ),
        "TouchMouseStickDeadZone" => entero(&mut panel.zona_muerta, n),
        "TouchMouseStickMaxZone" => entero(&mut panel.zona_maxima, n),
        "TouchMouseStickAntiDeadX" => {
            doble_convertido(&mut panel.antizona_x, n, |v| v * 100.0, |v| v * 0.01)
        }
        "TouchMouseStickAntiDeadY" => {
            doble_convertido(&mut panel.antizona_y, n, |v| v * 100.0, |v| v * 0.01)
        }
        "TouchMouseStickVertScale" => entero(&mut panel.escala_vertical, n),
        "TouchMouseStickMaxOutputEnabled" => si(&mut panel.salida_maxima_activa, n),
        "TouchMouseStickMaxOutput" => doble(&mut panel.salida_maxima, n),
        "TouchMouseStickInvertX" => bit_natural(&mut panel.invertir, 1, n),
        "TouchMouseStickInvertY" => bit_natural(&mut panel.invertir, 2, n),
        // Grados enteros en el archivo, radianes en memoria; al guardar, C# divide por 180 y
        // multiplica por pi en vez de al revés.
        "TouchMouseStickRotation" => match n {
            None => Some(real(panel.rotacion as f64 * PI / 180.0 * 180.0 / PI)),
            Some(v) => {
                let radianes = a_doble(v)? * PI / 180.0;
                panel.rotacion = (radianes / 180.0 * PI) as i32;
                HECHO
            }
        },
        // `UseSmoothing` es "hay método de suavizado".
        "TouchMouseStickSmooth" => match n {
            None => Some(json!(panel.suavizado.metodo != SuavizadoPanel::None)),
            Some(v) => {
                panel.suavizado.metodo = if a_bool(v)? {
                    SuavizadoPanel::OneEuro
                } else {
                    SuavizadoPanel::None
                };
                HECHO
            }
        },
        // No se guardan en el perfil: C# solo los tiene en memoria.
        "TouchMouseStickTrackball" => solo_memoria(json!(INERCIA_STICK_PANEL), n),
        "TouchMouseStickTrackballFriction" => solo_memoria(real(FRICCION_STICK_PANEL), n),
        "TouchAbsMouseMaxZoneX" => entero(&mut a.raton_absoluto_tactil.zona_maxima_x, n),
        "TouchAbsMouseMaxZoneY" => entero(&mut a.raton_absoluto_tactil.zona_maxima_y, n),
        "TouchAbsMouseSnapCenter" => si(&mut a.raton_absoluto_tactil.volver_al_centro, n),

        // Luz.
        "LightbarModeIndex" => indice(
            &mut a.modo_luz,
            &[ModoLuz::DS5Win, ModoLuz::Passthru],
            ModoLuz::DS5Win,
            n,
        ),
        "FlashAt" => entero(&mut a.destello_bateria_en, n),
        "FlashTypeIndex" => byte(&mut a.tipo_destello, n),
        "ChargingType" => entero(&mut a.tipo_carga, n),
        "LowColorR" => byte(&mut a.color_bajo.0, n),
        "LowColorG" => byte(&mut a.color_bajo.1, n),
        "LowColorB" => byte(&mut a.color_bajo.2, n),
        "MaxSatRainbow" => doble_convertido(
            &mut a.saturacion_arcoiris,
            n,
            |v| v.clamp(0.0, 100.0) / 100.0 * 100.0,
            |v| v / 100.0 * 100.0,
        ),
        _ => None,
    }
}

/// Un ajuste que C# no guarda en el perfil: se enseña el valor por defecto y cambiarlo no toca
/// el perfil.
fn solo_memoria(valor: Value, n: Nuevo) -> Option<Value> {
    match n {
        None => Some(valor),
        Some(_) => HECHO,
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// El ajuste `id` de lo que describe `describir`, esté en la sección que esté.
    fn ajuste(grupos: &Value, id: &str) -> Option<Value> {
        grupos
            .as_array()?
            .iter()
            .flat_map(|g| g["secciones"].as_array().unwrap())
            .flat_map(|s| s["ajustes"].as_array().unwrap())
            .find(|o| o["id"] == id)
            .cloned()
    }

    fn valor(p: &Perfil, id: &str) -> Value {
        ajuste(&describir(p, Idioma::Es), id).unwrap_or_else(|| panic!("no se ve {id}"))["valor"]
            .clone()
    }

    fn secciones(p: &Perfil, grupo: &str) -> Vec<String> {
        describir(p, Idioma::Es)
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["id"] == grupo)
            .unwrap()["secciones"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["nombre"].as_str().unwrap().to_string())
            .collect()
    }

    fn cerca(v: &Value, esperado: f64) -> bool {
        (v.as_f64().unwrap() - esperado).abs() < 1e-9
    }

    #[test]
    fn los_grupos_en_el_orden_de_csharp() {
        let d = describir(&Perfil::default(), Idioma::Es);
        let ids: Vec<&str> = d
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            ids,
            [
                "sticks",
                "gatillos",
                "giroscopio",
                "panel",
                "luz",
                "vibracion",
                "raton",
                "volante",
                "otros"
            ]
        );
        assert_eq!(d[0]["nombre"], "Sticks");
        assert_eq!(d[3]["nombre"], "Panel táctil");
    }

    #[test]
    fn en_ingles_y_en_portugues() {
        let p = Perfil::default();
        let en = describir(&p, Idioma::En);
        assert_eq!(en[0]["nombre"], "Sticks");
        assert_eq!(en[3]["nombre"], "Touchpad");
        let s = &en[0]["secciones"];
        assert_eq!(s[0]["id"], "Stick izquierdo");
        assert_eq!(s[0]["nombre"], "Left stick");
        assert_eq!(s[1]["id"], "Stick izquierdo: precisión");
        assert_eq!(s[1]["nombre"], "Left stick: precision");
        assert_eq!(s[2]["nombre"], "Left stick: drift correction");
        let zona = ajuste(&en, "LSDeadZone").unwrap();
        assert_eq!(zona["nombre"], "Dead zone");
        let curva = ajuste(&en, "LSOutputCurveIndex").unwrap();
        assert_eq!(curva["opciones"][1], "Enhanced precision");
        let botones = ajuste(&en, "gyroControles").unwrap();
        assert_eq!(botones["opciones"][0], "Cross");
        assert_eq!(botones["opciones"][16], "Options");
        let haptica = ajuste(&en, "DualSenseHapticPowerLevelPerIndex").unwrap();
        assert_eq!(haptica["opciones"][0], "100%");

        let pt = describir(&p, Idioma::Pt);
        assert_eq!(pt[0]["nombre"], "Analógicos");
        assert_eq!(pt[1]["secciones"][0]["nombre"], "L2");
        assert_eq!(
            ajuste(&pt, "RumbleBoost").unwrap()["nombre"],
            "Reforço de vibração"
        );
        assert_eq!(
            ajuste(&pt, "L2TriggerMode").unwrap()["opciones"][0],
            "Desativado"
        );
        assert_eq!(ajuste(&pt, "GyroOutModeIndex").unwrap()["nombre"], "Modo");
        assert_eq!(pt[4]["secciones"][1]["id"], "Color con batería baja");
        assert_eq!(pt[4]["secciones"][1]["nombre"], "Cor com bateria fraca");
        // Los ids de las secciones no cambian con el idioma.
        let ids = |d: &Value| -> Vec<Value> {
            d.as_array()
                .unwrap()
                .iter()
                .flat_map(|g| g["secciones"].as_array().unwrap().clone())
                .map(|s| s["id"].clone())
                .collect()
        };
        let es = describir(&p, Idioma::Es);
        assert_eq!(ids(&es), ids(&en));
        assert_eq!(ids(&es), ids(&pt));
        for s in es
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|g| g["secciones"].as_array().unwrap())
        {
            assert_eq!(s["id"], s["nombre"]);
        }
    }

    /// Todos los textos de las opciones están traducidos (salvo los que se escriben igual).
    #[test]
    fn no_falta_ninguna_traduccion() {
        let es = describir(&Perfil::default(), Idioma::Es);
        let mut faltan = Vec::new();
        let mut mirar = |t: &str| {
            let sin_traducir = t.chars().any(|c| c.is_alphabetic())
                && Idioma::En.texto(t) == t
                && !crate::textos::TEXTOS.iter().any(|(k, _, _)| *k == t)
                && !t.contains(": ");
            if sin_traducir
                && ![
                    "DS5Control",
                    "L2",
                    "R2",
                    "L1",
                    "R1",
                    "L3",
                    "R3",
                    "PS",
                    "Options",
                    "Mute",
                ]
                .contains(&t)
            {
                faltan.push(t.to_string());
            }
        };
        for g in es.as_array().unwrap() {
            mirar(g["nombre"].as_str().unwrap());
            for s in g["secciones"].as_array().unwrap() {
                mirar(s["nombre"].as_str().unwrap());
                for a in s["ajustes"].as_array().unwrap() {
                    mirar(a["nombre"].as_str().unwrap());
                    for o in a["opciones"].as_array().into_iter().flatten() {
                        mirar(o.as_str().unwrap());
                    }
                }
            }
        }
        assert!(faltan.is_empty(), "{faltan:?}");
    }

    #[test]
    fn cada_tipo_se_describe_como_en_csharp() {
        let d = describir(&Perfil::default(), Idioma::Es);
        assert_eq!(
            ajuste(&d, "LSDeadZone").unwrap(),
            json!({
                "id": "LSDeadZone", "nombre": "Zona muerta", "tipo": "numero",
                "min": 0, "max": 1, "paso": 0.01, "porcentaje": true, "opciones": null,
                "valor": 0.08,
            })
        );
        assert_eq!(
            ajuste(&d, "MouseAcceleration").unwrap(),
            json!({
                "id": "MouseAcceleration", "nombre": "Aceleración", "tipo": "interruptor",
                "min": 0, "max": 0, "paso": 0, "porcentaje": false, "opciones": null,
                "valor": false,
            })
        );
        assert_eq!(
            ajuste(&d, "FlashTypeIndex").unwrap(),
            json!({
                "id": "FlashTypeIndex", "nombre": "Forma de avisar", "tipo": "opciones",
                "min": 0, "max": 0, "paso": 0, "porcentaje": false,
                "opciones": ["Parpadeo", "Pulso"], "valor": 0,
            })
        );
        let botones = ajuste(&d, "gyroControles").unwrap();
        assert_eq!(botones["tipo"], "botones");
        assert_eq!(botones["nombre"], "Se activa con");
        assert_eq!(botones["opciones"].as_array().unwrap().len(), 27);
        assert_eq!(botones["valor"], json!([]));
        let corte = ajuste(&d, "SASteeringWheelSmoothMinCutoff").unwrap();
        assert!(cerca(&corte["min"], 0.00001));
        assert_eq!(corte["max"], 1000);
    }

    #[test]
    fn las_secciones_se_ven_segun_el_perfil() {
        let mut p = Perfil::default();
        assert_eq!(
            secciones(&p, "giroscopio"),
            [
                "Uso del giroscopio",
                "Sensores al juego",
                "Sensibilidad de sensores"
            ]
        );
        assert_eq!(
            secciones(&p, "panel"),
            ["Uso del panel", "Ratón con el panel"]
        );
        assert_eq!(
            secciones(&p, "sticks"),
            [
                "Stick izquierdo",
                "Stick izquierdo: precisión",
                "Stick izquierdo: corrección de deriva",
                "Stick derecho",
                "Stick derecho: precisión",
                "Stick derecho: corrección de deriva"
            ]
        );
        assert!(aplicar(&mut p, "GyroOutModeIndex", &json!(2)));
        assert!(aplicar(&mut p, "LSOutputCurveIndex", &json!(6)));
        assert!(aplicar(&mut p, "RSOutputIndex", &json!(2)));
        assert!(aplicar(&mut p, "TouchpadOutputIndex", &json!(3)));
        assert!(aplicar(&mut p, "SZOutputCurveIndex", &json!(6)));
        assert!(aplicar(&mut p, "R2OutputCurveIndex", &json!(6)));
        assert_eq!(
            secciones(&p, "giroscopio"),
            [
                "Uso del giroscopio",
                "Stick con giroscopio",
                "Sensibilidad de sensores",
                "Curva frontal personalizada"
            ]
        );
        assert_eq!(secciones(&p, "panel"), ["Uso del panel", "Ratón absoluto"]);
        let sticks = secciones(&p, "sticks");
        assert!(sticks.contains(&"Stick izquierdo: curva personalizada".to_string()));
        assert!(sticks.contains(&"Stick derecho: flick stick".to_string()));
        assert!(!sticks.contains(&"Stick izquierdo: flick stick".to_string()));
        assert!(secciones(&p, "gatillos").contains(&"R2: curva personalizada".to_string()));
    }

    #[test]
    fn los_valores_por_defecto_de_csharp() {
        let p = Perfil::default();
        for (id, esperado) in [
            ("LSDeadZone", json!(0.08)),
            ("LSDeadTypeIndex", json!(0)),
            ("LSAntiDeadZone", json!(0.2)),
            ("LSMaxZone", json!(1)),
            ("LSMaxOutput", json!(1)),
            ("LSSens", json!(1)),
            ("LSVerticalScale", json!(1)),
            ("LSRotation", json!(0)),
            ("LSOutputCurveIndex", json!(0)),
            ("LSOutputIndex", json!(1)),
            ("LSSquareRoundness", json!(5)),
            ("LSAntiSnapbackDelta", json!(135)),
            ("LSAntiSnapbackTimeout", json!(50)),
            ("LSOuterBindDead", json!(0.75)),
            ("LSDeltaMultiplier", json!(4)),
            ("LeftStickDriftXAxis", json!(0)),
            ("L2DeadZone", json!(0)),
            ("L2MaxZone", json!(1)),
            ("L2TriggerMode", json!(0)),
            ("L2HipFireTime", json!(100)),
            ("GyroOutModeIndex", json!(0)),
            ("GyroControlsEvalCondIndex", json!(0)),
            ("GyroControlsTurns", json!(true)),
            ("SXDeadZone", json!(0.25)),
            ("SXMaxZone", json!(1)),
            ("SXAntiDeadZone", json!(0)),
            ("TouchpadOutputIndex", json!(0)),
            ("TouchSens", json!(100)),
            ("TouchInvertIndex", json!(0)),
            ("TouchRelMouseMinThreshold", json!(1)),
            ("TouchJitter", json!(true)),
            ("LightbarModeIndex", json!(0)),
            ("MaxSatRainbow", json!(100)),
            ("RumbleBoost", json!(100)),
            ("RumbleAutostopTime", json!(0)),
            ("DualSenseRumbleEmulationPerIndex", json!(0)),
            ("ButtonMouseSensitivity", json!(25)),
            ("ButtonMouseVerticalScale", json!(100)),
            ("SASteeringWheelEmulationAxisIndex", json!(0)),
            ("SASteeringWheelEmulationRangeIndex", json!(3)),
            ("IdleDisconnect", json!(0)),
            ("EnableOutputDataToDS5", json!(true)),
            ("VirtualTriggerButtons", json!(true)),
        ] {
            assert_eq!(valor(&p, id), esperado, "{id}");
        }
        assert!(cerca(&valor(&p, "ButtonMouseOffset"), 0.8));
        assert!(cerca(&valor(&p, "LSDeltaMinTravel"), 0.01));
        let mut p = p;
        aplicar(&mut p, "GyroOutModeIndex", &json!(1));
        assert_eq!(valor(&p, "GyroMouseSmoothWeight"), json!(0.5));
        assert_eq!(valor(&p, "GyroMouseSmoothMethodIndex"), json!(0));
        aplicar(&mut p, "GyroOutModeIndex", &json!(2));
        assert!(cerca(&valor(&p, "GyroMouseStickAntiDeadX"), 40.0));
        assert_eq!(valor(&p, "GyroMouseStickMaxZone"), json!(830));
        aplicar(&mut p, "TouchpadOutputIndex", &json!(2));
        assert_eq!(valor(&p, "TouchMouseStickOutputStick"), json!(2));
        assert_eq!(valor(&p, "TouchMouseStickOutputAxes"), json!(1));
        assert_eq!(valor(&p, "TouchMouseStickTrackball"), json!(true));
        assert_eq!(valor(&p, "TouchMouseStickTrackballFriction"), json!(10));
    }

    #[test]
    fn los_interruptores_y_los_bits() {
        let mut p = Perfil::default();
        assert!(aplicar(&mut p, "MouseAcceleration", &json!(true)));
        assert!(p.ajustes.aceleracion_raton);
        assert!(!aplicar(&mut p, "MouseAcceleration", &json!(true)));
        // `Convert.ChangeType` pasa un número a booleano.
        assert!(aplicar(&mut p, "MouseAcceleration", &json!(0)));
        assert!(!p.ajustes.aceleracion_raton);
        assert!(!aplicar(&mut p, "MouseAcceleration", &json!("sí")));

        assert!(aplicar(&mut p, "GyroMouseInvertX", &json!(true)));
        assert_eq!(p.ajustes.invertir_giroscopio, 2);
        assert!(aplicar(&mut p, "GyroMouseInvertY", &json!(true)));
        assert_eq!(p.ajustes.invertir_giroscopio, 3);
        assert!(aplicar(&mut p, "GyroMouseInvertX", &json!(false)));
        assert_eq!(p.ajustes.invertir_giroscopio, 1);
        assert!(aplicar(&mut p, "GyroMouseStickInvertY", &json!(true)));
        assert_eq!(p.ajustes.invertir_giroscopio_stick, 2);
        assert!(aplicar(&mut p, "TouchMouseStickInvertX", &json!(true)));
        assert_eq!(p.ajustes.tactil_como_stick.invertir, 1);

        assert!(aplicar(&mut p, "TouchMouseStickSmooth", &json!(true)));
        assert_eq!(
            p.ajustes.tactil_como_stick.suavizado.metodo,
            SuavizadoPanel::OneEuro
        );
        assert!(aplicar(&mut p, "LSDeltaAccelEnabled", &json!(true)));
        assert!(p.ajustes.ls_aceleracion.activa);
        assert!(aplicar(&mut p, "UseDs3PitchRollSim", &json!(true)));
        assert!(p.ajustes.simular_ds3);
    }

    #[test]
    fn las_zonas_de_los_sticks() {
        let mut p = Perfil::default();
        // `Math.Round(value * 127d)` redondea al par.
        assert!(aplicar(&mut p, "LSDeadZone", &json!(0.5)));
        assert_eq!(p.ajustes.ls_zona_muerta, 64);
        assert_eq!(valor(&p, "LSDeadZone"), json!(0.5));
        // Lo que ya se ve igual con dos decimales no se toca.
        assert!(!aplicar(&mut p, "LSDeadZone", &json!(0.5)));
        assert!(aplicar(&mut p, "RSDeadZone", &json!(2.0)));
        assert_eq!(p.ajustes.rs_zona_muerta, 127);
        // `(int)(value * 100.0)` trunca: 0,29 * 100 es 28,999...
        assert!(aplicar(&mut p, "LSAntiDeadZone", &json!(0.29)));
        assert_eq!(p.ajustes.ls_antizona, 28);
        assert!(aplicar(&mut p, "LSMaxZone", &json!(0.5)));
        assert_eq!(p.ajustes.ls_zona_maxima, 50);
        assert!(aplicar(&mut p, "LSMaxZone", &json!(1.5)));
        assert_eq!(p.ajustes.ls_zona_maxima, 100);
        assert!(aplicar(&mut p, "LSMaxOutput", &json!(0.5)));
        assert_eq!(p.ajustes.ls_salida_maxima, 50.0);
        assert!(aplicar(&mut p, "RSVerticalScale", &json!(5.0)));
        assert_eq!(p.ajustes.rs_escala_vertical, 400.0);
        assert!(aplicar(&mut p, "LSFuzz", &json!(150)));
        assert_eq!(p.ajustes.ls_difuminado, 100);
        assert!(aplicar(&mut p, "LSOuterBindDead", &json!(0.5)));
        assert_eq!(p.ajustes.ls_zona_exterior, 50.0);
        assert!(aplicar(&mut p, "LSRotation", &json!(45)));
        assert!((p.ajustes.ls_rotacion - 45.0).abs() < 1e-9);
        assert!(cerca(&valor(&p, "LSRotation"), 45.0));
        assert!(aplicar(&mut p, "RSSens", &json!(2.5)));
        assert_eq!(p.ajustes.sensibilidades[1], 2.5);
        assert!(aplicar(&mut p, "LSDeltaMultiplier", &json!(20)));
        assert_eq!(p.ajustes.ls_aceleracion.multiplicador, 10.0);
        assert!(!aplicar(&mut p, "RSDeltaMinFactor", &json!(0)));
        assert_eq!(p.ajustes.rs_aceleracion.factor_minimo, 1.0);
        assert!(aplicar(&mut p, "LSFlickRWC", &json!(12.5)));
        assert_eq!(p.ajustes.ls_salida.flick.calibracion, 12.5);
        assert!(aplicar(&mut p, "LSAntiSnapbackTimeout", &json!(2.5)));
        assert_eq!(p.ajustes.ls_antirrebote_tiempo, 2);
        assert!(aplicar(&mut p, "LSDeadTypeIndex", &json!(1)));
        assert_eq!(p.ajustes.ls_tipo_zona_muerta, TipoZonaMuerta::Axial);
        assert!(aplicar(&mut p, "LSDeadTypeIndex", &json!(7)));
        assert_eq!(p.ajustes.ls_tipo_zona_muerta, TipoZonaMuerta::Radial);
    }

    #[test]
    fn las_derivas_son_sbyte() {
        let mut p = Perfil::default();
        assert!(aplicar(&mut p, "LeftStickDriftXAxis", &json!(-5)));
        assert_eq!(p.ajustes.deriva_izq_x, -5);
        assert!(aplicar(&mut p, "RightStickDriftYAxis", &json!(2.5)));
        assert_eq!(p.ajustes.deriva_der_y, 2);
        assert!(!aplicar(&mut p, "LeftStickDriftYAxis", &json!(200)));
        assert_eq!(p.ajustes.deriva_izq_y, 0);
    }

    #[test]
    fn los_gatillos() {
        let mut p = Perfil::default();
        // `(byte)(value * 255.0)` trunca.
        assert!(aplicar(&mut p, "L2DeadZone", &json!(0.5)));
        assert_eq!(p.ajustes.l2_zona_muerta, 127);
        assert!(aplicar(&mut p, "R2AntiDeadZone", &json!(0.1)));
        assert_eq!(p.ajustes.r2_antizona, 10);
        assert!(aplicar(&mut p, "L2MaxOutput", &json!(0.5)));
        assert_eq!(p.ajustes.l2_salida_maxima, 50.0);
        assert!(aplicar(&mut p, "L2MaxOutput", &json!(2)));
        assert_eq!(p.ajustes.l2_salida_maxima, 100.0);
        assert!(aplicar(&mut p, "R2Sens", &json!(3)));
        assert_eq!(p.ajustes.sensibilidades[3], 3.0);
        assert!(aplicar(&mut p, "L2HipFireTime", &json!(9000)));
        assert_eq!(p.ajustes.l2_tiempo_cadera, 5000);
        assert!(aplicar(&mut p, "R2TriggerMode", &json!(4)));
        assert_eq!(p.ajustes.r2_dos_fases, DosFases::HipFire);
        // `GetInt32` no acepta decimales y la enumeración no tiene el 9.
        assert!(!aplicar(&mut p, "R2TriggerMode", &json!(1.5)));
        assert!(!aplicar(&mut p, "R2TriggerMode", &json!(9)));
        assert_eq!(p.ajustes.r2_dos_fases, DosFases::HipFire);
    }

    #[test]
    fn las_curvas() {
        let mut p = Perfil::default();
        assert!(aplicar(&mut p, "SXOutputCurveIndex", &json!(3)));
        assert_eq!(p.ajustes.sx_curva, Curva::Cubica);
        // Un índice que no es una curva se guarda como lineal.
        assert!(aplicar(&mut p, "SXOutputCurveIndex", &json!(9)));
        assert_eq!(p.ajustes.sx_curva, Curva::Lineal);
        assert!(aplicar(&mut p, "LSOutputCurveIndex", &json!(6)));
        assert_eq!(valor(&p, "LSCustomCurve"), Value::Null);
        assert!(aplicar(&mut p, "LSCustomCurve", &json!("0.1,0.2,0.3,0.4")));
        assert_eq!(
            p.ajustes.ls_curva_personalizada.as_deref(),
            Some("0.1,0.2,0.3,0.4")
        );
        assert_eq!(valor(&p, "LSCustomCurve"), json!("0.1,0.2,0.3,0.4"));
        assert!(!aplicar(&mut p, "LSCustomCurve", &json!(null)));
        // Lo que decide si se ve la sección no es un ajuste que se pueda cambiar.
        assert!(!aplicar(&mut p, "LSCustomCurveSelected", &json!(false)));
    }

    #[test]
    fn los_sensores_en_porcentaje() {
        let mut p = Perfil::default();
        assert!(aplicar(&mut p, "SXMaxZone", &json!(0.5)));
        assert_eq!(p.ajustes.sx_zona_maxima, 50.0);
        assert_eq!(valor(&p, "SXMaxZone"), json!(0.5));
        assert!(aplicar(&mut p, "SZAntiDeadZone", &json!(0.25)));
        assert_eq!(p.ajustes.sz_antizona, 25.0);
        assert!(aplicar(&mut p, "SZDeadZone", &json!(0.1)));
        assert_eq!(p.ajustes.sz_zona_muerta, 0.1);
        assert!(aplicar(&mut p, "SXSens", &json!(2)));
        assert_eq!(p.ajustes.sensibilidades[4], 2.0);
    }

    #[test]
    fn los_botones_del_giroscopio() {
        let mut p = Perfil::default();
        assert!(aplicar(&mut p, "gyroRaton", &json!([7, 2, 40, -1, 2])));
        assert_eq!(p.ajustes.gatillos_giroscopio, "2,2,7");
        assert!(aplicar(&mut p, "gyroRaton", &json!([])));
        assert_eq!(p.ajustes.gatillos_giroscopio, "-1");
        assert!(aplicar(&mut p, "gyroGestos", &json!([0])));
        assert_eq!(p.ajustes.deslizar_giroscopio.gatillos, "0");
        assert!(aplicar(&mut p, "gyroControles", &json!([26])));
        assert_eq!(p.ajustes.controles_giroscopio.gatillos, "26");
        assert!(!aplicar(&mut p, "gyroStick", &json!([1.5])));
        assert!(!aplicar(&mut p, "gyroStick", &json!(3)));
        p.ajustes.gatillos_giroscopio_stick = "3,x,,5".into();
        aplicar(&mut p, "GyroOutModeIndex", &json!(2));
        assert_eq!(valor(&p, "gyroStick"), json!([3, 5]));
    }

    #[test]
    fn el_giroscopio() {
        let mut p = Perfil::default();
        assert!(aplicar(&mut p, "GyroMouseEvalCondIndex", &json!(1)));
        assert!(!p.ajustes.condicion_giroscopio);
        assert!(aplicar(&mut p, "GyroSwipeEvalCondIndex", &json!(1)));
        assert!(!p.ajustes.deslizar_giroscopio.condicion);
        assert!(aplicar(&mut p, "GyroMouseSmoothWeight", &json!(0.3)));
        assert!((p.ajustes.suavizado_giroscopio_raton.peso - 30.0).abs() < 1e-9);
        assert!(aplicar(&mut p, "GyroMouseSmoothMethodIndex", &json!(1)));
        assert_eq!(
            p.ajustes.suavizado_giroscopio_raton.metodo,
            MetodoSuavizado::MediaPonderada
        );
        assert!(aplicar(
            &mut p,
            "GyroMouseStickSmoothMethodIndex",
            &json!(1)
        ));
        assert!(aplicar(
            &mut p,
            "GyroMouseStickSmoothMethodIndex",
            &json!(0)
        ));
        assert_eq!(
            p.ajustes.suavizado_giroscopio_stick.metodo,
            MetodoSuavizado::UnEuro
        );
        assert!(aplicar(&mut p, "GyroMouseOneEuroBeta", &json!(3)));
        assert_eq!(p.ajustes.suavizado_giroscopio_raton.beta, 1.0);
        assert!(aplicar(
            &mut p,
            "GyroMouseStickOneEuroMinCutoff",
            &json!(500)
        ));
        assert_eq!(p.ajustes.suavizado_giroscopio_stick.corte_minimo, 100.0);
        assert!(aplicar(&mut p, "GyroMouseStickSmooth", &json!(true)));
        assert!(p.ajustes.suavizado_giroscopio_stick.activo);
        assert!(aplicar(&mut p, "GyroMouseMinThreshold", &json!(100)));
        assert_eq!(p.ajustes.umbral_giroscopio_raton, 40.0);
        assert!(aplicar(&mut p, "GyroMouseStickMaxZone", &json!(0)));
        assert_eq!(p.ajustes.zona_maxima_giroscopio_stick, 1);
        assert!(aplicar(&mut p, "GyroMouseStickAntiDeadX", &json!(50)));
        assert_eq!(p.ajustes.antizona_giroscopio_stick_x, 0.5);
        assert!(aplicar(&mut p, "GyroMouseStickAntiDeadY", &json!(300)));
        assert_eq!(p.ajustes.antizona_giroscopio_stick_y, 1.0);
        assert!(aplicar(&mut p, "GyroMouseStickMaxOutput", &json!(50)));
        assert!(aplicar(&mut p, "GyroMouseStickMaxOutput", &json!(120)));
        assert_eq!(p.ajustes.salida_maxima_giroscopio_stick, 100.0);
        assert!(aplicar(&mut p, "GyroMouseStickOutputStick", &json!(1)));
        assert_eq!(p.ajustes.stick_giroscopio, PalancaSalida::LeftStick);
        // `(Enum)value` sobre un `byte` se queda con los 8 bits de abajo.
        assert!(aplicar(&mut p, "GyroMouseStickOutputStick", &json!(258)));
        assert_eq!(p.ajustes.stick_giroscopio, PalancaSalida::RightStick);
        assert!(!aplicar(&mut p, "GyroMouseStickOutputAxes", &json!(4)));
        assert!(aplicar(&mut p, "GyroMouseXAxis", &json!(5)));
        assert_eq!(p.ajustes.eje_horizontal_giroscopio_raton, 1);
        assert!(aplicar(&mut p, "GyroSwipeXAxis", &json!(1)));
        assert_eq!(p.ajustes.deslizar_giroscopio.eje_x, EjeDeslizar::Roll);
        assert!(aplicar(&mut p, "GyroOutModeIndex", &json!(4)));
        assert_eq!(p.ajustes.salida_giroscopio, SalidaGiroscopio::Passthru);
        assert!(aplicar(&mut p, "GyroOutModeIndex", &json!(8)));
        assert_eq!(p.ajustes.salida_giroscopio, SalidaGiroscopio::Controls);
        assert!(aplicar(&mut p, "GyroSensitivity", &json!(12.5)));
        assert_eq!(p.ajustes.sensibilidad_giroscopio, 12);
    }

    #[test]
    fn el_panel() {
        let mut p = Perfil::default();
        assert!(aplicar(&mut p, "TouchpadOutputIndex", &json!(2)));
        assert_eq!(p.ajustes.salida_tactil, SalidaPanel::MouseJoystick);
        assert!(aplicar(&mut p, "TouchInvertIndex", &json!(1)));
        assert_eq!(p.ajustes.invertir_tactil, 2);
        assert!(!aplicar(&mut p, "TouchInvertIndex", &json!(4)));
        // `(byte)value` se queda con los 8 bits de abajo.
        assert!(aplicar(&mut p, "TouchSens", &json!(300)));
        assert_eq!(p.ajustes.sensibilidad_tactil, 44);
        assert!(aplicar(&mut p, "TouchScroll", &json!(300)));
        assert_eq!(p.ajustes.sensibilidad_desplazamiento, 300);
        assert!(aplicar(&mut p, "TouchRelMouseMinThreshold", &json!(5)));
        assert!(aplicar(&mut p, "TouchRelMouseMinThreshold", &json!(0)));
        assert_eq!(p.ajustes.umbral_raton_tactil, 1.0);
        assert!(aplicar(&mut p, "TouchMouseStickAntiDeadX", &json!(25)));
        assert_eq!(p.ajustes.tactil_como_stick.antizona_x, 0.25);
        assert!(aplicar(&mut p, "TouchAbsMouseMaxZoneY", &json!(70)));
        assert_eq!(p.ajustes.raton_absoluto_tactil.zona_maxima_y, 70);
        // Las rotaciones se quedan como las deja C# al guardar.
        assert!(!aplicar(&mut p, "TouchMouseStickRotation", &json!(90)));
        assert_eq!(p.ajustes.tactil_como_stick.rotacion, 0);
        assert!(aplicar(&mut p, "TouchRelMouseRotation", &json!(90)));
        assert!((p.ajustes.rotacion_raton_tactil - PI / 2.0 * PI / 180.0).abs() < 1e-12);
        // La inercia del stick no se guarda en el perfil.
        assert!(!aplicar(&mut p, "TouchMouseStickTrackball", &json!(false)));
        assert!(!aplicar(
            &mut p,
            "TouchMouseStickTrackballFriction",
            &json!(5)
        ));
    }

    #[test]
    fn la_luz_la_vibracion_y_el_raton() {
        let mut p = Perfil::default();
        assert!(aplicar(&mut p, "LightbarModeIndex", &json!(1)));
        assert_eq!(p.ajustes.modo_luz, ModoLuz::Passthru);
        assert!(aplicar(&mut p, "LowColorG", &json!(255)));
        assert_eq!(p.ajustes.color_bajo.1, 255);
        assert!(aplicar(&mut p, "MaxSatRainbow", &json!(50)));
        assert_eq!(p.ajustes.saturacion_arcoiris, 50.0);
        assert!(aplicar(&mut p, "FlashAt", &json!(30)));
        assert_eq!(p.ajustes.destello_bateria_en, 30);
        assert!(aplicar(&mut p, "ChargingType", &json!(2)));
        assert_eq!(p.ajustes.tipo_carga, 2);

        assert!(aplicar(&mut p, "RumbleAutostopTime", &json!(3)));
        assert_eq!(p.ajustes.parada_vibracion, 3000);
        assert_eq!(valor(&p, "RumbleAutostopTime"), json!(3));
        assert!(aplicar(&mut p, "RumbleBoost", &json!(150)));
        assert_eq!(p.ajustes.vibracion, 150);
        assert!(aplicar(
            &mut p,
            "DualSenseRumbleEmulationPerIndex",
            &json!(2)
        ));
        assert_eq!(
            p.ajustes.dualsense.vibracion.emulacion,
            EmulacionVibracion::Disabled
        );
        assert!(aplicar(
            &mut p,
            "DualSenseHapticPowerLevelPerIndex",
            &json!(3)
        ));
        assert_eq!(p.ajustes.dualsense.vibracion.potencia_haptica, 3);

        assert!(aplicar(&mut p, "ButtonMouseVerticalScale", &json!(250)));
        assert!((p.ajustes.escala_vertical_raton_botones - 250.0).abs() < 1e-9);
        assert_eq!(valor(&p, "ButtonMouseVerticalScale"), json!(250));
        assert!(aplicar(&mut p, "ButtonMouseOffset", &json!(2)));
        assert_eq!(p.ajustes.desfase_raton_botones, 0.02);
        assert!(aplicar(&mut p, "ButtonMouseSensitivity", &json!(40)));
        assert_eq!(p.ajustes.sensibilidad_raton_botones, 40);
    }

    #[test]
    fn el_volante_y_los_otros() {
        let mut p = Perfil::default();
        assert!(aplicar(
            &mut p,
            "SASteeringWheelEmulationAxisIndex",
            &json!(5)
        ));
        assert_eq!(p.ajustes.eje_volante, EjeVolante::L2R2);
        assert!(!aplicar(
            &mut p,
            "SASteeringWheelEmulationAxisIndex",
            &json!(12)
        ));
        assert!(aplicar(
            &mut p,
            "SASteeringWheelEmulationRangeIndex",
            &json!(8)
        ));
        assert_eq!(p.ajustes.rango_volante, 1440);
        assert!(!aplicar(
            &mut p,
            "SASteeringWheelEmulationRangeIndex",
            &json!(9)
        ));
        p.ajustes.rango_volante = 500;
        assert_eq!(valor(&p, "SASteeringWheelEmulationRangeIndex"), json!(360));
        assert!(aplicar(&mut p, "SASteeringWheelFuzz", &json!(10)));
        assert!(aplicar(&mut p, "SASteeringWheelFuzz", &json!(-3)));
        assert_eq!(p.ajustes.difuminado_volante, 0);
        assert!(aplicar(&mut p, "SASteeringWheelSmoothBeta", &json!(0.5)));
        assert_eq!(p.ajustes.suavizado_volante.beta, 0.5);

        assert!(aplicar(&mut p, "IdleDisconnect", &json!(5)));
        assert_eq!(p.ajustes.desconexion_inactivo, 300);
        p.ajustes.desconexion_inactivo = 330;
        assert_eq!(valor(&p, "IdleDisconnect"), json!(5));
        assert!(!aplicar(&mut p, "IdleDisconnect", &json!(5)));
        assert!(aplicar(&mut p, "DebouncingMs", &json!(20)));
        assert_eq!(p.ajustes.antirrebote_ms, 20);
        assert!(aplicar(&mut p, "DInputOnly", &json!(true)));
        assert!(p.ajustes.solo_dinput);
        // No se guarda en el perfil.
        assert!(!aplicar(&mut p, "VirtualTriggerButtons", &json!(false)));
    }

    #[test]
    fn lo_que_no_existe_no_cambia_nada() {
        let mut p = Perfil::default();
        assert!(!aplicar(&mut p, "NoExiste", &json!(1)));
        assert!(!aplicar(&mut p, "LSDeadZone", &json!("mucho")));
        assert!(!aplicar(&mut p, "RumbleBoost", &json!(true)));
        assert!(!aplicar(&mut p, "RumbleBoost", &json!(1e12)));
        assert_eq!(p, Perfil::default());
    }

    #[test]
    fn todos_los_ajustes_se_leen() {
        let mut a = Perfil::default().ajustes;
        for g in grupos() {
            for s in &g.secciones {
                if let Some((id, _)) = &s.cuando {
                    assert!(acceder(&mut a, id, None).is_some(), "{id}");
                }
                for o in &s.ajustes {
                    assert!(acceder(&mut a, &o.id, None).is_some(), "{}", o.id);
                }
            }
        }
    }
}
