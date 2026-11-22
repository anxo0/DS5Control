//! La calibración del volante de cada mando (`ControllerConfigs.xml`), como
//! `LoadControllerConfigsForDevice`: se busca el mando por su MAC y se leen los tres puntos.

use ds5_mapeo::{CalibracionVolante, Punto};
use ds5_perfiles::xml::{Elemento, Escritor, Nodo};

pub const ARCHIVO: &str = "ControllerConfigs.xml";

/// La MAC como la escribe C# (`ReadSerial`): `AA:BB:CC:DD:EE:FF`.
pub fn mac_texto(mac: [u8; 6]) -> String {
    mac.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// `Int32.TryParse` de una parte de `x,y`: si no se entiende, se queda el valor que había.
fn punto(e: Option<&Elemento>, antes: Punto) -> Punto {
    let Some(texto) = e.map(|e| e.texto()) else {
        return antes;
    };
    let mut partes = texto.split(',');
    let mut leer = |v: i32| {
        partes
            .next()
            .and_then(|p| p.trim().parse::<i32>().ok())
            .unwrap_or(v)
    };
    Punto {
        x: leer(antes.x),
        y: leer(antes.y),
    }
}

/// La calibración guardada del mando con esa MAC, si la hay.
pub fn leer(texto: &str, mac: [u8; 6]) -> Option<CalibracionVolante> {
    let raiz = Elemento::leer(texto).ok()?;
    let mac = mac_texto(mac);
    let nodo = raiz
        .elementos()
        .find(|e| e.nombre == "Controller" && e.atributo("Mac") == Some(mac.as_str()))?;
    let c = CalibracionVolante::default();
    Some(CalibracionVolante {
        centro: punto(nodo.hijo("wheelCenterPoint"), c.centro),
        izquierda_90: punto(nodo.hijo("wheel90DegPointLeft"), c.izquierda_90),
        derecha_90: punto(nodo.hijo("wheel90DegPointRight"), c.derecha_90),
    })
}

/// Los tres puntos del volante, en el orden en que los escribe C#.
const PUNTOS: [&str; 3] = [
    "wheelCenterPoint",
    "wheel90DegPointLeft",
    "wheel90DegPointRight",
];

/// Vuelve a escribir un elemento leído, con la sangría de `XmlDocument.Save`. Los espacios entre
/// elementos no cuentan, como al cargar sin `PreserveWhitespace`.
fn escribir_elemento(w: &mut Escritor, e: &Elemento) {
    let atributos: Vec<(&str, &str)> = e
        .atributos
        .iter()
        .map(|(n, v)| (n.as_str(), v.as_str()))
        .collect();
    if e.elementos().next().is_none() {
        w.hoja(&e.nombre, &atributos, &e.texto_campo());
        return;
    }
    w.abrir(&e.nombre, &atributos);
    for hijo in e.elementos() {
        escribir_elemento(w, hijo);
    }
    w.cerrar();
}

/// `SaveControllerConfigsForDevice`: el archivo con la calibración nueva del mando con esa MAC.
/// Los demás mandos y los otros ajustes de este (los que guarda `PersistSettings`) se quedan como
/// estaban; si el mando no estaba, se añade al final. `anterior` es el archivo que había.
pub fn escribir(
    anterior: Option<&str>,
    mac: [u8; 6],
    tipo: &str,
    c: &CalibracionVolante,
) -> String {
    let mac = mac_texto(mac);
    let mut raiz = anterior
        .and_then(|t| Elemento::leer(t).ok())
        .filter(|r| r.nombre == "Controllers")
        .unwrap_or_else(|| Elemento::nuevo("Controllers"));
    raiz.hijos.retain(|n| matches!(n, Nodo::Elemento(_)));
    let posicion = raiz.hijos.iter().position(|n| {
        matches!(n, Nodo::Elemento(e) if e.nombre == "Controller" && e.atributo("Mac") == Some(mac.as_str()))
    });
    let otros: Vec<Nodo> = match posicion {
        Some(i) => match &raiz.hijos[i] {
            Nodo::Elemento(e) => e
                .hijos
                .iter()
                .filter(|n| matches!(n, Nodo::Elemento(h) if !PUNTOS.contains(&h.nombre.as_str())))
                .cloned()
                .collect(),
            Nodo::Texto(_) => Vec::new(),
        },
        None => Vec::new(),
    };
    let mut nodo = Elemento::nuevo("Controller");
    nodo.atributos = vec![
        ("Mac".into(), mac.clone()),
        ("ControllerType".into(), tipo.into()),
    ];
    let valores = [c.centro, c.izquierda_90, c.derecha_90];
    for (nombre, p) in PUNTOS.iter().zip(valores) {
        nodo.hijos.push(Nodo::Elemento(Elemento::con_texto(
            nombre,
            &format!("{},{}", p.x, p.y),
        )));
    }
    nodo.hijos.extend(otros);
    match posicion {
        Some(i) => raiz.hijos[i] = Nodo::Elemento(nodo),
        None => raiz.hijos.push(Nodo::Elemento(nodo)),
    }

    let mut w = Escritor::new();
    w.crudo("<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n");
    w.crudo(&format!(
        "<!-- Controller config data. {} -->\r\n",
        crate::guardado::fecha_espanola()
    ));
    escribir_elemento(&mut w, &raiz);
    w.terminar()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    const MAC: [u8; 6] = [0xA0, 0x5A, 0x5C, 0x01, 0x02, 0xFF];

    #[test]
    fn la_mac_como_en_c() {
        assert_eq!(mac_texto(MAC), "A0:5A:5C:01:02:FF");
    }

    #[test]
    fn lee_la_del_mando_y_no_otra() {
        let x = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<Controllers>\
            <Controller Mac=\"11:22:33:44:55:66\"><wheelCenterPoint>9,9</wheelCenterPoint></Controller>\
            <Controller Mac=\"A0:5A:5C:01:02:FF\"><wheelCenterPoint>12,-340</wheelCenterPoint>\
            <wheel90DegPointLeft>-8000,100</wheel90DegPointLeft><wheel90DegPointRight>8100,90</wheel90DegPointRight></Controller>\
            </Controllers>";
        let c = leer(x, MAC).unwrap();
        assert_eq!(c.centro, Punto { x: 12, y: -340 });
        assert_eq!(c.izquierda_90, Punto { x: -8000, y: 100 });
        assert_eq!(c.derecha_90, Punto { x: 8100, y: 90 });
        assert_eq!(leer(x, [0; 6]), None);
        assert_eq!(leer("<Controllers />", MAC), None);
    }

    #[test]
    fn un_punto_a_medias_deja_lo_que_se_entiende() {
        let x = "<Controllers><Controller Mac=\"A0:5A:5C:01:02:FF\">\
            <wheelCenterPoint>5,abc</wheelCenterPoint></Controller></Controllers>";
        let c = leer(x, MAC).unwrap();
        assert_eq!(c.centro, Punto { x: 5, y: 0 });
        assert_eq!(c.derecha_90, Punto::default());
    }

    #[test]
    fn guarda_la_del_mando_y_respeta_lo_demas() {
        let c = CalibracionVolante {
            centro: Punto { x: 1, y: -2 },
            izquierda_90: Punto { x: -300, y: 4 },
            derecha_90: Punto { x: 300, y: 5 },
        };
        let antes = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<!-- x -->\r\n<Controllers>\r\n\
            <Controller Mac=\"11:22:33:44:55:66\" ControllerType=\"DS4\"><wheelCenterPoint>9,9</wheelCenterPoint></Controller>\r\n\
            <Controller Mac=\"A0:5A:5C:01:02:FF\" ControllerType=\"DualSense\"><wheelCenterPoint>0,0</wheelCenterPoint>\
            <DualSenseSupportSettings><EnableRumble>true</EnableRumble></DualSenseSupportSettings></Controller>\r\n\
            </Controllers>";
        let texto = escribir(Some(antes), MAC, "DualSense", &c);
        assert!(texto.starts_with(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<!-- Controller config data. "
        ));
        assert_eq!(leer(&texto, MAC), Some(c));
        assert_eq!(
            leer(&texto, [0x11, 0x22, 0x33, 0x44, 0x55, 0x66])
                .unwrap()
                .centro,
            Punto { x: 9, y: 9 }
        );
        assert!(texto.contains(
            "\r\n    <DualSenseSupportSettings>\r\n      <EnableRumble>true</EnableRumble>"
        ));
        assert_eq!(texto.matches("<Controller ").count(), 2);
        assert!(texto.ends_with("</Controllers>"));

        // Sin archivo, o con uno roto, se empieza de cero.
        for anterior in [None, Some("no es xml")] {
            let nuevo = escribir(anterior, MAC, "DualSense", &c);
            assert_eq!(leer(&nuevo, MAC), Some(c));
            assert!(nuevo.contains(
                "<Controller Mac=\"A0:5A:5C:01:02:FF\" ControllerType=\"DualSense\">\r\n    <wheelCenterPoint>1,-2</wheelCenterPoint>"
            ));
        }
    }
}
