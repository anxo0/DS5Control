//! Una entrada de `Auto Profiles.xml` preparada para buscar, como `AutoProfileEntity`.

use ds5_perfiles::otros::Autoperfil;

/// El perfil que significa "este mando no cambia".
pub const NINGUNO: &str = "(none)";

/// Pasa a minúsculas carácter a carácter, como `ToLower` de C#: cada carácter da otro, sin
/// mirar el contexto (la sigma final sigue siendo `σ`). Si la minúscula de un carácter son varios
/// caracteres, se queda como está.
pub(crate) fn minusculas(texto: &str) -> String {
    texto
        .chars()
        .map(|c| {
            let mut m = c.to_lowercase();
            match (m.next(), m.next()) {
                (Some(a), None) => a,
                _ => c,
            }
        })
        .collect()
}

/// La ruta del proceso tal como se compara: en minúsculas y con `\` en vez de `/`.
pub fn normalizar_ruta(ruta: &str) -> String {
    minusculas(&ruta.replace('/', "\\"))
}

/// El título de la ventana tal como se compara: en minúsculas.
pub fn normalizar_titulo(titulo: &str) -> String {
    minusculas(titulo)
}

/// Un texto de búsqueda con su comodín: `^` delante (empieza por), `$` detrás (acaba en) o `*`
/// delante (contiene). Sin comodín tiene que ser igual.
#[derive(Debug, Clone, Default, PartialEq)]
struct Patron {
    /// Tal cual viene del archivo: de aquí se miran los comodines.
    original: String,
    /// En minúsculas y sin el comodín.
    buscado: String,
}

impl Patron {
    /// Como `SetSearchPath` y `SetSearchTitle`: solo se quita un comodín y solo si el texto
    /// tiene al menos dos caracteres. `^` gana a `$` y `$` a `*`.
    fn nuevo(texto: &str, es_ruta: bool) -> Patron {
        if texto.is_empty() {
            return Patron::default();
        }
        let mut buscado = if es_ruta {
            minusculas(texto).replace('/', "\\")
        } else {
            minusculas(texto)
        };
        if texto.chars().count() >= 2 {
            if texto.starts_with('^') || (!texto.ends_with('$') && texto.starts_with('*')) {
                buscado.remove(0);
            } else if texto.ends_with('$') {
                buscado.pop();
            }
        }
        Patron {
            original: texto.to_string(),
            buscado,
        }
    }

    /// Como `IsMatch`: vale cualquiera de las condiciones, así que `^abc$` busca lo que empieza
    /// por `abc$` (solo se quitó el `^`).
    fn casa(&self, texto: &str) -> bool {
        if self.buscado.is_empty() {
            return true;
        }
        let o = &self.original;
        let b = self.buscado.as_str();
        b == texto
            || (o.starts_with('^') && texto.starts_with(b))
            || (o.ends_with('$') && texto.ends_with(b))
            || (o.starts_with('*') && texto.contains(b))
    }
}

/// Una regla de autoperfil lista para comparar.
#[derive(Debug, Clone, PartialEq)]
pub struct Regla {
    ruta: Patron,
    titulo: Patron,
    /// El perfil de cada mando: vacío o [`NINGUNO`] si no cambia.
    pub perfiles: [String; 8],
    /// Apaga DS5Control mientras esta ventana está delante.
    pub apagar: bool,
}

impl Regla {
    pub fn nueva(ruta: &str, titulo: &str) -> Regla {
        Regla {
            ruta: Patron::nuevo(ruta, true),
            titulo: Patron::nuevo(titulo, false),
            perfiles: Default::default(),
            apagar: false,
        }
    }

    pub fn desde(p: &Autoperfil) -> Regla {
        Regla {
            perfiles: p.perfiles.clone(),
            apagar: p.apagar,
            ..Regla::nueva(&p.ruta, &p.titulo)
        }
    }

    /// La ruta tal cual viene del archivo.
    pub fn ruta(&self) -> &str {
        &self.ruta.original
    }

    /// El título tal cual viene del archivo.
    pub fn titulo(&self) -> &str {
        &self.titulo.original
    }

