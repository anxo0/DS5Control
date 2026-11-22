//! Números de versión como `System.Version` de .NET: de dos a cuatro partes separadas por
//! puntos, y la comparación que hace la versión en C# (1.2 y 1.2.0 son la misma).

use std::fmt;

/// Una versión de dos a cuatro partes. Las partes que faltan se guardan como `None`, igual que
/// `System.Version` las guarda como -1, para escribirla luego tal como se leyó.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    pub mayor: u32,
    pub menor: u32,
    pub compilacion: Option<u32>,
    pub revision: Option<u32>,
}

impl Version {
    /// Una versión de tres partes, como `new Version(mayor, menor, compilacion)`.
    pub const fn nueva(mayor: u32, menor: u32, compilacion: u32) -> Self {
        Version {
            mayor,
            menor,
            compilacion: Some(compilacion),
            revision: None,
        }
    }

    /// `Version.TryParse`: de dos a cuatro números separados por puntos. Cada número se lee como
    /// `int.TryParse` con `NumberStyles.Integer` (espacios alrededor y signo permitidos) y no
    /// puede ser negativo.
    pub fn analizar(texto: &str) -> Option<Version> {
        let partes: Vec<&str> = texto.split('.').collect();
        if !(2..=4).contains(&partes.len()) {
            return None;
        }
        let mut numeros = [0u32; 4];
        for (numero, parte) in numeros.iter_mut().zip(&partes) {
            *numero = componente(parte)?;
        }
        Some(Version {
            mayor: numeros[0],
            menor: numeros[1],
            compilacion: (partes.len() > 2).then_some(numeros[2]),
            revision: (partes.len() > 3).then_some(numeros[3]),
        })
    }

    /// La versión de tres partes con la que se compara: 1.2 pasa a 1.2.0 y la revisión no
    /// cuenta, como `Normalize` en C#.
    pub fn normalizada(&self) -> (u32, u32, u32) {
        (self.mayor, self.menor, self.compilacion.unwrap_or(0))
    }

    /// Si es posterior a `otra`, comparando las versiones normalizadas.
    pub fn es_posterior_a(&self, otra: &Version) -> bool {
        self.normalizada() > otra.normalizada()
    }

    /// Si es la misma versión que `otra`, comparando las versiones normalizadas.
    pub fn es_la_misma_que(&self, otra: &Version) -> bool {
        self.normalizada() == otra.normalizada()
    }

    /// Las tres primeras partes, como `ToString(3)`, que es como se enseña al usuario.
    /// `ToString(3)` falla si no hay tercera parte; aquí se escribe un 0.
    pub fn texto_corto(&self) -> String {
        format!(
            "{}.{}.{}",
            self.mayor,
            self.menor,
            self.compilacion.unwrap_or(0)
        )
    }
}

/// Como `Version.ToString()`: solo las partes que tiene.
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.mayor, self.menor)?;
        if let Some(compilacion) = self.compilacion {
            write!(f, ".{compilacion}")?;
            if let Some(revision) = self.revision {
                write!(f, ".{revision}")?;
            }
        }
        Ok(())
    }
}

/// Los espacios que admite `NumberStyles.AllowLeadingWhite` y `AllowTrailingWhite`.
fn es_espacio_numero(c: u8) -> bool {
    matches!(c, 0x09..=0x0D | b' ')
}

/// Una parte de la versión: `int.TryParse` con `NumberStyles.Integer` y la cultura invariable,
/// y que no sea negativa. Como en .NET, tras los espacios finales se admiten caracteres nulos.
fn componente(texto: &str) -> Option<u32> {
    let bytes = texto.as_bytes();
    let mut i = 0;
    while i < bytes.len() && es_espacio_numero(bytes[i]) {
        i += 1;
    }
    let negativo = match bytes.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    let inicio = i;
    let mut valor: u64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        // Se para justo por encima del mayor valor posible para no desbordar con cifras de más.
        valor = (valor * 10 + u64::from(bytes[i] - b'0')).min(u64::from(u32::MAX));
        i += 1;
    }
    if i == inicio {
        return None;
    }
    while i < bytes.len() && es_espacio_numero(bytes[i]) {
        i += 1;
    }
    if !bytes[i..].iter().all(|&b| b == 0) {
        return None;
    }
    if negativo {
        // -0 es un entero válido y no es negativo; cualquier otro negativo no vale.
        return (valor == 0).then_some(0);
    }
    if valor > i32::MAX as u64 {
        return None;
    }
    Some(valor as u32)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn v(texto: &str) -> Option<Version> {
        Version::analizar(texto)
    }

    #[test]
    fn lee_de_dos_a_cuatro_partes() {
        assert_eq!(
            v("0.1"),
            Some(Version {
                mayor: 0,
                menor: 1,
                compilacion: None,
                revision: None
            })
        );
        assert_eq!(v("0.2.0"), Some(Version::nueva(0, 2, 0)));
        assert_eq!(
            v("1.2.3.4"),
            Some(Version {
                mayor: 1,
                menor: 2,
                compilacion: Some(3),
                revision: Some(4)
            })
        );
        assert_eq!(v("1"), None);
        assert_eq!(v("1.2.3.4.5"), None);
        assert_eq!(v(""), None);
        assert_eq!(v("1."), None);
        assert_eq!(v(".1"), None);
        assert_eq!(v("no"), None);
    }

    #[test]
    fn cada_parte_se_lee_como_int_try_parse() {
        assert_eq!(v(" 1 . +2 "), Some(Version::analizar("1.2").unwrap()));
        assert_eq!(v("-0.1"), v("0.1"));
        assert_eq!(v("-1.0"), None);
        assert_eq!(v("1.2\0\0"), v("1.2"));
        assert_eq!(v("1.2 \0"), v("1.2"));
        assert_eq!(v("1.2\0 "), None);
        assert_eq!(v("1.+"), None);
        assert_eq!(v("1.2a"), None);
        assert_eq!(
            v("1.2147483647"),
            Some(Version::analizar("1.2147483647").unwrap())
        );
        assert_eq!(v("1.2147483648"), None);
        assert_eq!(v("1.99999999999999999999999"), None);
        assert_eq!(v("1.0000000000000000000002").unwrap().menor, 2);
        assert_eq!(v("1.\u{a0}2"), None);
    }

    #[test]
    fn compara_sin_importar_las_partes_que_faltan() {
        let corta = v("1.2").unwrap();
        let larga = v("1.2.0.7").unwrap();
        assert!(corta.es_la_misma_que(&larga));
        assert!(!corta.es_posterior_a(&larga));
        assert!(v("1.2.1").unwrap().es_posterior_a(&corta));
        assert!(v("1.10").unwrap().es_posterior_a(&v("1.9.9").unwrap()));
    }

    #[test]
    fn se_escribe_como_net() {
        assert_eq!(v("0.2").unwrap().to_string(), "0.2");
        assert_eq!(v("0.2.8").unwrap().to_string(), "0.2.8");
        assert_eq!(v("0.2.8.0").unwrap().to_string(), "0.2.8.0");
        assert_eq!(v("0.2.8.0").unwrap().texto_corto(), "0.2.8");
        assert_eq!(v("0.3").unwrap().texto_corto(), "0.3.0");
    }
}
