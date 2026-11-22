//! La parte en disco: comprobar y descomprimir la descarga, lanzar el script de instalación y
//! el aviso que deja el programa antes de cerrarse para actualizarse.

use std::collections::hash_map::RandomState;
use std::collections::VecDeque;
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use sha2::{Digest, Sha256};

use crate::error::ErrorActualizacion;
use crate::release::PROGRAMA;
use crate::script::{argumentos_instalacion, NOMBRE_SCRIPT, POWERSHELL, SCRIPT_INSTALACION};
use crate::version::Version;

/// La carpeta de trabajo dentro de la carpeta temporal del usuario.
pub const NOMBRE_CARPETA_TRABAJO: &str = "DS5Control-actualizacion";

/// El zip descargado, dentro de la carpeta de trabajo.
pub const NOMBRE_ZIP: &str = "actualizacion.zip";

/// La carpeta con los archivos nuevos, dentro de la carpeta de trabajo.
pub const NOMBRE_CARPETA_NUEVA: &str = "nueva";

/// El aviso que se deja en la carpeta de ajustes antes de cerrarse para actualizarse.
pub const NOMBRE_AVISO: &str = "Actualizacion.txt";

/// La carpeta de trabajo por defecto: `%TEMP%\DS5Control-actualizacion`.
pub fn carpeta_trabajo_por_defecto() -> PathBuf {
    std::env::temp_dir().join(NOMBRE_CARPETA_TRABAJO)
}

/// Borra la carpeta de trabajo si ya estaba y la crea vacía. Devuelve la ruta del zip.
pub fn preparar_carpeta_trabajo(carpeta: &Path) -> io::Result<PathBuf> {
    if carpeta.is_dir() {
        fs::remove_dir_all(carpeta)?;
    }
    fs::create_dir_all(carpeta)?;
    Ok(carpeta.join(NOMBRE_ZIP))
}

/// La huella SHA-256 de un archivo en hexadecimal y mayúsculas, como `Convert.ToHexString`.
pub fn huella_sha256(archivo: &Path) -> io::Result<String> {
    let mut lector = fs::File::open(archivo)?;
    let mut sha = Sha256::new();
    let mut bloque = vec![0u8; 64 * 1024];
    loop {
        let leidos = lector.read(&mut bloque)?;
        if leidos == 0 {
            break;
        }
        sha.update(&bloque[..leidos]);
    }
    Ok(sha
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect())
}

/// Comprueba la huella del zip ya descargado en la carpeta de trabajo, lo descomprime en
/// `nueva` y devuelve la carpeta que tiene el programa.
pub fn verificar_y_descomprimir(
    carpeta_trabajo: &Path,
    sha256: &str,
) -> Result<PathBuf, ErrorActualizacion> {
    let zip = carpeta_trabajo.join(NOMBRE_ZIP);
    let huella = huella_sha256(&zip)?;
    if !huella.eq_ignore_ascii_case(sha256) {
        return Err(ErrorActualizacion::HuellaDistinta);
    }

    let nueva = carpeta_trabajo.join(NOMBRE_CARPETA_NUEVA);
    descomprimir(&zip, &nueva)?;
    buscar_programa(&nueva)?.ok_or(ErrorActualizacion::SinPrograma)
}

/// Como `ZipFile.ExtractToDirectory`: crea las carpetas, no sobrescribe nada y se niega a sacar
/// una entrada fuera de `destino`.
pub fn descomprimir(zip: &Path, destino: &Path) -> Result<(), ErrorActualizacion> {
    let danado = |error: zip::result::ZipError| match error {
        zip::result::ZipError::Io(error) => ErrorActualizacion::Disco(error),
        otro => ErrorActualizacion::ZipDanado(otro.to_string()),
    };
    let mut archivo = zip::ZipArchive::new(fs::File::open(zip)?).map_err(danado)?;
    fs::create_dir_all(destino)?;

    for indice in 0..archivo.len() {
        let mut entrada = archivo.by_index(indice).map_err(danado)?;
        let Some(relativa) = entrada.enclosed_name() else {
            return Err(ErrorActualizacion::Disco(io::Error::other(format!(
                "Extraer la entrada {} la sacaría de la carpeta de destino.",
                entrada.name().unwrap_or_default()
            ))));
        };
        let ruta = destino.join(relativa);
        let nombre = entrada.name().unwrap_or_default();
        if nombre.ends_with('/') || nombre.ends_with('\\') {
            if entrada.size() != 0 {
                return Err(ErrorActualizacion::Disco(io::Error::other(
                    "Una carpeta del zip trae datos.",
                )));
            }
            fs::create_dir_all(&ruta)?;
            continue;
        }
        if let Some(padre) = ruta.parent() {
            fs::create_dir_all(padre)?;
        }
        let mut salida = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&ruta)?;
        io::copy(&mut entrada, &mut salida).map_err(|error| {
            // Los fallos al descomprimir llegan como errores de lectura.
            if error.kind() == io::ErrorKind::InvalidData {
                ErrorActualizacion::ZipDanado(error.to_string())
            } else {
                ErrorActualizacion::Disco(error)
            }
        })?;
    }
    Ok(())
}

