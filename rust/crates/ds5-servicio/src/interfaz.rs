//! Lo que la interfaz ve de cada mando y los ajustes que cambia, con los mismos mensajes JSON que
//! `WebUiWindow.cs` (`ToState`, `RunCommand`) y `ControllerService.cs` (`SetLightColor`...).

use serde_json::{json, Value};

use ds5_hid::Modelo;
use ds5_informes::{boton, Conexion, Estado};
use ds5_perfiles::mapeos::{
    nombre_salida, salida_por_nombre_enum, Accion, AjusteControl, CONTROLES,
};
use ds5_perfiles::perfil::{EfectoGatillo, MandoVirtual};
use ds5_perfiles::{Perfil, Rgb};
use ds5_virtual::Tipo;

use crate::mando::{Mando, Vivo};

/// `RainbowSeconds`: el arcoíris que pone el interruptor de la interfaz.
pub const SEGUNDOS_ARCOIRIS: f64 = 4.0;
/// `MaxRumbleTestMs`.
pub const PRUEBA_VIBRACION_MAXIMA_MS: u64 = 10_000;
/// `ControllerService.MaxRumbleStrength`.
const VIBRACION_MAXIMA: i64 = 200;

/// Los controles que se pueden reasignar desde la interfaz (`MappableControls`).
pub const MAPEABLES: [&str; 17] = [
    "Cross",
    "Circle",
    "Square",
    "Triangle",
    "L1",
    "R1",
    "L2",
    "R2",
    "L3",
    "R3",
    "DpadUp",
    "DpadDown",
    "DpadLeft",
    "DpadRight",
    "Share",
    "Options",
    "PS",
];

fn redondear(v: f64) -> u8 {
    v.round_ties_even().min(255.0) as u8
}

fn escalar(c: Rgb, f: f64) -> Rgb {
    Rgb(
        redondear(c.0 as f64 * f),
        redondear(c.1 as f64 * f),
        redondear(c.2 as f64 * f),
    )
}

/// `LightLevel`: el color del perfil como color más brillo. El perfil no guarda brillo: va dentro
/// del color, y el brillo es la fuerza del canal más alto. Se recuerda el color sin escalar para
/// que bajar el brillo (incluso a 0) y volver a subirlo devuelva el mismo color.
#[derive(Debug, Clone)]
pub struct NivelLuz {
    pub base: Rgb,
    pub brillo: i32,
    ultimo: Option<Rgb>,
}

impl Default for NivelLuz {
    fn default() -> Self {
        NivelLuz {
            base: Rgb(255, 255, 255),
            brillo: 100,
            ultimo: None,
        }
    }
}

impl NivelLuz {
    /// `Sync`: se pone al día con el color guardado en el perfil.
    pub fn sincronizar(&mut self, guardado: Rgb) {
        if self.ultimo == Some(guardado) {
            return;
        }
        let fuerte = guardado.0.max(guardado.1).max(guardado.2);
        if fuerte > 0 {
            self.base = escalar(guardado, 255.0 / fuerte as f64);
        }
        self.brillo = (fuerte as f64 * 100.0 / 255.0).round_ties_even() as i32;
        self.ultimo = Some(guardado);
    }

    fn aplicar(&mut self) -> Rgb {
        let c = escalar(self.base, self.brillo as f64 / 100.0);
        self.ultimo = Some(c);
        c
    }

    /// `SetColor`: devuelve el color que se guarda en el perfil.
    pub fn poner_color(&mut self, c: Rgb) -> Rgb {
        self.base = c;
        self.aplicar()
    }

    /// `SetBrightness`.
    pub fn poner_brillo(&mut self, por_ciento: i32) -> Rgb {
        self.brillo = por_ciento;
        self.aplicar()
    }
}

pub fn hex(c: (u8, u8, u8)) -> String {
    format!("#{:02X}{:02X}{:02X}", c.0, c.1, c.2)
}

