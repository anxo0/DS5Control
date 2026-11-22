//! Un mando de mentira para las pruebas: contesta las órdenes con las páginas que se le den y
//! lleva un reloj propio que solo avanza con las pausas.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use crate::protocolo::{Canal, INFORME_ORDEN, INFORME_RESULTADO, INICIO_DATOS, LARGO};

/// Una respuesta 0x81 con `datos` desde el byte 4.
pub fn respuesta(dispositivo: u8, accion: u8, estado: u8, datos: &[u8]) -> [u8; LARGO] {
    let mut r = [0u8; LARGO];
    r[0] = INFORME_RESULTADO;
    r[1] = dispositivo;
    r[2] = accion;
    r[3] = estado;
    r[INICIO_DATOS..INICIO_DATOS + datos.len()].copy_from_slice(datos);
    r
}

pub struct Simulado {
    respuestas: HashMap<(u8, u8), Vec<[u8; LARGO]>>,
    pendientes: VecDeque<[u8; LARGO]>,
    restantes: usize,
    /// Lecturas sin respuesta después de cada orden, antes de la primera página.
    pub ocupado: usize,
    /// Lecturas sin respuesta antes de cada página.
    pub ocupado_por_pagina: usize,
    pub escritura_falla: bool,
    /// Lo que devuelve cada lectura del informe 0x20; la última se repite.
    pub firmware: Vec<[u8; LARGO]>,
    pub lecturas_firmware: usize,
    pub parche: Option<[u8; LARGO]>,
    pub enviados: Vec<Vec<u8>>,
    /// Lecturas del informe 0x81.
    pub lecturas: usize,
    inicio: Instant,
    reloj: Duration,
}

impl Simulado {
    pub fn nuevo() -> Simulado {
        Simulado {
            respuestas: HashMap::new(),
            pendientes: VecDeque::new(),
            restantes: 0,
            ocupado: 0,
            ocupado_por_pagina: 0,
            escritura_falla: false,
            firmware: Vec::new(),
            lecturas_firmware: 0,
            parche: None,
            enviados: Vec::new(),
            lecturas: 0,
            inicio: Instant::now(),
            reloj: Duration::ZERO,
        }
    }

    pub fn responder(&mut self, dispositivo: u8, accion: u8, paginas: Vec<[u8; LARGO]>) {
        self.respuestas.insert((dispositivo, accion), paginas);
    }

    /// Contesta con una sola página completa.
    pub fn responder_datos(&mut self, dispositivo: u8, accion: u8, datos: &[u8]) {
        self.responder(
            dispositivo,
            accion,
            vec![respuesta(dispositivo, accion, 2, datos)],
        );
    }

    pub fn transcurrido(&self) -> Duration {
        self.reloj
    }

    /// Las órdenes mandadas, como (dispositivo, acción).
    pub fn ordenes(&self) -> Vec<(u8, u8)> {
        self.enviados.iter().map(|o| (o[1], o[2])).collect()
    }
}

impl Canal for Simulado {
    fn leer_caracteristica(&mut self, bufer: &mut [u8]) -> bool {
        match bufer[0] {
            0x20 => {
                self.lecturas_firmware += 1;
                let Some(ultimo) = self.firmware.len().checked_sub(1) else {
                    return false;
                };
                let informe = self.firmware[(self.lecturas_firmware - 1).min(ultimo)];
                bufer.copy_from_slice(&informe);
                true
            }
            0x22 => match self.parche {
                Some(informe) => {
                    bufer.copy_from_slice(&informe);
                    true
                }
                None => false,
            },
            INFORME_RESULTADO => {
                self.lecturas += 1;
                if self.restantes > 0 {
                    self.restantes -= 1;
                    return false;
                }
                match self.pendientes.pop_front() {
                    Some(informe) => {
                        bufer.copy_from_slice(&informe);
                        self.restantes = self.ocupado_por_pagina;
                        true
                    }
                    None => false,
                }
            }
            _ => false,
        }
    }

    fn escribir_caracteristica(&mut self, informe: &[u8]) -> bool {
        assert_eq!(informe.len(), LARGO);
        assert_eq!(informe[0], INFORME_ORDEN);
        self.enviados.push(informe.to_vec());
        if self.escritura_falla {
            return false;
        }
        self.pendientes = self
            .respuestas
            .get(&(informe[1], informe[2]))
            .cloned()
            .unwrap_or_default()
            .into();
        self.restantes = self.ocupado + self.ocupado_por_pagina;
        true
    }

    fn ahora(&self) -> Instant {
        self.inicio + self.reloj
    }

    fn esperar(&mut self, tiempo: Duration) {
        self.reloj += tiempo;
    }
}
