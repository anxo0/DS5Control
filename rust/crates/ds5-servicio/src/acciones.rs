//! El panel de acciones especiales de la interfaz, como `ActionsPanel.cs`: la lista de acciones
//! con las que tiene activas el perfil, y guardar, borrar y activar.

use serde_json::{json, Value};

use ds5_mapeo::acciones::Especial;
use ds5_perfiles::otros::{AccionEspecial, TipoAccion};
use ds5_perfiles::{Acciones, Perfil};

fn nombre_tipo(t: TipoAccion) -> &'static str {
    match t {
        TipoAccion::Macro => "macro",
        TipoAccion::Programa => "programa",
        TipoAccion::Perfil => "perfil",
        TipoAccion::Tecla => "tecla",
        TipoAccion::DesconectarBt => "desconectar",
        TipoAccion::Bateria => "bateria",
        TipoAccion::CalibrarGiroscopio => "giroscopio",
        _ => "otra",
    }
}

/// `ActionsPanel.Describe`.
pub fn describir(acciones: &Acciones, perfil: &Perfil) -> Vec<Value> {
    acciones
        .acciones
        .iter()
        .map(|a| {
            let d = Especial::desde(a).descripcion();
            json!({
                "nombre": a.nombre,
                "tipo": nombre_tipo(a.tipo),
                "controles": d.controles.split('/').filter(|c| !c.is_empty()).collect::<Vec<_>>(),
                "detalles": d.detalles,
                "extra": d.extra,
                "retardo": d.retardo,
                "pasos": d.pasos,
                "repetir": d.repetir,
                "activa": perfil.acciones.contains(&a.nombre),
            })
        })
        .collect()
}

/// `SetEnabled`.
pub fn activar(perfil: &mut Perfil, nombre: &str, activa: bool) {
    if activa && !perfil.acciones.iter().any(|n| n == nombre) {
        perfil.acciones.push(nombre.to_string());
    }
    if !activa {
        // `List.Remove` quita solo la primera.
        if let Some(i) = perfil.acciones.iter().position(|n| n == nombre) {
            perfil.acciones.remove(i);
        }
    }
}

/// `Delete`: quita la acción de la lista y del perfil.
pub fn borrar(acciones: &mut Acciones, perfil: &mut Perfil, nombre: &str) {
    if let Some(i) = acciones.acciones.iter().position(|a| a.nombre == nombre) {
        acciones.acciones.remove(i);
    }
    activar(perfil, nombre, false);
}

/// `delay.ToString("#.##")` con punto decimal: hasta dos decimales, sin ceros de sobra y sin el
/// cero de delante (0,5 es ".5" y 0 queda vacío). .NET redondea alejándose del cero.
pub fn segundos(v: f64) -> String {
    let centesimas = (v.abs() * 100.0).round() as u64;
    let (entero, resto) = (centesimas / 100, centesimas % 100);
    let mut s = String::new();
    if v < 0.0 && centesimas > 0 {
        s.push('-');
    }
    if entero > 0 {
        s.push_str(&entero.to_string());
    }
    if resto > 0 {
        let d = format!("{resto:02}");
        s.push('.');
        s.push_str(d.trim_end_matches('0'));
    }
    s
}

fn nueva(nombre: &str, disparador: &str, tipo: TipoAccion) -> AccionEspecial {
    AccionEspecial {
        nombre: nombre.to_string(),
        disparador: disparador.to_string(),
        tipo,
        detalles: None,
        extras: String::new(),
        argumentos: String::new(),
        retardo: 0.0,
        deshacer: String::new(),
        estilo_deshacer: String::new(),
    }
}

