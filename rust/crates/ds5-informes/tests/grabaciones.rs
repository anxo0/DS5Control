//! Pasa las grabaciones de la fase 0 (`tools/fase0/grabaciones`) por el decodificador y compara
//! cada estado con el que decodificó C# (las líneas `st`). Las grabaciones no están en el
//! repositorio porque llevan la MAC del mando: sin ellas la prueba no hace nada.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ds5_informes::{desde_hex, Calibracion, Conexion, Decodificador, Estado};

/// Los campos de una línea de estado, en el orden de `ReportRecorder.StateFields`.
const CAMPOS: [&str; 52] = [
    "botones",
    "LX",
    "LY",
    "RX",
    "RY",
    "L2",
    "R2",
    "L2Raw",
    "R2Raw",
    "Battery",
    "FrameCounter",
    "TouchPacketCounter",
    "Touch1Identifier",
    "Touch2Identifier",
    "T0Activo",
    "T0Id",
    "T0X",
    "T0Y",
    "T0Raw",
    "T1Activo",
    "T1Id",
    "T1X",
    "T1Y",
    "T1Raw",
    "ds5Timestamp",
    "elapsedTime",
    "totalMicroSec",
    "OutputLSOuter",
    "OutputRSOuter",
    "gyroYawFull",
    "gyroPitchFull",
    "gyroRollFull",
    "accelXFull",
    "accelYFull",
    "accelZFull",
    "gyroYaw",
    "gyroPitch",
    "gyroRoll",
    "accelX",
    "accelY",
    "accelZ",
    "outputAccelX",
    "outputAccelY",
    "outputAccelZ",
    "outputGyroControls",
    "angVelYaw",
    "angVelPitch",
    "angVelRoll",
    "accelXG",
    "accelYG",
    "accelZG",
    "motionElapsed",
];

/// Lo que pone el mapeo en el mismo objeto de movimiento antes de que se grabe: es de la fase 5.
const DEL_MAPEO: [&str; 4] = [
    "outputAccelX",
    "outputAccelY",
    "outputAccelZ",
    "outputGyroControls",
];

/// `OutputTouchButton`: el decodificador lo copia del botón del panel, pero el ratón del panel
/// (`Mouse.cs`) lo apaga antes de que se grabe el estado. Es de la fase 5.
const BIT_DEL_MAPEO: u64 = ds5_informes::boton::PANEL_SALIDA;

const DOBLES: [&str; 8] = [
    "elapsedTime",
    "angVelYaw",
    "angVelPitch",
    "angVelRoll",
    "accelXG",
    "accelYG",
    "accelZG",
    "motionElapsed",
];

/// Un valor de la línea: los decimales se comparan por sus bits, el resto como texto.
#[derive(Debug, PartialEq)]
enum Valor {
    Texto(String),
    Doble(u64),
}

fn d(v: f64) -> Valor {
    Valor::Doble(v.to_bits())
}

fn t(v: impl ToString) -> Valor {
    Valor::Texto(v.to_string())
}

fn b(v: bool) -> Valor {
    t(if v { "1" } else { "0" })
}

/// El estado en el orden de `ReportRecorder.FormatState`.
fn valores(e: &Estado) -> Vec<Valor> {
    let mut v = vec![
        t(format!("{:X}", e.botones & !BIT_DEL_MAPEO)),
        t(e.lx),
        t(e.ly),
        t(e.rx),
        t(e.ry),
        t(e.l2),
        t(e.r2),
        t(e.l2_crudo),
        t(e.r2_crudo),
        t(e.bateria),
        t(e.contador_tramas),
        t(e.contador_panel),
        t(e.toque1_id),
        t(e.toque2_id),
    ];
    for k in &e.toques {
        v.extend([b(k.activo), t(k.id), t(k.x), t(k.y), t(k.crudo)]);
    }
    v.extend([
        t(e.marca_ds5),
        d(e.transcurrido),
        t(e.total_us),
        t(e.exterior_ls),
        t(e.exterior_rs),
    ]);
    match &e.movimiento {
        None => v.extend((0..23).map(|_| t(""))),
        Some(m) => v.extend([
            t(m.giro_yaw_completo),
            t(m.giro_pitch_completo),
            t(m.giro_roll_completo),
            t(m.acel_x_completo),
            t(m.acel_y_completo),
            t(m.acel_z_completo),
            t(m.giro_yaw),
            t(m.giro_pitch),
            t(m.giro_roll),
            t(m.acel_x),
            t(m.acel_y),
            t(m.acel_z),
            t(""),
            t(""),
            t(""),
            t(""),
            d(m.vel_yaw),
            d(m.vel_pitch),
            d(m.vel_roll),
            d(m.acel_x_g),
            d(m.acel_y_g),
            d(m.acel_z_g),
            d(m.transcurrido),
        ]),
    }
    v
}

