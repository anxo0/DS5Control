import { ref } from 'vue'
import clasico from '../assets/logo/ds5control-clasico.svg'
import azul from '../assets/logo/ds5control-azul.svg'
import fuego from '../assets/logo/ds5control-fuego.svg'
import menta from '../assets/logo/ds5control-menta.svg'

export const PALETAS = [
  { id: 'fuego', nombre: 'Fuego', logo: fuego },
  { id: 'azul', nombre: 'Azul', logo: azul },
  { id: 'menta', nombre: 'Menta', logo: menta },
  { id: 'clasico', nombre: 'Clásico', logo: clasico },
] as const

export type Paleta = (typeof PALETAS)[number]['id']

const CLAVE = 'paleta'

function leer(): Paleta {
  try {
    const guardada = localStorage.getItem(CLAVE)
    return PALETAS.find((p) => p.id === guardada)?.id ?? 'fuego'
  } catch {
    return 'fuego'
  }
}

export const paleta = ref<Paleta>(leer())

export function cambiarPaleta(valor: Paleta) {
  paleta.value = valor
  try {
    localStorage.setItem(CLAVE, valor)
  } catch {
    // Sin almacenamiento, la paleta dura lo que dure la sesión.
  }
}
