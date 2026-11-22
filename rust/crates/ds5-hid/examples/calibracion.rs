//! Lista los mandos y lee la calibración del giroscopio (informe de características 0x05).

fn main() {
    let mandos = ds5_hid::enumerar();
    if mandos.is_empty() {
        println!("No hay ningún mando.");
    }
    for info in &mandos {
        println!(
            "{:?} {:?} entrada {} salida {} caracteristica {}\n  {}",
            info.modelo,
            info.conexion,
            info.largo_entrada,
            info.largo_salida,
            info.largo_caracteristica,
            info.ruta
        );
        match ds5_hid::Dispositivo::abrir(info) {
            Ok(d) => {
                let mut cal = [0u8; 41];
                cal[0] = 0x05;
                match d.leer_caracteristica(&mut cal) {
                    Ok(()) => {
                        let hex: String = cal.iter().map(|b| format!("{b:02X}")).collect();
                        let crc = ds5_informes::crc::valido(
                            ds5_informes::crc::CABECERA_CARACTERISTICA,
                            &cal,
                        );
                        println!("  calibracion {hex} (crc {crc})");
                    }
                    Err(e) => println!("  no se pudo leer la calibracion: {e}"),
                }
            }
            Err(e) => println!("  no se pudo abrir: {e}"),
        }
    }
}