/// La primera carpeta que tiene el programa, buscando como `Directory.GetFiles` con
/// `AllDirectories`: primero los archivos de cada carpeta y luego sus subcarpetas, por capas.
pub fn buscar_programa(carpeta: &Path) -> io::Result<Option<PathBuf>> {
    let mut pendientes = VecDeque::from([carpeta.to_path_buf()]);
    while let Some(actual) = pendientes.pop_front() {
        for entrada in fs::read_dir(&actual)? {
            let entrada = entrada?;
            let tipo = entrada.file_type()?;
            if tipo.is_dir() {
                pendientes.push_back(entrada.path());
            } else if tipo.is_file() {
                let nombre = entrada.file_name();
                let nombre = nombre.to_string_lossy();
                // Windows no distingue mayúsculas en los nombres de archivo.
                if nombre.eq_ignore_ascii_case(PROGRAMA) {
                    return Ok(Some(actual));
                }
            }
        }
    }
    Ok(None)
}

/// Si se puede escribir en la carpeta del programa: crea un archivo vacío con nombre al azar y
/// lo borra.
pub fn puede_escribir_en(carpeta: &Path) -> bool {
    let prueba = carpeta.join(nombre_al_azar());
    fs::write(&prueba, []).is_ok() && fs::remove_file(&prueba).is_ok()
}

/// Un nombre como los de `Path.GetRandomFileName`: ocho letras o cifras, punto y tres más.
fn nombre_al_azar() -> String {
    const LETRAS: &[u8] = b"abcdefghijklmnopqrstuvwxyz012345";
    let mut azar = RandomState::new().build_hasher();
    azar.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    );
    let mut bits = azar.finish();
    let mut nombre = String::with_capacity(12);
    for posicion in 0..11 {
        if posicion == 8 {
            nombre.push('.');
        }
        nombre.push(LETRAS[(bits & 31) as usize] as char);
        bits >>= 5;
    }
    nombre
}

/// Escribe el script junto a la carpeta con los archivos nuevos y lo lanza sin ventana. Quien
/// llama tiene que cerrarse para que siga. `bandeja` reabre el programa en la bandeja, para que
/// no tape el juego al que se estaba jugando.
pub fn iniciar_instalacion(
    nuevos: &Path,
    destino: &Path,
    proceso: u32,
    programa: &str,
    bandeja: bool,
) -> io::Result<Child> {
    let completa = std::path::absolute(nuevos)?;
    let padre = completa
        .parent()
        .ok_or_else(|| io::Error::other("La carpeta de los archivos nuevos no tiene padre."))?;
    let script = padre.join(NOMBRE_SCRIPT);
    fs::write(&script, SCRIPT_INSTALACION)?;

    let mut orden = Command::new(POWERSHELL);
    orden.args(argumentos_instalacion(
        &script, nuevos, destino, proceso, programa, bandeja,
    ));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        orden.creation_flags(CREATE_NO_WINDOW);
    }
    orden.spawn()
}

/// Deja el aviso antes de cerrarse, para que el programa que se abra después diga que la
/// actualización salió bien: si no, cerrarse y abrirse solo parece un cuelgue. Los fallos se
/// ignoran.
pub fn guardar_aviso(carpeta_ajustes: &Path, version: &Version) {
    let _ = fs::write(carpeta_ajustes.join(NOMBRE_AVISO), version.to_string());
}

