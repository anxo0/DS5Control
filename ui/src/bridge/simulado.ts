import type { ControlMando, EstadoApp, EstadoMando, Eventos, Comando, Puente } from './tipos'

function estadoInicial(): EstadoApp {
  return {
    mandos: [
      {
        id: 'sim-1',
        nombre: 'DualSense',
        tipo: 'dualsense',
        conexion: 'bluetooth',
        conectado: true,
        bateria: 78,
        cargando: false,
        emulando: 'Xbox 360',
        luzActual: '#1F4FE0',
        pulsados: [],
        ajustes: {
          luz: '#1F4FE0',
          brillo: 80,
          luzSegunBateria: false,
          vibracion: 70,
          gatilloL2: 'rigido',
          gatilloR2: 'pulso',
        },
      },
    ],
    abrirConMute: true,
    ventanaVisible: true,
    servicios: { vigemBus: true, hidHide: true },
  }
}

/** Rojo con poca batería, verde con mucha. */
function colorSegunBateria(bateria: number): string {
  const t = Math.min(Math.max(bateria, 0), 100) / 100
  const canal = (a: number, b: number) =>
    Math.round(a + (b - a) * t).toString(16).padStart(2, '0')
  return `#${canal(0xe8, 0x00)}${canal(0x25, 0xb8)}${canal(0x4f, 0x5e)}`.toUpperCase()
}

export function crearPuenteSimulado(): Puente {
  let estado = estadoInicial()
  let eventos: Eventos | undefined

  const emitir = () => eventos?.alEstado(structuredClone(estado))

  const conMando = (mandoId: string | undefined, cambio: (m: EstadoMando) => void) => {
    const mando = mandoId ? estado.mandos.find((m) => m.id === mandoId) : estado.mandos[0]
    if (!mando) return
    cambio(mando)
    const { luz, luzSegunBateria } = mando.ajustes
    mando.luzActual = luzSegunBateria ? colorSegunBateria(mando.bateria ?? 0) : luz
    emitir()
  }

  const pulsar = (control: ControlMando, activo: boolean) =>
    conMando(undefined, (m) => {
      const resto = m.pulsados.filter((c) => c !== control)
      m.pulsados = activo ? [...resto, control] : resto
    })

  const mute = () => {
    pulsar('mute', true)
    setTimeout(() => pulsar('mute', false), 220)
    if (estado.abrirConMute) {
      estado.ventanaVisible = !estado.ventanaVisible
      emitir()
    }
    eventos?.alMute()
  }

  return {
    entrada: 'gamepad',
    depuracion: {
      pulsar,
      mute,
      fijarBateria: (valor) => conMando(undefined, (m) => { m.bateria = valor }),
      fijarCargando: (activo) => conMando(undefined, (m) => { m.cargando = activo }),
      fijarConectado: (activo) => conMando(undefined, (m) => { m.conectado = activo }),
      quitarMandos: () => {
        estado.mandos = []
        emitir()
      },
      restablecer: () => {
        estado = estadoInicial()
        emitir()
      },
    },
    iniciar(e) {
      eventos = e
      emitir()
    },
    enviar(comando: Comando) {
      switch (comando.tipo) {
        case 'luz':
          conMando(comando.mandoId, (m) => { m.ajustes.luz = comando.color })
          break
        case 'brillo':
          conMando(comando.mandoId, (m) => { m.ajustes.brillo = comando.valor })
          break
        case 'luzSegunBateria':
          conMando(comando.mandoId, (m) => { m.ajustes.luzSegunBateria = comando.activo })
          break
        case 'vibracion':
          conMando(comando.mandoId, (m) => { m.ajustes.vibracion = comando.fuerza })
          break
        case 'gatillo':
          conMando(comando.mandoId, (m) => {
            m.ajustes[comando.gatillo === 'l2' ? 'gatilloL2' : 'gatilloR2'] = comando.efecto
          })
          break
        case 'abrirConMute':
          estado.abrirConMute = comando.activo
          emitir()
          break
        case 'ocultar':
          estado.ventanaVisible = false
          emitir()
          break
        case 'probarVibracion':
        case 'probarGatillos':
          break
      }
    },
  }
}
