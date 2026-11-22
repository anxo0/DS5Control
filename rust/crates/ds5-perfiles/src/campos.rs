//! Los tipos de campo propios del perfil, que no son un simple número, texto o booleano.

use crate::formato::{enumerado, Formato};
use crate::numeros;
use crate::xml::{Elemento, Escritor};

/// Un color `r,g,b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// `Color`, `LowColor`, `ChargingColor` y `FlashColor`. Con tres partes, cada una tiene que ser
/// un byte (si no, C# falla al cargar); con otro número de partes el color queda en negro.
pub struct ColorRgb;
impl Formato for ColorRgb {
    type T = Rgb;
    fn leer(e: &Elemento) -> Result<Rgb, String> {
        let texto = e.texto_campo();
        if texto.is_empty() {
            return Ok(Rgb::default());
        }
        let partes: Vec<&str> = texto.split(',').collect();
        if partes.len() != 3 {
            return Ok(Rgb::default());
        }
        let byte = |p: &str| {
            numeros::entero_en::<u8>(p)
                .ok_or_else(|| format!("{} no es un color: {texto:?}", e.nombre))
        };
        Ok(Rgb(byte(partes[0])?, byte(partes[1])?, byte(partes[2])?))
    }
    fn escribir(v: &Rgb, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], &format!("{},{},{}", v.0, v.1, v.2));
    }
}

/// `Sensitivity`: las sensibilidades de LS, RS, L2, R2, SX y SZ separadas por `|`, escritas y
/// leídas con la configuración regional (coma decimal en español).
pub struct Sensibilidad;
const LIMITES_SENSIBILIDAD: [(f64, f64); 6] = [
    (0.1, 5.0),
    (0.1, 5.0),
    (0.1, 10.0),
    (0.1, 10.0),
    (0.0, 5.0),
    (0.0, 5.0),
];
impl Formato for Sensibilidad {
    type T = [f64; 6];
    fn leer(e: &Elemento) -> Result<[f64; 6], String> {
        let mut valores = [1.0; 6];
        let texto = e.texto_campo();
        if !texto.is_empty() {
            let mut partes: Vec<&str> = texto.split('|').collect();
            if partes.len() == 1 {
                partes = texto.split(',').collect();
            }
            // Si faltan partes, C# se queda con las que ha leído hasta ahí.
            for (i, (min, max)) in LIMITES_SENSIBILIDAD.iter().enumerate() {
                let Some(parte) = partes.get(i) else { break };
                valores[i] = numeros::doble_es(parte).map_or(1.0, |v| v.clamp(*min, *max));
            }
        }
        Ok(valores)
    }
    fn escribir(v: &[f64; 6], nombre: &str, w: &mut Escritor) {
        let texto: Vec<String> = v.iter().map(|x| numeros::formato_doble_es(*x)).collect();
        w.hoja(nombre, &[], &texto.join("|"));
    }
}

/// `TouchDisInvTriggers`: enteros separados por comas; los que no se entienden se descartan.
pub struct ListaEnteros;
impl Formato for ListaEnteros {
    type T = Vec<i32>;
    fn leer(e: &Elemento) -> Result<Vec<i32>, String> {
        Ok(e.texto()
            .split(',')
            .filter_map(numeros::entero_en::<i32>)
            .collect())
    }
    fn escribir(v: &Vec<i32>, nombre: &str, w: &mut Escritor) {
        let texto: Vec<String> = v.iter().map(|x| x.to_string()).collect();
        w.hoja(nombre, &[], &texto.join(","));
    }
}

/// La condición de los gatillos del giroscopio: `and` (verdadero) u `or`. Lo demás es `and`.
pub struct Condicion;
impl Formato for Condicion {
    type T = bool;
    fn leer(e: &Elemento) -> Result<bool, String> {
        Ok(e.texto_campo() != "or")
    }
    fn escribir(v: &bool, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], if *v { "and" } else { "or" });
    }
}

enumerado! {
    /// Curva de salida de sticks, gatillos y giroscopio.
    pub enum Curva {
        Lineal = "linear",
        PrecisionMejorada = "enhanced-precision",
        Cuadratica = "quadratic",
        Cubica = "cubic",
        SalidaCuadratica = "easeout-quad",
        SalidaCubica = "easeout-cubic",
        Personalizada = "custom",
    }
}

/// Una curva por su nombre; un nombre desconocido es la lineal.
pub struct ModoCurva;
impl Formato for ModoCurva {
    type T = Curva;
    fn leer(e: &Elemento) -> Result<Curva, String> {
        use crate::formato::NombreXml;
        Ok(Curva::desde_nombre(&e.texto_campo()).unwrap_or(Curva::Lineal))
    }
    fn escribir(v: &Curva, nombre: &str, w: &mut Escritor) {
        use crate::formato::NombreXml;
        w.hoja(nombre, &[], v.nombre());
    }
}

enumerado! {
    /// Suavizado del giroscopio como ratón o como stick.
    pub enum MetodoSuavizado {
        Ninguno = "none",
        UnEuro = "one-euro",
        MediaPonderada = "weighted-average",
    }
}

/// Un método de suavizado por su nombre; un nombre desconocido es ninguno.
pub struct Suavizado;
impl Formato for Suavizado {
    type T = MetodoSuavizado;
    fn leer(e: &Elemento) -> Result<MetodoSuavizado, String> {
        use crate::formato::NombreXml;
        Ok(MetodoSuavizado::desde_nombre(&e.texto_campo())
            .filter(|m| *m != MetodoSuavizado::Ninguno)
            .unwrap_or(MetodoSuavizado::Ninguno))
    }
    fn escribir(v: &MetodoSuavizado, nombre: &str, w: &mut Escritor) {
        use crate::formato::NombreXml;
        w.hoja(nombre, &[], v.nombre());
    }
}

/// `ProfileActions`: los nombres de las acciones separados por `/`, sin repetir.
pub struct AccionesPerfil;
impl Formato for AccionesPerfil {
    type T = Vec<String>;
    fn leer(e: &Elemento) -> Result<Vec<String>, String> {
        let texto = e.texto_campo();
        let mut acciones: Vec<String> = Vec::new();
        if !texto.is_empty() {
            for nombre in texto.split('/') {
                if !acciones.iter().any(|a| a == nombre) {
                    acciones.push(nombre.to_string());
                }
            }
        }
        Ok(acciones)
    }
    fn escribir(v: &Vec<String>, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], &v.join("/"));
    }
}
