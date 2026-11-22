/**
 * Todos los diccionarios. Cada archivo cubre un grupo de pantallas y lleva sus claves con un
 * prefijo propio (`datos.`, `acciones.`...), para que no choquen entre sí.
 */
import type { Diccionario } from '..'
import comun from './comun'
import general from './general'
import mando from './mando'
import secciones from './secciones'

export const diccionarios: Diccionario[] = [comun, general, mando, secciones]
