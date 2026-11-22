//! Compara el panel táctil, el giroscopio como ratón, stick y gestos, el teclado y el ratón
//! virtuales, el flick stick y los extras con C#: cada grabación de la fase 0 pasa por el panel, el
//! giroscopio, las curvas, la reasignación y los clics, y se compara con lo que dio C# con los
//! mismos perfiles (`tools/fase5/panel`, lo escribe `MappingOracleTests.Panel.cs`).
//! Sin las grabaciones o sin las referencias no se comprueba nada.

use std::fs;
use std::path::{Path, PathBuf};

use ds5_informes::{boton, Estado, Movimiento, Toque};
use ds5_mapeo::{
    AccionesEspeciales, AjustesMotor, EventoRaton, GiroRaton, Motor, RatonPanel, SalidaExtras,
    Sintetico,
};
use ds5_perfiles::otros::Acciones;
use ds5_perfiles::perfil::Perfil;

fn herramientas() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tools")
}

/// El estado de una línea `st`, con el botón del panel de salida encendido si el panel está
/// pulsado, como lo deja el decodificador antes de que lo apague el ratón del panel.
fn estado(valores: &str) -> Estado {
    let p: Vec<&str> = valores.split(',').collect();
    let n = |i: usize| p[i].parse::<u8>().unwrap();
    let e = |i: usize| p[i].parse::<i32>().unwrap();
    let d = |i: usize| p[i].parse::<f64>().unwrap();
    let toque = |i: usize| Toque {
        activo: p[i] == "1",
        id: n(i + 1),
        x: p[i + 2].parse().unwrap(),
        y: p[i + 3].parse().unwrap(),
        crudo: n(i + 4),
    };
    let mut botones = u64::from_str_radix(p[0], 16).unwrap() & !boton::PANEL_SALIDA;
    if botones & boton::PANEL != 0 {
        botones |= boton::PANEL_SALIDA;
    }
    Estado {
        botones,
        lx: n(1),
        ly: n(2),
        rx: n(3),
        ry: n(4),
        l2: n(5),
        r2: n(6),
        l2_crudo: n(7),
        r2_crudo: n(8),
        bateria: n(9),
        toques: [toque(14), toque(19)],
        transcurrido: p[25].parse().unwrap(),
        exterior_ls: n(27),
        exterior_rs: n(28),
        movimiento: (p.len() > 44 && !p[29].is_empty()).then(|| Movimiento {
            giro_yaw_completo: e(29),
            giro_pitch_completo: e(30),
            giro_roll_completo: e(31),
            acel_x_completo: e(32),
            acel_y_completo: e(33),
            acel_z_completo: e(34),
            giro_yaw: e(35),
            giro_pitch: e(36),
            giro_roll: e(37),
            acel_x: e(38),
            acel_y: e(39),
            acel_z: e(40),
            salida_acel_x: e(41),
            salida_acel_y: e(42),
            salida_acel_z: e(43),
            giro_controles: p[44] == "1",
            vel_yaw: d(45),
            vel_pitch: d(46),
            vel_roll: d(47),
            acel_x_g: d(48),
            acel_y_g: d(49),
            acel_z_g: d(50),
            transcurrido: d(51),
        }),
        ..Default::default()
    }
}

fn campos(e: &Estado) -> String {
    format!(
        "{:X},{},{},{},{},{},{},{},{}",
        e.botones, e.lx, e.ly, e.rx, e.ry, e.l2, e.r2, e.exterior_ls, e.exterior_rs
    )
}

/// Un evento de C# (`RecordingKBM`).
fn evento(t: &str) -> Option<EventoRaton> {
    let (tipo, resto) = t.split_at(1);
    let par = || {
        let (a, b) = resto.split_once(':').unwrap();
        (a.to_string(), b.to_string())
    };
    Some(match tipo {
        "m" => {
            let (a, b) = par();
            EventoRaton::Mover(a.parse().unwrap(), b.parse().unwrap())
        }
        "a" => {
            let (a, b) = par();
            EventoRaton::Absoluto(a.parse().unwrap(), b.parse().unwrap())
        }
        "w" => {
            let (a, b) = par();
            EventoRaton::Rueda(a.parse().unwrap(), b.parse().unwrap())
        }
        "b" => EventoRaton::Boton(resto.parse().unwrap()),
        "k" => EventoRaton::Tecla(resto.parse().unwrap()),
        "K" => EventoRaton::TeclaAlt(resto.parse().unwrap()),
        "u" => EventoRaton::SueltaTecla(resto.parse().unwrap()),
        "U" => EventoRaton::SueltaTeclaAlt(resto.parse().unwrap()),
        "x" => {
            let (a, b) = par();
            EventoRaton::BotonX(a.parse().unwrap(), b.parse().unwrap())
        }
        _ => return None,
    })
}

/// La luz forzada y la vibración que han pedido los extras, como las escribe el oráculo.
fn extras(s: &SalidaExtras) -> String {
    let l = &s.luz;
    let vibracion = match s.vibracion {
        Some((f, d)) => format!("{f},{d}"),
        None => "-".into(),
    };
    format!(
        "{}:{},{},{}:{}:{vibracion}",
        b(l.activa),
        l.color.0,
        l.color.1,
        l.color.2,
        l.destello
    )
}

fn b(v: bool) -> &'static str {
    if v {
        "1"
    } else {
        "0"
    }
}

