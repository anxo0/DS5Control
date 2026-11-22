//! Compara la reasignación con C#. Primero con lo que grabó la app de verdad (las líneas `map` de
//! las grabaciones de la fase 0, con el perfil y el mando virtual de cada momento) y luego con los
//! perfiles de `tools/fase5/perfiles` contra lo que dio `MapCustom` (`tools/fase5/mapeo`, lo
//! escribe `MappingOracleTests.cs`). Sin las grabaciones o sin las referencias no se comprueba nada.

use std::fs;
use std::path::{Path, PathBuf};

use ds5_informes::{Estado, Movimiento};
use ds5_mapeo::{Activador, AjustesBotones, AjustesCurvas, Botones, Curvas, Entradas};
use ds5_perfiles::perfil::{MandoVirtual, Perfil};

fn herramientas() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tools")
}

/// Lo que usa la reasignación de una línea `st`: botones, sticks, gatillos y el borde exterior.
fn estado(valores: &str) -> Estado {
    let p: Vec<&str> = valores.split(',').collect();
    let n = |i: usize| p[i].parse::<u8>().unwrap();
    Estado {
        botones: u64::from_str_radix(p[0], 16).unwrap(),
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

/// Los campos que se comparan, como los escribe `ReportRecorder.FormatState`.
fn campos(e: &Estado) -> String {
    format!(
        "{:X},{},{},{},{},{},{},{},{}",
        e.botones, e.lx, e.ly, e.rx, e.ry, e.l2, e.r2, e.exterior_ls, e.exterior_rs
    )
}

fn campos_grabados(valores: &str) -> String {
    let p: Vec<&str> = valores.split(',').collect();
    [p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[27], p[28]].join(",")
}

fn mando(nombre: &str) -> MandoVirtual {
    match nombre {
        "DS4" => MandoVirtual::DS4,
        "X360" => MandoVirtual::X360,
        _ => MandoVirtual::None,
    }
}

/// Las líneas de una grabación: tiempo en microsegundos, tipo y el resto.
fn lineas(texto: &str) -> impl Iterator<Item = (i64, &str, &str)> {
    texto.lines().filter_map(|l| {
        let p: Vec<&str> = l.splitn(4, '\t').collect();
        (p.len() == 4 && p[0] != "#").then(|| (p[0].parse().unwrap_or(0), p[1], p[3]))
    })
}

#[test]
fn el_perfil_grabado_da_lo_que_mando_c() {
    let Ok(carpetas) = fs::read_dir(herramientas().join("fase0/grabaciones")) else {
        return;
    };
    let mut errores = Vec::new();
    let mut comparados = 0;
    for carpeta in carpetas {
        let carpeta = carpeta.unwrap().path();
        let Ok(texto) = fs::read_to_string(carpeta.join("grabacion.txt")) else {
            continue;
        };
        let xml = fs::read_to_string(carpeta.join("ajustes/Profiles/Default.xml")).unwrap();
        let perfil = Perfil::leer(&xml).unwrap();
        let curvas = AjustesCurvas::desde_perfil(&perfil.ajustes);
        let ajustes = AjustesBotones::desde_perfil(&perfil);
        let mut c = Curvas::default();
        let mut b = Botones::default();
        let mut activador = Activador::default();
        let mut virtual_ = MandoVirtual::None;
        let mut pendiente = None;
        let mut fallos = 0;
        let mut primero = String::new();
        for (us, tipo, resto) in lineas(&texto) {
            match tipo {
                "perfil" => virtual_ = mando(resto.rsplit('\t').next().unwrap()),
                "st" => {
                    let mut entrada = estado(resto);
                    activador.aplicar(&mut entrada, &curvas.giro);
                    let curvado = c.aplicar(&entrada, &curvas, us / 1000);
                    let r = b.aplicar(
                        &curvado,
                        &Entradas::default(),
                        &ajustes,
                        virtual_,
                        us / 1000,
                    );
                    pendiente = Some((campos(&entrada), campos(&r)));
                }
                "map" => {
                    let (entrada, rust) = pendiente.take().expect("map sin st");
                    let csharp = campos_grabados(resto);
                    comparados += 1;
                    if rust != csharp {
                        if fallos == 0 {
                            primero = format!("entrada {entrada}, C# {csharp}, Rust {rust}");
                        }
                        fallos += 1;
                    }
                }
                _ => {}
            }
        }
        if fallos > 0 {
            let nombre = carpeta.file_name().unwrap().to_string_lossy().to_string();
            errores.push(format!("{nombre}: {fallos} distintos. {primero}"));
        }
    }
    eprintln!("{comparados} estados comparados");
    assert!(errores.is_empty(), "\n{}", errores.join("\n"));
}

#[test]
fn los_perfiles_de_prueba_dan_lo_mismo_que_en_c() {
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
        let Ok(referencias) = fs::read_dir(raiz.join("fase5/mapeo").join(&nombre)) else {
            continue;
        };
        let p = Perfil::leer(&fs::read_to_string(perfil).unwrap()).unwrap();
        let curvas = AjustesCurvas::desde_perfil(&p.ajustes);
        let ajustes = AjustesBotones::desde_perfil(&p);
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
            let mut c = Curvas::default();
            let mut b = Botones::default();
            let mut activador = Activador::default();
            for (i, (us, _, resto)) in lineas(&texto).filter(|l| l.1 == "st").enumerate() {
                let mut entrada = estado(resto);
                activador.aplicar(&mut entrada, &curvas.giro);
                let curvado = c.aplicar(&entrada, &curvas, us / 1000);
                let r = b.aplicar(
                    &curvado,
                    &Entradas::default(),
                    &ajustes,
                    p.ajustes.mando_virtual,
                    us / 1000,
                );
                let rust = campos(&r);
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
