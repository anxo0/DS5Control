//! Compara las macros con C#: cada macro de prueba de `tools/fase5/macros.txt` (la escribe
//! `MappingOracleTests.Macros.cs` pasándola por `PlayMacroTask`) se toca entera en Rust, y tienen
//! que salir las mismas pulsaciones del teclado y el ratón, la misma vibración, la misma luz y los
//! mismos botones del mando virtual pulsados al acabar. Sin el archivo no se comprueba nada.

use std::fs;
use std::path::Path;

use ds5_mapeo::{EventoRaton, LuzForzada, Macros};

fn evento(t: &str) -> EventoRaton {
    let (tipo, resto) = t.split_at(1);
    let par = || -> (u32, i32) {
        let (a, b) = resto.split_once(':').unwrap();
        (a.parse().unwrap(), b.parse().unwrap())
    };
    match tipo {
        "b" => EventoRaton::Boton(resto.parse().unwrap()),
        "x" => {
            let (a, b) = par();
            EventoRaton::BotonX(a, b)
        }
        "k" => EventoRaton::Tecla(resto.parse().unwrap()),
        "K" => EventoRaton::TeclaAlt(resto.parse().unwrap()),
        "u" => EventoRaton::SueltaTecla(resto.parse().unwrap()),
        "U" => EventoRaton::SueltaTeclaAlt(resto.parse().unwrap()),
        _ => panic!("evento desconocido {t}"),
    }
}

#[test]
fn las_macros_hacen_lo_mismo_que_en_c() {
    let ruta = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tools/fase5/macros.txt");
    let Ok(texto) = fs::read_to_string(ruta) else {
        return;
    };
    let mut errores = Vec::new();
    let mut n = 0;
    for linea in texto.lines() {
        let p: Vec<&str> = linea.split('|').collect();
        let pasos: Vec<i32> = p[0].split('/').map(|v| v.parse().unwrap()).collect();
        let tipo: u8 = p[1].parse().unwrap();
        let esperado: Vec<EventoRaton> = p[2]
            .split(';')
            .filter(|t| !t.is_empty())
            .map(evento)
            .collect();

        let mut m = Macros::default();
        let mut luz = LuzForzada::default();
        let eventos = m.tocar_entera(&pasos, tipo, &mut luz);
        let vibracion = m
            .vibracion
            .map_or("-".to_string(), |(f, s)| format!("{f},{s}"));
        let luz = format!(
            "{}:{},{},{}",
            luz.activa as u8, luz.color.0, luz.color.1, luz.color.2
        );
        let pulsados: String = m
            .botones_mando()
            .iter()
            .map(|&b| if b { '1' } else { '0' })
            .collect();
        n += 1;
        // La columna 3 dice si C# se paró con una excepción; Rust se para en el mismo paso y eso
        // ya se ve en lo demás.
        if eventos != esperado || vibracion != p[4] || luz != p[5] || pulsados != p[6] {
            errores.push(format!(
                "{linea}\n  Rust {eventos:?}|{vibracion}|{luz}|{pulsados}"
            ));
        }
    }
    eprintln!("{n} macros comparadas");
    assert!(errores.is_empty(), "\n{}", errores.join("\n"));
}
