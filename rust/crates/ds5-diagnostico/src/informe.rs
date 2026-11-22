//! Junta todo en el objeto `Diagnostico` de la interfaz, con las mismas lecturas, en el mismo
//! orden y con los mismos nombres de fila que `DualSenseDiagnostics.Read`.

use serde_json::{json, Value};

use crate::firmware::Firmware;
use crate::formato::{
    color, hex, hex_invertido, mac, placa, texto, tres_partes, u16_en, u32_en, u64_en, voltaje,
};
use crate::protocolo::{dispositivo, leer_orden, leer_paginas, Canal, LARGO};
use crate::telemetria::telemetria;

/// Número del informe con el parche de Bluetooth.
pub const INFORME_PARCHE_BT: u8 = 0x22;
/// El firmware antiguo contesta con un bloque de uso incompleto.
pub const ACTUALIZACION_MINIMA_USO: u32 = 1106;
pub const HARDWARE_MINIMO_TRAZABLE: u32 = 777;
pub const FIRMWARE_MINIMO_TRAZABLE: u32 = 65655;
/// Páginas del bloque de uso.
pub const PAGINAS_USO: usize = 4;

/// Las filas de fábrica: las vacías no se añaden.
#[derive(Default)]
struct Filas(Vec<Value>);

impl Filas {
    fn poner(&mut self, nombre: &str, valor: Option<String>) {
        if let Some(valor) = valor.filter(|v| !v.is_empty()) {
            self.0.push(json!({ "nombre": nombre, "valor": valor }));
        }
    }
}

/// Los milivoltios de la batería, de la respuesta de la orden 4/3.
pub fn milivoltios(datos: &[u8]) -> Option<u16> {
    (datos.len() >= 2).then(|| u16_en(datos, 0))
}

/// El valor del informe 0x22, o `None` si no es ese informe.
pub fn parche_bluetooth(informe: &[u8]) -> Option<String> {
    (informe.len() >= 35 && informe[0] == INFORME_PARCHE_BT)
        .then(|| format!("0x{:08X}", u32_en(informe, 31)))
}

/// El número de 64 bits de la orden 1/9, si su primer byte dice que es válido.
pub fn id_unico(datos: &[u8]) -> Option<String> {
    (datos.len() >= 9 && datos[0] == 0).then(|| format!("0x{:016X}", u64_en(datos, 1)))
}

/// Si el mando guarda los datos de trazabilidad (PCBA, serie, piezas...).
pub fn trazable(firmware: &Firmware) -> bool {
    matches!(firmware.tipo(), 2 | 3)
        && (firmware.hardware & 0xFFFF) >= HARDWARE_MINIMO_TRAZABLE
        && firmware.principal >= FIRMWARE_MINIMO_TRAZABLE
}

/// El color y la placa que salen del número de serie, si tiene al menos seis letras.
pub fn color_y_placa(serie: &str) -> Option<(String, String)> {
    let letras = serie.as_bytes();
    // Al salir de `texto` todo es ASCII, así que cada byte es una letra.
    (letras.len() >= 6).then(|| (color(&serie[4..6]).to_string(), placa(letras[1])))
}

/// Las filas que salen solo del informe de firmware, en el orden de la interfaz.
fn filas_firmware(filas: &mut Filas, fw: &Firmware) {
    let info = fw.informe.as_ref();
    filas.poner("Firmware compilado", Some(fw.compilado.clone()));
    filas.poner("Hardware", Some(format!("0x{:08X}", fw.hardware)));
    filas.poner(
        "Dispositivo",
        info.map(|i| {
            let mut id = i[32..44].to_vec();
            id.reverse();
            format!("0x{}", hex(&id))
        }),
    );
    filas.poner("Tipo de firmware", Some(format!("0x{:04X}", fw.tipo())));
    filas.poner(
        "Serie de software",
        info.map(|i| format!("0x{:04X}", u16_en(i, 22))),
    );
    filas.poner(
        "Actualización",
        Some(format!(
            "{:X}.{:02X}",
            fw.actualizacion >> 8,
            fw.actualizacion & 0xFF
        )),
    );
    filas.poner("Arranque (SBL)", Some(tres_partes(fw.arranque)));
    filas.poner("Firmware principal", Some(tres_partes(fw.principal)));
    filas.poner(
        "Firmware DSP",
        Some(format!("{:04X}_{:04X}", fw.audio >> 16, fw.audio & 0xFFFF)),
    );
    filas.poner("Firmware MCU DSP", info.map(|i| tres_partes(u32_en(i, 56))));
}

