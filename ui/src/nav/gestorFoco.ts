import type { AccionNavegacion } from '../bridge'

type Direccion = 'arriba' | 'abajo' | 'izquierda' | 'derecha'

const ENFOCABLES = 'button:not(:disabled), input:not(:disabled), a[href]'
/** Penaliza el desvío lateral: se prefiere lo alineado aunque esté más lejos. */
const PESO_DESVIO = 3

export interface OpcionesFoco {
  /** Círculo sin ningún sitio al que volver dentro de la pantalla. */
  alLlegarARaiz(): void
  alCambiarPestana(delta: 1 | -1): void
}

/**
 * Único responsable del foco. Teclado, Gamepad API y mensajes del host acaban en `accion`,
 * así que todos se comportan igual. El foco es el nativo del navegador.
 */
export class GestorFoco {
  constructor(
    private readonly raiz: HTMLElement,
    private readonly opciones: OpcionesFoco,
  ) {
    window.addEventListener('pointerdown', () => this.mostrarFoco(false), true)
    window.addEventListener('keydown', (e) => {
      if (e.key === 'Tab') this.mostrarFoco(true)
    }, true)
    this.mostrarFoco(true)
  }

  /** `:focus-visible` no se activa con foco por código tras usar el ratón. */
  mostrarFoco(visible: boolean) {
    document.documentElement.toggleAttribute('data-foco-activo', visible)
  }

  accion(accion: AccionNavegacion) {
    this.mostrarFoco(true)
    if (accion === 'pestanaAnterior') return this.opciones.alCambiarPestana(-1)
    if (accion === 'pestanaSiguiente') return this.opciones.alCambiarPestana(1)

    const actual = this.actual()
    if (!actual) return this.asegurarFoco()

    switch (accion) {
      case 'elegir':
        actual.click()
        break
      case 'atras':
        this.atras(actual)
        break
      case 'izquierda':
      case 'derecha':
        if (actual instanceof HTMLInputElement && actual.type === 'range') {
          this.ajustarNivel(actual, accion === 'derecha' ? 1 : -1)
          break
        }
        this.mover(actual, accion)
        break
      default:
        this.mover(actual, accion)
    }
  }

  /** Recoloca el foco si el elemento activo ha desaparecido o se ha desactivado. */
  asegurarFoco() {
    if (this.actual()) return
    const destino =
      this.ambito().querySelector<HTMLElement>('[data-foco-inicial]:not(:disabled)') ??
      this.raiz.querySelector<HTMLElement>(`[role="tabpanel"] :is(${ENFOCABLES})`) ??
      this.raiz.querySelector<HTMLElement>('[aria-current="page"]')
    destino?.focus()
  }

  /** Lleva el foco al primer control de `selector`, si existe. */
  enfocar(selector: string) {
    const contenedor = this.raiz.querySelector<HTMLElement>(selector)
    const destino = contenedor?.matches(ENFOCABLES)
      ? contenedor
      : contenedor?.querySelector<HTMLElement>(ENFOCABLES)
    destino?.focus()
  }

  private actual(): HTMLElement | null {
    const activo = document.activeElement
    if (!(activo instanceof HTMLElement) || !this.raiz.contains(activo)) return null
    return activo.matches(ENFOCABLES) && this.visible(activo) ? activo : null
  }

  private visible(el: HTMLElement) {
    const { width, height } = el.getBoundingClientRect()
    return width > 0 && height > 0
  }

  /** Con un diálogo abierto, el foco no sale de él. */
  private ambito(): HTMLElement {
    return this.raiz.querySelector<HTMLElement>('dialog[open]') ?? this.raiz
  }

  private candidatos() {
    return [...this.ambito().querySelectorAll<HTMLElement>(ENFOCABLES)].filter(
      (el) => this.visible(el) && !el.closest('[data-sin-foco]'),
    )
  }

  private mover(origen: HTMLElement, direccion: Direccion) {
    const a = origen.getBoundingClientRect()
    let mejor: HTMLElement | undefined
    let mejorPuntos = Infinity

    for (const candidato of this.candidatos()) {
      if (candidato === origen) continue
      const b = candidato.getBoundingClientRect()
      const horizontal = direccion === 'izquierda' || direccion === 'derecha'
      const avance = {
        arriba: a.top - b.bottom,
        abajo: b.top - a.bottom,
        izquierda: a.left - b.right,
        derecha: b.left - a.right,
      }[direccion]
      if (avance < -1) continue
      const desvio = horizontal
        ? Math.max(0, b.top - a.bottom, a.top - b.bottom)
        : Math.max(0, b.left - a.right, a.left - b.right)
      const puntos = avance + desvio * PESO_DESVIO
      if (puntos < mejorPuntos) {
        mejorPuntos = puntos
        mejor = candidato
      }
    }
    mejor?.focus()
    mejor?.scrollIntoView({ block: 'nearest', inline: 'nearest' })
  }

  private ajustarNivel(entrada: HTMLInputElement, sentido: 1 | -1) {
    if (sentido > 0) entrada.stepUp()
    else entrada.stepDown()
    entrada.dispatchEvent(new Event('input', { bubbles: true }))
  }

  private atras(actual: HTMLElement) {
    // Círculo en un diálogo es lo mismo que Escape: pide cerrarlo.
    const dialogo = actual.closest('dialog')
    if (dialogo) {
      dialogo.dispatchEvent(new Event('cancel', { cancelable: true }))
      return
    }
    const destino = actual.closest<HTMLElement>('[data-atras]')?.dataset.atras
    const elemento = destino ? this.raiz.querySelector<HTMLElement>(destino) : null
    if (elemento && elemento !== actual) elemento.focus()
    else this.opciones.alLlegarARaiz()
  }
}
