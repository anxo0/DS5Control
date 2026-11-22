//! Qué hay que hacer con cada ventana, como `AutoProfileChecker.Process`.

use ds5_perfiles::Autoperfiles;

use crate::observador::{Escritorio, Observador};
use crate::regla::{Regla, NINGUNO};

/// Los mandos que guarda cada regla.
pub const MAX_MANDOS: usize = 8;

/// Cómo está un mando antes de decidir.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EstadoMando {
    /// El nombre de su perfil normal (`ProfilePath` en C#).
    pub perfil_defecto: String,
    /// El perfil temporal que tiene cargado, si tiene uno (`tempprofilename` cuando
    /// `useTempProfile` es verdadero).
    pub perfil_temporal: Option<String>,
}

/// Cómo está la app antes de decidir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstadoApp<'a> {
    /// Si el servicio de los mandos está en marcha (`rootHub.running`).
    pub servicio_activo: bool,
    /// El ajuste `AutoProfileRevertDefaultProfile` (por defecto verdadero): al salir de un
    /// programa con autoperfil se vuelve al perfil normal.
    pub volver_al_defecto: bool,
    /// Los mandos que se miran (`CURRENT_DS5_CONTROLLER_LIMIT`: 8, o 4 en Windows 7). Se usan
    /// como mucho [`MAX_MANDOS`].
    pub mandos: &'a [EstadoMando],
}

/// Lo que la app tiene que hacer. Van en orden y hay que hacerlas en ese orden: tras encender o
/// apagar el servicio hay que esperar a que lo esté (como mucho [`crate::ESPERA_SERVICIO`])
/// antes de seguir, porque un perfil cargado con el servicio parado no hace nada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Peticion {
    /// Encender el servicio (lo había apagado una regla con "apagar").
    EncenderServicio,
    /// Apagar el servicio mientras la ventana esté delante.
    ApagarServicio,
    /// Cargar un perfil temporal en el mando (`LoadTempProfile`). Si el aviso de cambio está
    /// activado, se avisa con este nombre.
    CargarTemporal { mando: usize, perfil: String },
    /// Volver al perfil normal del mando (`LoadProfile`). El aviso dice `default`.
    VolverAlDefecto { mando: usize },
}

impl Peticion {
    /// El mando y el nombre que muestra el aviso `UsingAutoTempProfile` ("Controller {0} is using
    /// temp Profile "{1}""), contando los mandos desde 1, si la petición lleva aviso.
    pub fn aviso(&self) -> Option<(usize, &str)> {
        match self {
            Peticion::CargarTemporal { mando, perfil } => Some((mando + 1, perfil)),
            Peticion::VolverAlDefecto { mando } => Some((mando + 1, "default")),
            _ => None,
        }
    }
}

/// Lo que ha salido de una comprobación.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Resultado {
    /// La regla que ha casado (su posición en el archivo, desde 0), si alguna.
    pub regla: Option<usize>,
    pub peticiones: Vec<Peticion>,
}

/// Vigila la ventana de delante y decide los cambios de perfil.
#[derive(Debug, Clone, Default)]
pub struct Vigilante {
    reglas: Vec<Regla>,
    observador: Observador,
    /// Una regla con "apagar" ha apagado el servicio (`turnOffTemp`).
    apagado_temporal: bool,
    /// La última ventana casó con una regla (`tempAutoProfile != null`).
    con_autoperfil: bool,
}

impl Vigilante {
    pub fn new(archivo: &Autoperfiles) -> Vigilante {
        let mut v = Vigilante::default();
        v.cambiar_reglas(archivo);
        v
    }

    /// Cambia las reglas cuando cambia el archivo. Lo demás se conserva, como en C#, donde la
    /// lista se cambia sin tocar el resto: una regla nueva para la ventana que ya está delante no
    /// se aplica hasta que cambie la ventana o su título.
    pub fn cambiar_reglas(&mut self, archivo: &Autoperfiles) {
        self.reglas = archivo.programas.iter().map(Regla::desde).collect();
    }

    pub fn reglas(&self) -> &[Regla] {
        &self.reglas
    }

    /// Si hay que sondear: en C# el temporizador solo va mientras haya alguna regla.
    pub fn vigilando(&self) -> bool {
        !self.reglas.is_empty()
    }

