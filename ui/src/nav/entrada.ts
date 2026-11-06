import type { AccionNavegacion, ControlMando } from '../bridge'

export interface Salidas {
  accion(accion: AccionNavegacion): void
  /** Refleja en el dibujo lo que se está pulsando. */
  pulsar(control: ControlMando, activo: boolean): void
}

const TECLAS: Record<string, [AccionNavegacion, ControlMando]> = {
  ArrowUp: ['arriba', 'dpad-up'],
  ArrowDown: ['abajo', 'dpad-down'],
  ArrowLeft: ['izquierda', 'dpad-left'],
  ArrowRight: ['derecha', 'dpad-right'],
  Enter: ['elegir', 'cross'],
  Escape: ['atras', 'circle'],
  q: ['pestanaAnterior', 'l1'],
  Q: ['pestanaAnterior', 'l1'],
  e: ['pestanaSiguiente', 'r1'],
  E: ['pestanaSiguiente', 'r1'],
}

export function escucharTeclado(salidas: Salidas) {
  window.addEventListener('keydown', (e) => {
    const tecla = TECLAS[e.key]
    if (!tecla || e.ctrlKey || e.altKey || e.metaKey) return
    e.preventDefault()
    const [accion, control] = tecla
    const repetible = control.startsWith('dpad')
    if (e.repeat && !repetible) return
    salidas.pulsar(control, true)
    salidas.accion(accion)
  })
  window.addEventListener('keyup', (e) => {
    const tecla = TECLAS[e.key]
    if (tecla) salidas.pulsar(tecla[1], false)
  })
}

/** Orden del mapeo estándar de la Gamepad API. */
const BOTONES: (ControlMando | null)[] = [
  'cross', 'circle', 'rect', 'triangle', 'l1', 'r1', 'l2', 'r2', 'create', 'options',
  'l3', 'r3', 'dpad-up', 'dpad-down', 'dpad-left', 'dpad-right', 'ps', 'touchpad',
]

const ACCIONES: Partial<Record<ControlMando, AccionNavegacion>> = {
  cross: 'elegir',
  circle: 'atras',
  l1: 'pestanaAnterior',
  r1: 'pestanaSiguiente',
  'dpad-up': 'arriba',
  'dpad-down': 'abajo',
  'dpad-left': 'izquierda',
  'dpad-right': 'derecha',
}

const RETARDO_REPETICION_MS = 380
const INTERVALO_REPETICION_MS = 110

export function escucharGamepad(salidas: Salidas) {
  const pulsadoDesde = new Map<ControlMando, number>()
  const ultimaRepeticion = new Map<ControlMando, number>()
  let bucle = 0

  const leer = (ahora: number) => {
    const mando = navigator.getGamepads().find((g) => g?.connected && g.mapping === 'standard')
    BOTONES.forEach((control, i) => {
      if (!control) return
      const pulsado = mando?.buttons[i]?.pressed ?? false
      const desde = pulsadoDesde.get(control)
      if (pulsado && desde === undefined) {
        pulsadoDesde.set(control, ahora)
        ultimaRepeticion.set(control, ahora)
        salidas.pulsar(control, true)
        const accion = ACCIONES[control]
        if (accion) salidas.accion(accion)
      } else if (!pulsado && desde !== undefined) {
        pulsadoDesde.delete(control)
        salidas.pulsar(control, false)
      } else if (pulsado && desde !== undefined && control.startsWith('dpad')) {
        const listo = ahora - desde > RETARDO_REPETICION_MS
        if (listo && ahora - (ultimaRepeticion.get(control) ?? 0) > INTERVALO_REPETICION_MS) {
          ultimaRepeticion.set(control, ahora)
          salidas.accion(ACCIONES[control]!)
        }
      }
    })
    bucle = requestAnimationFrame(leer)
  }

  window.addEventListener('gamepadconnected', () => {
    if (!bucle) bucle = requestAnimationFrame(leer)
  })
  window.addEventListener('gamepaddisconnected', () => {
    if (navigator.getGamepads().some((g) => g?.connected)) return
    cancelAnimationFrame(bucle)
    bucle = 0
    for (const control of pulsadoDesde.keys()) salidas.pulsar(control, false)
    pulsadoDesde.clear()
  })
}
