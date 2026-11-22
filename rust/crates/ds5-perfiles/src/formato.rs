//! Cómo se lee y se escribe cada tipo de campo, y las macros que declaran los grupos de campos
//! en el mismo orden en que los escribe la versión en C#.

use crate::numeros::{self, entero_en};
use crate::xml::{Elemento, Escritor};

/// Un tipo de campo: cómo pasar del texto del elemento al valor y del valor al elemento.
pub trait Formato {
    type T;
    /// `Err` con el motivo si la versión en C# rechazaría el archivo entero.
    fn leer(e: &Elemento) -> Result<Self::T, String>;
    fn escribir(v: &Self::T, nombre: &str, w: &mut Escritor);
}

/// Un valor de enumeración con su nombre en el XML.
pub trait NombreXml: Sized + Copy {
    fn desde_nombre(nombre: &str) -> Option<Self>;
    fn nombre(&self) -> &'static str;
}

/// Un grupo de campos: el contenido de un elemento con hijos.
pub trait Grupo: Default {
    fn leer_campos(&mut self, e: &Elemento) -> Result<(), String>;
    fn escribir_campos(&self, w: &mut Escritor);
}

fn error_valor(e: &Elemento, tipo: &str) -> String {
    format!("{} no es {tipo}: {:?}", e.nombre, e.texto())
}

/// `int`, con las reglas de `XmlConvert`.
pub struct Entero;
impl Formato for Entero {
    type T = i32;
    fn leer(e: &Elemento) -> Result<i32, String> {
        entero_en(&e.texto()).ok_or_else(|| error_valor(e, "un entero"))
    }
    fn escribir(v: &i32, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], &v.to_string());
    }
}

/// `byte`.
pub struct Byte;
impl Formato for Byte {
    type T = u8;
    fn leer(e: &Elemento) -> Result<u8, String> {
        entero_en(&e.texto()).ok_or_else(|| error_valor(e, "un byte"))
    }
    fn escribir(v: &u8, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], &v.to_string());
    }
}

/// `sbyte`.
pub struct ByteConSigno;
impl Formato for ByteConSigno {
    type T = i8;
    fn leer(e: &Elemento) -> Result<i8, String> {
        entero_en(&e.texto()).ok_or_else(|| error_valor(e, "un byte con signo"))
    }
    fn escribir(v: &i8, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], &v.to_string());
    }
}

/// `uint`.
pub struct Natural;
impl Formato for Natural {
    type T = u32;
    fn leer(e: &Elemento) -> Result<u32, String> {
        entero_en(&e.texto()).ok_or_else(|| error_valor(e, "un natural"))
    }
    fn escribir(v: &u32, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], &v.to_string());
    }
}

/// `double`.
pub struct Doble;
impl Formato for Doble {
    type T = f64;
    fn leer(e: &Elemento) -> Result<f64, String> {
        numeros::doble_xml(&e.texto()).ok_or_else(|| error_valor(e, "un número"))
    }
    fn escribir(v: &f64, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], &numeros::formato_doble(*v));
    }
}

/// Un booleano guardado como texto `True`/`False`: lo que no se entiende es `False`.
pub struct BoolTexto;
impl Formato for BoolTexto {
    type T = bool;
    fn leer(e: &Elemento) -> Result<bool, String> {
        Ok(numeros::bool_net(&e.texto()).unwrap_or(false))
    }
    fn escribir(v: &bool, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], numeros::formato_bool(*v));
    }
}

/// Un booleano de `XmlConvert`: `true`/`false`, y cualquier otra cosa rechaza el archivo.
pub struct BoolXml;
impl Formato for BoolXml {
    type T = bool;
    fn leer(e: &Elemento) -> Result<bool, String> {
        numeros::bool_xml(&e.texto()).ok_or_else(|| error_valor(e, "true o false"))
    }
    fn escribir(v: &bool, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], if *v { "true" } else { "false" });
    }
}

