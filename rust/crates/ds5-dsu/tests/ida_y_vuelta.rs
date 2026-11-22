//! El servidor de verdad en 127.0.0.1 con un puerto libre y un cliente que le habla por UDP.

use std::net::UdpSocket;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use ds5_dsu::crc::crc32;
use ds5_dsu::paquetes::{MENSAJE_DATOS, MENSAJE_PUERTOS, MENSAJE_VERSION};
use ds5_dsu::{EstadoRanura, Meta, Modelo, NivelBateria, Servidor, Suavizado, TipoConexion};
use ds5_informes::{boton, Conexion, Estado, Movimiento};

const MAC: [u8; 6] = [0xA0, 0xB1, 0xC2, 0xD3, 0xE4, 0xF5];

/// Arma una petición de cliente con su CRC.
fn peticion(cuerpo: &[u8]) -> Vec<u8> {
    let mut p = b"DSUC".to_vec();
    p.extend_from_slice(&1001u16.to_le_bytes());
    p.extend_from_slice(&(cuerpo.len() as u16).to_le_bytes());
    p.extend_from_slice(&[0; 4]);
    p.extend_from_slice(&0x5555_AAAAu32.to_le_bytes());
    p.extend_from_slice(cuerpo);
    let crc = crc32(&p);
    p[8..12].copy_from_slice(&crc.to_le_bytes());
    p
}

fn cliente(servidor: &Servidor) -> UdpSocket {
    let c = UdpSocket::bind("127.0.0.1:0").unwrap();
    c.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    c.connect(servidor.direccion()).unwrap();
    c
}

fn recibir(c: &UdpSocket) -> Vec<u8> {
    let mut b = [0u8; 1024];
    let n = c.recv(&mut b).expect("el servidor no ha respondido");
    b[..n].to_vec()
}

/// Comprueba la cabecera y el CRC de una respuesta.
fn comprobar(p: &[u8], servidor: &Servidor, tipo: u32) {
    assert_eq!(&p[..4], b"DSUS");
    assert_eq!(u16::from_le_bytes([p[4], p[5]]), 1001);
    assert_eq!(u16::from_le_bytes([p[6], p[7]]) as usize, p.len() - 16);
    let mut sin_crc = p.to_vec();
    sin_crc[8..12].fill(0);
    assert_eq!(
        crc32(&sin_crc),
        u32::from_le_bytes(p[8..12].try_into().unwrap())
    );
    assert_eq!(
        u32::from_le_bytes(p[12..16].try_into().unwrap()),
        servidor.id()
    );
    assert_eq!(u32::from_le_bytes(p[16..20].try_into().unwrap()), tipo);
}

