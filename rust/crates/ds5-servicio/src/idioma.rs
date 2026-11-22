//! El idioma de los textos que la app manda ya escritos a la interfaz: los nombres y las ayudas
//! de las opciones y los mensajes del registro. Como en la interfaz (`ui/src/i18n`), sale del
//! ajuste "Idioma" (`UseLang` en `Profiles.xml`) y, en automático, del idioma de Windows:
//! español, inglés o portugués (también el de Brasil) y, si no es ninguno de los tres, inglés.
//!
//! Los textos se escriben en español en el código, como siempre, y se traducen al salir con
//! tablas de tres columnas: el español es la clave, así que en español todo queda igual. Lo que
//! no está en la tabla sale tal cual (nombres propios, números...).

use std::fmt::Display;

use ds5_perfiles::AjustesApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Idioma {
    Es,
    En,
    Pt,
}

/// Una tabla de textos: el español y su traducción al inglés y al portugués de Brasil. Los
/// huecos `{}` se rellenan en orden con [`rellenar`].
pub type Tabla = &'static [(&'static str, &'static str, &'static str)];

/// Los idiomas en el orden de la lista del ajuste, detrás de "Automático".
pub const IDIOMAS: [Idioma; 3] = [Idioma::Es, Idioma::En, Idioma::Pt];

impl Idioma {
    /// `es`, `en` o `pt`, lo que se guarda en `UseLang`.
    pub fn codigo(self) -> &'static str {
        match self {
            Idioma::Es => "es",
            Idioma::En => "en",
            Idioma::Pt => "pt",
        }
    }

    /// El nombre del idioma en su propio idioma.
    pub fn nombre(self) -> &'static str {
        match self {
            Idioma::Es => "Español",
            Idioma::En => "English",
            Idioma::Pt => "Português",
        }
    }

    /// El idioma de un código como `es`, `es-ES`, `pt-BR` o `en_US`; `None` si no es uno de los
    /// tres.
    pub fn de_codigo(codigo: &str) -> Option<Idioma> {
        let base = codigo.trim().split(['-', '_']).next().unwrap_or("");
        IDIOMAS
            .into_iter()
            .find(|i| base.eq_ignore_ascii_case(i.codigo()))
    }

    /// El idioma con el que se escriben los textos con estos ajustes.
    pub fn de_ajustes(a: &AjustesApp) -> Idioma {
        Eleccion::de_ajustes(a).efectivo
    }

    /// El idioma de Windows: el primero de los preferidos que sea uno de los tres, o inglés.
    pub fn del_sistema() -> Idioma {
        primero_conocido(&idiomas_del_sistema())
    }

    /// El texto en este idioma, buscándolo en las tablas por su español. Si no está, el propio
    /// texto.
    pub fn traducir<'a>(self, es: &'a str, tablas: &[Tabla]) -> &'a str {
        if self == Idioma::Es {
            return es;
        }
        tablas
            .iter()
            .find_map(|t| t.iter().find(|(clave, _, _)| *clave == es))
            .map_or(es, |&(_, en, pt)| match self {
                Idioma::En => en,
                _ => pt,
            })
    }

    /// Un texto del servicio (las opciones y sus avisos).
    pub fn texto(self, es: &str) -> &str {
        self.traducir(es, &[crate::textos::TEXTOS])
    }

    /// Un texto del servicio con sus huecos `{}` rellenos.
    pub fn formato(self, es: &str, valores: &[&dyn Display]) -> String {
        rellenar(self.texto(es), valores)
    }
}

/// Lo elegido en el ajuste y el idioma que sale de ello.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Eleccion {
    /// `None` es automático.
    pub elegido: Option<Idioma>,
    pub efectivo: Idioma,
}

impl Eleccion {
    /// Con el `UseLang` guardado y los idiomas preferidos de Windows.
    pub fn decidir(guardado: &str, sistema: &[String]) -> Eleccion {
        let elegido = Idioma::de_codigo(guardado);
        Eleccion {
            elegido,
            efectivo: elegido.unwrap_or_else(|| primero_conocido(sistema)),
        }
    }

    pub fn de_ajustes(a: &AjustesApp) -> Eleccion {
        match Idioma::de_codigo(&a.generales.idioma) {
            Some(i) => Eleccion {
                elegido: Some(i),
                efectivo: i,
            },
            None => Eleccion {
                elegido: None,
                efectivo: Idioma::del_sistema(),
            },
        }
    }

    /// Lo que va en el estado de la interfaz: el código elegido, o vacío si es automático.
    pub fn codigo(&self) -> &'static str {
        self.elegido.map_or("", Idioma::codigo)
    }
}

fn primero_conocido(codigos: &[String]) -> Idioma {
    codigos
        .iter()
        .find_map(|c| Idioma::de_codigo(c))
        .unwrap_or(Idioma::En)
}

