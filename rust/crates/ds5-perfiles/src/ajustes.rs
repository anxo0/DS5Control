//! Los ajustes generales de la app (`Profiles.xml`), como `AppSettingsDTO.cs`.

use crate::formato::{
    enumerado, grupo, Anidado, BoolTexto, BoolXml, Doble, Entero, Enumerado, Formato, Grupo, Texto,
};
use crate::numeros;
use crate::xml::{Elemento, Escritor};

/// Un booleano `True`/`False` que, si no se entiende, se queda con el valor por defecto `D`.
pub struct BoolSi<const D: bool>;
impl<const D: bool> Formato for BoolSi<D> {
    type T = bool;
    fn leer(e: &Elemento) -> Result<bool, String> {
        Ok(numeros::bool_net(&e.texto()).unwrap_or(D))
    }
    fn escribir(v: &bool, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], numeros::formato_bool(*v));
    }
}

/// `Notifications`: un número, o `True`/`False` de las versiones antiguas (2 y 0).
pub struct Avisos;
impl Formato for Avisos {
    type T = i32;
    fn leer(e: &Elemento) -> Result<i32, String> {
        let t = e.texto();
        Ok(numeros::entero_en::<i32>(&t)
            .or_else(|| numeros::bool_net(&t).map(|b| if b { 2 } else { 0 }))
            .unwrap_or(2))
    }
    fn escribir(v: &i32, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], &v.to_string());
    }
}

/// Un enum leído con `Enum.TryParse` (nombre exacto o número) que, si no se entiende, vale 0.
/// Se escribe con su nombre, o con el número si no tiene nombre.
pub struct EnumNet<const N: usize>;
pub const ICONOS: [&str; 5] = ["Default", "Colored", "White", "Black", "Battery"];
pub const TEMAS: [&str; 3] = ["Default", "Light", "Dark"];

fn enum_net(texto: &str, nombres: &[&str]) -> Option<u32> {
    let mut valor = 0u32;
    for parte in texto.split(',') {
        let parte = parte.trim();
        let v = if parte.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '+') {
            numeros::entero_en::<u32>(parte)?
        } else {
            nombres.iter().position(|n| *n == parte)? as u32
        };
        valor |= v;
    }
    Some(valor)
}

/// El icono de la bandeja.
pub struct Icono;
impl Formato for Icono {
    type T = u32;
    fn leer(e: &Elemento) -> Result<u32, String> {
        Ok(enum_net(&e.texto(), &ICONOS).unwrap_or(0))
    }
    fn escribir(v: &u32, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], &nombre_o_numero(*v, &ICONOS));
    }
}

/// El tema de la interfaz antigua.
pub struct Tema;
impl Formato for Tema {
    type T = u32;
    fn leer(e: &Elemento) -> Result<u32, String> {
        Ok(enum_net(&e.texto(), &TEMAS).unwrap_or(0))
    }
    fn escribir(v: &u32, nombre: &str, w: &mut Escritor) {
        w.hoja(nombre, &[], &nombre_o_numero(*v, &TEMAS));
    }
}

fn nombre_o_numero(v: u32, nombres: &[&str]) -> String {
    nombres
        .get(v as usize)
        .map_or_else(|| v.to_string(), |n| n.to_string())
}

/// Una fecha y hora, como `DateTime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FechaHora {
    pub anio: u16,
    pub mes: u8,
    pub dia: u8,
    pub hora: u8,
    pub minuto: u8,
    pub segundo: u8,
}

/// `LastChecked`: se escribe como `MM/dd/yyyy HH:mm:ss`. Al leer se aceptan esa forma (con o
/// sin ceros y con o sin hora) y la ISO `yyyy-MM-ddTHH:mm:ss`. Si no se entiende, queda la fecha
/// mínima de .NET, el 1 de enero del año 1.
pub struct Fecha;
impl Formato for Fecha {
    type T = FechaHora;
    fn leer(e: &Elemento) -> Result<FechaHora, String> {
        Ok(leer_fecha(e.texto().trim()).unwrap_or(FECHA_MINIMA))
    }
    fn escribir(v: &FechaHora, nombre: &str, w: &mut Escritor) {
        let texto = format!(
            "{:02}/{:02}/{:04} {:02}:{:02}:{:02}",
            v.mes, v.dia, v.anio, v.hora, v.minuto, v.segundo
        );
        w.hoja(nombre, &[], &texto);
    }
}

pub const FECHA_MINIMA: FechaHora = FechaHora {
    anio: 1,
    mes: 1,
    dia: 1,
    hora: 0,
    minuto: 0,
    segundo: 0,
};

