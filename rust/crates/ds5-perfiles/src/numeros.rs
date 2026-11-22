//! Lectura y escritura de números y booleanos con las mismas reglas que .NET.
//!
//! Hay dos familias, porque la versión en C# usa las dos según el campo:
//!
//! - las de `XmlConvert`, que usa el serializador XML: estrictas e independientes del idioma;
//! - las de `TryParse` y `ToString` con la configuración regional de Windows. Los perfiles de
//!   los usuarios de DS5Control se escriben en Windows en español, así que se reproduce
//!   `es-ES`: coma decimal y punto de millares.

/// Los espacios que .NET quita alrededor de un número (`NumberStyles.AllowLeadingWhite`).
fn es_espacio_numero(c: char) -> bool {
    matches!(c, '\u{9}'..='\u{d}' | ' ')
}

/// Los espacios que quita `XmlConvert` antes de leer un booleano o un decimal.
fn es_espacio_xml(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

/// Un entero con signo opcional, como `Int32.Parse` con `NumberStyles.Integer`. Vale para
/// `XmlConvert.ToInt32`, `ToByte`, `ToSByte` y `ToUInt32` y para `int.TryParse`.
pub fn entero(texto: &str) -> Option<i64> {
    let t = texto.trim_matches(es_espacio_numero);
    let (negativo, cifras) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    if cifras.is_empty() || !cifras.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // Los ceros a la izquierda no cuentan para el desbordamiento.
    let cifras = cifras.trim_start_matches('0');
    if cifras.len() > 18 {
        return None;
    }
    let valor: i64 = if cifras.is_empty() {
        0
    } else {
        cifras.parse().ok()?
    };
    Some(if negativo { -valor } else { valor })
}

/// Un entero dentro del rango de `T`, o `None` si no es un entero o se sale del rango.
pub fn entero_en<T: TryFrom<i64>>(texto: &str) -> Option<T> {
    entero(texto).and_then(|v| T::try_from(v).ok())
}

/// `XmlConvert.ToDouble`: punto decimal, exponente opcional, `INF`, `-INF` y `NaN`.
pub fn doble_xml(texto: &str) -> Option<f64> {
    let t = texto.trim_matches(es_espacio_xml);
    match t {
        "INF" | "Infinity" => return Some(f64::INFINITY),
        "-INF" | "-Infinity" => return Some(f64::NEG_INFINITY),
        "NaN" => return Some(f64::NAN),
        _ => {}
    }
    analizar_decimal(t, '.', None)
}

/// `double.TryParse` con la configuración española: coma decimal y punto de millares.
pub fn doble_es(texto: &str) -> Option<f64> {
    let t = texto.trim_matches(|c: char| es_espacio_numero(c) || c == '\0');
    if t == "NaN" {
        return Some(f64::NAN);
    }
    analizar_decimal(t, ',', Some('.'))
}

/// Analiza `[signo] cifras [decimal cifras] [e [signo] cifras]` con los separadores dados y lo
/// pasa al analizador de Rust, que redondea igual que .NET.
fn analizar_decimal(t: &str, decimal: char, millares: Option<char>) -> Option<f64> {
    let mut normal = String::with_capacity(t.len());
    let mut caracteres = t.chars().peekable();
    if let Some(&c) = caracteres.peek() {
        if c == '-' || c == '+' {
            normal.push(c);
            caracteres.next();
        }
    }
    let mut cifras_enteras = 0;
    let mut cifras_decimales = 0;
    let mut en_decimales = false;
    while let Some(&c) = caracteres.peek() {
        if c.is_ascii_digit() {
            normal.push(c);
            if en_decimales {
                cifras_decimales += 1;
            } else {
                cifras_enteras += 1;
            }
        } else if c == decimal && !en_decimales {
            en_decimales = true;
            normal.push('.');
        } else if Some(c) == millares && !en_decimales && cifras_enteras > 0 {
            // .NET acepta el separador de millares en cualquier sitio de la parte entera.
        } else {
            break;
        }
        caracteres.next();
    }
    if cifras_enteras + cifras_decimales == 0 {
        return None;
    }
    if let Some(&c) = caracteres.peek() {
        if c == 'e' || c == 'E' {
            caracteres.next();
            normal.push('e');
            if let Some(&s) = caracteres.peek() {
                if s == '-' || s == '+' {
                    normal.push(s);
                    caracteres.next();
                }
            }
            let mut cifras_exponente = 0;
            while let Some(&c) = caracteres.peek() {
                if !c.is_ascii_digit() {
                    break;
                }
                normal.push(c);
                cifras_exponente += 1;
                caracteres.next();
            }
            if cifras_exponente == 0 {
                return None;
            }
        }
    }
    if caracteres.next().is_some() {
        return None;
    }
    normal.parse().ok()
}

/// `XmlConvert.ToString(double)`: el texto más corto que vuelve al mismo valor, como `"R"`.
pub fn formato_doble(v: f64) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "INF" } else { "-INF" }.into();
    }
    corto(v)
}

