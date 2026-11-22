//! Los mensajes que escribe la propia app (registro, notificaciones y bandeja) en inglés y en
//! portugués de Brasil. Se escriben en español en el código y se traducen justo antes de salir,
//! en `App::anotar` y en las notificaciones: cada mensaje se compara con las plantillas y los
//! huecos `{}` (nombres de archivo, versiones, errores) pasan tal cual a la traducción.
//!
//! Los mensajes que llegan ya traducidos del servicio no casan con ninguna plantilla española y
//! salen como vienen.

use ds5_servicio::idioma::{rellenar, Idioma, Tabla};

pub const TEXTOS: Tabla = &[
    // Bandeja y selector de archivos.
    ("Abrir DS5Control", "Open DS5Control", "Abrir o DS5Control"),
    ("Salir", "Exit", "Sair"),
    ("Perfil del mando {}", "Controller {} profile", "Perfil do comando {}"),
    ("Mando {}", "Controller {}", "Comando {}"),
    ("Desconectar", "Disconnect", "Desconectar"),
    ("Atrás", "Back", "Voltar"),
    ("Todos los archivos", "All files", "Todos os arquivos"),
    ("Programas", "Programs", "Programas"),
    ("Juegos", "Games", "Jogos"),
    // Guardar y abrir.
    ("No se pudo guardar {}: {}", "Couldn't save {}: {}", "Não foi possível salvar {}: {}"),
    ("No se pudo guardar: {}", "Couldn't save: {}", "Não foi possível salvar: {}"),
    ("No se pudo borrar el perfil: {}", "Couldn't delete the profile: {}", "Não foi possível excluir o perfil: {}"),
    ("Abriendo {}.", "Opening {}.", "Abrindo {}."),
    ("No se pudo abrir {}.", "Couldn't open {}.", "Não foi possível abrir {}."),
    ("Ajustes en {}", "Settings folder: {}", "Pasta de configurações: {}"),
    (
        "No se pudo cambiar el arranque con Windows: {}",
        "Couldn't change the start with Windows setting: {}",
        "Não foi possível alterar a inicialização com o Windows: {}",
    ),
    (
        "Todavía no está en la versión de Rust: {}",
        "Not available in the Rust version yet: {}",
        "Ainda não disponível na versão em Rust: {}",
    ),
    (
        "Borrados {} archivos que sobraban de la versión en C#",
        "Removed {} leftover files from the C# version",
        "Removidos {} arquivos que sobraram da versão em C#",
    ),
    (
        "No se pudo cambiar el icono del menú Inicio: {}",
        "Couldn't change the Start menu icon: {}",
        "Não foi possível alterar o ícone do menu Iniciar: {}",
    ),
    // Mandos y motor.
    ("Mando {} conectado", "Controller {} connected", "Controle {} conectado"),
    ("Mando {} desconectado", "Controller {} disconnected", "Controle {} desconectado"),
    ("Mando {}: no se pudo abrir {}", "Controller {}: couldn't open {}", "Controle {}: não foi possível abrir {}"),
    (
        "Mando {}: acción sin hacer todavía: {}",
        "Controller {}: action not available yet: {}",
        "Controle {}: ação ainda não disponível: {}",
    ),
    ("Motor reiniciado", "Engine restarted", "Mecanismo reiniciado"),
    ("Motor parado por un autoperfil", "Engine stopped by an automatic profile", "Mecanismo parado por um perfil automático"),
    // Controladores.
    ("Falta el instalador de {} en {}", "The {} installer is missing from {}", "O instalador do {} não está em {}"),
    (
        "No se encuentra el desinstalador de {} en Windows.",
        "Windows doesn't list an uninstaller for {}.",
        "O Windows não encontrou o desinstalador do {}.",
    ),
    ("Desinstalación de {} cancelada.", "{} uninstall cancelled.", "Desinstalação do {} cancelada."),
    (
        "{} desinstalado. Reinicia el PC para acabar.",
        "{} uninstalled. Restart your PC to finish.",
        "{} desinstalado. Reinicie o PC para concluir.",
    ),
    ("{} desinstalado.", "{} uninstalled.", "{} desinstalado."),
    ("{} no se ha desinstalado.", "{} wasn't uninstalled.", "O {} não foi desinstalado."),
    ("Instalación de HidHide cancelada.", "HidHide setup cancelled.", "Instalação do HidHide cancelada."),
    (
        "HidHide instalado. Reinicia el PC para que empiece a funcionar.",
        "HidHide installed. Restart your PC for it to take effect.",
        "HidHide instalado. Reinicie o PC para que ele comece a funcionar.",
    ),
    (
        "HidHide instalado. Apaga y enciende el mando para que los juegos dejen de verlo.",
        "HidHide installed. Turn your controller off and on so games stop seeing it twice.",
        "HidHide instalado. Desligue e ligue o controle para que os jogos parem de vê-lo em dobro.",
    ),
    ("HidHide no se ha instalado.", "HidHide wasn't installed.", "O HidHide não foi instalado."),
    ("Instalación de ViGEmBus cancelada.", "ViGEmBus setup cancelled.", "Instalação do ViGEmBus cancelada."),
    ("ViGEmBus instalado.", "ViGEmBus installed.", "ViGEmBus instalado."),
    (
        "ViGEmBus no se ha instalado. Si lo acabas de instalar, reinicia el PC.",
        "ViGEmBus wasn't installed. If you just installed it, restart your PC.",
        "O ViGEmBus não foi instalado. Se você acabou de instalá-lo, reinicie o PC.",
    ),
    // Actualizaciones.
    ("Tienes la última versión.", "You're on the latest version.", "Você já tem a versão mais recente."),
    (
        "DS5Control se ha actualizado a la versión {}.",
        "DS5Control has been updated to version {}.",
        "O DS5Control foi atualizado para a versão {}.",
    ),
    ("Actualizando a la versión {}...", "Updating to version {}...", "Atualizando para a versão {}..."),
    (
        "Actualizando a la versión {}. DS5Control se cerrará y volverá a abrirse solo en unos segundos.",
        "Updating to version {}. DS5Control will close and reopen on its own in a few seconds.",
        "Atualizando para a versão {}. O DS5Control vai fechar e abrir de novo sozinho em alguns segundos.",
    ),
    (
        "Hay una versión nueva ({}), pero no se puede escribir en {}. Descárgala de {}",
        "Version {} is out, but DS5Control can't write to {}. Download it from {}",
        "A versão {} saiu, mas o DS5Control não consegue gravar em {}. Baixe em {}",
    ),
    ("No se pudo actualizar: {}", "Couldn't update: {}", "Não foi possível atualizar: {}"),
    (
        "Las compilaciones de desarrollo no se actualizan.",
        "Development builds don't update themselves.",
        "Compilações de desenvolvimento não se atualizam.",
    ),
];