fn leer_fecha(t: &str) -> Option<FechaHora> {
    let (fecha, hora) = match t.split_once([' ', 'T']) {
        Some((f, h)) => (f, Some(h.trim())),
        None => (t, None),
    };
    let numeros_de = |s: &str, sep: char| -> Option<Vec<u32>> {
        s.split(sep).map(|p| p.parse::<u32>().ok()).collect()
    };
    let (anio, mes, dia) = if fecha.contains('/') {
        let p = numeros_de(fecha, '/')?;
        if p.len() != 3 {
            return None;
        }
        (p[2], p[0], p[1])
    } else {
        let p = numeros_de(fecha, '-')?;
        if p.len() != 3 {
            return None;
        }
        (p[0], p[1], p[2])
    };
    let (h, mi, s) = match hora {
        None | Some("") => (0, 0, 0),
        Some(h) => {
            let h = h.split('.').next()?;
            let p = numeros_de(h, ':')?;
            match p.as_slice() {
                [h, m] => (*h, *m, 0),
                [h, m, s] => (*h, *m, *s),
                _ => return None,
            }
        }
    };
    let dias_mes = match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (anio % 4 == 0 && anio % 100 != 0) || anio % 400 == 0 => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=9999).contains(&anio) || dia == 0 || dia > dias_mes || h > 23 || mi > 59 || s > 59 {
        return None;
    }
    Some(FechaHora {
        anio: anio as u16,
        mes: mes as u8,
        dia: dia as u8,
        hora: h as u8,
        minuto: mi as u8,
        segundo: s as u8,
    })
}

/// Un LED personalizado por mando: `True:255,128,0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedPersonalizado {
    pub activo: bool,
    pub color: (u8, u8, u8),
}

impl Default for LedPersonalizado {
    fn default() -> Self {
        LedPersonalizado {
            activo: false,
            color: (0, 0, 255),
        }
    }
}

/// `ParseCustomLedString`.
pub struct Led;
impl Formato for Led {
    type T = LedPersonalizado;
    fn leer(e: &Elemento) -> Result<LedPersonalizado, String> {
        let texto = e.texto();
        let mut led = LedPersonalizado::default();
        let partes: Vec<&str> = texto.split(':').collect();
        led.activo = numeros::bool_net(partes[0]).unwrap_or(false);
        let Some(color) = partes.get(1) else {
            return Ok(LedPersonalizado::default());
        };
        // Como `DS5Color.TryParse`: cada componente que no se entiende queda en 0 y se deja de
        // leer en el primero que falla.
        let componentes: Vec<&str> = color.split(',').collect();
        let destino = [&mut led.color.0, &mut led.color.1, &mut led.color.2];
        for (i, d) in destino.into_iter().enumerate() {
            let Some(c) = componentes.get(i) else { break };
            match numeros::entero_en::<u8>(c) {
                Some(v) => *d = v,
                None => {
                    *d = 0;
                    break;
                }
            }
        }
        Ok(led)
    }
    fn escribir(v: &LedPersonalizado, nombre: &str, w: &mut Escritor) {
        let (r, g, b) = v.color;
        let texto = format!("{}:{r},{g},{b}", numeros::formato_bool(v.activo));
        w.hoja(nombre, &[], &texto);
    }
}

enumerado! { pub enum ModoJoyCon { Split = "Split", Joined = "Joined" } }
enumerado! { pub enum GiroscopioJoyCon { JoyConL = "JoyConL", JoyConR = "JoyConR" } }
enumerado! { pub enum AvisoAutoperfil { None = "None", Log = "Log", Notification = "Notification", LogAndNotification = "LogAndNotification" } }

grupo! {
    pub struct SuavizadoServidorUdp {
        "UseSmoothing" => activo: BoolSi<false> = false;
        "UdpSmoothMinCutoff" => corte_minimo: Doble = 0.4;
        "UdpSmoothBeta" => beta: Doble = 0.2;
    }
}

grupo! { pub struct SoporteDs4 { "Enabled" => activo: BoolSi<true> = true; } }
grupo! { pub struct SoporteDualSense { "Enabled" => activo: BoolSi<false> = false; } }
grupo! { pub struct SoporteSwitchPro { "Enabled" => activo: BoolSi<false> = false; } }
grupo! { pub struct SoporteDs3 { "Enabled" => activo: BoolSi<false> = false; } }
grupo! {
    pub struct SoporteJoyCon {
        "Enabled" => activo: BoolSi<false> = false;
        "LinkMode" => modo: Enumerado<ModoJoyCon> = ModoJoyCon::Joined;
        "JoinedGyroProvider" => giroscopio: Enumerado<GiroscopioJoyCon> = GiroscopioJoyCon::JoyConL;
    }
}