/// `ParseColor`.
pub fn color_de_hex(texto: &str) -> Option<Rgb> {
    let v = u32::from_str_radix(texto.trim_start_matches('#'), 16).ok()?;
    Some(Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// `ToAxis`: de -1 a 1 con dos decimales.
fn eje(v: u8) -> f64 {
    (((v as f64 - 128.0) / 127.0).clamp(-1.0, 1.0) * 100.0).round_ties_even() / 100.0
}

fn recorrido(v: u8) -> f64 {
    (v as f64 / 255.0 * 100.0).round_ties_even() / 100.0
}

/// `PressedControls`, con los id de los dibujos de la interfaz.
pub fn pulsados(e: &Estado) -> Vec<&'static str> {
    [
        ("cross", boton::EQUIS),
        ("circle", boton::CIRCULO),
        ("rect", boton::CUADRADO),
        ("triangle", boton::TRIANGULO),
        ("dpad-up", boton::ARRIBA),
        ("dpad-down", boton::ABAJO),
        ("dpad-left", boton::IZQUIERDA),
        ("dpad-right", boton::DERECHA),
        ("l1", boton::L1),
        ("r1", boton::R1),
        ("l2", boton::L2),
        ("r2", boton::R2),
        ("l3", boton::L3),
        ("r3", boton::R3),
        ("create", boton::CREAR),
        ("options", boton::OPCIONES),
        ("ps", boton::PS),
        ("touchpad", boton::PANEL),
        ("mute", boton::MUTE),
    ]
    .into_iter()
    .filter(|(_, b)| e.pulsado(*b))
    .map(|(n, _)| n)
    .collect()
}

fn indice(control: &str) -> Option<usize> {
    CONTROLES.iter().position(|c| *c == control)
}

/// `MappedButtons`.
pub fn mapeo(p: &Perfil) -> Vec<Value> {
    MAPEABLES
        .iter()
        .map(|&control| {
            let salida = indice(control)
                .and_then(|i| p.mapeos.controles.get(i))
                .and_then(|a| match &a.accion {
                    Accion::Predeterminada => None,
                    Accion::Boton(b) => Some(nombre_salida(*b).unwrap_or("Unbound").to_string()),
                    Accion::Tecla(k) => Some(format!("Key:{k}")),
                    Accion::Macro(_) => Some("Macro".to_string()),
                });
            json!({ "control": control, "salida": salida })
        })
        .collect()
}

fn nombre_efecto(e: EfectoGatillo) -> &'static str {
    match e {
        EfectoGatillo::FullClick => "clicCompleto",
        EfectoGatillo::Rigid => "rigido",
        EfectoGatillo::Pulse => "pulso",
        EfectoGatillo::None => "ninguno",
    }
}

fn efecto_de(nombre: &str) -> EfectoGatillo {
    match nombre {
        "clicCompleto" => EfectoGatillo::FullClick,
        "rigido" => EfectoGatillo::Rigid,
        "pulso" => EfectoGatillo::Pulse,
        _ => EfectoGatillo::None,
    }
}

/// `Telemetry`: lo que pinta la sección Datos, solo mientras está abierta.
pub fn datos(e: &Estado) -> Value {
    let m = e.movimiento.unwrap_or_default();
    let toques: Vec<Value> = e
        .toques
        .iter()
        .filter(|t| t.activo)
        .map(|t| json!({ "x": t.x as i32, "y": t.y as i32 }))
        .collect();
    json!({
        "bateria": { "nivel": e.estado_bateria & 0x0F, "estado": e.estado_bateria >> 4 },
        "audio": { "cascos": e.audio & 1 != 0, "micro": e.audio & 2 != 0 },
        "giro": { "x": m.giro_yaw_completo, "y": m.giro_pitch_completo, "z": m.giro_roll_completo },
        "acel": { "x": m.acel_x_g, "y": m.acel_y_g, "z": m.acel_z_g },
        "toques": toques,
    })
}