/// `Save`: crea o cambia una acción con lo que manda la interfaz y la activa en el perfil.
/// Devuelve si ha cambiado algo.
pub fn guardar(c: &Value, acciones: &mut Acciones, perfil: &mut Perfil) -> bool {
    let nombre = c["nombre"].as_str().unwrap_or("").trim().to_string();
    let anterior = c["original"].as_str();
    let controles: Vec<&str> = c["controles"]
        .as_array()
        .map(|v| v.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    // El perfil guarda sus acciones separadas por barras.
    if nombre.is_empty() || nombre.contains('/') || controles.is_empty() {
        return false;
    }
    if Some(nombre.as_str()) != anterior && acciones.acciones.iter().any(|a| a.nombre == nombre) {
        return false;
    }
    let editar = anterior == Some(nombre.as_str());
    let disparador = controles.join("/");
    let retardo = c["retardo"].as_f64().unwrap_or(0.0);
    let s = segundos(retardo);
    let texto = |k: &str| c[k].as_str().unwrap_or("").to_string();

    let mut a = match c["tipoAccion"].as_str().unwrap_or("") {
        "macro" => {
            let pasos: Vec<String> = c["pasos"]
                .as_array()
                .map(|v| {
                    v.iter()
                        .filter_map(Value::as_i64)
                        .map(|p| p.to_string())
                        .collect()
                })
                .unwrap_or_default();
            if pasos.is_empty() {
                return false;
            }
            let mut a = nueva(&nombre, &disparador, TipoAccion::Macro);
            a.detalles = Some(pasos.join("/"));
            if c["repetir"].as_bool().unwrap_or(false) {
                a.extras = "Repeat".into();
            }
            a
        }
        "programa" => {
            let ruta = texto("detalles");
            if ruta.trim().is_empty() {
                return false;
            }
            let mut a = nueva(&nombre, &disparador, TipoAccion::Programa);
            a.detalles = Some(ruta);
            a.argumentos = texto("extra");
            a.retardo = retardo;
            a
        }
        "perfil" => {
            let mut a = nueva(&nombre, &disparador, TipoAccion::Perfil);
            a.detalles = Some(texto("detalles"));
            a
        }
        "tecla" => {
            let mut a = nueva(&nombre, &disparador, TipoAccion::Tecla);
            a.detalles = Some(texto("detalles"));
            a
        }
        "desconectar" => {
            let mut a = nueva(&nombre, &disparador, TipoAccion::DesconectarBt);
            a.detalles = Some(s);
            a
        }
        "bateria" => {
            let mut a = nueva(&nombre, &disparador, TipoAccion::Bateria);
            a.detalles = Some(format!("{s}|True|False|255|0|0|0|255|0"));
            a
        }
        "giroscopio" => {
            let mut a = nueva(&nombre, &disparador, TipoAccion::CalibrarGiroscopio);
            a.detalles = Some(s);
            a
        }
        _ => return false,
    };
    if let Some(anterior) = anterior.filter(|_| !editar) {
        borrar(acciones, perfil, anterior);
    }
    if editar {
        if let Some(i) = acciones.acciones.iter().position(|x| x.nombre == nombre) {
            std::mem::swap(&mut acciones.acciones[i], &mut a);
        }
    } else {
        acciones.acciones.push(a);
    }
    activar(perfil, &nombre, true);
    true
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn los_segundos_como_en_net() {
        assert_eq!(segundos(0.0), "");
        assert_eq!(segundos(0.5), ".5");
        assert_eq!(segundos(1.0), "1");
        assert_eq!(segundos(2.25), "2.25");
        assert_eq!(segundos(1.005), "1");
        assert_eq!(segundos(1.999), "2");
    }

    #[test]
    fn guardar_activar_y_borrar() {
        let mut acciones = Acciones::default();
        let mut p = Perfil::default();
        let c = json!({
            "nombre": " Abrir ", "original": null, "controles": ["PS", "Cross"],
            "tipoAccion": "programa", "detalles": "C:\\a.exe", "extra": "-x", "retardo": 0.5,
            "pasos": [], "repetir": false,
        });
        assert!(guardar(&c, &mut acciones, &mut p));
        assert_eq!(p.acciones, ["Abrir"]);
        let a = &acciones.acciones[0];
        assert_eq!(a.disparador, "PS/Cross");
        assert_eq!(a.argumentos, "-x");
        assert_eq!(a.retardo, 0.5);
        // El mismo nombre otra vez sin editar no vale.
        assert!(!guardar(&c, &mut acciones, &mut p));

        let d = describir(&acciones, &p);
        assert_eq!(d[0]["tipo"], "programa");
        assert_eq!(d[0]["controles"], json!(["PS", "Cross"]));
        assert_eq!(d[0]["activa"], true);

        // Renombrar borra la anterior.
        let mut c2 = c.clone();
        c2["nombre"] = json!("Otra");
        c2["original"] = json!("Abrir");
        assert!(guardar(&c2, &mut acciones, &mut p));
        assert_eq!(acciones.acciones.len(), 1);
        assert_eq!(p.acciones, ["Otra"]);

        borrar(&mut acciones, &mut p, "Otra");
        assert!(acciones.acciones.is_empty() && p.acciones.is_empty());
    }

    #[test]
    fn la_macro_y_la_bateria() {
        let mut acciones = Acciones::default();
        let mut p = Perfil::default();
        let c = json!({
            "nombre": "M", "original": null, "controles": ["L1"], "tipoAccion": "macro",
            "detalles": "", "extra": "", "retardo": 0, "pasos": [65, 100, 65], "repetir": true,
        });
        assert!(guardar(&c, &mut acciones, &mut p));
        assert_eq!(acciones.acciones[0].detalles.as_deref(), Some("65/100/65"));
        assert_eq!(acciones.acciones[0].extras, "Repeat");
        let mut b = c.clone();
        b["nombre"] = json!("B");
        b["tipoAccion"] = json!("bateria");
        b["retardo"] = json!(2);
        assert!(guardar(&b, &mut acciones, &mut p));
        assert_eq!(
            acciones.acciones[1].detalles.as_deref(),
            Some("2|True|False|255|0|0|0|255|0")
        );
        // Una macro sin pasos no se guarda.
        let mut v = c.clone();
        v["nombre"] = json!("V");
        v["pasos"] = json!([]);
        assert!(!guardar(&v, &mut acciones, &mut p));
    }
}
