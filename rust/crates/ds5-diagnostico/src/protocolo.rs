//! Las órdenes de prueba de fábrica: la orden sale en el informe 0x80 y la respuesta vuelve en el
//! 0x81, a páginas de 56 bytes. Los reintentos y las esperas son los de C#.

use std::time::{Duration, Instant};

use ds5_informes::crc;

/// Largo de todos los informes de característica que se usan.
pub const LARGO: usize = 64;
/// Informe por el que sale la orden.
pub const INFORME_ORDEN: u8 = 0x80;
/// Informe en el que vuelve la respuesta.
pub const INFORME_RESULTADO: u8 = 0x81;
/// Cabecera del CRC de las órdenes por Bluetooth.
pub const SEMILLA_BT: u8 = 0x53;
/// Bytes de datos en cada página de la respuesta.
pub const TAMANO_PAGINA: usize = 56;
/// Dónde empiezan los datos en la respuesta, tras el informe, el dispositivo, la acción y el estado.
pub const INICIO_DATOS: usize = 4;
/// Lo que se espera a cada página antes de rendirse.
pub const ESPERA: Duration = Duration::from_millis(1000);
/// La pausa entre dos lecturas sin respuesta.
pub const PAUSA: Duration = Duration::from_millis(10);

/// La respuesta está completa.
pub const ESTADO_COMPLETO: u8 = 2;
/// Hay más páginas detrás de esta.
pub const ESTADO_MAS_PAGINAS: u8 = 3;

/// Los dispositivos internos del mando a los que se pregunta.
pub mod dispositivo {
    pub const SISTEMA: u8 = 1;
    pub const ANALOGICO: u8 = 4;
    pub const TACTIL: u8 = 5;
    pub const BLUETOOTH: u8 = 9;
    pub const TELEMETRIA: u8 = 0x70;
}

/// Lo que hace falta del mando: leer y escribir informes de característica y medir el tiempo.
/// `buffer[0]` lleva el número del informe. Con un mando simulado, el tiempo también se simula.
pub trait Canal {
    /// Como `readFeatureData`: `false` si Windows no lo lee.
    fn leer_caracteristica(&mut self, bufer: &mut [u8]) -> bool;
    /// Como `WriteFeatureReport`: `false` si Windows no lo escribe.
    fn escribir_caracteristica(&mut self, informe: &[u8]) -> bool;
    fn ahora(&self) -> Instant;
    fn esperar(&mut self, tiempo: Duration);
}

/// El informe 0x80 con una orden. Por Bluetooth lleva el CRC en los cuatro últimos bytes.
pub fn orden(dispositivo: u8, accion: u8, bluetooth: bool) -> [u8; LARGO] {
    let mut informe = [0u8; LARGO];
    informe[0] = INFORME_ORDEN;
    informe[1] = dispositivo;
    informe[2] = accion;
    if bluetooth {
        crc::firmar(SEMILLA_BT, &mut informe);
    }
    informe
}

/// El estado de una respuesta, o `None` si es de otra orden (o demasiado corta para saberlo).
pub fn estado_respuesta(informe: &[u8], dispositivo: u8, accion: u8) -> Option<u8> {
    if informe.len() < INICIO_DATOS {
        return None;
    }
    let es_esta =
        informe[0] == INFORME_RESULTADO && informe[1] == dispositivo && informe[2] == accion;
    es_esta.then_some(informe[3])
}

fn es_pagina(estado: Option<u8>) -> bool {
    matches!(estado, Some(ESTADO_COMPLETO | ESTADO_MAS_PAGINAS))
}

fn enviar<C: Canal>(canal: &mut C, bluetooth: bool, dispositivo: u8, accion: u8) -> bool {
    canal.escribir_caracteristica(&orden(dispositivo, accion, bluetooth))
}

fn leer_respuesta<C: Canal>(
    canal: &mut C,
    dispositivo: u8,
    accion: u8,
    informe: &mut [u8; LARGO],
) -> Option<u8> {
    *informe = [0u8; LARGO];
    informe[0] = INFORME_RESULTADO;
    if !canal.leer_caracteristica(informe) {
        return None;
    }
    estado_respuesta(informe, dispositivo, accion)
}

