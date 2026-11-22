//! Lo que sale del servicio hacia el mando físico y el virtual, en funciones puras: el refuerzo de
//! la vibración (`SetDevRumble`), los gatillos adaptativos del perfil (`PrepareTriggerEffect`) y el
//! informe del mando virtual con el eje del volante.

use ds5_informes::salida::{Efecto, EfectoGatillo as BytesGatillo};
use ds5_informes::Estado;
use ds5_perfiles::perfil::{EfectoGatillo, EjeVolante as EjePerfil, MandoVirtual, Perfil};
use ds5_virtual::informes::{informe_ds4, poner_volante_ds4};
use ds5_virtual::{EjeVolante, InformeXbox, Tipo};

/// `SetDevRumble`: cada motor por el refuerzo del perfil (en %), sin pasar de 255. Devuelve
/// (derecho rápido, izquierdo lento), y al revés con `InverseRumbleMotors`.
pub fn vibracion_reforzada(grande: u8, pequeno: u8, refuerzo: u8, invertir: bool) -> (u8, u8) {
    let reforzar = |m: u8| ((m as u32 * refuerzo as u32) / 100).min(255) as u8;
    let (ligero, pesado) = (reforzar(pequeno), reforzar(grande));
    if invertir {
        (pesado, ligero)
    } else {
        (ligero, pesado)
    }
}

/// El valor máximo del clic completo: `Math.Max(maxOutput, maxZone) / 100 * 255` en un byte.
fn maximo_gatillo(salida_maxima: f64, zona_maxima: i32) -> u8 {
    (salida_maxima.max(zona_maxima as f64) / 100.0 * 255.0) as u8
}

fn efecto(e: EfectoGatillo, maximo: u8) -> BytesGatillo {
    BytesGatillo::de(match e {
        EfectoGatillo::None => Efecto::Ninguno,
        EfectoGatillo::Rigid => Efecto::Rigido,
        EfectoGatillo::Pulse => Efecto::Pulso,
        EfectoGatillo::FullClick => Efecto::ClicCompleto { maximo },
    })
}

/// Los efectos de L2 y R2 que pide el perfil, como al cargarlo en `ControlService`.
pub fn gatillos(p: &Perfil) -> (BytesGatillo, BytesGatillo) {
    let a = &p.ajustes;
    (
        efecto(
            a.l2_efecto,
            maximo_gatillo(a.l2_salida_maxima, a.l2_zona_maxima),
        ),
        efecto(
            a.r2_efecto,
            maximo_gatillo(a.r2_salida_maxima, a.r2_zona_maxima),
        ),
    )
}

/// El mando virtual que pide el perfil; `None` es sin mando virtual (solo DirectInput).
pub fn tipo_virtual(m: MandoVirtual) -> Option<Tipo> {
    match m {
        MandoVirtual::X360 => Some(Tipo::Xbox360),
        MandoVirtual::DS4 => Some(Tipo::Ds4),
        MandoVirtual::None => None,
    }
}

/// El mando virtual de un perfil: ninguno con `DinputOnly` (el modo nativo de la interfaz).
pub fn virtual_del_perfil(p: &Perfil) -> Option<Tipo> {
    if p.ajustes.solo_dinput {
        None
    } else {
        tipo_virtual(p.ajustes.mando_virtual)
    }
}

/// El eje del volante en el mando virtual. Los de vJoy no se mudan.
pub fn eje_volante(e: EjePerfil) -> Option<EjeVolante> {
    match e {
        EjePerfil::LX => Some(EjeVolante::LX),
        EjePerfil::LY => Some(EjeVolante::LY),
        EjePerfil::RX => Some(EjeVolante::RX),
        EjePerfil::RY => Some(EjeVolante::RY),
        EjePerfil::L2R2 => Some(EjeVolante::L2R2),
        _ => None,
    }
}

/// Un informe para el mando virtual.
#[derive(Debug, Clone, PartialEq)]
pub enum InformeVirtual {
    Xbox(InformeXbox),
    Ds4(Box<[u8; ds5_virtual::informes::LARGO_DS4_EX]>),
}

/// El informe que va al mando virtual a partir del estado ya mapeado.
pub fn informe(tipo: Tipo, e: &Estado, volante: Option<EjeVolante>) -> InformeVirtual {
    match tipo {
        Tipo::Xbox360 => {
            let mut x = InformeXbox::desde(e);
            if let Some(eje) = volante {
                x.poner_volante(eje, e.volante);
            }
            InformeVirtual::Xbox(x)
        }
        Tipo::Ds4 => {
            let mut r = informe_ds4(e);
            if let Some(eje) = volante {
                poner_volante_ds4(&mut r, eje, e.volante);
            }
            InformeVirtual::Ds4(Box::new(r))
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_vibracion_se_refuerza_y_se_corta_en_255() {
        assert_eq!(vibracion_reforzada(100, 50, 100, false), (50, 100));
        assert_eq!(vibracion_reforzada(200, 100, 150, false), (150, 255));
        assert_eq!(vibracion_reforzada(200, 100, 50, true), (100, 50));
        // La división es entera, como en C#: 3 * 50 / 100 = 1.
        assert_eq!(vibracion_reforzada(3, 3, 50, false), (1, 1));
    }

    #[test]
    fn el_clic_completo_usa_la_mayor_de_salida_y_zona() {
        let mut p = Perfil::default();
        p.ajustes.l2_efecto = EfectoGatillo::FullClick;
        p.ajustes.l2_salida_maxima = 50.0;
        p.ajustes.l2_zona_maxima = 80;
        p.ajustes.r2_efecto = EfectoGatillo::Rigid;
        let (l2, r2) = gatillos(&p);
        assert_eq!(l2, BytesGatillo::de(Efecto::ClicCompleto { maximo: 204 }));
        assert_eq!(r2, BytesGatillo::de(Efecto::Rigido));
    }

    #[test]
    fn el_volante_sale_por_su_eje() {
        let e = Estado {
            volante: -1234,
            lx: 128,
            ly: 128,
            rx: 128,
            ry: 128,
            ..Default::default()
        };
        match informe(Tipo::Xbox360, &e, Some(EjeVolante::LX)) {
            InformeVirtual::Xbox(x) => assert_eq!(x.lx, -1234),
            _ => unreachable!(),
        }
        assert_eq!(eje_volante(EjePerfil::VJoy1X), None);
        assert_eq!(tipo_virtual(MandoVirtual::None), None);
    }
}