fn leer_valor(campo: &str, texto: &str) -> Valor {
    if campo == "botones" {
        let b = u64::from_str_radix(texto, 16).unwrap();
        return t(format!("{:X}", b & !BIT_DEL_MAPEO));
    }
    if DOBLES.contains(&campo) && !texto.is_empty() {
        d(texto.parse::<f64>().unwrap())
    } else {
        t(texto)
    }
}

fn carpeta() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tools/fase0/grabaciones")
}

#[derive(Default)]
struct Resultado {
    estados: usize,
    repetidas: usize,
    fallos: BTreeMap<&'static str, usize>,
    primeros: Vec<String>,
}

/// Repasa una grabación. `adelanto_us` es cuánto antes del primer informe empezó a leer C#: ahí
/// arranca la calibración continua y no queda grabado.
fn repasar(texto: &str, calibracion: Option<Calibracion>, adelanto_us: i64) -> Resultado {
    let mut res = Resultado::default();
    let mut dec: Option<Decodificador> = None;
    let mut conexion = Conexion::Bt;
    let mut ultimo: Option<Estado> = None;
    let mut dec_actual = false;
    for linea in texto.lines() {
        let p: Vec<&str> = linea.split('\t').collect();
        if p.len() < 4 || p[0] == "#" {
            continue;
        }
        let us: i64 = p[0].parse().unwrap();
        match p[1] {
            "dev" => {
                conexion = if p.get(4) == Some(&"USB") {
                    Conexion::Usb
                } else {
                    Conexion::Bt
                };
            }
            "in" => {
                let informe = desde_hex(p[3]).unwrap();
                let d = dec.get_or_insert_with(|| {
                    Decodificador::new(conexion, calibracion, us - adelanto_us)
                });
                assert!(d.valido(&informe), "informe no válido: {linea}");
                ultimo = Some(d.decodificar(&informe, us));
                dec_actual = true;
            }
            "st" => {
                let e = ultimo.take().expect("un estado sin informe");
                res.estados += 1;
                let rust = valores(&e);
                let csharp: Vec<&str> = p[3].split(',').collect();
                if e.marca_repetida {
                    // C# midió el tiempo con su reloj: se acepta si se parece al de la grabación
                    // y se sigue con su total para no arrastrar la diferencia.
                    res.repetidas += 1;
                    let suyo: f64 = csharp[25].parse().unwrap();
                    assert!(
                        (suyo - e.transcurrido).abs() < 0.002,
                        "marca repetida con un tiempo muy distinto: C# {suyo}, Rust {}",
                        e.transcurrido
                    );
                    if dec_actual {
                        dec.as_mut()
                            .unwrap()
                            .fijar_total_us(csharp[26].parse().unwrap());
                    }
                }
                dec_actual = false;
                for (i, texto) in csharp.into_iter().enumerate() {
                    let campo = CAMPOS[i];
                    if DEL_MAPEO.contains(&campo) {
                        continue;
                    }
                    let tiempo = matches!(campo, "elapsedTime" | "totalMicroSec" | "motionElapsed");
                    if e.marca_repetida && tiempo {
                        continue;
                    }
                    if leer_valor(campo, texto) != rust[i] {
                        *res.fallos.entry(campo).or_default() += 1;
                        if res.primeros.len() < 8 {
                            res.primeros.push(format!(
                                "estado {} {campo}: C# {texto}, Rust {:?}",
                                res.estados, rust[i]
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    res
}

/// Las líneas de los primeros `us` microsegundos, contados desde el primer informe.
fn principio(texto: &str, us: i64) -> &str {
    let mut primero = None;
    let mut fin = 0;
    for linea in texto.split_inclusive('\n') {
        if let Some(t) = linea.split('\t').next().and_then(|t| t.parse::<i64>().ok()) {
            let p = *primero.get_or_insert(t);
            if t - p > us {
                break;
            }
        }
        fin += linea.len();
    }
    &texto[..fin]
}

fn calibracion(g: &Path) -> Option<Calibracion> {
    let texto = fs::read_to_string(g.join("calibracion.txt")).ok()?;
    let linea = texto.lines().find(|l| !l.starts_with('#'))?;
    Calibracion::desde_informe(&desde_hex(linea.trim())?)
}

#[test]
fn las_grabaciones_dan_el_mismo_estado_que_en_c() {
    let Ok(lista) = fs::read_dir(carpeta()) else {
        eprintln!("No hay grabaciones: no se comprueba nada.");
        return;
    };
    let mut grabaciones: Vec<PathBuf> = lista.map(|e| e.unwrap().path()).collect();
    grabaciones.sort();
    let mut errores = Vec::new();
    for g in &grabaciones {
        let Ok(texto) = fs::read_to_string(g.join("grabacion.txt")) else {
            continue;
        };
        let cal = calibracion(g);
        // C# empieza a leer un poco antes del primer informe y no queda grabado cuánto: se busca
        // el adelanto con el que la calibración continua termina en el mismo informe.
        // Basta con los primeros diez segundos: la calibración termina a los cinco.
        let inicio = principio(&texto, 10_000_000);
        let adelanto = (0..=80)
            .map(|i| i * 250)
            .find(|&a| repasar(inicio, cal, a).fallos.is_empty())
            .unwrap_or(0);
        eprintln!("  adelanto {} ms", adelanto as f64 / 1000.0);
        let res = repasar(&texto, cal, adelanto);
        let nombre = g.file_name().unwrap().to_string_lossy().to_string();
        eprintln!(
            "{nombre}: {} estados ({} con la marca repetida), fallos {:?}",
            res.estados, res.repetidas, res.fallos
        );
        for p in &res.primeros {
            eprintln!("  {p}");
        }
        if !res.fallos.is_empty() {
            errores.push(nombre);
        }
    }
    assert!(errores.is_empty(), "no coinciden: {errores:?}");
}

/// Cada informe de salida grabado se lee y se vuelve a codificar: tiene que dar los mismos bytes,
/// CRC incluido. Así se comprueba que no queda ningún byte sin entender.
#[test]
fn los_informes_de_salida_se_codifican_igual() {
    let Ok(lista) = fs::read_dir(carpeta()) else {
        return;
    };
    let mut total = 0;
    for g in lista.map(|e| e.unwrap().path()) {
        let Ok(texto) = fs::read_to_string(g.join("grabacion.txt")) else {
            continue;
        };
        let mut conexion = Conexion::Bt;
        for linea in texto.lines() {
            let p: Vec<&str> = linea.split('\t').collect();
            if p.len() < 4 {
                continue;
            }
            match p[1] {
                "dev" if p.get(4) == Some(&"USB") => conexion = Conexion::Usb,
                "out" => {
                    let b = desde_hex(p[3]).unwrap();
                    let s = ds5_informes::Salida::leer(conexion, &b)
                        .unwrap_or_else(|| panic!("informe de salida raro: {linea}"));
                    assert_eq!(s.codificar(conexion), b, "{linea}");
                    total += 1;
                }
                _ => {}
            }
        }
    }
    eprintln!("{total} informes de salida iguales");
}
