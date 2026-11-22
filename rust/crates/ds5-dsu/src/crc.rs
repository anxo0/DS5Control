//! El CRC32 de los paquetes DSU: el normal (el de zip, polinomio 0xEDB88320), sin cabecera,
//! como `Crc32Algorithm.Compute` en C#.

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

/// El CRC32 de `datos`.
pub fn crc32(datos: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in datos {
        c = TABLA[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valor_de_comprobacion() {
        // El valor de comprobación del CRC32 estándar.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }
}
