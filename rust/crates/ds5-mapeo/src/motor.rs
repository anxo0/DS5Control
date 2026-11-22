//! El motor entero de un mando: lo que en C# hacen `ControlService.On_Report` y `Mapping` con cada
//! informe. El estado decodificado pasa, en este orden, por el panel táctil, el giroscopio (activador
//! y ratón), las curvas, la reasignación con las acciones especiales y los clics del teclado y el
//! ratón virtuales.
//!
//! Es el mismo orden que sigue la prueba de equivalencia con C# (`tests/panel.rs`), que pasa por
//! aquí: si el motor cambia, la prueba lo nota.

use crate::botones::{AjustesBotones, Botones, Contexto, Entradas};
use crate::curvas::{AjustesCurvas, Curvas};
use crate::giro_raton::{AjustesGiroRaton, GiroRaton};
use crate::giroscopio::Activador;
use crate::panel::{AjustesPanel, Panel, RatonPanel};
use crate::raton::{EventoRaton, Sintetico};
use ds5_informes::Estado;
use ds5_perfiles::perfil::{MandoVirtual, Perfil};

/// Todo lo que el motor saca de un perfil. Se calcula una vez al cargarlo.
#[derive(Debug, Clone)]
pub struct AjustesMotor {
    pub curvas: AjustesCurvas,
    pub botones: AjustesBotones,
    pub panel: AjustesPanel,
    pub giro: AjustesGiroRaton,
    /// El mando virtual que pide el perfil (`OutputContDevice`).
    pub mando: MandoVirtual,
}

impl AjustesMotor {
    pub fn desde_perfil(p: &Perfil) -> AjustesMotor {
        AjustesMotor {
            curvas: AjustesCurvas::desde_perfil(&p.ajustes),
            botones: AjustesBotones::desde_perfil(p),
            panel: AjustesPanel::desde_perfil(p),
            giro: AjustesGiroRaton::desde_perfil(p),
            mando: p.ajustes.mando_virtual,
        }
    }
}

/// Lo que sale de un informe.
#[derive(Debug, Clone)]
pub struct Paso {
    /// El estado tras el panel y el giroscopio, antes de las curvas.
    pub entrada: Estado,
    /// El estado que va al mando virtual.
    pub salida: Estado,
    /// Lo que hay que mandar al teclado y al ratón, en orden.
    pub eventos: Vec<EventoRaton>,
}

/// El estado del motor de un mando entre informes.
#[derive(Debug)]
pub struct Motor {
    pub panel: Panel,
    pub raton: RatonPanel,
    pub sintetico: Sintetico,
    pub curvas: Curvas,
    pub botones: Botones,
    pub activador: Activador,
    pub giro: GiroRaton,
    /// `TouchActive`: si el panel mueve el ratón. Se cambia con PS + panel.
    pub panel_activo: bool,
    /// El informe anterior sin tocar y su hora, para los retardos y los flancos de las acciones.
    antes: Option<(i64, Estado)>,
    /// Entre `preparar` y `acabar`: el informe sin tocar y lo pedido al ratón.
    crudo: Estado,
    eventos: Vec<EventoRaton>,
}

impl Motor {
    pub fn nuevo(a: &AjustesMotor) -> Motor {
        Motor {
            panel: Panel::default(),
            raton: RatonPanel::nuevo(&a.panel),
            sintetico: Sintetico::default(),
            curvas: Curvas::default(),
            botones: Botones::default(),
            activador: Activador::default(),
            giro: GiroRaton::nuevo(&a.giro),
            panel_activo: true,
            antes: None,
            crudo: Estado::default(),
            eventos: Vec::new(),
        }
    }

    /// Pasa un informe ya decodificado por todo el motor. `us` es su hora en microsegundos.
    pub fn procesar(&mut self, crudo: &Estado, a: &AjustesMotor, us: i64) -> Paso {
        let entrada = self.preparar(crudo, a, us);
        self.acabar(entrada, a, us)
    }

    /// Primera mitad: el panel táctil y el giroscopio. Devuelve el estado que entra en las curvas;
    /// lo que piden al ratón queda guardado para `acabar`.
    pub fn preparar(&mut self, crudo: &Estado, a: &AjustesMotor, us: i64) -> Estado {
        let mut e = crudo.clone();
        self.eventos.clear();
        self.sintetico.ahora_us = us;
        let sucesos = self.panel.sucesos(&e, us);
        self.raton.procesar(
            &sucesos,
            &mut e,
            &a.panel,
            self.panel_activo,
            &mut self.sintetico,
            &mut self.botones.sticks,
            &mut self.eventos,
        );
        self.activador.aplicar(&mut e, &a.curvas.giro);
        self.giro.procesar(
            &e,
            &a.giro,
            &mut self.activador,
            &mut self.botones.sticks,
            &mut self.eventos,
            us,
        );
        self.crudo = crudo.clone();
        e
    }

    /// Segunda mitad: las curvas, la reasignación con las acciones y los clics.
    pub fn acabar(&mut self, entrada: Estado, a: &AjustesMotor, us: i64) -> Paso {
        let mut eventos = std::mem::take(&mut self.eventos);
        let crudo = std::mem::take(&mut self.crudo);
        let (transcurrido_ms, crudo_antes) = match self.antes.take() {
            Some((t, antes)) => ((us - t) as f64 / 1000.0, antes),
            None => (0.0, Estado::default()),
        };
        let curvado = self.curvas.aplicar(&entrada, &a.curvas, us / 1000);
        let entradas = if a.botones.personalizado {
            let (gestos, antes) = self.giro.gestos();
            Entradas {
                giro_deslizar: gestos,
                giro_deslizar_antes: antes,
                ..self.raton.entradas()
            }
        } else {
            Entradas::default()
        };
        let mut ctx = Contexto {
            sintetico: &mut self.sintetico,
            eventos: &mut eventos,
            crudo: &crudo,
            crudo_antes: &crudo_antes,
            transcurrido_ms,
        };
        let salida = self
            .botones
            .aplicar_con(&curvado, &entradas, &a.botones, a.mando, &mut ctx);
        self.sintetico.confirmar(&mut eventos);
        self.antes = Some((us, crudo));
        Paso {
            entrada,
            salida,
            eventos,
        }
    }
}
