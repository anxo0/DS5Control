//! Compara la luz del mando con C#: cada caso de `tools/fase5/luz.txt` (lo escribe
//! `MappingOracleTests.Luz.cs` con `DS5LightBar.updateLightBar`) son unos ajustes y un estado del
//! mando al azar, sin lo que va con el reloj. Sin el archivo no se comprueba nada.

use std::fs;
use std::path::Path;

use ds5_mapeo::Luz;
use ds5_mapeo::{AjustesLuz, EntradaLuz, LuzForzada};
use ds5_perfiles::perfil::ModoLuz;

#[test]
fn la_luz_da_lo_mismo_que_en_c() {
    let ruta = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tools/fase5/luz.txt");
    let Ok(texto) = fs::read_to_string(ruta) else {
        return;
    };
    let mut errores = Vec::new();
    let mut n = 0;
    for linea in texto.lines() {
        let (entrada, esperado) = linea.split_once('|').unwrap();
        let v: Vec<i32> = entrada.split(',').map(|x| x.parse().unwrap()).collect();
        let b = |i: usize| v[i] != 0;
        let c = |i: usize| (v[i] as u8, v[i + 1] as u8, v[i + 2] as u8);
        let a = AjustesLuz {
            modo: [ModoLuz::None, ModoLuz::DS5Win, ModoLuz::Passthru][v[0] as usize],
            color: c(8),
            color_bajo: c(11),
            color_carga: c(14),
            color_destello: c(17),
            por_bateria: b(20),
            tipo_destello: 0,
            destello_en: v[21],
            arcoiris: 0.0,
            saturacion: 1.0,
            tipo_carga: v[22],
            desconexion_s: 0,
        };
        let en = EntradaLuz {
            bateria: v[1],
            cargando: b(2),
            bluetooth: b(3),
            personal: b(4).then(|| c(5)),
            por_defecto: b(28),
            apagando: b(29),
            distancia: b(30),
            vibracion_fuerte: v[31] as u8,
            ultimo_uso_us: 0,
            ahora_us: 0,
        };
        let forzada = LuzForzada {
            activa: b(23),
            color: c(24),
            destello: v[27] as u8,
        };
        let mut luz = Luz::nueva(0);
        luz.estado.encendido = v[32] as u8;
        let rust = if luz.actualizar(&a, &en, &forzada) {
            let e = luz.estado;
            format!(
                "{},{},{},{},{},{}",
                e.color.0, e.color.1, e.color.2, e.encendido, e.apagado, e.apagada as u8
            )
        } else {
            "-".to_string()
        };
        n += 1;
        if rust != esperado {
            errores.push(format!("{linea}\n  Rust {rust}"));
        }
    }
    eprintln!("{n} casos de luz comparados");
    assert!(
        errores.is_empty(),
        "{} distintos:\n{}",
        errores.len(),
        errores
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
