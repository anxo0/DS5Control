import { t } from '../i18n'

const ESPECIALES: Record<number, string> = {
  8: 'retroceso', 9: 'tab', 13: 'intro', 16: 'mayus', 17: 'ctrl', 18: 'alt', 19: 'pausa',
  20: 'bloqMayus', 27: 'esc', 32: 'espacio', 33: 'rePag', 34: 'avPag', 35: 'fin', 36: 'inicio',
  37: 'izquierda', 38: 'arriba', 39: 'derecha', 40: 'abajo', 44: 'imprPant', 45: 'insert', 46: 'supr',
  91: 'windows', 186: 'enie',
  256: 'clicIzquierdo', 257: 'clicDerecho', 258: 'clicCentral', 259: 'boton4', 260: 'boton5',
}

/** Teclas cuyo nombre es el propio signo en cualquier idioma. */
const SIGNOS: Record<number, string> = { 187: '+', 188: ',', 189: '-', 190: '.' }

/** Nombre legible de un código de tecla de Windows. */
export function nombreTecla(codigo: number) {
  if (ESPECIALES[codigo]) return t(`general.tecla.${ESPECIALES[codigo]}`)
  if (SIGNOS[codigo]) return SIGNOS[codigo]
  if ((codigo >= 48 && codigo <= 57) || (codigo >= 65 && codigo <= 90)) return String.fromCharCode(codigo)
  if (codigo >= 96 && codigo <= 105) return `Num ${codigo - 96}`
  if (codigo >= 112 && codigo <= 123) return `F${codigo - 111}`
  return t('general.tecla.otra', { codigo })
}
