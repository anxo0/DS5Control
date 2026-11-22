//! Un árbol XML mínimo para leer los archivos y un escritor que da exactamente el mismo texto que
//! el `XmlWriter` de .NET con `Indent = true`: dos espacios por nivel, saltos de línea `\r\n` y
//! `<X />` para los elementos vacíos.

use std::fmt::Write as _;

/// Un elemento con sus atributos y su contenido. Los comentarios no se guardan.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Elemento {
    pub nombre: String,
    pub atributos: Vec<(String, String)>,
    pub hijos: Vec<Nodo>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Nodo {
    Elemento(Elemento),
    Texto(String),
}

/// El documento no es XML válido.
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorXml(pub String);

impl Elemento {
    pub fn nuevo(nombre: &str) -> Self {
        Elemento {
            nombre: nombre.into(),
            ..Default::default()
        }
    }

    /// Un elemento con solo texto dentro.
    pub fn con_texto(nombre: &str, texto: &str) -> Self {
        Elemento {
            nombre: nombre.into(),
            atributos: Vec::new(),
            hijos: vec![Nodo::Texto(texto.into())],
        }
    }

    /// Lee un documento y devuelve su elemento raíz.
    pub fn leer(texto: &str) -> Result<Elemento, ErrorXml> {
        let documento = roxmltree::Document::parse(texto).map_err(|e| ErrorXml(e.to_string()))?;
        Ok(Self::desde(documento.root_element()))
    }

    fn desde(nodo: roxmltree::Node) -> Elemento {
        let mut elemento = Elemento::nuevo(nodo.tag_name().name());
        elemento.atributos = nodo
            .attributes()
            .map(|a| (a.name().to_string(), a.value().to_string()))
            .collect();
        for hijo in nodo.children() {
            if hijo.is_element() {
                elemento.hijos.push(Nodo::Elemento(Self::desde(hijo)));
            } else if let Some(texto) = hijo.text().filter(|_| hijo.is_text()) {
                elemento.hijos.push(Nodo::Texto(texto.to_string()));
            }
        }
        elemento
    }

    /// Los elementos hijos, en orden.
    pub fn elementos(&self) -> impl Iterator<Item = &Elemento> {
        self.hijos.iter().filter_map(|n| match n {
            Nodo::Elemento(e) => Some(e),
            Nodo::Texto(_) => None,
        })
    }

    /// El primer hijo con ese nombre.
    pub fn hijo(&self, nombre: &str) -> Option<&Elemento> {
        self.elementos().find(|e| e.nombre == nombre)
    }

    pub fn atributo(&self, nombre: &str) -> Option<&str> {
        self.atributos
            .iter()
            .find(|(n, _)| n == nombre)
            .map(|(_, v)| v.as_str())
    }

    /// Todo el texto de dentro, como `InnerText` en .NET.
    pub fn texto(&self) -> String {
        let mut salida = String::new();
        self.juntar_texto(&mut salida);
        salida
    }

    fn juntar_texto(&self, salida: &mut String) {
        for hijo in &self.hijos {
            match hijo {
                Nodo::Texto(t) => salida.push_str(t),
                Nodo::Elemento(e) => e.juntar_texto(salida),
            }
        }
    }

    /// El texto como lo da el serializador de .NET a un campo de texto: si solo hay espacios,
    /// no cuentan como contenido.
    pub fn texto_campo(&self) -> String {
        let t = self.texto();
        if t.chars().all(|c| matches!(c, ' ' | '\t' | '\n' | '\r')) {
            String::new()
        } else {
            t
        }
    }
}

/// Escribe XML con el mismo formato que .NET.
pub struct Escritor {
    salida: String,
    abiertos: Vec<String>,
    /// La etiqueta de apertura del último elemento aún no se ha cerrado con `>`.
    pendiente: bool,
}

impl Default for Escritor {
    fn default() -> Self {
        Self::new()
    }
}