/// Texto que siempre se escribe, aunque esté vacío.
pub struct Texto;
impl Formato for Texto {
    type T = String;
    fn leer(e: &Elemento) -> Result<String, String> {
        Ok(e.texto_campo())
    }
    fn escribir(v: &String, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], v);
    }
}

/// Texto que no se escribe si no tiene valor (un `null` en C#).
pub struct TextoOpcional;
impl Formato for TextoOpcional {
    type T = Option<String>;
    fn leer(e: &Elemento) -> Result<Option<String>, String> {
        Ok(Some(e.texto_campo()))
    }
    fn escribir(v: &Option<String>, nombre: &str, w: &mut Escritor) {
        if let Some(t) = v {
            w.hoja(nombre, &[], t);
        }
    }
}

/// Una enumeración: el nombre tiene que coincidir exactamente.
pub struct Enumerado<E>(std::marker::PhantomData<E>);
impl<E: NombreXml> Formato for Enumerado<E> {
    type T = E;
    fn leer(e: &Elemento) -> Result<E, String> {
        E::desde_nombre(&e.texto()).ok_or_else(|| error_valor(e, "un valor conocido"))
    }
    fn escribir(v: &E, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], v.nombre());
    }
}

/// Un elemento con hijos.
pub struct Anidado<G>(std::marker::PhantomData<G>);
impl<G: Grupo> Formato for Anidado<G> {
    type T = G;
    fn leer(e: &Elemento) -> Result<G, String> {
        let mut g = G::default();
        g.leer_campos(e)?;
        Ok(g)
    }
    fn escribir(v: &G, nombre: &str, w: &mut Escritor) {
        w.abrir(nombre, &[]);
        v.escribir_campos(w);
        w.cerrar();
    }
}

/// Declara una enumeración con el nombre que tiene cada valor en el XML.
macro_rules! enumerado {
    ($(#[$m:meta])* pub enum $nombre:ident { $( $variante:ident = $xml:literal ),* $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $nombre { $( $variante ),* }

        impl $crate::formato::NombreXml for $nombre {
            fn desde_nombre(nombre: &str) -> Option<Self> {
                match nombre { $( $xml => Some(Self::$variante), )* _ => None }
            }
            fn nombre(&self) -> &'static str {
                match self { $( Self::$variante => $xml, )* }
            }
        }
    };
}
pub(crate) use enumerado;

/// Declara un grupo de campos en el orden en que se escriben. Cada campo lleva el nombre del
/// elemento, su tipo, el valor que toma si falta y, si hace falta, el ajuste que le aplica la
/// versión en C# al leerlo (límites, conversiones de unidades...).
///
/// Si un elemento aparece dos veces, cuenta el primero, como en el serializador de .NET.
macro_rules! grupo {
    ($(#[$m:meta])* pub struct $nombre:ident {
        $( $xml:literal => $campo:ident : $f:ty = $def:expr $(, $ajuste:expr)? ; )*
    }) => {
        $(#[$m])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $nombre { $( pub $campo: <$f as $crate::formato::Formato>::T, )* }

        impl Default for $nombre {
            fn default() -> Self { Self { $( $campo: $def, )* } }
        }

        impl $crate::formato::Grupo for $nombre {
            #[allow(clippy::redundant_closure_call)]
            fn leer_campos(&mut self, e: &$crate::xml::Elemento) -> Result<(), String> {
                let mut leidos: Vec<&str> = Vec::new();
                for hijo in e.elementos() {
                    let nombre = hijo.nombre.as_str();
                    if leidos.contains(&nombre) { continue; }
                    match nombre {
                        $( $xml => {
                            leidos.push($xml);
                            let v = <$f as $crate::formato::Formato>::leer(hijo)?;
                            $( let v = ($ajuste)(v); )?
                            self.$campo = v;
                        } )*
                        _ => {}
                    }
                }
                Ok(())
            }

            fn escribir_campos(&self, w: &mut $crate::xml::Escritor) {
                $( <$f as $crate::formato::Formato>::escribir(&self.$campo, $xml, w); )*
            }
        }
    };
}
pub(crate) use grupo;
