//! El teclado y el ratón virtuales con `SendInput`, como `SendInputHandler.cs`: cada función es una
//! llamada de `VirtualKBMBase` y manda lo mismo, con los mismos indicadores y códigos de
//! exploración.

/// `VK_PAUSE`: `MapVirtualKey` no sabe convertirla.
const VK_PAUSE: u32 = 0x13;
const EXTENDIDA: u16 = 0x100;
const MAXIMO_ABSOLUTO: f64 = 65535.0;

const MOVER: u32 = 0x0001;
const ABSOLUTO: u32 = 0x8000;
const ESCRITORIO_VIRTUAL: u32 = 0x4000;
const RUEDA: u32 = 0x0800;
const RUEDA_HORIZONTAL: u32 = 0x1000;
const TECLA_EXTENDIDA: u32 = 0x0001;
const TECLA_SUELTA: u32 = 0x0002;
const TECLA_EXPLORACION: u32 = 0x0008;

/// Las teclas que llevan el bit de tecla extendida, como `scancodeFromVK`.
fn es_extendida(vk: u32) -> bool {
    matches!(
        vk,
        0x25..=0x28 // flechas
            | 0x21..=0x24 // Re Pág, Av Pág, Fin, Inicio
            | 0x2D | 0x2E // Insert, Supr
            | 0x6F | 0x90 // / del teclado numérico, Bloq Num
            | 0xA3 | 0xA5 // Ctrl y Alt derechos
            | 0xAD..=0xAF // volumen
            | 0xB0 | 0xB1 // pista siguiente y anterior
            | 0xB5 | 0xAC | 0xB4 | 0xB6 | 0xB7 // medios, inicio, correo, apps
            | 0x5D // menú contextual
    )
}

#[cfg(windows)]
mod so {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
        KEYBD_EVENT_FLAGS, MAPVK_VK_TO_VSC, MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
    };

    pub fn codigo(vk: u32) -> u16 {
        unsafe { MapVirtualKeyW(vk, MAPVK_VK_TO_VSC) as u16 }
    }

    fn mandar(entradas: &[INPUT]) {
        if !entradas.is_empty() {
            unsafe { SendInput(entradas, std::mem::size_of::<INPUT>() as i32) };
        }
    }

    pub fn raton(lista: &[(i32, i32, u32, u32)]) {
        let v: Vec<INPUT> = lista
            .iter()
            .map(|&(x, y, datos, indicadores)| INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: x,
                        dy: y,
                        mouseData: datos,
                        dwFlags: MOUSE_EVENT_FLAGS(indicadores),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            })
            .collect();
        mandar(&v);
    }

    pub fn tecla(vk: u16, exploracion: u16, indicadores: u32) {
        mandar(&[INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: exploracion,
                    dwFlags: KEYBD_EVENT_FLAGS(indicadores),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }]);
    }
}

#[cfg(not(windows))]
mod so {
    pub fn codigo(_vk: u32) -> u16 {
        0
    }
    pub fn raton(_lista: &[(i32, i32, u32, u32)]) {}
    pub fn tecla(_vk: u16, _exploracion: u16, _indicadores: u32) {}
}

/// `scancodeFromVK`.
pub fn codigo_exploracion(vk: u32) -> u16 {
    let c = if vk == VK_PAUSE { 0x45 } else { so::codigo(vk) };
    if es_extendida(vk) {
        c | EXTENDIDA
    } else {
        c
    }
}

/// `MoveRelativeMouse`: no manda nada si no hay movimiento.
pub fn mover(x: i32, y: i32) {
    if x != 0 || y != 0 {
        so::raton(&[(x, y, 0, MOVER)]);
    }
}

/// `MoveAbsoluteMouse`, de 0 a 1 sobre todo el escritorio.
pub fn mover_absoluto(x: f64, y: f64) {
    so::raton(&[(
        (x * MAXIMO_ABSOLUTO) as i32,
        (y * MAXIMO_ABSOLUTO) as i32,
        0,
        MOVER | ESCRITORIO_VIRTUAL | ABSOLUTO,
    )]);
}

/// `PerformMouseButtonEvent`: las banderas de `SendInput` del botón.
pub fn boton(indicadores: u32) {
    so::raton(&[(0, 0, 0, indicadores)]);
}

/// `PerformMouseButtonEventAlt`: los botones 4 y 5, con el número del botón.
pub fn boton_x(indicadores: u32, cual: i32) {
    so::raton(&[(0, 0, cual as u32, indicadores)]);
}

/// `PerformMouseWheelEvent`: la vertical y la horizontal en una sola llamada.
pub fn rueda(vertical: i32, horizontal: i32) {
    let mut v = Vec::with_capacity(2);
    if vertical != 0 {
        v.push((0, 0, vertical as u32, RUEDA));
    }
    if horizontal != 0 {
        v.push((0, 0, horizontal as u32, RUEDA_HORIZONTAL));
    }
    so::raton(&v);
}

fn indicadores_tecla(exploracion: u16) -> u32 {
    if exploracion & EXTENDIDA != 0 {
        TECLA_EXTENDIDA
    } else {
        0
    }
}

/// `PerformKeyPress`: la tecla virtual con su código de exploración.
pub fn pulsar(vk: u32) {
    let c = codigo_exploracion(vk);
    so::tecla(vk as u16, c, indicadores_tecla(c));
}

/// `PerformKeyRelease`.
pub fn soltar(vk: u32) {
    let c = codigo_exploracion(vk);
    so::tecla(vk as u16, c, indicadores_tecla(c) | TECLA_SUELTA);
}

/// `PerformKeyPressAlt`: solo el código de exploración, para los juegos que leen el teclado así.
pub fn pulsar_exploracion(vk: u32) {
    let c = codigo_exploracion(vk);
    so::tecla(0, c, TECLA_EXPLORACION | indicadores_tecla(c));
}

/// `PerformKeyReleaseAlt`.
pub fn soltar_exploracion(vk: u32) {
    let c = codigo_exploracion(vk);
    so::tecla(
        0,
        c,
        TECLA_EXPLORACION | TECLA_SUELTA | indicadores_tecla(c),
    );
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn las_flechas_y_supr_son_extendidas() {
        for vk in [0x25, 0x26, 0x27, 0x28, 0x2E, 0xA3, 0xA5, 0x5D] {
            assert!(codigo_exploracion(vk) & EXTENDIDA != 0, "{vk:#x}");
        }
        for vk in [0x41, 0x20, 0x0D, 0xA2] {
            assert_eq!(codigo_exploracion(vk) & EXTENDIDA, 0, "{vk:#x}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn los_codigos_de_exploracion_son_los_de_siempre() {
        // A, espacio, Intro y Pausa.
        assert_eq!(codigo_exploracion(0x41), 0x1E);
        assert_eq!(codigo_exploracion(0x20), 0x39);
        assert_eq!(codigo_exploracion(0x0D), 0x1C);
        assert_eq!(codigo_exploracion(VK_PAUSE), 0x45);
        assert_eq!(codigo_exploracion(0x26), 0x148);
    }
}