/// `ToState`: un mando tal como lo pinta la interfaz. Con `telemetria` lleva también `datos`.
pub fn estado_mando(m: &Mando, v: &Vivo, nivel: &mut NivelLuz, telemetria: bool) -> Value {
    let a = &v.ajustes.ajustes;
    nivel.sincronizar(a.color);
    let e = &v.crudo;
    json!({
        "id": m.id(),
        "nombre": match m.info.modelo {
            Modelo::DualSense => "DualSense",
            Modelo::DualSenseEdge => "DualSense Edge",
        },
        "tipo": "dualsense",
        "conexion": if m.info.conexion == Conexion::Bt { "bluetooth" } else { "usb" },
        "conectado": true,
        "bateria": v.bateria,
        "cargando": v.cargando,
        "emulando": v.emulando.map(|t| match t {
            Tipo::Ds4 => "DualShock 4",
            Tipo::Xbox360 => "Xbox 360",
        }),
        "luzActual": hex(v.luz),
        "pulsados": pulsados(e),
        "datos": if telemetria { datos(e) } else { Value::Null },
        "perfil": v.perfil,
        "mapeo": mapeo(&v.ajustes),
        "sticks": {
            "izq": { "x": eje(e.lx), "y": eje(e.ly) },
            "der": { "x": eje(e.rx), "y": eje(e.ry) },
        },
        "gatillos": { "l2": recorrido(e.l2), "r2": recorrido(e.r2) },
        "ajustes": {
            "luz": hex((nivel.base.0, nivel.base.1, nivel.base.2)),
            "brillo": nivel.brillo,
            "luzSegunBateria": a.luz_bateria,
            "emulacion": if a.solo_dinput {
                "nativo"
            } else if a.mando_virtual == MandoVirtual::DS4 {
                "ds4"
            } else {
                "xbox"
            },
            "arcoiris": a.arcoiris > 0.0,
            "vibracion": a.vibracion.min(100),
            "gatilloL2": nombre_efecto(a.l2_efecto),
            "gatilloR2": nombre_efecto(a.r2_efecto),
        },
    })
}