/// Los campos del panel y de los gestos del giroscopio, en el orden de `RunTheTouchpad`.
fn campos_panel(r: &RatonPanel, g: &GiroRaton, s: &Sintetico, e: &Estado) -> String {
    let (empieza, acaba) = r.toque_pendiente();
    let mut v: Vec<String> = Vec::new();
    v.extend(r.zonas.iter().map(|&z| b(z).to_string()));
    v.extend(r.deslizar.iter().map(|&z| b(z).to_string()));
    v.extend(r.deslizar_b.iter().map(|z| z.to_string()));
    for x in [
        r.desliza_izquierda,
        r.desliza_derecha,
        r.arrastrando,
        r.arrastrando2,
        empieza,
        acaba,
    ] {
        v.push(b(x).to_string());
    }
    v.push(s.actual.izquierdo.to_string());
    v.push(s.actual.central.to_string());
    v.push(s.actual.derecho.to_string());
    v.push(b(e.pulsado(boton::PANEL_SALIDA)).to_string());
    v.extend(g.gestos().0.iter().map(|&x| b(x).to_string()));
    v.join(",")
}

#[test]
fn el_panel_da_lo_mismo_que_en_c() {
    let raiz = herramientas();
    let Ok(carpetas) = fs::read_dir(raiz.join("fase5/panel")) else {
        return;
    };
    let mut carpetas: Vec<PathBuf> = carpetas.map(|e| e.unwrap().path()).collect();
    carpetas.sort();
    // `DS5_PERFILES=17,49` compara solo los perfiles que empiezan así, para ir más rápido.
    if let Ok(solo) = std::env::var("DS5_PERFILES") {
        let prefijos: Vec<&str> = solo.split(',').collect();
        carpetas.retain(|c| {
            let n = c.file_name().unwrap().to_string_lossy();
            prefijos.iter().any(|p| n.starts_with(p))
        });
    }
    let mut errores = Vec::new();
    let mut comparados = 0usize;
    let mut con_eventos = 0usize;
    for carpeta in &carpetas {
        let nombre = carpeta.file_name().unwrap().to_string_lossy().to_string();
        let xml =
            fs::read_to_string(raiz.join("fase5/perfiles").join(format!("{nombre}.xml"))).unwrap();
        let p = Perfil::leer(&xml).unwrap();
        let a_motor = AjustesMotor::desde_perfil(&p);
        let mut fallos = [0usize; 5];
        let mut primero = String::new();
        for referencia in fs::read_dir(carpeta).unwrap() {
            let referencia = referencia.unwrap().path();
            let grabacion = raiz
                .join("fase0/grabaciones")
                .join(referencia.file_stem().unwrap())
                .join("grabacion.txt");
            let Ok(texto) = fs::read_to_string(&grabacion) else {
                continue;
            };
            let esperado = fs::read_to_string(&referencia).unwrap();
            let mut esperado = esperado.lines();
            let mut motor = Motor::nuevo(&a_motor);
            // Las acciones especiales de los perfiles de prueba, las mismas que carga C#.
            if !p.acciones.is_empty() {
                let xml = fs::read_to_string(raiz.join("fase5/acciones.xml")).unwrap();
                motor.botones.acciones = AccionesEspeciales::nuevas(&Acciones::leer(&xml).unwrap());
            }
            let estados = texto.lines().filter_map(|l| {
                let p: Vec<&str> = l.splitn(4, '\t').collect();
                (p.len() == 4 && p[1] == "st").then(|| (p[0].parse::<i64>().unwrap(), p[3]))
            });
            for (i, (us, resto)) in estados.enumerate() {
                let entrada = motor.preparar(&estado(resto), &a_motor, us);
                let rust_panel =
                    campos_panel(&motor.raton, &motor.giro, &motor.sintetico, &entrada);
                let paso = motor.acabar(entrada, &a_motor, us);
                let (r, eventos) = (paso.salida, paso.eventos);
                let rust_extras = extras(&motor.botones.salida_extras);

                let linea = esperado.next().expect("faltan referencias");
                let partes: Vec<&str> = linea.split('|').collect();
                let csharp_eventos: Vec<EventoRaton> = partes[2]
                    .split(';')
                    .filter(|t| !t.is_empty())
                    .filter_map(evento)
                    .collect();
                comparados += 1;
                if !eventos.is_empty() {
                    con_eventos += 1;
                }
                let distinto = [
                    campos(&r) != partes[0],
                    rust_panel != partes[1],
                    eventos != csharp_eventos,
                    rust_extras != partes[3],
                    partes.get(4).is_some_and(|v| r.volante.to_string() != *v),
                ];
                if distinto.iter().any(|&d| d) {
                    if fallos.iter().sum::<usize>() == 0 {
                        primero = format!(
                            "{} estado {i}: entrada {resto}\n  C#   {linea}\n  Rust {}|{rust_panel}|{eventos:?}|{rust_extras}|{}",
                            referencia.file_stem().unwrap().to_string_lossy(),
                            campos(&r),
                            r.volante
                        );
                    }
                    for (f, d) in fallos.iter_mut().zip(distinto) {
                        *f += d as usize;
                    }
                }
            }
        }
        if fallos.iter().sum::<usize>() > 0 {
            errores.push(format!(
                "{nombre}: mapeo {}, panel {}, ratón y teclado {}, extras {}, volante {} distintos. {primero}",
                fallos[0], fallos[1], fallos[2], fallos[3], fallos[4]
            ));
        }
    }
    eprintln!("{comparados} estados comparados, {con_eventos} con el ratón");
    assert!(errores.is_empty(), "\n{}", errores.join("\n"));
}