fn estado() -> Estado {
    Estado {
        botones: boton::EQUIS,
        lx: 128,
        ly: 0,
        rx: 128,
        ry: 128,
        bateria: 100,
        contador_paquetes: 42,
        total_us: 123_456,
        transcurrido: 0.004,
        movimiento: Some(Movimiento {
            vel_yaw: 90.0,
            acel_z_g: 1.0,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Repite el envío hasta que el servidor haya apuntado al cliente: el registro va por otro
/// hilo.
fn enviar_hasta_que_llegue(s: &Servidor, meta: &Meta, e: &Estado) {
    let limite = Instant::now() + Duration::from_secs(2);
    while s.enviar_estado(meta, e) == 0 {
        assert!(Instant::now() < limite, "el cliente no se ha apuntado");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn version_puertos_y_datos() {
    let s = Servidor::arrancar("127.0.0.1", 0).unwrap();
    assert_ne!(s.direccion().port(), 0);
    let c = cliente(&s);

    // Versión.
    c.send(&peticion(&MENSAJE_VERSION.to_le_bytes())).unwrap();
    let p = recibir(&c);
    assert_eq!(p.len(), 24);
    comprobar(&p, &s, MENSAJE_VERSION);
    assert_eq!(&p[20..22], &1001u16.to_le_bytes());

    // Una petición con el CRC mal no tiene respuesta: la siguiente que llega es la buena.
    let mut mala = peticion(&MENSAJE_VERSION.to_le_bytes());
    mala[8] ^= 0xFF;
    c.send(&mala).unwrap();

    // Puertos: la ranura 0 con mando y la 1 vacía.
    let e = estado();
    let meta = Meta::de_mando(0, MAC, Conexion::Bt, true, &e);
    s.actualizar_meta(meta);
    let mut cuerpo = MENSAJE_PUERTOS.to_le_bytes().to_vec();
    cuerpo.extend_from_slice(&2i32.to_le_bytes());
    cuerpo.extend_from_slice(&[0, 1]);
    c.send(&peticion(&cuerpo)).unwrap();
    let p0 = recibir(&c);
    assert_eq!(p0.len(), 32);
    comprobar(&p0, &s, MENSAJE_PUERTOS);
    assert_eq!(p0[20], 0);
    assert_eq!(p0[21], EstadoRanura::Conectado as u8);
    assert_eq!(p0[22], Modelo::Ds4 as u8);
    assert_eq!(p0[23], TipoConexion::Bluetooth as u8);
    assert_eq!(&p0[24..30], &MAC);
    assert_eq!(p0[30], NivelBateria::Llena as u8);
    let p1 = recibir(&c);
    comprobar(&p1, &s, MENSAJE_PUERTOS);
    assert_eq!(&p1[20..32], &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

    // Sin suscripción no se manda nada.
    assert_eq!(s.enviar_estado(&meta, &e), 0);

    // Suscripción a la ranura 0.
    let mut cuerpo = MENSAJE_DATOS.to_le_bytes().to_vec();
    cuerpo.extend_from_slice(&[0x01, 0, 0, 0, 0, 0, 0, 0]);
    c.send(&peticion(&cuerpo)).unwrap();
    enviar_hasta_que_llegue(&s, &meta, &e);
    assert_eq!(s.clientes(), 1);
    let d = recibir(&c);
    assert_eq!(d.len(), 100);
    comprobar(&d, &s, MENSAJE_DATOS);
    assert_eq!(d[20], 0);
    assert_eq!(d[31], 1, "activo: tiene la equis pulsada");
    assert_eq!(u32::from_le_bytes(d[32..36].try_into().unwrap()), 42);
    assert_eq!(d[37], 0x40);
    assert_eq!(d[41], 255, "el eje Y va invertido");
    assert_eq!(u64::from_le_bytes(d[68..76].try_into().unwrap()), 123_456);
    assert_eq!(f32::from_le_bytes(d[84..88].try_into().unwrap()), -1.0);
    assert_eq!(f32::from_le_bytes(d[92..96].try_into().unwrap()), 90.0);

    // Otra ranura no le interesa.
    let otra = Meta::de_mando(1, [1; 6], Conexion::Usb, true, &e);
    assert_eq!(s.enviar_estado(&otra, &e), 0);
    // Una ranura que no existe se ignora.
    assert_eq!(s.enviar_estado(&Meta::desconectado(9), &e), 0);

    s.parar();
    assert!(!s.en_marcha());
    assert_eq!(s.enviar_estado(&meta, &e), 0);
    // Parar dos veces no pasa nada.
    s.parar();
}

#[test]
fn suscripcion_a_todos_desde_otro_hilo_y_suavizado() {
    let s = Arc::new(Servidor::arrancar("", 0).unwrap());
    s.fijar_suavizado(
        true,
        Suavizado {
            corte_minimo: 0.4,
            beta: 0.0,
        },
    );
    let c = cliente(&s);
    let mut cuerpo = MENSAJE_DATOS.to_le_bytes().to_vec();
    cuerpo.extend_from_slice(&[0; 8]);
    c.send(&peticion(&cuerpo)).unwrap();

    let e = estado();
    let meta = Meta::de_mando(3, MAC, Conexion::Usb, true, &e);

    // El hilo del mando.
    let s2 = Arc::clone(&s);
    let (m2, e2) = (meta, e.clone());
    thread::spawn(move || enviar_hasta_que_llegue(&s2, &m2, &e2))
        .join()
        .unwrap();
    let d = recibir(&c);
    assert_eq!(d[20], 3);
    // El primer valor del filtro pasa tal cual.
    assert_eq!(f32::from_le_bytes(d[92..96].try_into().unwrap()), 90.0);

    // El segundo ya va suavizado hacia el nuevo valor.
    let mut e = e;
    e.movimiento.as_mut().unwrap().vel_yaw = 0.0;
    assert_eq!(s.enviar_estado(&meta, &e), 1);
    let d = recibir(&c);
    let yaw = f32::from_le_bytes(d[92..96].try_into().unwrap());
    assert!(yaw > 0.0 && yaw < 90.0, "{yaw}");

    // Sin tiempo entre informes, con el suavizado encendido no se manda.
    e.transcurrido = 0.0;
    assert_eq!(s.enviar_estado(&meta, &e), 0);
    // Apagado sí.
    s.fijar_suavizado(false, Suavizado::default());
    assert_eq!(s.enviar_estado(&meta, &e), 1);
    let d = recibir(&c);
    assert_eq!(f32::from_le_bytes(d[92..96].try_into().unwrap()), 0.0);
}

#[test]
fn el_puerto_ocupado_da_error() {
    let s = Servidor::arrancar("127.0.0.1", 0).unwrap();
    assert!(Servidor::arrancar("127.0.0.1", s.direccion().port()).is_err());
}
