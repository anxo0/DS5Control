import type { AccionEspecial, AjusteAvanzado, ControlMando, EstadoApp, EstadoMando, Eventos, Comando, GrupoAvanzado, Puente } from './tipos'

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
        perfil: 'Default',
        mapeo: [],
        sticks: { izq: { x: 0, y: 0 }, der: { x: 0, y: 0 } },
        gatillos: { l2: 0, r2: 0 },
        ajustes: {
          luz: '#1F4FE0',
          brillo: 80,
          luzSegunBateria: false,
          arcoiris: false,
          emulacion: 'xbox',
          vibracion: 70,
          gatilloL2: 'rigido',
          gatilloR2: 'pulso',
        },
      },
    ],
    abrirConMute: true,
    controlConMando: false,
    perfiles: ['Default', 'Carreras'],
    registro: [
      { hora: '18:00:36', texto: 'Mando encontrado (Bluetooth)', aviso: false },
      { hora: '18:00:37', texto: 'Mando virtual conectado', aviso: false },
    ],
    ventanaVisible: true,
    // ?sinvigem enseña el aviso de controlador que falta.
    servicios: { vigemBus: !location.search.includes('sinvigem'), hidHide: true, instalandoVigem: false },
  }
}

/** Rojo con poca batería, verde con mucha. */
function colorSegunBateria(bateria: number): string {
  const t = Math.min(Math.max(bateria, 0), 100) / 100
  const canal = (a: number, b: number) =>
    Math.round(a + (b - a) * t).toString(16).padStart(2, '0')
  return `#${canal(0xe8, 0x00)}${canal(0x25, 0xb8)}${canal(0x4f, 0x5e)}`.toUpperCase()
}

const ajuste = (id: string, nombre: string, tipo: AjusteAvanzado['tipo'], valor: AjusteAvanzado['valor'], extra: Partial<AjusteAvanzado> = {}): AjusteAvanzado => ({
  id, nombre, tipo, valor, min: 0, max: 1, paso: 0.01, porcentaje: false, opciones: null, ...extra,
})

let acciones: AccionEspecial[] = []

const opciones: GrupoAvanzado[] = [
  {
    id: 'sticks',
    nombre: 'Sticks',
    secciones: [
      {
        nombre: 'Stick izquierdo',
        ajustes: [
          ajuste('LSDeadZone', 'Zona muerta', 'numero', 0.1, { porcentaje: true }),
          ajuste('LSSquareStick', 'Recorrido cuadrado', 'interruptor', false),
          ajuste('LSOutputCurveIndex', 'Curva de respuesta', 'opciones', 0, { opciones: ['Lineal', 'Cuadrática', 'Personalizada'] }),
          ajuste('LSCustomCurve', 'Curva', 'curva', '0.25, 0.10, 0.75, 0.90'),
        ],
      },
    ],
  },
  {
    id: 'giroscopio',
    nombre: 'Giroscopio',
    secciones: [
      {
        nombre: 'Ratón con giroscopio',
        ajustes: [ajuste('gyroRaton', 'Se activa con', 'botones', [], { opciones: ['Cruz', 'Círculo', 'L1', 'R1'] })],
      },
    ],
  },
]

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
        case 'emulacion':
          conMando(comando.mandoId, (m) => {
            m.ajustes.emulacion = comando.valor
            m.emulando = comando.valor === 'ds4' ? 'DualShock 4' : 'Xbox 360'
          })
          break
        case 'perfil':
          conMando(comando.mandoId, (m) => { m.perfil = comando.nombre })
          break
        case 'nuevoPerfil':
          estado.perfiles = [...estado.perfiles, comando.nombre]
          break
        case 'borrarPerfil':
          estado.perfiles = estado.perfiles.filter((p) => p !== comando.nombre)
          break
        case 'mapear':
          conMando(comando.mandoId, (m) => {
            m.mapeo = m.mapeo.filter((b) => b.control !== comando.control)
            m.mapeo.push({ control: comando.control, salida: comando.salida })
          })
          break
        case 'pedirGenerales':
          eventos?.alGenerales([{ nombre: 'Inicio', ajustes: [ajuste('inicioWindows', 'Abrir al iniciar Windows', 'interruptor', false)] }])
          break
        case 'pedirAuto':
          eventos?.alAutoPerfiles([])
          break
        case 'pedirAcciones':
          eventos?.alAcciones(structuredClone(acciones))
          break
        case 'guardarAccion':
          acciones = acciones.filter((a) => a.nombre !== comando.original)
          acciones.push({ ...comando, tipo: comando.tipoAccion, activa: true })
          eventos?.alAcciones(structuredClone(acciones))
          break
        case 'borrarAccion':
          acciones = acciones.filter((a) => a.nombre !== comando.nombre)
          eventos?.alAcciones(structuredClone(acciones))
          break
        case 'activarAccion':
          acciones = acciones.map((a) => (a.nombre === comando.nombre ? { ...a, activa: comando.activa } : a))
          eventos?.alAcciones(structuredClone(acciones))
          break
        case 'pedirOpciones':
          eventos?.alOpciones(structuredClone(opciones))
          break
        case 'opcion':
          for (const g of opciones) {
            for (const s of g.secciones) {
              for (const a of s.ajustes) {
                if (a.id === comando.id) a.valor = comando.valor
              }
            }
          }
          eventos?.alOpciones(structuredClone(opciones))
          break
        case 'arcoiris':
          conMando(comando.mandoId, (m) => { m.ajustes.arcoiris = comando.activo })
          break
        case 'vibracion':
          conMando(comando.mandoId, (m) => { m.ajustes.vibracion = comando.fuerza })
          break
        case 'gatillo':
          conMando(comando.mandoId, (m) => {
            m.ajustes[comando.gatillo === 'l2' ? 'gatilloL2' : 'gatilloR2'] = comando.efecto
          })
          break
        case 'controlConMando':
          estado.controlConMando = comando.activo
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