grupo! {
    pub struct OpcionesMandos {
        "DS4SupportSettings" => ds4: Anidado<SoporteDs4> = SoporteDs4::default();
        "DualSenseSupportSettings" => dualsense: Anidado<SoporteDualSense> = SoporteDualSense::default();
        "SwitchProSupportSettings" => switch_pro: Anidado<SoporteSwitchPro> = SoporteSwitchPro::default();
        "JoyConSupportSettings" => joycon: Anidado<SoporteJoyCon> = SoporteJoyCon::default();
        "DS3SupportSettings" => ds3: Anidado<SoporteDs3> = SoporteDs3::default();
    }
}

grupo! {
    /// La primera parte del archivo, hasta los perfiles de cada mando.
    pub struct Ventana {
        "useExclusiveMode" => modo_exclusivo: BoolTexto = false;
        "startMinimized" => empezar_minimizada: BoolTexto = false;
        "minimizeToTaskbar" => minimizar_a_barra: BoolTexto = false;
        "formWidth" => ancho: Entero = 782;
        "formHeight" => alto: Entero = 550;
        "formLocationX" => x: Entero = 0;
        "formLocationY" => y: Entero = 0;
        "ProcessPriority" => prioridad: Entero = 0;
    }
}

grupo! {
    /// La parte del archivo entre los perfiles de cada mando y `LastVersionChecked`.
    pub struct Comprobacion {
        "LastChecked" => ultima_comprobacion: Fecha = FECHA_MINIMA;
        "CheckWhen" => comprobar_cada: Entero = 24;
    }
}

grupo! {
    /// El resto, desde `Notifications` hasta `AutoProfileRevertDefaultProfile`.
    pub struct Generales {
        "Notifications" => avisos: Avisos = 2;
        "ProfileChangedNotification" => aviso_cambio_perfil: BoolSi<false> = false;
        "DisconnectBTAtStop" => desconectar_bt_al_parar: BoolSi<false> = false;
        "SwipeProfiles" => deslizar_perfiles: BoolSi<true> = true;
        "QuickCharge" => carga_rapida: BoolSi<false> = false;
        "UseMoonlight" => moonlight: BoolXml = false;
        "UseAdvancedMoonlight" => moonlight_avanzado: BoolXml = false;
        "CloseMinimizes" => cerrar_minimiza: BoolSi<false> = false;
        "UseLang" => idioma: Texto = String::new();
        "DownloadLang" => descargar_idioma: BoolSi<false> = false;
        "FlashWhenLate" => destello_si_retraso: BoolSi<true> = true;
        "FlashWhenLateAt" => destello_retraso_ms: Entero = 500;
        "AppIcon" => icono: Icono = 0;
        "AppTheme" => tema: Tema = 0;
        "UseOSCServer" => servidor_osc: BoolSi<false> = false;
        "OSCServerPort" => puerto_servidor_osc: Entero = 9000;
        "InterpretingOscMonitoring" => monitor_osc: BoolSi<false> = false;
        "UseOSCSender" => emisor_osc: BoolSi<false> = false;
        "OSCSenderPort" => puerto_emisor_osc: Entero = 9001;
        "OSCSenderAddress" => direccion_emisor_osc: Texto = "127.0.0.1".into();
        "UseUDPServer" => servidor_udp: BoolSi<false> = false;
        "UDPServerPort" => puerto_servidor_udp: Entero = 26760;
        "UDPServerListenAddress" => direccion_servidor_udp: Texto = "127.0.0.1".into();
        "UDPServerSmoothingOptions" => suavizado_udp: Anidado<SuavizadoServidorUdp> = SuavizadoServidorUdp::default();
        "UseCustomSteamFolder" => carpeta_steam_propia: BoolSi<false> = false;
        "CustomSteamFolder" => carpeta_steam: Texto = String::new();
        "AutoProfileRevertDefaultProfile" => autoperfil_vuelve_a_defecto: BoolSi<true> = true;
    }
}

