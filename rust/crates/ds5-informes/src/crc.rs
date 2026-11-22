//! El CRC32 de los informes por Bluetooth. Es el CRC32 normal (el de zip) calculado sobre un byte
//! de cabecera y el informe sin sus cuatro últimos bytes, que es donde va el resultado.

/// Cabecera de los informes de entrada.
pub const CABECERA_ENTRADA: u8 = 0xA1;
/// Cabecera de los informes de salida.
pub const CABECERA_SALIDA: u8 = 0xA2;
/// Cabecera de los informes de características (la calibración).
pub const CABECERA_CARACTERISTICA: u8 = 0xA3;

const TABLA: [u32; 256] = {
    let mut tabla = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        tabla[i] = c;
        i += 1;
    }
    tabla
};

/// El CRC32 de `cabecera` seguido de `datos`.
pub fn crc32(cabecera: u8, datos: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in std::iter::once(&cabecera).chain(datos) {
        c = TABLA[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

/// Si los cuatro últimos bytes de `informe` son su CRC.
pub fn valido(cabecera: u8, informe: &[u8]) -> bool {
    if informe.len() < 4 {
        return false;
    }
    let (datos, crc) = informe.split_at(informe.len() - 4);
    crc32(cabecera, datos) == u32::from_le_bytes([crc[0], crc[1], crc[2], crc[3]])
}

/// Escribe el CRC en los cuatro últimos bytes de `informe`.
pub fn firmar(cabecera: u8, informe: &mut [u8]) {
    let n = informe.len() - 4;
    let crc = crc32(cabecera, &informe[..n]);
    informe[n..].copy_from_slice(&crc.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coincide_con_la_semilla_de_c() {
        // `HamSeed` en DS5Device.cs: el estado del CRC después de la cabecera 0xA1, sin invertir.
        let mut c = !0u32;
        c = TABLA[((c ^ 0xA1) & 0xFF) as usize] ^ (c >> 8);
        assert_eq!(c, 2_351_727_372);
    }

    #[test]
    fn crc_conocido() {
        // CRC32 de "123456789" con la cabecera '1'.
        assert_eq!(crc32(b'1', b"23456789"), 0xCBF4_3926);
    }
}
