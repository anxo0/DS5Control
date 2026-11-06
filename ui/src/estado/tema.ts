import { ref } from 'vue'

export type Tema = 'claro' | 'oscuro'

const CLAVE = 'tema'

function leerTema(): Tema {
  try {
    return localStorage.getItem(CLAVE) === 'oscuro' ? 'oscuro' : 'claro'
  } catch {
    return 'claro'
  }
}

export const tema = ref<Tema>(leerTema())

export function cambiarTema() {
  tema.value = tema.value === 'claro' ? 'oscuro' : 'claro'
  document.documentElement.dataset.theme = tema.value
  try {
    localStorage.setItem(CLAVE, tema.value)
  } catch {
    // Sin almacenamiento, el tema dura lo que dure la sesión.
  }
}
