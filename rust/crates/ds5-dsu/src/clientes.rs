//! Los clientes suscritos y cuándo caducan, como `ClientRequestTimes` y el reparto de
//! `NewReportIncoming`.
//!
//! Un cliente pide los datos de todos los mandos, de una ranura o de una MAC, y tiene que
//! repetir la petición cada poco: si pasan cinco segundos sin pedir lo de un mando, deja de
//! recibirlo. Cuando no le queda ninguna petición viva se le olvida.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use crate::paquetes::RANURAS;

/// Lo que dura una suscripción (`TimeoutLimit`).
pub const CADUCIDAD: Duration = Duration::from_secs(5);

/// Cuándo pidió cada cosa un cliente. `None` es que nunca.
#[derive(Debug, Clone, Default)]
struct Tiempos {
    todos: Option<Instant>,
    ranuras: [Option<Instant>; RANURAS],
    macs: HashMap<[u8; 6], Instant>,
}

fn vivo(t: Option<Instant>, ahora: Instant) -> bool {
    t.is_some_and(|t| ahora.saturating_duration_since(t) < CADUCIDAD)
}

impl Tiempos {
    /// Como `RequestPadInfo`.
    fn registrar(&mut self, banderas: u8, ranura: u8, mac: [u8; 6], ahora: Instant) {
        if banderas == 0 {
            self.todos = Some(ahora);
            return;
        }
        if banderas & 0x01 != 0 {
            if let Some(t) = self.ranuras.get_mut(ranura as usize) {
                *t = Some(ahora);
            }
        }
        if banderas & 0x02 != 0 {
            self.macs.insert(mac, ahora);
        }
    }

    fn quiere(&self, ranura: u8, mac: Option<[u8; 6]>, ahora: Instant) -> bool {
        vivo(self.todos, ahora)
            || self
                .ranuras
                .get(ranura as usize)
                .is_some_and(|t| vivo(*t, ahora))
            || mac.is_some_and(|m| vivo(self.macs.get(&m).copied(), ahora))
    }

    /// Si le queda alguna petición de ranura o de MAC viva. Como en C#, la de todos los mandos
    /// no se mira aquí: si estuviera viva el cliente ya habría recibido el paquete.
    fn alguna_viva(&self, ahora: Instant) -> bool {
        self.ranuras.iter().any(|t| vivo(*t, ahora))
            || self.macs.values().any(|t| vivo(Some(*t), ahora))
    }
}

/// La lista de clientes suscritos, por su dirección.
#[derive(Debug, Clone, Default)]
pub struct Clientes {
    lista: HashMap<SocketAddr, Tiempos>,
}

impl Clientes {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apunta una petición de datos (`DSUC_PadDataReq`).
    pub fn registrar(
        &mut self,
        cliente: SocketAddr,
        banderas: u8,
        ranura: u8,
        mac: [u8; 6],
        ahora: Instant,
    ) {
        self.lista
            .entry(cliente)
            .or_default()
            .registrar(banderas, ranura, mac, ahora);
    }

    /// A quién hay que mandar los datos del mando de `ranura` con `mac`. De paso olvida a los
    /// clientes que ya no tienen ninguna petición viva, como `NewReportIncoming`.
    pub fn destinatarios(
        &mut self,
        ranura: u8,
        mac: Option<[u8; 6]>,
        ahora: Instant,
    ) -> Vec<SocketAddr> {
        let mut destino = Vec::new();
        self.lista.retain(|dir, t| {
            if t.quiere(ranura, mac, ahora) {
                destino.push(*dir);
                true
            } else {
                t.alguna_viva(ahora)
            }
        });
        destino
    }

    /// Cuántos clientes hay apuntados, vivos o no.
    pub fn len(&self) -> usize {
        self.lista.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lista.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(puerto: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], puerto))
    }

    const MAC: [u8; 6] = [1, 2, 3, 4, 5, 6];
    const OTRA: [u8; 6] = [9, 9, 9, 9, 9, 9];

    #[test]
    fn todos_los_mandos_durante_cinco_segundos() {
        let t0 = Instant::now();
        let mut c = Clientes::new();
        c.registrar(dir(1), 0, 0, [0; 6], t0);
        for r in 0..4 {
            assert_eq!(c.destinatarios(r, Some(MAC), t0), vec![dir(1)]);
        }
        let casi = t0 + CADUCIDAD - Duration::from_millis(1);
        assert_eq!(c.destinatarios(3, None, casi), vec![dir(1)]);
        // A los cinco segundos justos ya no (`< 5`), y como no pidió nada más se le olvida.
        assert!(c.destinatarios(3, None, t0 + CADUCIDAD).is_empty());
        assert!(c.is_empty());
    }

    #[test]
    fn por_ranura() {
        let t0 = Instant::now();
        let mut c = Clientes::new();
        c.registrar(dir(1), 0x01, 2, [0; 6], t0);
        assert!(c.destinatarios(1, Some(MAC), t0).is_empty());
        // Sigue apuntado: la petición de la ranura 2 está viva.
        assert_eq!(c.len(), 1);
        assert_eq!(c.destinatarios(2, Some(MAC), t0), vec![dir(1)]);
        // Una ranura que no existe no apunta nada, pero el cliente queda en la lista.
        c.registrar(dir(2), 0x01, 7, [0; 6], t0);
        assert_eq!(c.len(), 2);
        assert!(c.destinatarios(0, None, t0).is_empty());
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn por_mac() {
        let t0 = Instant::now();
        let mut c = Clientes::new();
        c.registrar(dir(1), 0x02, 0, MAC, t0);
        assert_eq!(c.destinatarios(3, Some(MAC), t0), vec![dir(1)]);
        assert!(c.destinatarios(3, Some(OTRA), t0).is_empty());
        assert!(c.destinatarios(3, None, t0).is_empty());
        assert_eq!(c.len(), 1);
        // Las dos banderas a la vez.
        c.registrar(dir(2), 0x03, 1, OTRA, t0);
        assert_eq!(c.destinatarios(1, None, t0), vec![dir(2)]);
        assert_eq!(c.destinatarios(0, Some(OTRA), t0), vec![dir(2)]);
    }

    #[test]
    fn renovar_alarga_la_suscripcion() {
        let t0 = Instant::now();
        let mut c = Clientes::new();
        c.registrar(dir(1), 0x01, 0, [0; 6], t0);
        let t1 = t0 + Duration::from_secs(4);
        c.registrar(dir(1), 0x01, 0, [0; 6], t1);
        assert_eq!(
            c.destinatarios(0, None, t0 + Duration::from_secs(8)),
            vec![dir(1)]
        );
        assert!(c
            .destinatarios(0, None, t1 + CADUCIDAD + Duration::from_millis(1))
            .is_empty());
        assert!(c.is_empty());
    }

    #[test]
    fn caducado_para_todos_pero_vivo_para_una_ranura() {
        let t0 = Instant::now();
        let mut c = Clientes::new();
        c.registrar(dir(1), 0, 0, [0; 6], t0);
        let t1 = t0 + Duration::from_secs(4);
        c.registrar(dir(1), 0x01, 1, [0; 6], t1);
        let t2 = t0 + Duration::from_secs(6);
        // Lo de todos ha caducado; la ranura 1 no.
        assert!(c.destinatarios(0, None, t2).is_empty());
        assert_eq!(c.len(), 1);
        assert_eq!(c.destinatarios(1, None, t2), vec![dir(1)]);
    }
}