/// La versión a la que pasó una actualización, una sola vez y solo si este programa es esa
/// versión. El aviso se borra siempre que se lee.
pub fn tomar_aviso(carpeta_ajustes: &Path, actual: &Version) -> Option<Version> {
    let archivo = carpeta_ajustes.join(NOMBRE_AVISO);
    if !archivo.is_file() {
        return None;
    }
    let bytes = fs::read(&archivo).ok()?;
    fs::remove_file(&archivo).ok()?;
    let version = Version::analizar(texto_de_archivo(&bytes).trim())?;
    version.es_la_misma_que(actual).then_some(version)
}

/// Como `File.ReadAllText`: UTF-8 salvo que empiece por la marca de UTF-16, y sin la marca.
fn texto_de_archivo(bytes: &[u8]) -> String {
    let utf16 = |pares: &mut dyn Iterator<Item = u16>| {
        char::decode_utf16(pares)
            .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    };
    if let Some(resto) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(resto).into_owned()
    } else if let Some(resto) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        utf16(
            &mut resto
                .as_chunks::<2>()
                .0
                .iter()
                .map(|p| u16::from_le_bytes(*p)),
        )
    } else if let Some(resto) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        utf16(
            &mut resto
                .as_chunks::<2>()
                .0
                .iter()
                .map(|p| u16::from_be_bytes(*p)),
        )
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

#[cfg(test)]
pub(crate) mod pruebas {
    use super::*;
    use std::io::{Cursor, Write};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{Duration, Instant};

    /// Una carpeta temporal propia de cada prueba, que se borra al terminar.
    pub(crate) struct Temporal(pub PathBuf);

    impl Temporal {
        pub(crate) fn nueva() -> Temporal {
            static CONTADOR: AtomicU32 = AtomicU32::new(0);
            let carpeta = std::env::temp_dir().join(format!(
                "ds5test-{}-{}-{}",
                std::process::id(),
                CONTADOR.fetch_add(1, Ordering::Relaxed),
                nombre_al_azar()
            ));
            fs::create_dir_all(&carpeta).unwrap();
            Temporal(carpeta)
        }
    }

    impl Drop for Temporal {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Un zip con una sola entrada que dice "nuevo", como `ZipWith` en C#.
    pub(crate) fn zip_con(entrada: &str) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(entrada, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"nuevo").unwrap();
        zip.finish().unwrap().into_inner()
    }

    pub(crate) fn huella(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect()
    }

    fn descomprimir_en(
        trabajo: &Path,
        zip: &[u8],
        sha: &str,
    ) -> Result<PathBuf, ErrorActualizacion> {
        let ruta = preparar_carpeta_trabajo(trabajo).unwrap();
        fs::write(ruta, zip).unwrap();
        verificar_y_descomprimir(trabajo, sha)
    }

    #[test]
    fn descomprime_la_descarga_comprobada() {
        let trabajo = Temporal::nueva();
        let zip = zip_con("DS5Control-0.2.0/DS5Control.exe");

        let carpeta = descomprimir_en(&trabajo.0, &zip, &huella(&zip)).unwrap();

        assert!(carpeta.join(PROGRAMA).is_file());
        assert_eq!(carpeta, trabajo.0.join("nueva").join("DS5Control-0.2.0"));
    }

    #[test]
    fn la_huella_no_distingue_mayusculas() {
        let trabajo = Temporal::nueva();
        let zip = zip_con("DS5Control.exe");
        let carpeta = descomprimir_en(&trabajo.0, &zip, &huella(&zip).to_lowercase()).unwrap();
        assert_eq!(carpeta, trabajo.0.join("nueva"));
    }

    #[test]
    fn rechaza_una_descarga_corrupta() {
        let trabajo = Temporal::nueva();
        let error = descomprimir_en(&trabajo.0, &zip_con("DS5Control.exe"), "00").unwrap_err();
        assert!(matches!(error, ErrorActualizacion::HuellaDistinta));
        assert!(!trabajo.0.join("nueva").exists());
    }

    #[test]
    fn rechaza_una_release_sin_programa() {
        let trabajo = Temporal::nueva();
        let zip = zip_con("otro.exe");
        let error = descomprimir_en(&trabajo.0, &zip, &huella(&zip)).unwrap_err();
        assert!(matches!(error, ErrorActualizacion::SinPrograma));
    }

    #[test]
    fn rechaza_un_zip_que_no_es_zip() {
        let trabajo = Temporal::nueva();
        let basura = b"esto no es un zip";
        let error = descomprimir_en(&trabajo.0, basura, &huella(basura)).unwrap_err();
        assert!(matches!(error, ErrorActualizacion::ZipDanado(_)));
    }

