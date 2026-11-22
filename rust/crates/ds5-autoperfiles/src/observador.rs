//! Qué ventana está en primer plano, con la misma caché que `GetTopWindowName`: solo se avisa
//! cuando cambia la ventana o su título.

use crate::regla::{normalizar_ruta, normalizar_titulo};

/// Lo que hace falta preguntarle al sistema. En Windows es [`crate::EscritorioWindows`]; en
/// las pruebas, uno de mentira.
pub trait Escritorio {
    /// La ventana en primer plano, o `None` si no hay ninguna.
    fn ventana_primer_plano(&mut self) -> Option<isize>;
    /// El proceso de la ventana (0 si no se sabe).
    fn proceso_de_ventana(&mut self, ventana: isize) -> u32;
    /// La ruta completa del ejecutable, o `None` si no se puede abrir el proceso.
    fn ruta_proceso(&mut self, proceso: u32) -> Option<String>;
    /// El título de la ventana (vacío si no tiene).
    fn titulo_ventana(&mut self, ventana: isize) -> String;
}

/// La ventana de delante, ya normalizada para comparar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ventana {
    /// La ruta del ejecutable en minúsculas y con `\`; vacía si no se pudo leer.
    pub ruta: String,
    /// El título en minúsculas.
    pub titulo: String,
}

/// Recuerda la última ventana para no volver a preguntar por el proceso si no ha cambiado.
#[derive(Debug, Clone, Default)]
pub struct Observador {
    ventana: isize,
    proceso: u32,
    ruta: String,
    titulo: String,
}

impl Observador {
    pub fn new() -> Observador {
        Observador::default()
    }

    /// Mira la ventana de delante. Devuelve `Some` solo si es otra ventana o si su título ha
    /// cambiado; si es la misma de antes, o no hay ventana, devuelve `None` y no hay nada que
    /// hacer.
    pub fn mirar(&mut self, escritorio: &mut impl Escritorio) -> Option<Ventana> {
        let ventana = match escritorio.ventana_primer_plano() {
            Some(v) if v != 0 => v,
            _ => {
                // Sin ventana se olvidan la ventana y el proceso, pero no la ruta ni el título.
                self.ventana = 0;
                self.proceso = 0;
                return None;
            }
        };

        if ventana == self.ventana {
            // La misma ventana: solo se mira si ha cambiado el título; la ruta es la de antes.
            let titulo = normalizar_titulo(&escritorio.titulo_ventana(ventana));
            if titulo == self.titulo {
                return None;
            }
            self.titulo = titulo;
            return Some(self.actual());
        }
        self.ventana = ventana;

        // Otra ventana del mismo proceso no vuelve a leer la ruta (tampoco con el proceso 0).
        let proceso = escritorio.proceso_de_ventana(ventana);
        if proceso != self.proceso {
            self.proceso = proceso;
            self.ruta = escritorio
                .ruta_proceso(proceso)
                .map(|r| normalizar_ruta(&r))
                .unwrap_or_default();
        }
        self.titulo = normalizar_titulo(&escritorio.titulo_ventana(ventana));
        Some(self.actual())
    }

    fn actual(&self) -> Ventana {
        Ventana {
            ruta: self.ruta.clone(),
            titulo: self.titulo.clone(),
        }
    }
}

#[cfg(test)]
pub(crate) mod pruebas {
    use super::*;
    use std::collections::HashMap;

    /// Un escritorio de mentira que cuenta cuántas veces se lee la ruta.
    #[derive(Default)]
    pub struct Falso {
        pub delante: Option<isize>,
        pub procesos: HashMap<isize, u32>,
        pub rutas: HashMap<u32, String>,
        pub titulos: HashMap<isize, String>,
        pub lecturas_ruta: usize,
    }

    impl Falso {
        pub fn poner(&mut self, ventana: isize, proceso: u32, ruta: &str, titulo: &str) {
            self.delante = Some(ventana);
            self.procesos.insert(ventana, proceso);
            self.rutas.insert(proceso, ruta.to_string());
            self.titulos.insert(ventana, titulo.to_string());
        }
    }