/// Los idiomas de la interfaz de Windows por orden de preferencia (`es-ES`, `en-US`...).
#[cfg(windows)]
pub fn idiomas_del_sistema() -> Vec<String> {
    use windows::core::PWSTR;
    use windows::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};

    let (mut cuantos, mut largo) = (0u32, 0u32);
    if unsafe { GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut cuantos, None, &mut largo) }
        .is_err()
        || largo == 0
    {
        return Vec::new();
    }
    let mut bufer = vec![0u16; largo as usize];
    let leido = unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut cuantos,
            Some(PWSTR(bufer.as_mut_ptr())),
            &mut largo,
        )
    };
    if leido.is_err() {
        return Vec::new();
    }
    // Una lista de textos terminados en cero, con otro cero al final.
    bufer
        .split(|&c| c == 0)
        .filter(|t| !t.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

#[cfg(not(windows))]
pub fn idiomas_del_sistema() -> Vec<String> {
    std::env::var("LANG").map(|l| vec![l]).unwrap_or_default()
}

/// Cambia cada `{}` de la plantilla, en orden, por uno de los valores.
pub fn rellenar(plantilla: &str, valores: &[&dyn Display]) -> String {
    let mut salida = String::with_capacity(plantilla.len());
    let mut valores = valores.iter();
    let mut resto = plantilla;
    while let Some(i) = resto.find("{}") {
        salida.push_str(&resto[..i]);
        match valores.next() {
            Some(v) => salida.push_str(&v.to_string()),
            None => salida.push_str("{}"),
        }
        resto = &resto[i + 2..];
    }
    salida.push_str(resto);
    salida
}

/// Lo que está mal en una tabla: claves repetidas, traducciones vacías o con otros huecos, o
/// con la raya larga. Para las pruebas de cada tabla.
pub fn fallos_de_tabla(tabla: Tabla) -> Vec<String> {
    let huecos = |t: &str| t.matches("{}").count();
    let mut fallos = Vec::new();
    for (n, &(es, en, pt)) in tabla.iter().enumerate() {
        if tabla[..n].iter().any(|(otra, _, _)| *otra == es) {
            fallos.push(format!("repetido: {es}"));
        }
        for t in [es, en, pt] {
            if t.trim().is_empty() {
                fallos.push(format!("vacío en: {es}"));
            }
            if t.contains('\u{2014}') {
                fallos.push(format!("raya larga en: {t}"));
            }
            if huecos(t) != huecos(es) {
                fallos.push(format!("otros huecos en: {t}"));
            }
        }
    }
    fallos
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn lista(c: &[&str]) -> Vec<String> {
        c.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn los_codigos() {
        assert_eq!(Idioma::de_codigo("es"), Some(Idioma::Es));
        assert_eq!(Idioma::de_codigo("es-ES"), Some(Idioma::Es));
        assert_eq!(Idioma::de_codigo("PT_br"), Some(Idioma::Pt));
        assert_eq!(Idioma::de_codigo(" en-US "), Some(Idioma::En));
        assert_eq!(Idioma::de_codigo("de-DE"), None);
        assert_eq!(Idioma::de_codigo(""), None);
        assert_eq!(Idioma::de_codigo("espanol"), None);
        for i in IDIOMAS {
            assert_eq!(Idioma::de_codigo(i.codigo()), Some(i));
        }
    }

    #[test]
    fn como_se_decide() {
        let windows = lista(&["fr-FR", "pt-BR", "en-US"]);
        // Lo guardado manda.
        let e = Eleccion::decidir("es", &windows);
        assert_eq!(
            (e.elegido, e.efectivo, e.codigo()),
            (Some(Idioma::Es), Idioma::Es, "es")
        );
        // Vacío o desconocido: el primero de Windows que conozcamos.
        let e = Eleccion::decidir("", &windows);
        assert_eq!((e.elegido, e.efectivo, e.codigo()), (None, Idioma::Pt, ""));
        assert_eq!(Eleccion::decidir("de-DE", &windows).efectivo, Idioma::Pt);
        // Y si no, inglés.
        assert_eq!(
            Eleccion::decidir("", &lista(&["fr-FR", "de-DE"])).efectivo,
            Idioma::En
        );
        assert_eq!(Eleccion::decidir("", &[]).efectivo, Idioma::En);
    }

    #[test]
    fn traduce_con_las_tablas() {
        const T: Tabla = &[
            ("Hola", "Hello", "Olá"),
            ("Mando {}", "Controller {}", "Controle {}"),
        ];
        assert_eq!(Idioma::Es.traducir("Hola", &[T]), "Hola");
        assert_eq!(Idioma::En.traducir("Hola", &[T]), "Hello");
        assert_eq!(Idioma::Pt.traducir("Hola", &[T]), "Olá");
        assert_eq!(Idioma::En.traducir("DualSense", &[T]), "DualSense");
        assert_eq!(
            rellenar(Idioma::Pt.traducir("Mando {}", &[T]), &[&2]),
            "Controle 2"
        );
        assert_eq!(rellenar("{}:{} y {}", &[&"a", &1]), "a:1 y {}");
        assert!(fallos_de_tabla(T).is_empty());
        let mala: Tabla = &[("A {}", "B", "C {}"), ("A {}", "B {}", "")];
        // "B" sin hueco, la clave repetida y la traducción vacía, que además pierde el hueco.
        assert_eq!(fallos_de_tabla(mala).len(), 4);
    }

    #[test]
    fn la_tabla_del_servicio_esta_bien() {
        assert_eq!(fallos_de_tabla(crate::textos::TEXTOS), Vec::<String>::new());
    }

    #[test]
    fn el_idioma_de_windows_es_uno_de_los_tres() {
        // Solo que no falle: depende del PC.
        let _ = Idioma::del_sistema();
    }
}