/// `Profiles.xml` completo.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AjustesApp {
    pub ventana: Ventana,
    /// El perfil de cada uno de los 8 mandos; vacío si no tiene.
    pub perfiles: [String; 8],
    pub comprobacion: Comprobacion,
    /// La última versión comprobada; no se escribe si está vacía.
    pub ultima_version: String,
    pub generales: Generales,
    /// Solo se escribe si no es `None`.
    pub aviso_autoperfil: Option<AvisoAutoperfil>,
    /// La pantalla del ratón absoluto (`AbsRegionDisplay`).
    pub pantalla_raton_absoluto: String,
    pub mandos: OpcionesMandos,
    pub leds: [LedPersonalizado; 8],
}

/// La versión actual del formato de los ajustes generales (`Global.APP_CONFIG_VERSION`).
pub const VERSION_AJUSTES: &str = "2";

impl AjustesApp {
    pub fn leer(texto: &str) -> Result<AjustesApp, String> {
        let raiz = Elemento::leer(texto).map_err(|e| e.0)?;
        if raiz.nombre != "Profile" {
            return Err(format!("el elemento raíz es {}", raiz.nombre));
        }
        let mut a = AjustesApp::default();
        a.ventana.leer_campos(&raiz)?;
        a.comprobacion.leer_campos(&raiz)?;
        a.generales.leer_campos(&raiz)?;
        for (i, perfil) in a.perfiles.iter_mut().enumerate() {
            if let Some(e) = raiz.hijo(&format!("Controller{}", i + 1)) {
                *perfil = e.texto_campo();
            }
        }
        if let Some(e) = raiz.hijo("LastVersionChecked") {
            a.ultima_version = e.texto_campo();
        }
        // Una dirección vacía deja la de por defecto.
        let g = &mut a.generales;
        if g.direccion_emisor_osc.is_empty() {
            g.direccion_emisor_osc = "127.0.0.1".into();
        }
        if g.direccion_servidor_udp.is_empty() {
            g.direccion_servidor_udp = "127.0.0.1".into();
        }
        if let Some(e) = raiz.hijo("AutoProfileSwitchNotifyChoice") {
            let aviso = Enumerado::<AvisoAutoperfil>::leer(e)?;
            a.aviso_autoperfil = Some(aviso).filter(|v| *v != AvisoAutoperfil::None);
        }
        if let Some(e) = raiz.hijo("AbsRegionDisplay") {
            a.pantalla_raton_absoluto = e.texto_campo();
        }
        if let Some(e) = raiz.hijo("DeviceOptions") {
            a.mandos = Anidado::<OpcionesMandos>::leer(e)?;
        }
        for (i, led) in a.leds.iter_mut().enumerate() {
            if let Some(e) = raiz.hijo(&format!("CustomLed{}", i + 1)) {
                *led = Led::leer(e)?;
            }
        }
        Ok(a)
    }

    /// El archivo como lo guarda la app. Con encabezado, lleva el de la app
    /// (`Profile Configuration Data`, la versión y los atributos de versión).
    pub fn escribir(&self, encabezado: Option<&crate::Encabezado>) -> String {
        let mut w = Escritor::new();
        w.crudo("<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n");
        let atributos: Vec<(&str, &str)> = match encabezado {
            Some(e) => {
                w.crudo(&format!(
                    "<!-- Profile Configuration Data. {} -->\r\n",
                    e.fecha
                ));
                w.crudo(&format!(
                    "<!-- Made with DS5Control version {} -->\r\n",
                    e.version_app
                ));
                vec![
                    ("app_version", e.version_app.as_str()),
                    ("config_version", VERSION_AJUSTES),
                ]
            }
            None => Vec::new(),
        };
        w.crudo("\r\n");
        w.abrir("Profile", &atributos);
        self.ventana.escribir_campos(&mut w);
        for (i, perfil) in self.perfiles.iter().enumerate() {
            if !perfil.is_empty() {
                w.hoja(&format!("Controller{}", i + 1), &[], perfil);
            }
        }
        self.comprobacion.escribir_campos(&mut w);
        if !self.ultima_version.is_empty() {
            w.hoja("LastVersionChecked", &[], &self.ultima_version);
        }
        self.generales.escribir_campos(&mut w);
        if let Some(aviso) = self.aviso_autoperfil {
            Enumerado::<AvisoAutoperfil>::escribir(&aviso, "AutoProfileSwitchNotifyChoice", &mut w);
        }
        w.hoja("AbsRegionDisplay", &[], &self.pantalla_raton_absoluto);
        Anidado::<OpcionesMandos>::escribir(&self.mandos, "DeviceOptions", &mut w);
        for (i, led) in self.leds.iter().enumerate() {
            Led::escribir(led, &format!("CustomLed{}", i + 1), &mut w);
        }
        w.cerrar();
        w.terminar()
    }
}