    #[test]
    fn no_saca_entradas_fuera_del_destino() {
        let trabajo = Temporal::nueva();
        let zip = zip_con("../fuera/DS5Control.exe");
        let error = descomprimir_en(&trabajo.0, &zip, &huella(&zip)).unwrap_err();
        assert!(matches!(error, ErrorActualizacion::Disco(_)));
        assert!(!trabajo.0.join("fuera").exists());
    }

    #[test]
    fn vacia_la_carpeta_de_trabajo_antes_de_usarla() {
        let trabajo = Temporal::nueva();
        fs::create_dir_all(trabajo.0.join("nueva/vieja")).unwrap();
        fs::write(trabajo.0.join("nueva/vieja/DS5Control.exe"), "viejo").unwrap();

        let zip = preparar_carpeta_trabajo(&trabajo.0).unwrap();

        assert_eq!(zip, trabajo.0.join("actualizacion.zip"));
        assert_eq!(fs::read_dir(&trabajo.0).unwrap().count(), 0);
    }

    #[test]
    fn busca_el_programa_por_capas_y_sin_mayusculas() {
        let raiz = Temporal::nueva();
        fs::create_dir_all(raiz.0.join("a/b")).unwrap();
        fs::create_dir_all(raiz.0.join("c")).unwrap();
        fs::create_dir_all(raiz.0.join("d/DS5Control.exe")).unwrap();
        fs::write(raiz.0.join("a/b/DS5Control.exe"), "").unwrap();
        fs::write(raiz.0.join("c/ds5control.EXE"), "").unwrap();

        assert_eq!(buscar_programa(&raiz.0).unwrap(), Some(raiz.0.join("c")));
        fs::remove_file(raiz.0.join("c/ds5control.EXE")).unwrap();
        assert_eq!(buscar_programa(&raiz.0).unwrap(), Some(raiz.0.join("a/b")));
        fs::write(raiz.0.join("xDS5Control.exe"), "").unwrap();
        assert_eq!(buscar_programa(&raiz.0).unwrap(), Some(raiz.0.join("a/b")));
    }

    #[test]
    fn sabe_si_puede_escribir() {
        let carpeta = Temporal::nueva();
        assert!(puede_escribir_en(&carpeta.0));
        assert_eq!(fs::read_dir(&carpeta.0).unwrap().count(), 0);
        assert!(!puede_escribir_en(&carpeta.0.join("no-existe")));
    }

    #[test]
    fn los_nombres_al_azar_son_como_los_de_net() {
        let nombre = nombre_al_azar();
        assert_eq!(nombre.len(), 12);
        assert_eq!(nombre.as_bytes()[8], b'.');
        assert_ne!(nombre, nombre_al_azar());
    }

    #[test]
    fn el_aviso_lo_ensena_una_vez_la_version_nueva() {
        let carpeta = Temporal::nueva();
        guardar_aviso(&carpeta.0, &Version::nueva(0, 2, 8));
        assert_eq!(
            fs::read_to_string(carpeta.0.join("Actualizacion.txt")).unwrap(),
            "0.2.8"
        );

        let actual = Version::analizar("0.2.8.0").unwrap();
        assert_eq!(
            tomar_aviso(&carpeta.0, &actual),
            Some(Version::nueva(0, 2, 8))
        );
        assert_eq!(tomar_aviso(&carpeta.0, &Version::nueva(0, 2, 8)), None);
    }

    #[test]
    fn el_aviso_se_tira_si_la_actualizacion_no_se_aplico() {
        let carpeta = Temporal::nueva();
        guardar_aviso(&carpeta.0, &Version::nueva(0, 2, 8));

        assert_eq!(tomar_aviso(&carpeta.0, &Version::nueva(0, 2, 7)), None);
        assert!(!carpeta.0.join("Actualizacion.txt").exists());
    }