/// Aplica al perfil un comando de la interfaz que cambia los ajustes del mando. Devuelve si lo
/// ha cambiado (los comandos de otras cosas devuelven `false`).
pub fn aplicar(comando: &Value, p: &mut Perfil, nivel: &mut NivelLuz) -> bool {
    let tipo = comando["tipo"].as_str().unwrap_or("");
    let a = &mut p.ajustes;
    match tipo {
        "luz" | "brillo" => {
            nivel.sincronizar(a.color);
            let c = if tipo == "luz" {
                let Some(c) = comando["color"].as_str().and_then(color_de_hex) else {
                    return false;
                };
                nivel.poner_color(c)
            } else {
                let Some(v) = comando["valor"].as_i64().filter(|v| (0..=100).contains(v)) else {
                    return false;
                };
                nivel.poner_brillo(v as i32)
            };
            // Cualquier otra cosa que mande en la luz taparía el color elegido: elegir un color
            // apaga el arcoíris y también el color según la batería, que se contradicen con él.
            a.color = c;
            a.arcoiris = 0.0;
            if tipo == "luz" {
                a.luz_bateria = false;
            }
        }
        "luzSegunBateria" => {
            let activo = comando["activo"].as_bool().unwrap_or(false);
            a.luz_bateria = activo;
            if activo {
                a.arcoiris = 0.0;
            }
        }
        "arcoiris" => {
            let segundos = if comando["activo"].as_bool().unwrap_or(false) {
                SEGUNDOS_ARCOIRIS
            } else {
                0.0
            };
            a.arcoiris = segundos;
            if segundos > 0.0 {
                a.luz_bateria = false;
            }
        }
        "vibracion" => {
            let Some(f) = comando["fuerza"]
                .as_i64()
                .filter(|f| (0..=VIBRACION_MAXIMA).contains(f))
            else {
                return false;
            };
            a.vibracion = f as u8;
        }
        "gatillo" => {
            let efecto = efecto_de(comando["efecto"].as_str().unwrap_or(""));
            if comando["gatillo"].as_str() == Some("l2") {
                a.l2_efecto = efecto;
            } else {
                a.r2_efecto = efecto;
            }
        }
        "emulacion" => {
            // Sin mando virtual y sin escribir en el mando, un juego con soporte del DualSense
            // maneja sus gatillos y su vibración sin interferencias.
            let valor = comando["valor"].as_str().unwrap_or("");
            let nativo = valor == "nativo";
            let querido = if valor == "ds4" {
                MandoVirtual::DS4
            } else {
                MandoVirtual::X360
            };
            if nativo == a.solo_dinput && (nativo || a.mando_virtual == querido) {
                return false;
            }
            a.solo_dinput = nativo;
            a.enviar_al_mando = !nativo;
            if !nativo {
                a.mando_virtual = querido;
            }
        }
        "mapear" => {
            let control = comando["control"].as_str().unwrap_or("");
            if !MAPEABLES.contains(&control) {
                return false;
            }
            let Some(i) = indice(control) else {
                return false;
            };
            let nuevo = match comando["salida"].as_str() {
                None => AjusteControl::default(),
                Some(s) => match salida_por_nombre_enum(s) {
                    Some(b) => AjusteControl {
                        accion: Accion::Boton(b),
                        ..p.mapeos.controles[i].clone()
                    },
                    None => return false,
                },
            };
            let c = &mut p.mapeos.controles[i];
            if let Accion::Boton(_) = nuevo.accion {
                // `UpdateDS5CSetting` sin shift: sin extras y sin tipo de tecla.
                c.accion = nuevo.accion;
                c.extras = None;
                c.tipo_tecla = 0;
            } else {
                *c = nuevo;
            }
        }
        // `MapKey` y `MapMacro`: la tecla o la macro sin shift, sin extras.
        "mapearTecla" | "mapearMacro" => {
            let control = comando["control"].as_str().unwrap_or("");
            let Some(i) = indice(control).filter(|_| MAPEABLES.contains(&control)) else {
                return false;
            };
            let (accion, tipo_tecla) = if tipo == "mapearTecla" {
                let Some(k) = comando["tecla"].as_i64() else {
                    return false;
                };
                (Accion::Tecla(k as u16), 0)
            } else {
                let pasos: Vec<i32> = comando["pasos"]
                    .as_array()
                    .map(|v| {
                        v.iter()
                            .filter_map(Value::as_i64)
                            .map(|p| p as i32)
                            .collect()
                    })
                    .unwrap_or_default();
                if pasos.is_empty() {
                    return false;
                }
                // `DS5KeyType.Macro`.
                (Accion::Macro(pasos), 8)
            };
            let c = &mut p.mapeos.controles[i];
            c.accion = accion;
            c.extras = None;
            c.tipo_tecla = tipo_tecla;
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn el_brillo_sale_del_canal_mas_fuerte() {
        let mut n = NivelLuz::default();
        n.sincronizar(Rgb(0, 64, 128));
        assert_eq!(n.brillo, 50);
        assert_eq!(n.base, Rgb(0, 128, 255));
    }

    #[test]
    fn bajar_a_cero_y_subir_devuelve_el_color() {
        let mut n = NivelLuz::default();
        n.sincronizar(Rgb(255, 0, 128));
        assert_eq!(n.poner_brillo(0), Rgb(0, 0, 0));
        assert_eq!(n.poner_brillo(100), Rgb(255, 0, 128));
        // Con el color guardado igual al último que salió, no se recalcula nada.
        n.sincronizar(Rgb(255, 0, 128));
        assert_eq!(n.base, Rgb(255, 0, 128));
    }

    #[test]
    fn la_luz_apaga_el_arcoiris_y_guarda_el_color_escalado() {
        let mut p = Perfil::default();
        p.ajustes.color = Rgb(0, 0, 128);
        p.ajustes.arcoiris = 4.0;
        let mut n = NivelLuz::default();
        assert!(aplicar(
            &json!({"tipo": "luz", "color": "#FF0000"}),
            &mut p,
            &mut n
        ));
        assert_eq!(p.ajustes.color, Rgb(128, 0, 0));
        assert_eq!(p.ajustes.arcoiris, 0.0);
    }

    #[test]
    fn elegir_un_color_quita_el_color_segun_bateria() {
        let mut p = Perfil::default();
        p.ajustes.luz_bateria = true;
        let mut n = NivelLuz::default();
        assert!(aplicar(
            &json!({"tipo": "luz", "color": "#00FF00"}),
            &mut p,
            &mut n
        ));
        assert!(!p.ajustes.luz_bateria);
        // Mover el brillo no lo toca.
        p.ajustes.luz_bateria = true;
        assert!(aplicar(
            &json!({"tipo": "brillo", "valor": 50}),
            &mut p,
            &mut n
        ));
        assert!(p.ajustes.luz_bateria);
    }

    #[test]
    fn el_arcoiris_y_la_bateria_se_excluyen() {
        let mut p = Perfil::default();
        let mut n = NivelLuz::default();
        aplicar(
            &json!({"tipo": "luzSegunBateria", "activo": true}),
            &mut p,
            &mut n,
        );
        aplicar(&json!({"tipo": "arcoiris", "activo": true}), &mut p, &mut n);
        assert!(!p.ajustes.luz_bateria);
        assert_eq!(p.ajustes.arcoiris, SEGUNDOS_ARCOIRIS);
    }

    #[test]
    fn la_emulacion_nativa_no_escribe_en_el_mando() {
        let mut p = Perfil::default();
        let mut n = NivelLuz::default();
        assert!(!aplicar(
            &json!({"tipo": "emulacion", "valor": "xbox"}),
            &mut p,
            &mut n
        ));
        assert!(aplicar(
            &json!({"tipo": "emulacion", "valor": "nativo"}),
            &mut p,
            &mut n
        ));
        assert!(p.ajustes.solo_dinput && !p.ajustes.enviar_al_mando);
        assert!(aplicar(
            &json!({"tipo": "emulacion", "valor": "ds4"}),
            &mut p,
            &mut n
        ));
        assert_eq!(p.ajustes.mando_virtual, MandoVirtual::DS4);
        assert!(p.ajustes.enviar_al_mando);
    }

    #[test]
    fn mapear_y_quitar_un_boton() {
        let mut p = Perfil::default();
        let mut n = NivelLuz::default();
        let cruz = indice("Cross").unwrap();
        assert!(aplicar(
            &json!({"tipo": "mapear", "control": "Cross", "salida": "B"}),
            &mut p,
            &mut n
        ));
        let m = mapeo(&p);
        assert_eq!(m[0], json!({"control": "Cross", "salida": "B"}));
        assert!(aplicar(
            &json!({"tipo": "mapear", "control": "Cross", "salida": null}),
            &mut p,
            &mut n
        ));
        assert_eq!(p.mapeos.controles[cruz], AjusteControl::default());
        // TouchLeft no se puede reasignar desde la interfaz.
        assert!(!aplicar(
            &json!({"tipo": "mapear", "control": "TouchLeft", "salida": "Circle"}),
            &mut p,
            &mut n
        ));
    }

    #[test]
    fn los_ejes_y_los_pulsados() {
        let e = Estado {
            lx: 0,
            ly: 255,
            rx: 128,
            ry: 191,
            l2: 128,
            botones: boton::EQUIS | boton::MUTE,
            ..Default::default()
        };
        assert_eq!(eje(e.lx), -1.0);
        assert_eq!(eje(e.ly), 1.0);
        assert_eq!(eje(e.rx), 0.0);
        assert_eq!(eje(e.ry), 0.5);
        assert_eq!(recorrido(e.l2), 0.5);
        assert_eq!(pulsados(&e), ["cross", "mute"]);
    }
}