/// Manda una orden y junta `largo` bytes de respuesta. `None` si el mando no la contesta: cada
/// página tiene un segundo para llegar.
pub fn leer_orden<C: Canal>(
    canal: &mut C,
    bluetooth: bool,
    dispositivo: u8,
    accion: u8,
    largo: usize,
) -> Option<Vec<u8>> {
    if !enviar(canal, bluetooth, dispositivo, accion) {
        return None;
    }
    let mut resultado = vec![0u8; largo];
    let mut pagina = 0;
    let mut limite = canal.ahora() + ESPERA;
    let mut informe = [0u8; LARGO];
    while canal.ahora() < limite {
        let estado = leer_respuesta(canal, dispositivo, accion, &mut informe);
        if es_pagina(estado) {
            let inicio = TAMANO_PAGINA * pagina;
            let cuantos = TAMANO_PAGINA.min(largo.saturating_sub(inicio));
            if cuantos > 0 {
                resultado[inicio..inicio + cuantos]
                    .copy_from_slice(&informe[INICIO_DATOS..INICIO_DATOS + cuantos]);
            }
            pagina += 1;
            if estado == Some(ESTADO_COMPLETO) {
                return Some(resultado);
            }
            limite = canal.ahora() + ESPERA;
            continue;
        }
        canal.esperar(PAUSA);
    }
    None
}

