//! Prueba manual de la fase 3 con un DualSense de verdad: lee sus informes durante un rato, le
//! cambia la luz, lo hace vibrar y pone los gatillos adaptativos. Al final saca un resumen de
//! todo lo que ha leído.
//!
//! `cargo run -p ds5-hid --example probar -- [segundos]`

use std::time::{Duration, Instant};

use ds5_hid::{enumerar, Dispositivo, ErrorHid};
use ds5_informes::salida::{Ajustes, Efecto, EfectoGatillo, Salida};
use ds5_informes::{crc, Calibracion, Conexion, Decodificador};

const NOMBRES: [&str; 33] = [
    "Cuadrado",
    "Triángulo",
    "Círculo",
    "Equis",
    "Arriba",
    "Abajo",
    "Izquierda",
    "Derecha",
    "L1",
    "L2",
    "L3",
    "R1",
    "R2",
    "R3",
    "Crear",
    "Opciones",
    "PS",
    "Mute",
    "Dedo1",
    "Dedo2",
    "Panel",
    "PanelDer",
    "PanelIzq",
    "UnDedo",
    "DosDedos",
    "PanelSalida",
    "Captura",
    "LatIzq",
    "LatDer",
    "FnIzq",
    "FnDer",
    "PaletaIzq",
    "PaletaDer",
];

/// El color de DS5Control en las grabaciones, para dejar el mando como estaba.
const COLOR_FINAL: (u8, u8, u8) = (0x00, 0xFF, 0xE5);

fn leer_calibracion(d: &Dispositivo) -> Option<Calibracion> {
    for _ in 0..5 {
        let mut c = [0u8; 41];
        c[0] = 0x05;
        if d.leer_caracteristica(&mut c).is_err() {
            continue;
        }
        if d.info.conexion == Conexion::Usb || crc::valido(crc::CABECERA_CARACTERISTICA, &c) {
            return Calibracion::desde_informe(&c);
        }
    }
    None
}

fn main() -> Result<(), ErrorHid> {
    let segundos: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    // Si sale por los dos sitios, el de USB: el de Bluetooth puede ser el que se acaba de apagar.
    let mut mandos = enumerar();
    mandos.sort_by_key(|m| m.conexion != Conexion::Usb);
    let Some(info) = mandos.into_iter().next() else {
        println!("No hay ningún DualSense conectado.");
        return Ok(());
    };
    println!("{:?} por {:?}", info.modelo, info.conexion);
    let d = Dispositivo::abrir(&info)?;
    let cal = leer_calibracion(&d);
    println!("Calibración: {}", if cal.is_some() { "bien" } else { "no" });

    let conexion = info.conexion;
    let mandar = |s: &Salida| d.escribir(&s.codificar(conexion), Duration::from_millis(100));
    if conexion == Conexion::Bt {
        mandar(&Salida::inicial_bt())?;
    }

    let inicio = Instant::now();
    let us = || inicio.elapsed().as_micros() as i64;
    let mut dec = Decodificador::new(conexion, cal, us());
    let mut bufer = vec![0u8; info.largo_entrada.max(78)];

    let mut informes = 0u64;
    let mut malos = 0u64;
    let mut botones = 0u64;
    let mut sticks_min = [255u8; 4];
    let mut sticks_max = [0u8; 4];
    let mut gatillos_max = [0u8; 2];
    let mut dedos_max = 0;
    let mut giro_max = 0.0f64;
    let mut ultimo: Option<ds5_informes::Estado> = None;
    let mut ajustes_previos = None;

    while inicio.elapsed() < Duration::from_secs(segundos) {
        // Lo que se manda según el momento de la prueba.
        let t = inicio.elapsed().as_secs_f64();
        let mut a = Ajustes {
            color: COLOR_FINAL,
            ..Default::default()
        };
        if t < 4.0 {
            a.color = [(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 255)][t as usize];
        } else if t < 5.0 {
            a.motor_izquierdo = 0xC0;
        } else if t < 6.0 {
            a.motor_derecho = 0xC0;
        } else {
            a.gatillo_izquierdo = EfectoGatillo::de(Efecto::Rigido);
            a.gatillo_derecho = EfectoGatillo::de(Efecto::Pulso);
        }
        if let Some(e) = &ultimo {
            a.luces_jugador = ds5_informes::salida::luces_por_bateria(e.bateria);
        }
        if ajustes_previos != Some(a) {
            mandar(&Salida::normal(&a))?;
            ajustes_previos = Some(a);
        }

        let n = match d.leer(&mut bufer, Duration::from_millis(500)) {
            Ok(n) => n,
            Err(ErrorHid::TiempoAgotado) => continue,
            Err(e) => return Err(e),
        };
        let informe = &bufer[..n];
        if !dec.valido(informe) {
            if conexion == Conexion::Bt && informe.first() == Some(&0x31) {
                malos += 1;
                dec.perdido();
            }
            continue;
        }
        let e = dec.decodificar(informe, us());
        informes += 1;
        botones |= e.botones;
        for (i, v) in [e.lx, e.ly, e.rx, e.ry].into_iter().enumerate() {
            sticks_min[i] = sticks_min[i].min(v);
            sticks_max[i] = sticks_max[i].max(v);
        }
        gatillos_max[0] = gatillos_max[0].max(e.l2);
        gatillos_max[1] = gatillos_max[1].max(e.r2);
        dedos_max = dedos_max.max(e.toques.iter().filter(|t| t.activo).count());
        if let Some(m) = &e.movimiento {
            giro_max = giro_max
                .max(m.vel_yaw.abs())
                .max(m.vel_pitch.abs())
                .max(m.vel_roll.abs());
        }
        ultimo = Some(e);
    }

    // Lo deja con la luz de siempre, sin vibración ni gatillos.
    mandar(&Salida::normal(&Ajustes {
        color: COLOR_FINAL,
        ..Default::default()
    }))?;

    let s = inicio.elapsed().as_secs_f64();
    println!(
        "{informes} informes en {s:.1} s ({:.0} por segundo), {malos} con el CRC mal",
        informes as f64 / s
    );
    let pulsados: Vec<&str> = (0..33)
        .filter(|i| botones & (1u64 << i) != 0)
        .map(|i| NOMBRES[i])
        .collect();
    println!("Botones: {}", pulsados.join(", "));
    println!(
        "Sticks (mín-máx): LX {}-{}, LY {}-{}, RX {}-{}, RY {}-{}",
        sticks_min[0],
        sticks_max[0],
        sticks_min[1],
        sticks_max[1],
        sticks_min[2],
        sticks_max[2],
        sticks_min[3],
        sticks_max[3]
    );
    println!(
        "Gatillos al máximo: L2 {}, R2 {}",
        gatillos_max[0], gatillos_max[1]
    );
    println!("Dedos en el panel a la vez: {dedos_max}");
    println!("Giro más rápido: {giro_max:.0} grados por segundo");
    if let Some(e) = &ultimo {
        println!(
            "Batería: {} % (byte 0x{:02X}), {}",
            e.bateria,
            e.estado_bateria,
            if e.cargando { "cargando" } else { "sin cargar" }
        );
    }
    Ok(())
}