    /// Si la regla vale para esta ventana. La ruta y el título tienen que venir ya normalizados
    /// ([`normalizar_ruta`] y [`normalizar_titulo`]). Si la regla tiene ruta y título, tienen
    /// que casar los dos; lo que esté vacío casa con todo (una regla sin ruta ni título vale
    /// para cualquier ventana).
    pub fn casa(&self, ruta: &str, titulo: &str) -> bool {
        self.ruta.casa(ruta) && self.titulo.casa(titulo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn casa(ruta: &str, titulo: &str, ruta_ventana: &str, titulo_ventana: &str) -> bool {
        Regla::nueva(ruta, titulo).casa(
            &normalizar_ruta(ruta_ventana),
            &normalizar_titulo(titulo_ventana),
        )
    }

    #[test]
    fn ruta_exacta_sin_mayusculas_y_con_barras() {
        assert!(casa(
            r"C:\Juegos\Juego.exe",
            "",
            r"c:\juegos\juego.exe",
            "x"
        ));
        assert!(casa(
            r"c:\juegos\juego.exe",
            "",
            r"C:\JUEGOS\JUEGO.EXE",
            "x"
        ));
        assert!(casa("C:/Juegos/Juego.exe", "", r"C:\Juegos\Juego.exe", "x"));
        assert!(casa(r"C:\Juegos\Juego.exe", "", "C:/Juegos/Juego.exe", "x"));
        assert!(!casa(
            r"C:\Juegos\Juego.exe",
            "",
            r"C:\Juegos\Juego2.exe",
            "x"
        ));
        // Sin comodín no basta con que lo contenga.
        assert!(!casa("juego.exe", "", r"c:\juegos\juego.exe", "x"));
    }

    #[test]
    fn comodines_de_la_ruta() {
        assert!(casa(r"^C:\Juegos", "", r"c:\juegos\juego.exe", ""));
        assert!(!casa(r"^Juegos", "", r"c:\juegos\juego.exe", ""));
        assert!(casa(r"\Juego.exe$", "", r"d:\otra\juego.exe", ""));
        assert!(!casa(r"\Juego$", "", r"d:\otra\juego.exe", ""));
        assert!(casa("*juegos", "", r"c:\mis juegos\a.exe", ""));
        assert!(!casa("*steam", "", r"c:\mis juegos\a.exe", ""));
    }

    #[test]
    fn solo_se_quita_un_comodin() {
        // `^abc$`: se quita el `^` y se busca lo que empiece por `abc$`... o acabe en `abc$`.
        assert!(!casa("^abc$", "", "abc", ""));
        assert!(casa("^abc$", "", "abc$", ""));
        assert!(casa("^abc$", "", "abc$def", ""));
        assert!(casa("^abc$", "", "xabc$", ""));
        // `*abc$`: gana el `$`, que se quita, y el `*` se queda como letra. Además casa por
        // "contiene" con el `*` delante, porque el texto original empieza por `*`.
        assert!(!casa("*abc$", "", "xxabc", ""));
        assert!(casa("*abc$", "", "x*abc", ""));
        assert!(casa("*abc$", "", "x*abcx", ""));
    }

    #[test]
    fn comodin_solo_no_se_quita() {
        // Con un solo carácter no se quita nada: `*` busca un asterisco.
        assert!(!casa("*", "", "c:\\a.exe", ""));
        assert!(casa("*", "", "c:\\a*b.exe", ""));
        assert!(casa("^", "", "^a", ""));
        assert!(casa("$", "", "a$", ""));
    }

    #[test]
    fn titulo_con_y_sin_comodines() {
        assert!(casa("", "Mi Juego", "c:\\x.exe", "mi juego"));
        assert!(!casa("", "Mi Juego", "c:\\x.exe", "mi juego 2"));
        assert!(casa("", "^Mi", "c:\\x.exe", "MI JUEGO"));
        assert!(casa("", "Juego$", "c:\\x.exe", "Mi juego"));
        assert!(casa("", "*jue", "c:\\x.exe", "Mi juego"));
        // En el título `/` no se cambia por `\`.
        assert!(!casa("", "a/b", "", "a\\b"));
        assert!(casa("", "a/b", "", "A/B"));
    }

    #[test]
    fn ruta_y_titulo_tienen_que_casar_los_dos() {
        assert!(casa(
            "*juego.exe",
            "*menu",
            r"c:\juego.exe",
            "Menú principal - menu"
        ));
        assert!(!casa("*juego.exe", "*menu", r"c:\juego.exe", "Partida"));
        assert!(!casa("*otro.exe", "*menu", r"c:\juego.exe", "menu"));
    }

    #[test]
    fn regla_vacia_casa_con_todo() {
        assert!(casa("", "", r"c:\lo-que-sea.exe", "cualquier cosa"));
        assert!(casa("", "", "", ""));
    }

    #[test]
    fn minusculas_caracter_a_caracter() {
        assert_eq!(minusculas("ÁÉÑ Ü"), "áéñ ü");
        // La sigma final sigue siendo `σ`, como en C#.
        assert_eq!(minusculas("ΟΔΟΣ"), "οδοσ");
        assert_eq!(normalizar_ruta("C:/A/B.EXE"), r"c:\a\b.exe");
    }

    #[test]
    fn desde_el_archivo() {
        let mut p = Autoperfil {
            ruta: r"^C:\Juegos".into(),
            titulo: String::new(),
            apagar: true,
            ..Default::default()
        };
        p.perfiles[0] = "Carreras".into();
        let r = Regla::desde(&p);
        assert_eq!(r.ruta(), r"^C:\Juegos");
        assert_eq!(r.titulo(), "");
        assert!(r.apagar);
        assert_eq!(r.perfiles[0], "Carreras");
        assert!(r.casa(r"c:\juegos\a.exe", "x"));
    }
}
