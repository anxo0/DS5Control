//! El script de PowerShell que sustituye los archivos y vuelve a abrir el programa.
//!
//! El programa en marcha no puede sobrescribir sus propios archivos, así que el script espera a
//! que se cierre, copia los archivos nuevos y lo abre.

use std::ffi::OsString;
use std::path::Path;

/// El nombre del script, que se escribe junto a la carpeta con los archivos nuevos.
pub const NOMBRE_SCRIPT: &str = "instalar.ps1";

/// El mismo script que la versión en C#, con los finales de línea CRLF que tiene en las
/// releases (se compilan en Windows con `core.autocrlf`) y sin salto de línea al final.
pub const SCRIPT_INSTALACION: &str = concat!(
    "param([int]$Proceso, [string]$Origen, [string]$Destino, [string]$Programa, [switch]$Bandeja)\r\n",
    "Wait-Process -Id $Proceso -Timeout 60 -ErrorAction SilentlyContinue\r\n",
    "for ($intento = 0; $intento -lt 10; $intento++) {\r\n",
    "    robocopy $Origen $Destino /E /IS /IT /R:2 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null\r\n",
    "    if ($LASTEXITCODE -lt 8) { break }\r\n",
    "    Start-Sleep -Seconds 1\r\n",
    "}\r\n",
    "$argumentos = @{ FilePath = (Join-Path $Destino $Programa); WorkingDirectory = $Destino }\r\n",
    "if ($Bandeja) { $argumentos.ArgumentList = '-m' }\r\n",
    "Start-Process @argumentos",
);

/// El ejecutable con el que se lanza el script.
pub const POWERSHELL: &str = "powershell.exe";

/// Los argumentos de `powershell.exe`, en el mismo orden que en C#. `bandeja` añade
/// `-Bandeja`, que reabre el programa en la bandeja para no tapar el juego al que se jugaba.
pub fn argumentos_instalacion(
    script: &Path,
    nuevos: &Path,
    destino: &Path,
    proceso: u32,
    programa: &str,
    bandeja: bool,
) -> Vec<OsString> {
    let mut argumentos: Vec<OsString> = vec![
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-ExecutionPolicy".into(),
        "Bypass".into(),
        "-File".into(),
        script.into(),
        "-Proceso".into(),
        proceso.to_string().into(),
        "-Origen".into(),
        nuevos.into(),
        "-Destino".into(),
        destino.into(),
        "-Programa".into(),
        programa.into(),
    ];
    if bandeja {
        argumentos.push("-Bandeja".into());
    }
    argumentos
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// El script de `AppUpdater.InstallScript` tal como queda tras quitar la sangría del literal.
    const ORIGINAL: &str = r#"param([int]$Proceso, [string]$Origen, [string]$Destino, [string]$Programa, [switch]$Bandeja)
Wait-Process -Id $Proceso -Timeout 60 -ErrorAction SilentlyContinue
for ($intento = 0; $intento -lt 10; $intento++) {
    robocopy $Origen $Destino /E /IS /IT /R:2 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null
    if ($LASTEXITCODE -lt 8) { break }
    Start-Sleep -Seconds 1
}
$argumentos = @{ FilePath = (Join-Path $Destino $Programa); WorkingDirectory = $Destino }
if ($Bandeja) { $argumentos.ArgumentList = '-m' }
Start-Process @argumentos"#;

    #[test]
    fn el_script_es_el_de_csharp() {
        let sin_cr = SCRIPT_INSTALACION.replace("\r\n", "\n");
        assert_eq!(sin_cr, ORIGINAL.replace("\r\n", "\n"));
        assert!(!SCRIPT_INSTALACION.contains("\n\n"));
        assert_eq!(SCRIPT_INSTALACION.matches("\r\n").count(), 9);
        assert!(!SCRIPT_INSTALACION.replace("\r\n", "").contains('\n'));
        assert!(!SCRIPT_INSTALACION.ends_with('\n'));
    }

    /// Si sigue estando la versión en C#, saca de ella el literal del script y lo compara.
    #[test]
    fn el_script_sigue_igual_que_en_el_codigo_de_csharp() {
        let ruta = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../DS5Control/Interfaz/AppUpdater.cs");
        let Ok(codigo) = std::fs::read_to_string(ruta) else {
            return;
        };
        let codigo = codigo.replace("\r\n", "\n");
        let inicio = codigo.find("InstallScript = \"\"\"\n").unwrap() + 20;
        let fin = inicio + codigo[inicio..].find("\"\"\";").unwrap();
        let bloque = &codigo[inicio..fin];
        // La sangría del literal es la de las comillas que lo cierran.
        let sangria = bloque.len() - bloque.trim_end_matches(' ').len();
        let lineas: Vec<&str> = bloque
            .trim_end_matches(' ')
            .trim_end_matches('\n')
            .split('\n')
            .map(|linea| &linea[sangria.min(linea.len())..])
            .collect();
        assert_eq!(lineas.join("\r\n"), SCRIPT_INSTALACION);
    }

    #[test]
    fn los_argumentos_siguen_el_orden_de_csharp() {
        let argumentos = argumentos_instalacion(
            Path::new(r"C:\t\instalar.ps1"),
            Path::new(r"C:\t\nueva"),
            Path::new(r"C:\Programas\DS5 Control"),
            1234,
            "DS5Control.exe",
            false,
        );
        let esperados = [
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            r"C:\t\instalar.ps1",
            "-Proceso",
            "1234",
            "-Origen",
            r"C:\t\nueva",
            "-Destino",
            r"C:\Programas\DS5 Control",
            "-Programa",
            "DS5Control.exe",
        ];
        assert_eq!(argumentos, esperados.map(OsString::from));

        let con_bandeja =
            argumentos_instalacion(Path::new("s"), Path::new("n"), Path::new("d"), 1, "p", true);
        assert_eq!(con_bandeja.last().unwrap(), "-Bandeja");
        assert_eq!(con_bandeja.len(), 15);
    }
}