fn leer_parche<C: Canal>(canal: &mut C) -> Option<String> {
    let mut informe = [0u8; LARGO];
    informe[0] = INFORME_PARCHE_BT;
    if !canal.leer_caracteristica(&mut informe) {
        return None;
    }
    parche_bluetooth(&informe)
}

fn fabrica<C: Canal>(
    canal: &mut C,
    fw: &Firmware,
    bluetooth: bool,
    milivoltios: Option<u16>,
) -> Vec<Value> {
    use dispositivo::{BLUETOOTH, SISTEMA, TACTIL};
    let leer = |c: &mut C, disp: u8, accion: u8, largo: usize| {
        leer_orden(c, bluetooth, disp, accion, largo)
    };
    let mut filas = Filas::default();
    filas_firmware(&mut filas, fw);
    filas.poner("Parche de Bluetooth", leer_parche(canal));

    if trazable(fw) {
        let pcba = leer(canal, SISTEMA, 17, 24).map(|d| texto(&d).chars().rev().collect());
        filas.poner("PCBA", pcba);
        let serie = leer(canal, SISTEMA, 19, 32).map(|d| texto(&d));
        filas.poner("Número de serie", serie.clone());
        if let Some((color, placa)) = serie.as_deref().and_then(color_y_placa) {
            filas.poner("Color", Some(color));
            filas.poner("Placa", Some(placa));
        }
        let piezas = leer(canal, SISTEMA, 21, 32).map(|d| hex_invertido(&d));
        filas.poner("Piezas de montaje", piezas);
        filas.poner(
            "Código de batería",
            leer(canal, SISTEMA, 24, 32).map(|d| texto(&d)),
        );
        filas.poner(
            "Motor izquierdo",
            leer(canal, SISTEMA, 26, 32).map(|d| texto(&d)),
        );
        filas.poner(
            "Motor derecho",
            leer(canal, SISTEMA, 28, 32).map(|d| texto(&d)),
        );
    } else {
        let pcba = leer(canal, SISTEMA, 4, 6).map(|d| hex_invertido(&d));
        filas.poner("PCBA", pcba);
    }

    filas.poner(
        "ID único",
        leer(canal, SISTEMA, 9, 9).and_then(|d| id_unico(&d)),
    );
    filas.poner(
        "MAC Bluetooth",
        leer(canal, BLUETOOTH, 2, 6).map(|d| mac(&d)),
    );

    // El panel táctil solo contesta por USB.
    if !bluetooth {
        let panel = leer(canal, TACTIL, 2, 8).map(|d| format!("0x{}", hex(&d)));
        filas.poner("Panel táctil", panel);
        let version = leer(canal, TACTIL, 4, 8).map(|d| format!("0x{}", hex(&d)));
        filas.poner("Firmware del panel", version);
    }

    filas.poner("Voltaje de batería", milivoltios.map(voltaje));
    filas.0
}

fn uso<C: Canal>(canal: &mut C, fw: &Firmware, bluetooth: bool) -> Value {
    if fw.actualizacion <= ACTUALIZACION_MINIMA_USO {
        return Value::Null;
    }
    leer_paginas(canal, bluetooth, dispositivo::TELEMETRIA, 1, PAGINAS_USO)
        .and_then(|bloque| telemetria(&bloque))
        .unwrap_or(Value::Null)
}