impl Escritor {
    pub fn new() -> Self {
        Escritor {
            salida: String::new(),
            abiertos: Vec::new(),
            pendiente: false,
        }
    }

    /// Texto tal cual, para la declaración, los comentarios y los saltos de línea del encabezado.
    pub fn crudo(&mut self, texto: &str) {
        self.salida.push_str(texto);
    }

    fn sangria(&mut self) {
        if !self.abiertos.is_empty() || !self.salida.is_empty() {
            // El primer elemento del documento va justo después del encabezado.
            if !self.salida.ends_with('\n') {
                self.salida.push_str("\r\n");
            }
        }
        for _ in 0..self.abiertos.len() {
            self.salida.push_str("  ");
        }
    }

    fn cerrar_apertura(&mut self) {
        if self.pendiente {
            self.salida.push('>');
            self.pendiente = false;
        }
    }

    pub fn abrir(&mut self, nombre: &str, atributos: &[(&str, &str)]) {
        self.cerrar_apertura();
        self.sangria();
        self.salida.push('<');
        self.salida.push_str(nombre);
        for (n, v) in atributos {
            let _ = write!(self.salida, " {n}=\"{}\"", escapar(v, true));
        }
        self.abiertos.push(nombre.to_string());
        self.pendiente = true;
    }

    pub fn cerrar(&mut self) {
        let nombre = self.abiertos.pop().expect("cerrar sin abrir");
        if self.pendiente {
            self.salida.push_str(" />");
            self.pendiente = false;
        } else {
            self.sangria();
            let _ = write!(self.salida, "</{nombre}>");
        }
    }

    /// Un elemento con texto. Si el texto está vacío, queda como `<X />`.
    pub fn hoja(&mut self, nombre: &str, atributos: &[(&str, &str)], texto: &str) {
        self.abrir(nombre, atributos);
        if !texto.is_empty() {
            self.salida.push('>');
            self.pendiente = false;
            self.salida.push_str(&escapar(texto, false));
            let nombre = self.abiertos.pop().expect("hoja");
            let _ = write!(self.salida, "</{nombre}>");
        } else {
            self.cerrar();
        }
    }

    pub fn terminar(self) -> String {
        assert!(self.abiertos.is_empty(), "quedan elementos abiertos");
        self.salida
    }
}

/// Escapa el texto como el `XmlWriter` de .NET.
fn escapar(texto: &str, atributo: bool) -> String {
    let mut salida = String::with_capacity(texto.len());
    let mut caracteres = texto.chars().peekable();
    while let Some(c) = caracteres.next() {
        match c {
            '&' => salida.push_str("&amp;"),
            '<' => salida.push_str("&lt;"),
            '>' => salida.push_str("&gt;"),
            '"' if atributo => salida.push_str("&quot;"),
            '\r' if atributo => salida.push_str("&#xD;"),
            '\n' if atributo => salida.push_str("&#xA;"),
            '\t' if atributo => salida.push_str("&#x9;"),
            '\r' => {
                if caracteres.peek() == Some(&'\n') {
                    caracteres.next();
                }
                salida.push_str("\r\n");
            }
            '\n' => salida.push_str("\r\n"),
            _ => salida.push(c),
        }
    }
    salida
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn mismo_formato_que_net() {
        let mut w = Escritor::new();
        w.crudo("<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n\r\n");
        w.abrir("Raiz", &[]);
        w.hoja("A", &[], "1 & 2");
        w.hoja("B", &[], "");
        w.abrir("Vacio", &[]);
        w.cerrar();
        w.abrir("Lleno", &[]);
        w.hoja("C", &[("Trigger", "5")], "x");
        w.cerrar();
        w.cerrar();
        assert_eq!(
            w.terminar(),
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n\r\n<Raiz>\r\n  <A>1 &amp; 2</A>\r\n  <B />\r\n  <Vacio />\r\n  <Lleno>\r\n    <C Trigger=\"5\">x</C>\r\n  </Lleno>\r\n</Raiz>"
        );
    }
}
