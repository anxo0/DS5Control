//! Cómo se escriben los valores en las filas, igual que los `ToString` de C#.

/// Los colores por el código que va en las posiciones 4 y 5 del número de serie.
const COLORES: &[(&str, &str)] = &[
    ("00", "Blanco"),
    ("01", "Negro medianoche"),
    ("02", "Rojo cósmico"),
    ("03", "Rosa nova"),
    ("04", "Púrpura galáctico"),
    ("05", "Azul estelar"),
    ("06", "Camuflaje gris"),
    ("07", "Rojo volcánico"),
    ("08", "Plata esterlina"),
    ("09", "Azul cobalto"),
    ("10", "Chroma Teal"),
    ("11", "Chroma Indigo"),
    ("12", "Chroma Pearl"),
    ("13", "Rojo techno"),
    ("30", "30 aniversario"),
    ("Z1", "God of War Ragnarök"),
    ("Z2", "Spider-Man 2"),
    ("Z3", "Astro Bot"),
    ("Z4", "Fortnite"),
    ("Z5", "Monster Hunter Wilds"),
    ("Z6", "The Last of Us"),
    ("Z7", "Ghost of Yotei"),
    ("ZB", "Icon Blue"),
    ("ZC", "Astro Bot Joyful"),
    ("ZE", "Genshin Impact"),
];

/// Como `Encoding.ASCII.GetString`: lo que no es ASCII sale como `?`.
pub fn ascii(datos: &[u8]) -> String {
    datos
        .iter()
        .map(|&b| if b < 0x80 { b as char } else { '?' })
        .collect()
}

/// Un texto guardado en el mando, sin los ceros ni los espacios del final.
pub fn texto(datos: &[u8]) -> String {
    ascii(datos).trim_end_matches(['\0', ' ']).to_string()
}

/// Los bytes en hexadecimal en mayúsculas, sin separar.
pub fn hex(datos: &[u8]) -> String {
    datos.iter().map(|b| format!("{b:02X}")).collect()
}

/// `0x` y los bytes al revés, como los guarda el mando (el menos significativo primero).
pub fn hex_invertido(datos: &[u8]) -> String {
    let mut bytes = datos.to_vec();
    bytes.reverse();
    format!("0x{}", hex(&bytes))
}

/// Una versión en tres partes: el byte alto, el siguiente y los dos bajos.
pub fn tres_partes(version: u32) -> String {
    format!(
        "{}.{}.{}",
        version >> 24,
        (version >> 16) & 0xFF,
        version & 0xFFFF
    )
}

/// La dirección Bluetooth, que el mando guarda al revés.
pub fn mac(datos: &[u8]) -> String {
    datos
        .iter()
        .rev()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Los voltios con dos decimales y coma, como `{0:0.00} V` con la cultura española. Se redondea
/// la mitad hacia arriba, igual que .NET.
pub fn voltaje(milivoltios: u16) -> String {
    let centesimas = (u32::from(milivoltios) + 5) / 10;
    format!("{},{:02} V", centesimas / 100, centesimas % 100)
}

/// El color por el código del número de serie.
pub fn color(codigo: &str) -> &'static str {
    COLORES
        .iter()
        .find(|(c, _)| *c == codigo)
        .map_or("Desconocido", |(_, nombre)| nombre)
}

/// La placa por la segunda letra del número de serie.
pub fn placa(letra: u8) -> String {
    if (b'1'..=b'5').contains(&letra) {
        format!("BDM-0{}0", letra as char)
    } else {
        "Desconocida".to_string()
    }
}

pub(crate) fn u16_en(datos: &[u8], posicion: usize) -> u16 {
    u16::from_le_bytes([datos[posicion], datos[posicion + 1]])
}

pub(crate) fn u32_en(datos: &[u8], posicion: usize) -> u32 {
    u32::from_le_bytes([
        datos[posicion],
        datos[posicion + 1],
        datos[posicion + 2],
        datos[posicion + 3],
    ])
}

pub(crate) fn u64_en(datos: &[u8], posicion: usize) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&datos[posicion..posicion + 8]);
    u64::from_le_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texto_quita_el_final_y_cambia_lo_que_no_es_ascii() {
        assert_eq!(texto(b"ABC 12\0\0  \0"), "ABC 12");
        assert_eq!(texto(&[b'A', 0xC3, b'B', 0]), "A?B");
        assert_eq!(texto(&[0, 0, 0]), "");
        // Los ceros de en medio se quedan, como en C#.
        assert_eq!(texto(b"A\0B"), "A\0B");
    }

    #[test]
    fn hexadecimal() {
        assert_eq!(hex(&[0x0A, 0xFF, 0x00]), "0AFF00");
        assert_eq!(hex(&[]), "");
        assert_eq!(hex_invertido(&[0x01, 0x02, 0xAB]), "0xAB0201");
    }

    #[test]
    fn version_en_tres_partes() {
        assert_eq!(tres_partes(0x0104_0010), "1.4.16");
        assert_eq!(tres_partes(0), "0.0.0");
        assert_eq!(tres_partes(u32::MAX), "255.255.65535");
    }

    #[test]
    fn direccion_bluetooth() {
        assert_eq!(
            mac(&[0x01, 0x02, 0x03, 0x04, 0x05, 0xAF]),
            "AF:05:04:03:02:01"
        );
    }

    #[test]
    fn voltaje_redondea_como_net() {
        assert_eq!(voltaje(3855), "3,86 V");
        assert_eq!(voltaje(3854), "3,85 V");
        assert_eq!(voltaje(4000), "4,00 V");
        assert_eq!(voltaje(5), "0,01 V");
        assert_eq!(voltaje(0), "0,00 V");
        assert_eq!(voltaje(u16::MAX), "65,54 V");
    }

    #[test]
    fn colores_y_placas() {
        assert_eq!(color("02"), "Rojo cósmico");
        assert_eq!(color("Z1"), "God of War Ragnarök");
        assert_eq!(color("ZE"), "Genshin Impact");
        assert_eq!(color("99"), "Desconocido");
        assert_eq!(placa(b'1'), "BDM-010");
        assert_eq!(placa(b'5'), "BDM-050");
        assert_eq!(placa(b'0'), "Desconocida");
        assert_eq!(placa(b'6'), "Desconocida");
    }

    #[test]
    fn numeros_en_orden_little_endian() {
        let datos = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        assert_eq!(u16_en(&datos, 1), 0x0302);
        assert_eq!(u32_en(&datos, 0), 0x0403_0201);
        assert_eq!(u64_en(&datos, 1), 0x0908_0706_0504_0302);
    }
}