/// `double.ToString()` con la configuración española.
pub fn formato_doble_es(v: f64) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "∞" } else { "-∞" }.into();
    }
    corto(v).replace('.', ",")
}

/// El formato `"R"` de .NET: notación científica si el exponente es menor que -4 o mayor que 14.
fn corto(v: f64) -> String {
    let cientifico = format!("{v:e}");
    let (mantisa, exponente) = cientifico.split_once('e').expect("formato {:e}");
    let exponente: i32 = exponente.parse().expect("exponente");
    let (signo, mantisa) = match mantisa.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", mantisa),
    };
    let cifras: String = mantisa.chars().filter(|c| *c != '.').collect();
    if v == 0.0 {
        return format!("{signo}0");
    }
    if !(-4..15).contains(&exponente) {
        let signo_exponente = if exponente < 0 { '-' } else { '+' };
        return format!("{signo}{mantisa}E{signo_exponente}{:02}", exponente.abs());
    }
    let mut salida = String::from(signo);
    if exponente < 0 {
        salida.push_str("0.");
        salida.extend(std::iter::repeat_n('0', (-exponente - 1) as usize));
        salida.push_str(&cifras);
    } else {
        let enteras = exponente as usize + 1;
        if cifras.len() <= enteras {
            salida.push_str(&cifras);
            salida.extend(std::iter::repeat_n('0', enteras - cifras.len()));
        } else {
            salida.push_str(&cifras[..enteras]);
            salida.push('.');
            salida.push_str(&cifras[enteras..]);
        }
    }
    salida
}

/// `XmlConvert.ToBoolean`: solo `true`, `false`, `1` y `0`.
pub fn bool_xml(texto: &str) -> Option<bool> {
    match texto.trim_matches(es_espacio_xml) {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

/// `bool.TryParse`: `true` o `false` sin distinguir mayúsculas; cualquier otra cosa es `false`.
pub fn bool_net(texto: &str) -> Option<bool> {
    let t = texto.trim_matches(|c: char| c.is_whitespace() || c == '\0');
    if t.eq_ignore_ascii_case("true") {
        Some(true)
    } else if t.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

/// `bool.ToString()`.
pub fn formato_bool(v: bool) -> &'static str {
    if v {
        "True"
    } else {
        "False"
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn enteros_como_net() {
        assert_eq!(entero(" 5 "), Some(5));
        assert_eq!(entero("+7"), Some(7));
        assert_eq!(entero("-0"), Some(0));
        assert_eq!(entero("1e3"), None);
        assert_eq!(entero(""), None);
        assert_eq!(entero_en::<u8>("256"), None);
        assert_eq!(entero_en::<u8>("-0"), Some(0));
        assert_eq!(entero_en::<i32>("99999999999"), None);
    }

    #[test]
    fn decimales_como_xmlconvert() {
        assert_eq!(doble_xml("1e3"), Some(1000.0));
        assert_eq!(doble_xml(" 5 "), Some(5.0));
        assert_eq!(doble_xml("0,5"), None);
        assert_eq!(doble_xml("inf"), None);
        assert_eq!(doble_xml(".5"), Some(0.5));
    }

    #[test]
    fn decimales_en_espanol() {
        assert_eq!(doble_es("0,5"), Some(0.5));
        assert_eq!(doble_es("0.5"), Some(5.0));
        assert_eq!(doble_es("1.123456789"), Some(1123456789.0));
        assert_eq!(doble_es("abc"), None);
    }

    #[test]
    fn formato_r() {
        assert_eq!(formato_doble(0.008), "0.008");
        assert_eq!(formato_doble(100.0), "100");
        assert_eq!(formato_doble(1e15), "1E+15");
        assert_eq!(formato_doble(1e-5), "1E-05");
        assert_eq!(formato_doble(0.0001), "0.0001");
        assert_eq!(formato_doble(-0.0), "-0");
        assert_eq!(formato_doble(14.999999999999998), "14.999999999999998");
        assert_eq!(formato_doble_es(0.1), "0,1");
    }
}