/// El objeto `Diagnostico`: `milivoltios`, `fabrica` y `uso`. Con `fw` el informe de firmware ya
/// leído (ver [`crate::leer_firmware`]).
pub fn diagnostico<C: Canal>(canal: &mut C, fw: &Firmware, bluetooth: bool) -> Value {
    let milivoltios = leer_orden(canal, bluetooth, dispositivo::ANALOGICO, 3, 4)
        .as_deref()
        .and_then(milivoltios);
    let fabrica = fabrica(canal, fw, bluetooth, milivoltios);
    let uso = uso(canal, fw, bluetooth);
    json!({
        "milivoltios": milivoltios,
        "fabrica": fabrica,
        "uso": uso,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::firmware::tests::informe_firmware;
    use crate::simulado::{respuesta, Simulado};
    use crate::telemetria::tests::Bloque;

    fn filas(v: &Value) -> Vec<(String, String)> {
        v["fabrica"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    f["nombre"].as_str().unwrap().to_string(),
                    f["valor"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    }

    fn nombres(v: &Value) -> Vec<String> {
        filas(v).into_iter().map(|(n, _)| n).collect()
    }

    fn valor(v: &Value, nombre: &str) -> Option<String> {
        filas(v)
            .into_iter()
            .find(|(n, _)| n == nombre)
            .map(|(_, v)| v)
    }

    fn con_texto(texto: &[u8], largo: usize) -> Vec<u8> {
        let mut d = vec![0u8; largo];
        d[..texto.len()].copy_from_slice(texto);
        d
    }

    /// Un mando trazable por USB que lo contesta todo.
    fn mando_completo() -> Simulado {
        let mut m = Simulado::nuevo();
        m.responder_datos(4, 3, &3855u16.to_le_bytes());
        let mut parche = [0u8; LARGO];
        parche[0] = INFORME_PARCHE_BT;
        parche[31..35].copy_from_slice(&0x0012_ABCDu32.to_le_bytes());
        m.parche = Some(parche);
        m.responder_datos(1, 17, &con_texto(b"4321DCBA", 24));
        m.responder_datos(1, 19, &con_texto(b"A3BC02XYZ   ", 32));
        let piezas: Vec<u8> = (1..=32).collect();
        m.responder_datos(1, 21, &piezas);
        m.responder_datos(1, 24, &con_texto(b"BAT-01", 32));
        m.responder_datos(1, 26, &con_texto(b"MOT-L", 32));
        m.responder_datos(1, 28, &con_texto(b"MOT-R", 32));
        let mut unico = vec![0u8; 9];
        unico[1..9].copy_from_slice(&0x0123_4567_89AB_CDEFu64.to_le_bytes());
        m.responder_datos(1, 9, &unico);
        m.responder_datos(9, 2, &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        m.responder_datos(5, 2, &[0xA0, 1, 2, 3, 4, 5, 6, 7]);
        m.responder_datos(5, 4, &[0, 0, 0, 0, 0, 0, 1, 0x10]);
        let mut bloque = Bloque::nuevo();
        bloque.u32(17, 77);
        let paginas = bloque.0[..224]
            .chunks(56)
            .map(|p| respuesta(0x70, 1, 3, p))
            .collect();
        m.responder(0x70, 1, paginas);
        m
    }

    fn firmware() -> Firmware {
        Firmware::desde_informe(&informe_firmware()).unwrap()
    }

    #[test]
    fn mando_trazable_por_usb() {
        let mut m = mando_completo();
        let d = diagnostico(&mut m, &firmware(), false);

        assert_eq!(d["milivoltios"], 3855);
        assert_eq!(
            filas(&d),
            [
                ("Firmware compilado", "Jun 10 2024 12:34:56"),
                ("Hardware", "0x00000310"),
                ("Dispositivo", "0x0C0B0A090807060504030201"),
                ("Tipo de firmware", "0x0003"),
                ("Serie de software", "0x0102"),
                ("Actualización", "6.30"),
                ("Arranque (SBL)", "0.1.2"),
                ("Firmware principal", "1.4.16"),
                ("Firmware DSP", "0004_0105"),
                ("Firmware MCU DSP", "2.3.4"),
                ("Parche de Bluetooth", "0x0012ABCD"),
                ("PCBA", "ABCD1234"),
                ("Número de serie", "A3BC02XYZ"),
                ("Color", "Rojo cósmico"),
                ("Placa", "BDM-030"),
                (
                    "Piezas de montaje",
                    "0x201F1E1D1C1B1A191817161514131211100F0E0D0C0B0A090807060504030201"
                ),
                ("Código de batería", "BAT-01"),
                ("Motor izquierdo", "MOT-L"),
                ("Motor derecho", "MOT-R"),
                ("ID único", "0x0123456789ABCDEF"),
                ("MAC Bluetooth", "06:05:04:03:02:01"),
                ("Panel táctil", "0xA001020304050607"),
                ("Firmware del panel", "0x0000000000000110"),
                ("Voltaje de batería", "3,86 V"),
            ]
            .map(|(n, v)| (n.to_string(), v.to_string()))
        );
        assert_eq!(d["uso"]["registros"], 77);
        assert_eq!(
            m.ordenes(),
            [
                (4, 3),
                (1, 17),
                (1, 19),
                (1, 21),
                (1, 24),
                (1, 26),
                (1, 28),
                (1, 9),
                (9, 2),
                (5, 2),
                (5, 4),
                (0x70, 1)
            ]
        );
    }

    #[test]
    fn por_bluetooth_no_pregunta_al_panel_y_firma_las_ordenes() {
        let mut m = mando_completo();
        let d = diagnostico(&mut m, &firmware(), true);
        let n = nombres(&d);
        assert!(!n.contains(&"Panel táctil".to_string()));
        assert!(!n.contains(&"Firmware del panel".to_string()));
        assert!(!m.ordenes().contains(&(5, 2)));
        assert!(m
            .enviados
            .iter()
            .all(|o| ds5_informes::crc::valido(0x53, o)));
    }

    #[test]
    fn mando_sin_trazabilidad() {
        let mut m = mando_completo();
        m.responder_datos(1, 4, &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
        let mut fw = firmware();
        fw.principal = FIRMWARE_MINIMO_TRAZABLE - 1;
        let d = diagnostico(&mut m, &fw, false);
        assert_eq!(valor(&d, "PCBA").as_deref(), Some("0x665544332211"));
        assert_eq!(valor(&d, "Número de serie"), None);
        assert_eq!(valor(&d, "Color"), None);
        assert!(!m.ordenes().contains(&(1, 19)));
        assert!(m.ordenes().contains(&(1, 4)));
    }

    #[test]
    fn limites_de_la_trazabilidad() {
        let mut fw = firmware();
        assert!(trazable(&fw));
        fw.hardware = 0xFFFF_0000 | 776;
        assert!(!trazable(&fw));
        fw.hardware = 0xFFFF_0000 | 777;
        assert!(trazable(&fw));
        fw.principal = 65655;
        assert!(trazable(&fw));
        let mut informe = informe_firmware();
        informe[20] = 2;
        assert!(trazable(&Firmware::desde_informe(&informe).unwrap()));
        informe[20] = 1;
        assert!(!trazable(&Firmware::desde_informe(&informe).unwrap()));
        assert!(!trazable(&Firmware::default()));
    }

    #[test]
    fn sin_informe_de_firmware_ni_respuestas() {
        let mut m = Simulado::nuevo();
        let d = diagnostico(&mut m, &Firmware::default(), false);
        assert_eq!(d["milivoltios"], Value::Null);
        assert_eq!(d["uso"], Value::Null);
        assert_eq!(
            filas(&d),
            [
                ("Hardware", "0x00000000"),
                ("Tipo de firmware", "0x0000"),
                ("Actualización", "0.00"),
                ("Arranque (SBL)", "0.0.0"),
                ("Firmware principal", "0.0.0"),
                ("Firmware DSP", "0000_0000"),
            ]
            .map(|(n, v)| (n.to_string(), v.to_string()))
        );
        // Sin trazabilidad ni uso: batería, PCBA, ID, MAC y los dos del panel.
        assert_eq!(
            m.ordenes(),
            [(4, 3), (1, 4), (1, 9), (9, 2), (5, 2), (5, 4)]
        );
    }

    #[test]
    fn firmware_antiguo_sin_uso() {
        let mut m = mando_completo();
        let mut fw = firmware();
        fw.actualizacion = ACTUALIZACION_MINIMA_USO;
        let d = diagnostico(&mut m, &fw, false);
        assert_eq!(d["uso"], Value::Null);
        assert!(!m.ordenes().contains(&(0x70, 1)));
        fw.actualizacion = ACTUALIZACION_MINIMA_USO + 1;
        let mut m = mando_completo();
        assert!(diagnostico(&mut m, &fw, false)["uso"].is_object());
    }

    #[test]
    fn serie_corta_sin_color_ni_placa() {
        let mut m = mando_completo();
        m.responder_datos(1, 19, &con_texto(b"A9Z", 32));
        let d = diagnostico(&mut m, &firmware(), false);
        assert_eq!(valor(&d, "Número de serie").as_deref(), Some("A9Z"));
        assert_eq!(valor(&d, "Color"), None);
        assert_eq!(valor(&d, "Placa"), None);
    }

    #[test]
    fn serie_vacia_no_sale() {
        let mut m = mando_completo();
        m.responder_datos(1, 19, &[0u8; 32]);
        m.responder_datos(1, 17, &[b' '; 24]);
        let d = diagnostico(&mut m, &firmware(), false);
        assert_eq!(valor(&d, "Número de serie"), None);
        assert_eq!(valor(&d, "PCBA"), None);
    }

    #[test]
    fn color_desconocido_y_placa_desconocida() {
        assert_eq!(
            color_y_placa("X9ABQQ"),
            Some(("Desconocido".to_string(), "Desconocida".to_string()))
        );
        assert_eq!(
            color_y_placa("X1ABZ7"),
            Some(("Ghost of Yotei".to_string(), "BDM-010".to_string()))
        );
        assert_eq!(color_y_placa("X1ABZ"), None);
    }

    #[test]
    fn id_unico_solo_si_es_valido() {
        let mut d = vec![0u8; 9];
        d[1] = 0xFF;
        assert_eq!(id_unico(&d).as_deref(), Some("0x00000000000000FF"));
        d[0] = 1;
        assert_eq!(id_unico(&d), None);
        assert_eq!(id_unico(&[0; 8]), None);
    }

    #[test]
    fn parche_solo_del_informe_22() {
        let mut p = [0u8; LARGO];
        p[0] = INFORME_PARCHE_BT;
        p[31..35].copy_from_slice(&[0x78, 0x56, 0x34, 0x12]);
        assert_eq!(parche_bluetooth(&p).as_deref(), Some("0x12345678"));
        p[0] = 0x21;
        assert_eq!(parche_bluetooth(&p), None);
        assert_eq!(parche_bluetooth(&[INFORME_PARCHE_BT; 34]), None);
    }

    #[test]
    fn milivoltios_de_la_respuesta() {
        assert_eq!(milivoltios(&[0x0F, 0x0F, 0, 0]), Some(0x0F0F));
        assert_eq!(milivoltios(&[1]), None);
    }

    #[test]
    fn sin_bateria_sin_voltaje() {
        let mut m = mando_completo();
        m.responder(4, 3, vec![]);
        let d = diagnostico(&mut m, &firmware(), false);
        assert_eq!(d["milivoltios"], Value::Null);
        assert_eq!(valor(&d, "Voltaje de batería"), None);
    }

    #[test]
    fn claves_del_contrato() {
        let mut m = mando_completo();
        let d = diagnostico(&mut m, &firmware(), false);
        let mut claves: Vec<&str> = d.as_object().unwrap().keys().map(|k| k.as_str()).collect();
        claves.sort_unstable();
        assert_eq!(claves, ["fabrica", "milivoltios", "uso"]);
        let mut uso: Vec<&str> = d["uso"]
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        uso.sort_unstable();
        assert_eq!(
            uso,
            [
                "activo",
                "carga",
                "cargasCompletas",
                "ciclosCarga",
                "conexion",
                "errores",
                "gatilloDer",
                "gatilloIzq",
                "haptico",
                "pulsaciones",
                "registros",
                "sticks"
            ]
        );
    }
}