    #[test]
    fn el_aviso_admite_espacios_marcas_y_basura() {
        let carpeta = Temporal::nueva();
        let archivo = carpeta.0.join(NOMBRE_AVISO);
        let actual = Version::nueva(1, 0, 0);

        fs::write(&archivo, b"\xEF\xBB\xBF 1.0 \r\n").unwrap();
        assert_eq!(tomar_aviso(&carpeta.0, &actual), Version::analizar("1.0"));

        let utf16: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain("1.0.0".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        fs::write(&archivo, utf16).unwrap();
        assert_eq!(tomar_aviso(&carpeta.0, &actual), Some(actual));

        fs::write(&archivo, "no es versión").unwrap();
        assert_eq!(tomar_aviso(&carpeta.0, &actual), None);
        assert!(!archivo.exists());
    }

    #[test]
    fn guardar_el_aviso_no_falla_aunque_no_se_pueda() {
        let carpeta = Temporal::nueva();
        guardar_aviso(&carpeta.0.join("no-existe"), &Version::nueva(1, 0, 0));
        assert_eq!(
            tomar_aviso(&carpeta.0.join("no-existe"), &Version::nueva(1, 0, 0)),
            None
        );
    }

    /// Espera a que termine un proceso, como `Process.WaitForExit(ms)`.
    fn esperar(proceso: &mut Child, maximo: Duration) -> bool {
        let inicio = Instant::now();
        while inicio.elapsed() < maximo {
            if proceso.try_wait().unwrap().is_some() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        proceso.try_wait().unwrap().is_some()
    }

    fn esperar_archivo(archivo: &Path) -> bool {
        let inicio = Instant::now();
        while !archivo.exists() && inicio.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(50));
        }
        archivo.exists()
    }

    #[cfg(windows)]
    fn sin_ventana(orden: &mut Command) -> &mut Command {
        use std::os::windows::process::CommandExt;
        orden.creation_flags(0x0800_0000)
    }

    #[cfg(windows)]
    #[test]
    fn la_instalacion_espera_sustituye_los_archivos_y_abre_el_programa() {
        let trabajo = Temporal::nueva();
        let nuevos = trabajo.0.join("nueva");
        fs::create_dir_all(&nuevos).unwrap();
        let destino = Temporal::nueva();
        let abierto = destino.0.join("abierto.txt");

        fs::write(destino.0.join("datos.txt"), "viejo").unwrap();
        fs::write(nuevos.join("datos.txt"), "nuevo").unwrap();
        fs::write(
            nuevos.join("lanzar.cmd"),
            format!("@echo listo> \"{}\"", abierto.display()),
        )
        .unwrap();

        let mut en_marcha = sin_ventana(Command::new("ping").args(["-n", "3", "127.0.0.1"]))
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let mut instalacion =
            iniciar_instalacion(&nuevos, &destino.0, en_marcha.id(), "lanzar.cmd", false).unwrap();

        assert!(
            !esperar(&mut instalacion, Duration::from_millis(500)),
            "Tiene que esperar a que se cierre el programa"
        );
        assert_eq!(
            fs::read_to_string(destino.0.join("datos.txt")).unwrap(),
            "viejo"
        );

        assert!(esperar(&mut instalacion, Duration::from_secs(30)));
        assert_eq!(
            fs::read_to_string(destino.0.join("datos.txt")).unwrap(),
            "nuevo"
        );
        assert!(
            esperar_archivo(&abierto),
            "Tiene que abrir el programa nuevo"
        );
        assert!(trabajo.0.join(NOMBRE_SCRIPT).is_file());
        let _ = en_marcha.wait();
    }

    #[cfg(windows)]
    #[test]
    fn la_instalacion_reabre_en_la_bandeja_si_la_ventana_estaba_oculta() {
        let trabajo = Temporal::nueva();
        let nuevos = trabajo.0.join("nueva");
        fs::create_dir_all(&nuevos).unwrap();
        let destino = Temporal::nueva();
        let abierto = destino.0.join("abierto.txt");
        fs::write(
            nuevos.join("lanzar.cmd"),
            format!("@echo %1> \"{}\"", abierto.display()),
        )
        .unwrap();

        let mut terminado = sin_ventana(Command::new("cmd").args(["/c", "exit"]))
            .spawn()
            .unwrap();
        terminado.wait().unwrap();
        let mut instalacion =
            iniciar_instalacion(&nuevos, &destino.0, terminado.id(), "lanzar.cmd", true).unwrap();

        assert!(esperar(&mut instalacion, Duration::from_secs(30)));
        assert!(esperar_archivo(&abierto));
        // El archivo puede existir antes de que termine de escribirse.
        let inicio = Instant::now();
        let mut texto = String::new();
        while inicio.elapsed() < Duration::from_secs(5) {
            texto = fs::read_to_string(&abierto).unwrap_or_default();
            if !texto.trim().is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(texto.trim(), "-m");
    }
}