/// Un bloque de varias páginas. Nunca avisa de que ha terminado: la cabecera deja de coincidir
/// cuando se acaba. Devuelve `paginas * 56` bytes si llegó al menos una página.
pub fn leer_paginas<C: Canal>(
    canal: &mut C,
    bluetooth: bool,
    dispositivo: u8,
    accion: u8,
    paginas: usize,
) -> Option<Vec<u8>> {
    if !enviar(canal, bluetooth, dispositivo, accion) {
        return None;
    }
    let mut resultado = vec![0u8; paginas * TAMANO_PAGINA];
    let mut pagina = 0;
    let mut informe = [0u8; LARGO];
    let mut intento = 0;
    while intento < paginas * 4 + 8 && pagina < paginas {
        let estado = leer_respuesta(canal, dispositivo, accion, &mut informe);
        if es_pagina(estado) {
            let inicio = pagina * TAMANO_PAGINA;
            resultado[inicio..inicio + TAMANO_PAGINA]
                .copy_from_slice(&informe[INICIO_DATOS..INICIO_DATOS + TAMANO_PAGINA]);
            pagina += 1;
            if estado == Some(ESTADO_COMPLETO) {
                break;
            }
        } else if pagina > 0 && estado.is_none() {
            break;
        }
        canal.esperar(PAUSA);
        intento += 1;
    }
    (pagina > 0).then_some(resultado)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulado::{respuesta, Simulado};

    #[test]
    fn orden_por_usb_sin_crc() {
        let o = orden(dispositivo::SISTEMA, 19, false);
        assert_eq!(&o[..3], &[0x80, 1, 19]);
        assert!(o[3..].iter().all(|&b| b == 0));
    }

    #[test]
    fn orden_por_bluetooth_con_crc() {
        let o = orden(dispositivo::BLUETOOTH, 2, true);
        assert_eq!(&o[..3], &[0x80, 9, 2]);
        assert!(crc::valido(SEMILLA_BT, &o));
        assert!(!crc::valido(crc::CABECERA_CARACTERISTICA, &o));
        let crc = u32::from_le_bytes([o[60], o[61], o[62], o[63]]);
        assert_eq!(crc, crc::crc32(0x53, &o[..60]));
    }

    #[test]
    fn estado_de_la_respuesta() {
        let r = respuesta(1, 19, 2, b"hola");
        assert_eq!(estado_respuesta(&r, 1, 19), Some(2));
        assert_eq!(estado_respuesta(&r, 1, 20), None);
        assert_eq!(estado_respuesta(&r, 4, 19), None);
        let mut otro = r;
        otro[0] = 0x80;
        assert_eq!(estado_respuesta(&otro, 1, 19), None);
        assert_eq!(estado_respuesta(&r[..3], 1, 19), None);
        assert_eq!(estado_respuesta(&r[..4], 1, 19), Some(2));
    }

    #[test]
    fn orden_de_una_pagina() {
        let mut m = Simulado::nuevo();
        m.responder(4, 3, vec![respuesta(4, 3, 2, &[0x0A, 0x0F, 9, 9])]);
        assert_eq!(
            leer_orden(&mut m, false, 4, 3, 4),
            Some(vec![0x0A, 0x0F, 9, 9])
        );
        assert_eq!(m.enviados.len(), 1);
        assert_eq!(&m.enviados[0][..3], &[0x80, 4, 3]);
    }

    #[test]
    fn orden_de_varias_paginas_con_esperas() {
        let primera: Vec<u8> = (0..56).collect();
        let segunda: Vec<u8> = (100..156).collect();
        let mut m = Simulado::nuevo();
        m.responder(
            1,
            21,
            vec![respuesta(1, 21, 3, &primera), respuesta(1, 21, 2, &segunda)],
        );
        m.ocupado = 5;
        let r = leer_orden(&mut m, true, 1, 21, 70).unwrap();
        assert_eq!(&r[..56], &primera[..]);
        assert_eq!(&r[56..], &segunda[..14]);
        // Cinco lecturas sin respuesta antes de la primera página, cada una con su pausa.
        assert_eq!(m.transcurrido(), PAUSA * 5);
        assert!(crc::valido(SEMILLA_BT, &m.enviados[0]));
    }

    #[test]
    fn orden_ignora_estados_que_no_son_pagina() {
        let mut m = Simulado::nuevo();
        m.responder(
            9,
            2,
            vec![
                respuesta(9, 2, 1, &[0xEE; 6]),
                respuesta(9, 2, 0, &[0xEE; 6]),
                respuesta(9, 2, 2, &[1, 2, 3, 4, 5, 6]),
            ],
        );
        assert_eq!(
            leer_orden(&mut m, false, 9, 2, 6),
            Some(vec![1, 2, 3, 4, 5, 6])
        );
        assert_eq!(m.transcurrido(), PAUSA * 2);
    }

    #[test]
    fn orden_sin_respuesta_se_rinde_al_segundo() {
        let mut m = Simulado::nuevo();
        assert_eq!(leer_orden(&mut m, false, 5, 2, 8), None);
        assert_eq!(m.transcurrido(), ESPERA);
        assert_eq!(m.lecturas, 100);
    }

    #[test]
    fn orden_que_no_se_puede_mandar() {
        let mut m = Simulado::nuevo();
        m.escritura_falla = true;
        m.responder(4, 3, vec![respuesta(4, 3, 2, &[1, 2])]);
        assert_eq!(leer_orden(&mut m, false, 4, 3, 4), None);
        assert_eq!(m.lecturas, 0);
    }

    #[test]
    fn la_espera_vuelve_a_empezar_con_cada_pagina() {
        let mut m = Simulado::nuevo();
        m.responder(
            1,
            9,
            vec![respuesta(1, 9, 3, &[7; 56]), respuesta(1, 9, 2, &[8; 56])],
        );
        // 90 lecturas en vano antes de cada página: más de un segundo en total, pero menos por página.
        m.ocupado_por_pagina = 90;
        let r = leer_orden(&mut m, false, 1, 9, 60).unwrap();
        assert_eq!(&r[..56], &[7; 56]);
        assert_eq!(&r[56..], &[8; 4]);
        assert_eq!(m.transcurrido(), PAUSA * 180);
    }

    #[test]
    fn bloque_de_paginas_completo() {
        let mut m = Simulado::nuevo();
        let paginas: Vec<_> = (0..4u8)
            .map(|i| respuesta(0x70, 1, 3, &[i + 1; 56]))
            .collect();
        m.responder(0x70, 1, paginas);
        let r = leer_paginas(&mut m, false, 0x70, 1, 4).unwrap();
        assert_eq!(r.len(), 224);
        for i in 0..4 {
            assert!(r[i * 56..(i + 1) * 56].iter().all(|&b| b == i as u8 + 1));
        }
        // Una pausa tras cada página, también tras la última, como en C#.
        assert_eq!(m.transcurrido(), PAUSA * 4);
    }

    #[test]
    fn bloque_que_se_acaba_antes() {
        let mut m = Simulado::nuevo();
        m.responder(
            0x70,
            1,
            vec![
                respuesta(0x70, 1, 3, &[5; 56]),
                respuesta(0x70, 1, 3, &[6; 56]),
            ],
        );
        let r = leer_paginas(&mut m, false, 0x70, 1, 4).unwrap();
        assert!(r[..56].iter().all(|&b| b == 5));
        assert!(r[56..112].iter().all(|&b| b == 6));
        assert!(r[112..].iter().all(|&b| b == 0));
        // La tercera lectura no contesta y corta el bloque.
        assert_eq!(m.lecturas, 3);
    }

    #[test]
    fn bloque_que_termina_con_completo() {
        let mut m = Simulado::nuevo();
        m.responder(
            0x70,
            1,
            vec![
                respuesta(0x70, 1, 3, &[5; 56]),
                respuesta(0x70, 1, 2, &[6; 56]),
            ],
        );
        assert!(leer_paginas(&mut m, false, 0x70, 1, 4).is_some());
        assert_eq!(m.lecturas, 2);
    }

    #[test]
    fn bloque_sin_respuesta() {
        let mut m = Simulado::nuevo();
        assert_eq!(leer_paginas(&mut m, false, 0x70, 1, 4), None);
        // 4 * 4 + 8 intentos.
        assert_eq!(m.lecturas, 24);
        assert_eq!(m.transcurrido(), PAUSA * 24);
    }

    #[test]
    fn bloque_que_no_se_puede_mandar() {
        let mut m = Simulado::nuevo();
        m.escritura_falla = true;
        assert_eq!(leer_paginas(&mut m, false, 0x70, 1, 4), None);
        assert_eq!(m.lecturas, 0);
    }
}
