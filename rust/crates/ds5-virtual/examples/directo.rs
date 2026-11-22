//! Prueba de la fase 4: el DualSense pasa tal cual a un mando virtual (Xbox 360 o DS4), con la
//! vibración de vuelta y el mando de verdad escondido con HidHide para que no haya entrada doble.
//!
//! `cargo run --release -p ds5-virtual --example directo -- [x360|ds4] [segundos]`
//!
//! Al acabar lo deja todo como estaba: el mando visible, la lista blanca de HidHide sin este
//! programa y la luz de siempre.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use ds5_hid::{enumerar, Dispositivo, ErrorHid};
use ds5_informes::salida::{Ajustes, Salida};
use ds5_informes::{crc, Calibracion, Conexion, Decodificador};
use ds5_virtual::hidhide::{self, HidHide, Ocultador};
use ds5_virtual::informes::informe_ds4;
use ds5_virtual::{Bus, ErrorVirtual, InformeXbox, MandoVirtual, Tipo};

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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let tipo = if args.get(1).map(String::as_str) == Some("ds4") {
        Tipo::Ds4
    } else {
        Tipo::Xbox360
    };
    let segundos: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(60);

    let bus = match Bus::abrir() {
        Ok(b) => b,
        Err(e) => {
            println!("No se puede abrir ViGEmBus: {e}");
            return;
        }
    };

    // Primero este programa en la lista blanca, para seguir viendo el mando cuando se esconda.
    let exe = std::env::current_exe().unwrap();
    let hh = HidHide::abrir();
    let anadido = hh.as_ref().is_some_and(|h| h.permitir_programa(&exe));
    println!(
        "HidHide: {}",
        match &hh {
            None => "no está instalado (habrá entrada doble)",
            Some(_) if anadido => "programa apuntado en la lista blanca",
            Some(_) => "el programa ya estaba en la lista blanca",
        }
    );

    let mut mandos = enumerar();
    mandos.sort_by_key(|m| m.conexion != Conexion::Usb);
    let Some(info) = mandos.into_iter().next() else {
        println!("No hay ningún DualSense conectado.");
        return;
    };
    println!(
        "{:?} por {:?}, mando virtual {:?}",
        info.modelo, info.conexion, tipo
    );

    let mut ocultador = Ocultador::new(std::env::temp_dir().join("ds5-rust-hidhide.txt"));
    if hh.is_some() {
        if let Some(i) = hidhide::instancia_de(&info.ruta) {
            println!("Escondo {i}");
            ocultador.aplicar(&[i]);
        }
    }

    let resultado = jugar(&bus, tipo, &info, segundos);
    if let Err(e) = &resultado {
        println!("Error: {e}");
    }

    ocultador.aplicar(&[]);
    // La entrada de la lista blanca es de este mismo programa: se quita aunque viniera de una
    // prueba anterior que se cortó.
    if let Some(h) = &hh {
        h.retirar_programa(&exe);
    }
    println!("Mando visible otra vez y HidHide como estaba.");
}

#[derive(Default)]
struct Medidas {
    informes: u64,
    malos: u64,
    envio_total: Duration,
    envio_max: Duration,
    respuestas: u64,
}

fn jugar(
    bus: &std::sync::Arc<Bus>,
    tipo: Tipo,
    info: &ds5_hid::InfoMando,
    segundos: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let d = Dispositivo::abrir(info)?;
    let cal = leer_calibracion(&d);
    let conexion = info.conexion;
    let mandar = |s: &Salida| d.escribir(&s.codificar(conexion), Duration::from_millis(100));
    if conexion == Conexion::Bt {
        mandar(&Salida::inicial_bt())?;
    }
    let ajustes = Mutex::new(Ajustes {
        color: COLOR_FINAL,
        ..Default::default()
    });
    mandar(&Salida::normal(&ajustes.lock().unwrap()))?;

    let virtual_ = MandoVirtual::enchufar(bus, tipo).map_err(|e| format!("al enchufar: {e}"))?;
    if let Some(i) = virtual_.indice_xinput() {
        println!("Mando de Xbox en el puesto {} de XInput", i + 1);
    }
    println!("Juega durante {segundos} s.");

    let fin = AtomicBool::new(false);
    let mut m = Medidas::default();
    std::thread::scope(|s| -> Result<(), Box<dyn std::error::Error>> {
        // La vibración (y la luz del DS4) que pide el juego, de vuelta al mando.
        let respuestas = s.spawn(|| -> u64 {
            let mut n = 0;
            while !fin.load(Ordering::Relaxed) {
                match virtual_.esperar_respuesta(Duration::from_millis(200)) {
                    Ok(r) => {
                        n += 1;
                        let mut a = ajustes.lock().unwrap();
                        let antes = *a;
                        if r.con_vibracion {
                            a.motor_izquierdo = r.motor_grande;
                            a.motor_derecho = r.motor_pequeno;
                        }
                        if let Some(c) = r.color {
                            a.color = c;
                        }
                        if *a != antes {
                            let _ = mandar(&Salida::normal(&a));
                        }
                    }
                    Err(ErrorVirtual::TiempoAgotado) => {}
                    Err(_) => std::thread::sleep(Duration::from_millis(50)),
                }
            }
            n
        });

        let inicio = Instant::now();
        let us = || inicio.elapsed().as_micros() as i64;
        let mut dec = Decodificador::new(conexion, cal, us());
        let mut bufer = vec![0u8; info.largo_entrada.max(78)];
        let r = (|| -> Result<(), Box<dyn std::error::Error>> {
            while inicio.elapsed() < Duration::from_secs(segundos) {
                let n = match d.leer(&mut bufer, Duration::from_millis(500)) {
                    Ok(n) => n,
                    Err(ErrorHid::TiempoAgotado) => continue,
                    Err(e) => return Err(e.into()),
                };
                let informe = &bufer[..n];
                if !dec.valido(informe) {
                    if conexion == Conexion::Bt && informe.first() == Some(&0x31) {
                        m.malos += 1;
                        dec.perdido();
                    }
                    continue;
                }
                let llegada = Instant::now();
                let e = dec.decodificar(informe, us());
                match tipo {
                    Tipo::Xbox360 => virtual_.enviar_xbox(&InformeXbox::desde(&e)),
                    Tipo::Ds4 => virtual_.enviar_ds4(&informe_ds4(&e)),
                }
                .map_err(|e| format!("al enviar: {e}"))?;
                let t = llegada.elapsed();
                m.informes += 1;
                m.envio_total += t;
                m.envio_max = m.envio_max.max(t);
            }
            Ok(())
        })();
        fin.store(true, Ordering::Relaxed);
        m.respuestas = respuestas.join().unwrap_or(0);
        r
    })?;

    drop(virtual_);
    let _ = mandar(&Salida::normal(&Ajustes {
        color: COLOR_FINAL,
        ..Default::default()
    }));

    let s = segundos as f64;
    println!(
        "{} informes ({:.0} por segundo), {} con el CRC mal",
        m.informes,
        m.informes as f64 / s,
        m.malos
    );
    if m.informes > 0 {
        println!(
            "Del informe del mando al mando virtual: media {:.0} µs, máximo {:.0} µs",
            m.envio_total.as_micros() as f64 / m.informes as f64,
            m.envio_max.as_micros() as f64
        );
    }
    println!("Respuestas del juego (vibración o luz): {}", m.respuestas);
    Ok(())
}
