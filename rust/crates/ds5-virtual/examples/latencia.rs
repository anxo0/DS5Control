//! Mide desde fuera cuánto tarda una pulsación en llegar al juego, con cualquier programa que haga
//! de puente (DS5Control en C# o el modo directo en Rust). Lee el DualSense y XInput a la vez: cada
//! vez que Equis cambia en el mando, cuenta el tiempo hasta que A cambia en el mando de Xbox.
//!
//! `cargo run --release -p ds5-virtual --example latencia -- [segundos]`
//!
//! Mientras dura, pulsa Equis muchas veces (pulsaciones cortas, sin prisa).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ds5_hid::{enumerar, Dispositivo, ErrorHid};
use ds5_informes::{boton, Conexion, Decodificador};
use ds5_virtual::hidhide::HidHide;
use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_GAMEPAD_A, XINPUT_STATE};

fn main() {
    let segundos: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    // Si HidHide esconde el mando, este programa también tiene que verlo.
    let exe = std::env::current_exe().unwrap();
    let hh = HidHide::abrir();
    let anadido = hh.as_ref().is_some_and(|h| h.permitir_programa(&exe));

    let resultado = medir(segundos);

    if anadido {
        if let Some(h) = &hh {
            h.retirar_programa(&exe);
        }
    }
    if let Err(e) = resultado {
        println!("Error: {e}");
    }
}

fn puestos_xinput() -> Vec<u32> {
    (0..4)
        .filter(|&i| {
            let mut s = XINPUT_STATE::default();
            unsafe { XInputGetState(i, &mut s) == 0 }
        })
        .collect()
}

fn medir(segundos: u64) -> Result<(), Box<dyn std::error::Error>> {
    let mut mandos = enumerar();
    mandos.sort_by_key(|m| m.conexion != Conexion::Usb);
    let Some(info) = mandos.into_iter().next() else {
        println!("No hay ningún DualSense conectado.");
        return Ok(());
    };
    let puestos = puestos_xinput();
    if puestos.is_empty() {
        println!("No hay ningún mando de Xbox: abre antes el programa que hace de puente.");
        return Ok(());
    }
    println!(
        "{:?} por {:?}; mandos de Xbox en los puestos {:?}",
        info.modelo,
        info.conexion,
        puestos.iter().map(|p| p + 1).collect::<Vec<_>>()
    );
    let d = Dispositivo::abrir(&info)?;
    println!("Pulsa Equis muchas veces durante {segundos} s.");

    let inicio = Instant::now();
    let fin = AtomicBool::new(false);
    let (fisico, virtual_) = std::thread::scope(|s| {
        let xinput = s.spawn(|| {
            let mut cambios: Vec<(Instant, bool)> = Vec::new();
            let mut antes = false;
            while !fin.load(Ordering::Relaxed) {
                let mut a = false;
                for &p in &puestos {
                    let mut st = XINPUT_STATE::default();
                    if unsafe { XInputGetState(p, &mut st) } == 0
                        && st.Gamepad.wButtons.0 & XINPUT_GAMEPAD_A.0 != 0
                    {
                        a = true;
                    }
                }
                if a != antes {
                    cambios.push((Instant::now(), a));
                    antes = a;
                }
                std::hint::spin_loop();
            }
            cambios
        });

        let mut cambios: Vec<(Instant, bool)> = Vec::new();
        let mut dec = Decodificador::new(info.conexion, None, 0);
        let mut bufer = vec![0u8; info.largo_entrada.max(78)];
        let mut antes = false;
        while inicio.elapsed() < Duration::from_secs(segundos) {
            let n = match d.leer(&mut bufer, Duration::from_millis(200)) {
                Ok(n) => n,
                Err(ErrorHid::TiempoAgotado) => continue,
                Err(_) => break,
            };
            let llegada = Instant::now();
            if !dec.valido(&bufer[..n]) {
                continue;
            }
            let e = dec.decodificar(&bufer[..n], inicio.elapsed().as_micros() as i64);
            let x = e.botones & boton::EQUIS != 0;
            if x != antes {
                cambios.push((llegada, x));
                antes = x;
            }
        }
        fin.store(true, Ordering::Relaxed);
        (cambios, xinput.join().unwrap())
    });

    // Cada cambio del mando con el primer cambio igual en XInput que llega después.
    let mut retrasos: Vec<f64> = Vec::new();
    let mut j = 0;
    for (t, v) in &fisico {
        while j < virtual_.len() && virtual_[j].0 < *t {
            j += 1;
        }
        if let Some((tv, vv)) = virtual_.get(j) {
            let ms = (*tv - *t).as_secs_f64() * 1000.0;
            if vv == v && ms < 50.0 {
                retrasos.push(ms);
                j += 1;
            }
        }
    }
    println!(
        "Cambios de Equis: {} en el mando, {} en XInput, {} emparejados",
        fisico.len(),
        virtual_.len(),
        retrasos.len()
    );
    if retrasos.is_empty() {
        return Ok(());
    }
    retrasos.sort_by(|a, b| a.total_cmp(b));
    let p = |q: f64| retrasos[((retrasos.len() - 1) as f64 * q).round() as usize];
    println!(
        "Retraso hasta el juego: mediana {:.2} ms, 95 % por debajo de {:.2} ms, mínimo {:.2} ms, máximo {:.2} ms",
        p(0.5),
        p(0.95),
        retrasos[0],
        retrasos[retrasos.len() - 1]
    );
    Ok(())
}