    /// Una comprobación del temporizador: mira la ventana y, si ha cambiado, decide.
    pub fn sondear(&mut self, escritorio: &mut impl Escritorio, estado: &EstadoApp) -> Resultado {
        match self.observador.mirar(escritorio) {
            Some(v) => self.decidir(&v.ruta, &v.titulo, estado),
            None => Resultado::default(),
        }
    }

    /// Decide para una ventana que acaba de cambiar. La ruta y el título tienen que venir
    /// normalizados (ver [`crate::normalizar_ruta`] y [`crate::normalizar_titulo`]).
    pub fn decidir(&mut self, ruta: &str, titulo: &str, estado: &EstadoApp) -> Resultado {
        let mut peticiones = Vec::new();
        // La primera que case, en el orden del archivo.
        let regla = self.reglas.iter().position(|r| r.casa(ruta, titulo));
        let mandos = &estado.mandos[..estado.mandos.len().min(MAX_MANDOS)];

        if let Some(i) = regla {
            let r = &self.reglas[i];
            let mut forzar = false;
            if !r.apagar && self.apagado_temporal {
                // Otra regla había apagado el servicio: se enciende antes de cargar y se cargan
                // todos los perfiles aunque ya estén, para que valgan.
                self.apagado_temporal = false;
                if !estado.servicio_activo {
                    peticiones.push(Peticion::EncenderServicio);
                }
                forzar = true;
            }

            for (mando, m) in mandos.iter().enumerate() {
                let perfil = &r.perfiles[mando];
                if perfil.is_empty() || perfil == NINGUNO {
                    continue;
                }
                let distinto = match &m.perfil_temporal {
                    Some(t) => perfil != t,
                    None => *perfil != m.perfil_defecto,
                };
                if distinto || forzar {
                    peticiones.push(Peticion::CargarTemporal {
                        mando,
                        perfil: perfil.clone(),
                    });
                }
            }

            if r.apagar {
                self.apagado_temporal = true;
                if estado.servicio_activo {
                    peticiones.push(Peticion::ApagarServicio);
                }
            }
            self.con_autoperfil = true;
        } else if self.con_autoperfil {
            // Se ha salido del programa con autoperfil.
            if self.apagado_temporal && estado.volver_al_defecto {
                self.apagado_temporal = false;
                if !estado.servicio_activo {
                    peticiones.push(Peticion::EncenderServicio);
                }
            }
            self.con_autoperfil = false;
            if estado.volver_al_defecto {
                for (mando, m) in mandos.iter().enumerate() {
                    if m.perfil_temporal.is_some() {
                        peticiones.push(Peticion::VolverAlDefecto { mando });
                    }
                }
            }
        }

        Resultado { regla, peticiones }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observador::pruebas::Falso;
    use crate::regla::{normalizar_ruta, normalizar_titulo};
    use ds5_perfiles::otros::Autoperfil;
    use Peticion::*;

    fn regla(ruta: &str, titulo: &str, perfiles: &[&str], apagar: bool) -> Autoperfil {
        let mut p = Autoperfil {
            ruta: ruta.into(),
            titulo: titulo.into(),
            apagar,
            ..Default::default()
        };
        for (i, n) in perfiles.iter().enumerate() {
            p.perfiles[i] = n.to_string();
        }
        p
    }

    fn vigilante(reglas: Vec<Autoperfil>) -> Vigilante {
        Vigilante::new(&Autoperfiles { programas: reglas })
    }

    /// Una app de mentira que aplica las peticiones como lo haría C#.
    struct App {
        servicio: bool,
        volver: bool,
        mandos: Vec<EstadoMando>,
    }

    impl App {
        fn new(n: usize) -> App {
            App {
                servicio: true,
                volver: true,
                mandos: (0..n)
                    .map(|i| EstadoMando {
                        perfil_defecto: format!("Normal{}", i + 1),
                        perfil_temporal: None,
                    })
                    .collect(),
            }
        }

        fn ir(&mut self, v: &mut Vigilante, ruta: &str, titulo: &str) -> Resultado {
            let estado = EstadoApp {
                servicio_activo: self.servicio,
                volver_al_defecto: self.volver,
                mandos: &self.mandos,
            };
            let r = v.decidir(&normalizar_ruta(ruta), &normalizar_titulo(titulo), &estado);
            for p in &r.peticiones {
                match p {
                    EncenderServicio => self.servicio = true,
                    ApagarServicio => self.servicio = false,
                    CargarTemporal { mando, perfil } => {
                        self.mandos[*mando].perfil_temporal = Some(perfil.clone())
                    }
                    VolverAlDefecto { mando } => self.mandos[*mando].perfil_temporal = None,
                }
            }
            r
        }
    }

    fn cargar(mando: usize, perfil: &str) -> Peticion {
        CargarTemporal {
            mando,
            perfil: perfil.into(),
        }
    }

    #[test]
    fn carga_el_perfil_de_cada_mando() {
        let mut v = vigilante(vec![regla(
            "*juego.exe",
            "",
            &["Carreras", "(none)", "", "Lucha"],
            false,
        )]);
        let mut app = App::new(4);
        let r = app.ir(&mut v, r"C:\Juegos\Juego.exe", "Juego");
        assert_eq!(r.regla, Some(0));
        // `(none)` y vacío no cambian el mando.
        assert_eq!(
            r.peticiones,
            vec![cargar(0, "Carreras"), cargar(3, "Lucha")]
        );
    }

    #[test]
    fn no_recarga_lo_que_ya_esta() {
        let mut v = vigilante(vec![
            regla("*a.exe", "", &["Uno", "Normal2"], false),
            regla("*b.exe", "", &["Uno", "Dos"], false),
        ]);
        let mut app = App::new(2);
        // El mando 2 ya tiene `Normal2` como perfil normal: no se carga.
        assert_eq!(
            app.ir(&mut v, "a.exe", "").peticiones,
            vec![cargar(0, "Uno")]
        );
        // El mando 1 ya tiene `Uno` como temporal: no se vuelve a cargar.
        assert_eq!(
            app.ir(&mut v, "b.exe", "").peticiones,
            vec![cargar(1, "Dos")]
        );
        assert_eq!(app.ir(&mut v, "b.exe", "otro título").peticiones, vec![]);
    }

    #[test]
    fn con_temporal_se_compara_con_el_temporal_y_no_con_el_normal() {
        // Si el mando tiene un temporal y la regla pide su perfil normal, sí se carga (como
        // temporal), igual que en C#.
        let mut v = vigilante(vec![
            regla("*a.exe", "", &["Uno"], false),
            regla("*b.exe", "", &["Normal1"], false),
        ]);
        let mut app = App::new(1);
        app.ir(&mut v, "a.exe", "");
        assert_eq!(
            app.ir(&mut v, "b.exe", "").peticiones,
            vec![cargar(0, "Normal1")]
        );
    }

    #[test]
    fn gana_la_primera_regla_que_casa() {
        let mut v = vigilante(vec![
            regla("*juego", "", &["General"], false),
            regla(r"c:\juego.exe", "", &["Exacto"], false),
        ]);
        let mut app = App::new(1);
        let r = app.ir(&mut v, r"C:\juego.exe", "");
        assert_eq!(r.regla, Some(0));
        assert_eq!(r.peticiones, vec![cargar(0, "General")]);
    }

    #[test]
    fn mayusculas_en_la_regla_y_en_la_ventana() {
        let mut v = vigilante(vec![regla(r"C:\JUEGOS\Juego.EXE", "^MENÚ", &["P"], false)]);
        let mut app = App::new(1);
        assert_eq!(
            app.ir(&mut v, r"c:\juegos\juego.exe", "Menú principal")
                .regla,
            Some(0)
        );
    }

    #[test]
    fn vuelve_al_defecto_al_salir() {
        let mut v = vigilante(vec![regla("*a.exe", "", &["Uno", "", "Tres"], false)]);
        let mut app = App::new(3);
        app.ir(&mut v, "a.exe", "");
        let r = app.ir(&mut v, "explorer.exe", "");
        assert_eq!(r.regla, None);
        // Solo los mandos con temporal vuelven.
        assert_eq!(
            r.peticiones,
            vec![VolverAlDefecto { mando: 0 }, VolverAlDefecto { mando: 2 }]
        );
        // Ya fuera, otra ventana sin regla no hace nada.
        assert_eq!(app.ir(&mut v, "otro.exe", "").peticiones, vec![]);
    }

    #[test]
    fn sin_autoperfil_antes_no_se_toca_un_temporal_ajeno() {
        // Un temporal cargado por otra cosa (una acción especial) no se quita si la ventana
        // anterior no tenía autoperfil.
        let mut v = vigilante(vec![regla("*a.exe", "", &["Uno"], false)]);
        let mut app = App::new(1);
        app.mandos[0].perfil_temporal = Some("DeUnaAccion".into());
        assert_eq!(app.ir(&mut v, "otro.exe", "").peticiones, vec![]);
    }

    #[test]
    fn sin_volver_al_defecto_se_queda_el_perfil() {
        let mut v = vigilante(vec![regla("*a.exe", "", &["Uno"], false)]);
        let mut app = App::new(1);
        app.volver = false;
        app.ir(&mut v, "a.exe", "");
        assert_eq!(app.ir(&mut v, "b.exe", "").peticiones, vec![]);
        assert_eq!(app.mandos[0].perfil_temporal.as_deref(), Some("Uno"));
        // Ya no cuenta como "con autoperfil": al volver a entrar no se recarga lo que hay.
        assert_eq!(app.ir(&mut v, "a.exe", "").peticiones, vec![]);
    }

    #[test]
    fn apagar_y_volver_a_encender() {
        let mut v = vigilante(vec![
            regla("*apaga.exe", "", &[], true),
            regla("*juego.exe", "", &["Uno"], false),
        ]);
        let mut app = App::new(1);
        assert_eq!(
            app.ir(&mut v, "apaga.exe", "").peticiones,
            vec![ApagarServicio]
        );
        assert!(!app.servicio);
        // Ya apagado: no se vuelve a pedir.
        assert_eq!(app.ir(&mut v, "apaga.exe", "otro").peticiones, vec![]);
        // Otra regla sin apagar: se enciende primero y luego se carga.
        assert_eq!(
            app.ir(&mut v, "juego.exe", "").peticiones,
            vec![EncenderServicio, cargar(0, "Uno")]
        );
    }

    #[test]
    fn tras_apagar_se_fuerzan_los_perfiles() {
        let mut v = vigilante(vec![
            regla("*juego.exe", "", &["Uno"], false),
            regla("*apaga.exe", "", &["Uno"], true),
        ]);
        let mut app = App::new(1);
        assert_eq!(
            app.ir(&mut v, "juego.exe", "").peticiones,
            vec![cargar(0, "Uno")]
        );
        // La regla que apaga carga sus perfiles (si son otros) y después apaga.
        assert_eq!(
            app.ir(&mut v, "apaga.exe", "").peticiones,
            vec![ApagarServicio]
        );
        // Al volver, `Uno` ya estaba, pero se carga igual porque el servicio estuvo parado.
        assert_eq!(
            app.ir(&mut v, "juego.exe", "").peticiones,
            vec![EncenderServicio, cargar(0, "Uno")]
        );
    }

    #[test]
    fn apagar_con_servicio_ya_parado_no_pide_nada_pero_lo_recuerda() {
        let mut v = vigilante(vec![
            regla("*apaga.exe", "", &[], true),
            regla("*juego.exe", "", &["Uno"], false),
        ]);
        let mut app = App::new(1);
        app.servicio = false;
        assert_eq!(app.ir(&mut v, "apaga.exe", "").peticiones, vec![]);
        // Como C# lo recuerda, la siguiente regla enciende el servicio y fuerza la carga.
        assert_eq!(
            app.ir(&mut v, "juego.exe", "").peticiones,
            vec![EncenderServicio, cargar(0, "Uno")]
        );
    }

    #[test]
    fn al_salir_de_una_regla_que_apaga_se_enciende() {
        let mut v = vigilante(vec![regla("*apaga.exe", "", &["Uno"], true)]);
        let mut app = App::new(1);
        assert_eq!(
            app.ir(&mut v, "apaga.exe", "").peticiones,
            vec![cargar(0, "Uno"), ApagarServicio]
        );
        assert_eq!(
            app.ir(&mut v, "explorer.exe", "").peticiones,
            vec![EncenderServicio, VolverAlDefecto { mando: 0 }]
        );
        assert!(app.servicio);
    }

    #[test]
    fn sin_volver_al_defecto_el_servicio_se_queda_apagado() {
        // Rareza de C#: sin volver al defecto, salir de la regla que apaga no enciende.
        let mut v = vigilante(vec![
            regla("*apaga.exe", "", &[], true),
            regla("*juego.exe", "", &["Uno"], false),
        ]);
        let mut app = App::new(1);
        app.volver = false;
        app.ir(&mut v, "apaga.exe", "");
        assert_eq!(app.ir(&mut v, "explorer.exe", "").peticiones, vec![]);
        assert!(!app.servicio);
        // La siguiente regla sin apagar sí lo enciende.
        assert_eq!(
            app.ir(&mut v, "juego.exe", "").peticiones,
            vec![EncenderServicio, cargar(0, "Uno")]
        );
    }

    #[test]
    fn solo_se_miran_los_mandos_que_hay() {
        let mut v = vigilante(vec![regla(
            "*a.exe",
            "",
            &["1", "2", "3", "4", "5", "6", "7", "8"],
            false,
        )]);
        // Windows 7: cuatro mandos.
        let mut app = App::new(4);
        assert_eq!(app.ir(&mut v, "a.exe", "").peticiones.len(), 4);
        // Más de ocho no se miran.
        let mut v = vigilante(vec![regla("*a.exe", "", &["1"; 8], false)]);
        let mut app = App::new(10);
        assert_eq!(app.ir(&mut v, "a.exe", "").peticiones.len(), 8);
    }

    #[test]
    fn regla_vacia_casa_con_cualquier_ventana() {
        let mut v = vigilante(vec![regla("", "", &["Todo"], false)]);
        let mut app = App::new(1);
        assert_eq!(
            app.ir(&mut v, "lo-que-sea.exe", "").peticiones,
            vec![cargar(0, "Todo")]
        );
    }

    #[test]
    fn vigilando_solo_con_reglas() {
        let mut v = vigilante(vec![]);
        assert!(!v.vigilando());
        v.cambiar_reglas(&Autoperfiles {
            programas: vec![regla("a", "", &[], false)],
        });
        assert!(v.vigilando());
        assert_eq!(v.reglas().len(), 1);
    }

    #[test]
    fn cambiar_reglas_conserva_el_estado() {
        let mut v = vigilante(vec![regla("*a.exe", "", &["Uno"], false)]);
        let mut app = App::new(1);
        app.ir(&mut v, "a.exe", "");
        v.cambiar_reglas(&Autoperfiles {
            programas: vec![regla("*c.exe", "", &["Tres"], false)],
        });
        // Sigue sabiendo que venía de un autoperfil.
        assert_eq!(
            app.ir(&mut v, "b.exe", "").peticiones,
            vec![VolverAlDefecto { mando: 0 }]
        );
    }

    #[test]
    fn sondear_solo_decide_cuando_cambia_la_ventana() {
        let mut v = vigilante(vec![regla("*a.exe", "", &["Uno"], false)]);
        let mut e = Falso::default();
        let mandos = vec![EstadoMando {
            perfil_defecto: "Normal".into(),
            perfil_temporal: None,
        }];
        let estado = EstadoApp {
            servicio_activo: true,
            volver_al_defecto: true,
            mandos: &mandos,
        };
        e.poner(1, 10, r"C:\A.EXE", "A");
        let r = v.sondear(&mut e, &estado);
        assert_eq!(r.peticiones, vec![cargar(0, "Uno")]);
        // La misma ventana: aunque el estado diga que no tiene el perfil, no se decide nada.
        assert_eq!(v.sondear(&mut e, &estado), Resultado::default());
        e.delante = None;
        assert_eq!(v.sondear(&mut e, &estado), Resultado::default());
    }

    #[test]
    fn avisos() {
        assert_eq!(cargar(0, "Uno").aviso(), Some((1, "Uno")));
        assert_eq!(VolverAlDefecto { mando: 2 }.aviso(), Some((3, "default")));
        assert_eq!(ApagarServicio.aviso(), None);
        assert_eq!(EncenderServicio.aviso(), None);
    }

    #[test]
    fn desde_el_xml() {
        // Un `Auto Profiles.xml` escrito aquí (no se lee ningún archivo del usuario).
        let xml = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<Programs>\r\n\
            <Program path=\"C:/Juegos/Juego.exe\" title=\"\"><Controller1>Carreras</Controller1>\
            <Controller2>(none)</Controller2><TurnOff>False</TurnOff></Program>\r\n\
            <Program path=\"*launcher\" title=\"\"><TurnOff>True</TurnOff></Program>\r\n\
            </Programs>";
        let mut v = Vigilante::new(&Autoperfiles::leer(xml).unwrap());
        let mut app = App::new(2);
        assert_eq!(
            app.ir(&mut v, r"C:\JUEGOS\juego.exe", "").peticiones,
            vec![cargar(0, "Carreras")]
        );
        assert_eq!(
            app.ir(&mut v, r"C:\Epic\Launcher.exe", "").peticiones,
            vec![ApagarServicio]
        );
    }
}
