/**
 * Idiomas de la interfaz: español, inglés y portugués.
 *
 * Cada grupo de pantallas tiene su diccionario en `textos/*.ts`, con las mismas claves en los tres
 * idiomas. El idioma sale del ajuste "Idioma" (que guarda la app en `UseLang`) o, en automático,
 * del idioma de Windows: español si es español, portugués si es portugués (también el de Brasil)
 * y, si no, inglés.
 */
import { computed, shallowRef } from 'vue'
import { diccionarios } from './textos'

export type Idioma = 'es' | 'en' | 'pt'
export const IDIOMAS: Idioma[] = ['es', 'en', 'pt']

/** Diccionario de un grupo: las mismas claves en los tres idiomas. */
export type Diccionario = Record<Idioma, Record<string, string>>

/** El idioma de un código como `es-ES`, `pt-BR` o `en-US`; `null` si no es uno de los tres. */
export function idiomaDe(codigo: string | null | undefined): Idioma | null {
  const base = (codigo ?? '').trim().toLowerCase().split(/[-_]/)[0]
  return (IDIOMAS as string[]).includes(base) ? (base as Idioma) : null
}

/** El idioma del sistema: el primero de los preferidos que sea uno de los tres, o inglés. */
export function idiomaDelSistema(preferidos: readonly string[] = navigator.languages ?? [navigator.language]): Idioma {
  for (const codigo of preferidos) {
    const i = idiomaDe(codigo)
    if (i) return i
  }
  return 'en'
}

/** Lo que pide la app: vacío o `null` es automático. */
const elegido = shallowRef<Idioma | null>(null)

export const idioma = computed<Idioma>(() => elegido.value ?? idiomaDelSistema())

/** Lo llama la app con el ajuste guardado (`UseLang`); vacío es automático. */
export function fijarIdioma(codigo: string | null | undefined) {
  elegido.value = idiomaDe(codigo)
}

const tabla = computed(() => {
  const salida: Record<string, string> = {}
  for (const d of diccionarios) Object.assign(salida, d[idioma.value])
  return salida
})

const espanol = (() => {
  const salida: Record<string, string> = {}
  for (const d of diccionarios) Object.assign(salida, d.es)
  return salida
})()

/**
 * El texto de una clave en el idioma actual, con `{nombre}` sustituido por `vars.nombre`. Si a un
 * idioma le falta una clave sale el español, y si tampoco existe, la propia clave (así se nota).
 */
export function t(clave: string, vars?: Record<string, string | number>): string {
  let texto = tabla.value[clave] ?? espanol[clave] ?? clave
  if (vars) {
    for (const [k, v] of Object.entries(vars)) texto = texto.replaceAll(`{${k}}`, String(v))
  }
  return texto
}

/** El formato de números y fechas del idioma actual. */
export const local = computed(() => ({ es: 'es-ES', en: 'en-US', pt: 'pt-BR' })[idioma.value])