    impl Escritorio for Falso {
        fn ventana_primer_plano(&mut self) -> Option<isize> {
            self.delante
        }
        fn proceso_de_ventana(&mut self, ventana: isize) -> u32 {
            self.procesos.get(&ventana).copied().unwrap_or(0)
        }
        fn ruta_proceso(&mut self, proceso: u32) -> Option<String> {
            self.lecturas_ruta += 1;
            self.rutas.get(&proceso).cloned()
        }
        fn titulo_ventana(&mut self, ventana: isize) -> String {
            self.titulos.get(&ventana).cloned().unwrap_or_default()
        }
    }

    fn v(ruta: &str, titulo: &str) -> Option<Ventana> {
        Some(Ventana {
            ruta: ruta.into(),
            titulo: titulo.into(),
        })
    }

    #[test]
    fn avisa_solo_cuando_cambia() {
        let mut e = Falso::default();
        let mut o = Observador::new();
        e.poner(10, 1, "C:/Juegos/A.exe", "Juego A");
        assert_eq!(o.mirar(&mut e), v(r"c:\juegos\a.exe", "juego a"));
        // La misma ventana con el mismo título: nada.
        assert_eq!(o.mirar(&mut e), None);
        // Cambia el título: se avisa con la ruta de antes, sin volver a leerla.
        e.titulos.insert(10, "Juego A - Partida".into());
        assert_eq!(o.mirar(&mut e), v(r"c:\juegos\a.exe", "juego a - partida"));
        assert_eq!(e.lecturas_ruta, 1);
        // Cambiar solo de mayúsculas no es un cambio.
        e.titulos.insert(10, "JUEGO A - PARTIDA".into());
        assert_eq!(o.mirar(&mut e), None);
    }

    #[test]
    fn otra_ventana_del_mismo_proceso_no_relee_la_ruta() {
        let mut e = Falso::default();
        let mut o = Observador::new();
        e.poner(10, 1, r"C:\A.exe", "Uno");
        o.mirar(&mut e);
        e.poner(11, 1, r"C:\A.exe", "Uno");
        // Otra ventana avisa aunque el título sea el mismo.
        assert_eq!(o.mirar(&mut e), v(r"c:\a.exe", "uno"));
        assert_eq!(e.lecturas_ruta, 1);
        e.poner(12, 2, r"C:\B.exe", "Dos");
        assert_eq!(o.mirar(&mut e), v(r"c:\b.exe", "dos"));
        assert_eq!(e.lecturas_ruta, 2);
    }

    #[test]
    fn sin_ventana_no_hay_nada_y_se_olvida() {
        let mut e = Falso::default();
        let mut o = Observador::new();
        e.poner(10, 1, r"C:\A.exe", "Uno");
        o.mirar(&mut e);
        e.delante = None;
        assert_eq!(o.mirar(&mut e), None);
        // Al volver la misma ventana se vuelve a preguntar todo y se avisa.
        e.delante = Some(10);
        assert_eq!(o.mirar(&mut e), v(r"c:\a.exe", "uno"));
        assert_eq!(e.lecturas_ruta, 2);
        // La ventana 0 cuenta como ninguna.
        e.delante = Some(0);
        assert_eq!(o.mirar(&mut e), None);
    }

    #[test]
    fn proceso_que_no_se_puede_abrir_da_ruta_vacia() {
        let mut e = Falso::default();
        let mut o = Observador::new();
        e.delante = Some(5);
        e.procesos.insert(5, 7);
        e.titulos.insert(5, "Protegido".into());
        assert_eq!(o.mirar(&mut e), v("", "protegido"));
    }

    #[test]
    fn proceso_cero_al_empezar_no_lee_la_ruta() {
        // Como en C#: el proceso guardado empieza en 0, así que un proceso 0 usa la ruta
        // guardada (vacía) sin preguntar.
        let mut e = Falso::default();
        let mut o = Observador::new();
        e.delante = Some(5);
        e.titulos.insert(5, "Algo".into());
        assert_eq!(o.mirar(&mut e), v("", "algo"));
        assert_eq!(e.lecturas_ruta, 0);
    }
}
