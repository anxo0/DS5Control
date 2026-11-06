const ARCHIVOS = import.meta.glob<string>('../assets/iconos/*.svg', { eager: true, import: 'default' })

/** Cada nombre del pack, con las formas en que lo llaman el motor y la interfaz. */
const NOMBRES: Record<string, string[]> = {
  Cross: ['Cross', 'Cruz', 'A'],
  Circle: ['Circle', 'Círculo', 'B'],
  Square: ['Square', 'Cuadrado', 'X'],
  Triangle: ['Triangle', 'Triángulo', 'Y'],
  L1: ['L1', 'LB'],
  R1: ['R1', 'RB'],
  L2: ['L2', 'LT'],
  R2: ['R2', 'RT'],
  'Left Stick Click': ['L3', 'LS'],
  'Right Stick Click': ['R3', 'RS'],
  'Left Stick': ['Stick izquierdo', 'Izquierdo'],
  'Right Stick': ['Stick derecho', 'Derecho'],
  'D-Pad Up': ['DpadUp', 'Up', 'Arriba', 'Cruceta arriba'],
  'D-Pad Down': ['DpadDown', 'Down', 'Abajo', 'Cruceta abajo'],
  'D-Pad Left': ['DpadLeft', 'Left', 'Izquierda', 'Cruceta izquierda'],
  'D-Pad Right': ['DpadRight', 'Right', 'Derecha', 'Cruceta derecha'],
  Create: ['Share', 'Crear', 'Back'],
  Options: ['Options', 'Start'],
  Home: ['PS', 'Guide'],
  'Touch Pad Press': ['Clic del panel', 'Panel táctil', 'Touchpad'],
}

const ICONOS = new Map(
  Object.entries(NOMBRES).flatMap(([archivo, alias]) =>
    alias.map((a) => [a, ARCHIVOS[`../assets/iconos/${archivo}.svg`]] as const),
  ),
)

/** Devuelve el icono de un control, o undefined si el pack no lo tiene. */
export const iconoDe = (control: string) => ICONOS.get(control)
