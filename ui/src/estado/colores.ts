import { ref } from 'vue'

const CLAVE = 'colores'

const DE_FABRICA = ['#1F4FE0', '#00A3FF', '#7ED321', '#FFC400', '#FF6B2C', '#E8254F', '#FFFFFF']

interface Guardado {
  lista: string[]
  /** Hueco que ocupará el próximo color: los propios van sustituyendo por orden. */
  siguiente: number
}

function leer(): Guardado {
  try {
    const dato = JSON.parse(localStorage.getItem(CLAVE) ?? 'null') as Guardado | null
    if (dato?.lista?.length === DE_FABRICA.length) return dato
  } catch {
    // Datos dañados o sin almacenamiento: se vuelve a los de fábrica.
  }
  return { lista: DE_FABRICA, siguiente: 0 }
}

const guardado = leer()

export const coloresGuardados = ref<string[]>(guardado.lista)
let siguiente = guardado.siguiente

export function guardarColor(hex: string) {
  if (coloresGuardados.value.includes(hex)) return

  const lista = [...coloresGuardados.value]
  lista[siguiente] = hex
  coloresGuardados.value = lista
  siguiente = (siguiente + 1) % lista.length
  try {
    localStorage.setItem(CLAVE, JSON.stringify({ lista, siguiente }))
  } catch {
    // Sin almacenamiento, los colores duran lo que dure la sesión.
  }
}
