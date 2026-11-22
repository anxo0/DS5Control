//! Compara las zonas muertas y las curvas con C#: cada perfil de `tools/fase5/perfiles` con cada
//! grabación de la fase 0, estado a estado, contra lo que dio `SetCurveAndDeadzone`
//! (`tools/fase5/referencias`, lo escribe `MappingOracleTests.cs`). Sin las grabaciones o sin las
//! referencias no se comprueba nada.

use std::fs;
use std::path::{Path, PathBuf};

use ds5_informes::{Estado, Movimiento};
use ds5_mapeo::{Activador, AjustesCurvas, Curvas};
use ds5_perfiles::perfil::Perfil;

fn herramientas() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tools")
}

/// Lo que usa este bloque de una línea `st`: sticks, gatillos y el borde exterior.
fn estado(valores: &str) -> Estado {
    let p: Vec<&str> = valores.split(',').collect();
    let n = |i: usize| p[i].parse::<u8>().unwrap();
    Estado {
        lx: n(1),
        ly: n(2),
        rx: n(3),
        ry: n(4),
        l2: n(5),
        r2: n(6),
        l2_crudo: n(7),
        r2_crudo: n(8),
        exterior_ls: n(27),
        exterior_rs: n(28),
        movimiento: (p.len() > 44 && !p[29].is_empty()).then(|| Movimiento {
            acel_x: p[38].parse().unwrap(),
            acel_z: p[40].parse().unwrap(),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// El activador del giroscopio tiene que dar lo que grabó C# (`outputGyroControls`).
fn giro_grabado(valores: &str) -> Option<bool> {
    let p: Vec<&str> = valores.split(',').collect();
    (p.len() > 44 && !p[29].is_empty()).then(|| p[44] == "1")
}

fn campos(e: &Estado) -> String {
    let (x, z) = e.movimiento.map_or((String::new(), String::new()), |m| {
        (m.salida_acel_x.to_string(), m.salida_acel_z.to_string())
    });
    format!(
        "{},{},{},{},{},{},{},{},{x},{z}",
        e.lx, e.ly, e.rx, e.ry, e.l2, e.r2, e.exterior_ls, e.exterior_rs
    )
}

#[test]
fn las_curvas_dan_lo_mismo_que_en_c() {
    let raiz = herramientas();
    let Ok(perfiles) = fs::read_dir(raiz.join("fase5/perfiles")) else {
        return;
    };
    let mut perfiles: Vec<PathBuf> = perfiles.map(|e| e.unwrap().path()).collect();
    perfiles.sort();
    let mut errores = Vec::new();
    let mut comparados = 0usize;
    for perfil in &perfiles {
        let nombre = perfil.file_stem().unwrap().to_string_lossy().to_string();
        let carpeta = raiz.join("fase5/referencias").join(&nombre);
        let Ok(referencias) = fs::read_dir(&carpeta) else {
            continue;
        };
        let p = Perfil::leer(&fs::read_to_string(perfil).unwrap()).unwrap();
        let ajustes = AjustesCurvas::desde_perfil(&p.ajustes);
        let mut fallos = 0;
        let mut primero = String::new();
        for referencia in referencias {
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
            let mut curvas = Curvas::default();
            let mut activador = Activador::default();
            for (i, linea) in texto
                .lines()
                .filter_map(|l| {
                    let p: Vec<&str> = l.splitn(4, '\t').collect();
                    (p.len() == 4 && p[1] == "st").then(|| p[3])
                })
                .enumerate()
            {
                let mut entrada = estado(linea);
                activador.aplicar(&mut entrada, &ajustes.giro);
                let grabado = giro_grabado(linea);
                assert_eq!(
                    entrada.movimiento.map(|m| m.giro_controles),
                    grabado.filter(|_| ajustes.giro.controles),
                    "activador del giroscopio, estado {i}"
                );
                let rust = campos(&curvas.aplicar(&entrada, &ajustes, 0));
                let csharp = esperado.next().expect("faltan referencias");
                comparados += 1;
                if rust != csharp {
                    if fallos == 0 {
                        primero = format!(
                            "{} estado {i}: entrada {}, C# {csharp}, Rust {rust}",
                            referencia.file_stem().unwrap().to_string_lossy(),
                            campos(&entrada)
                        );
                    }
                    fallos += 1;
                }
            }
        }
        if fallos > 0 {
            errores.push(format!("{nombre}: {fallos} distintos. {primero}"));
        }
    }
    eprintln!("{comparados} estados comparados");
    assert!(errores.is_empty(), "\n{}", errores.join("\n"));
}

/// El antirrebote depende del reloj y no se puede comparar con las grabaciones: si el stick cruza
/// el centro de golpe dentro del tiempo, sale el centro; pasado el tiempo, el valor de verdad.
#[test]
fn el_antirrebote_corta_el_cruce_rapido_del_centro() {
    let p = Perfil::leer(include_str!("datos/perfil-defecto.xml")).unwrap();
    let mut a = AjustesCurvas::desde_perfil(&p.ajustes);
    a.antirrebote_ls.activo = true;
    a.ls.zona_muerta = 0;
    a.ls.antizona = 0;
    let mut c = Curvas::default();
    let e = |lx| Estado {
        lx,
        ly: 128,
        rx: 128,
        ry: 128,
        ..Default::default()
    };
    assert_eq!(c.aplicar(&e(255), &a, 1000).lx, 255);
    // Se suelta y rebota al otro lado 10 ms después: se queda en el centro.
    assert_eq!(c.aplicar(&e(0), &a, 1010).lx, 128);
    // Mucho después, el valor tal cual.
    assert_eq!(c.aplicar(&e(0), &a, 2000).lx, 0);
}
