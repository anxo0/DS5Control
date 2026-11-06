const ESPECIALES: Record<number, string> = {
  8: 'Retroceso', 9: 'Tab', 13: 'Intro', 16: 'Mayús', 17: 'Ctrl', 18: 'Alt', 19: 'Pausa',
  20: 'Bloq Mayús', 27: 'Esc', 32: 'Espacio', 33: 'Re Pág', 34: 'Av Pág', 35: 'Fin', 36: 'Inicio',
  37: 'Izquierda', 38: 'Arriba', 39: 'Derecha', 40: 'Abajo', 44: 'Impr Pant', 45: 'Insert', 46: 'Supr',
  91: 'Windows', 186: 'Ñ', 187: '+', 188: ',', 189: '-', 190: '.',
  256: 'Clic izquierdo', 257: 'Clic derecho', 258: 'Clic central', 259: 'Botón 4', 260: 'Botón 5',
}

/** Nombre legible de un código de tecla de Windows. */
export function nombreTecla(codigo: number) {
  if (ESPECIALES[codigo]) return ESPECIALES[codigo]
  if ((codigo >= 48 && codigo <= 57) || (codigo >= 65 && codigo <= 90)) return String.fromCharCode(codigo)
  if (codigo >= 96 && codigo <= 105) return `Num ${codigo - 96}`
  if (codigo >= 112 && codigo <= 123) return `F${codigo - 111}`
  return `Tecla ${codigo}`
}