/// Los valores de los huecos si `texto` es la plantilla rellena.
fn casar<'a>(plantilla: &str, texto: &'a str) -> Option<Vec<&'a str>> {
    let partes: Vec<&str> = plantilla.split("{}").collect();
    let (primera, resto) = partes.split_first()?;
    let mut queda = texto.strip_prefix(primera)?;
    if resto.is_empty() {
        return queda.is_empty().then(Vec::new);
    }
    let mut valores = Vec::new();
    for (i, parte) in resto.iter().enumerate() {
        let ultima = i == resto.len() - 1;
        let fin = if ultima {
            // El último trozo fijo tiene que cerrar el texto.
            queda.strip_suffix(parte).map(|v| v.len())?
        } else if parte.is_empty() {
            return None;
        } else {
            queda.find(parte)?
        };
        valores.push(&queda[..fin]);
        queda = &queda[fin + parte.len()..];
    }
    Some(valores)
}

/// Un mensaje de la app en `idioma`. Gana la plantilla con más texto fijo, la más concreta.
pub fn traducir(idioma: Idioma, texto: &str) -> String {
    if idioma == Idioma::Es {
        return texto.to_string();
    }
    let fijo = |p: &str| p.len() - 2 * p.matches("{}").count();
    TEXTOS
        .iter()
        .filter_map(|&(es, en, pt)| casar(es, texto).map(|v| (fijo(es), en, pt, v)))
        .max_by_key(|(n, ..)| *n)
        .map(|(_, en, pt, valores)| {
            let destino = if idioma == Idioma::En { en } else { pt };
            let valores: Vec<&dyn std::fmt::Display> = valores
                .iter()
                .map(|v| v as &dyn std::fmt::Display)
                .collect();
            rellenar(destino, &valores)
        })
        .unwrap_or_else(|| texto.to_string())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_tabla_esta_bien() {
        assert_eq!(
            ds5_servicio::idioma::fallos_de_tabla(TEXTOS),
            Vec::<String>::new()
        );
    }

    #[test]
    fn traduce_con_los_huecos() {
        let t = |i, s| traducir(i, s);
        assert_eq!(t(Idioma::En, "Mando 2 conectado"), "Controller 2 connected");
        assert_eq!(
            t(Idioma::Pt, "Mando 1 desconectado"),
            "Controle 1 desconectado"
        );
        assert_eq!(
            t(
                Idioma::En,
                "No se pudo guardar Profiles.xml: acceso denegado"
            ),
            "Couldn't save Profiles.xml: acceso denegado"
        );
        assert_eq!(
            t(Idioma::En, "Mando 1: no se pudo abrir C:\\a.exe"),
            "Controller 1: couldn't open C:\\a.exe"
        );
        assert_eq!(
            t(
                Idioma::Pt,
                "HidHide desinstalado. Reinicia el PC para acabar."
            ),
            "HidHide desinstalado. Reinicie o PC para concluir."
        );
        assert_eq!(
            t(Idioma::En, "ViGEmBus desinstalado."),
            "ViGEmBus uninstalled."
        );
        assert_eq!(
            t(Idioma::En, "Hay una versión nueva (0.3.1), pero no se puede escribir en C:\\P. Descárgala de https://x"),
            "Version 0.3.1 is out, but DS5Control can't write to C:\\P. Download it from https://x"
        );
        assert_eq!(t(Idioma::En, "Salir"), "Exit");
        // Lo que no está en la tabla, o en español, sale igual.
        assert_eq!(
            t(Idioma::En, "Controller 1 connected"),
            "Controller 1 connected"
        );
        assert_eq!(t(Idioma::Es, "Mando 2 conectado"), "Mando 2 conectado");
    }
}
