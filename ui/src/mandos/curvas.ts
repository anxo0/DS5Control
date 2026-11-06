/** Curvas de respuesta en el mismo orden que el motor. */
export const PERSONALIZADA = 6

function bezier(x: number, [x1, y1, x2, y2]: number[]) {
  let t = x
  const a = x1 ?? 0.25
  const b = x2 ?? 0.75
  for (let i = 0; i < 8; i++) {
    const u = 1 - t
    const valor = 3 * u * u * t * a + 3 * u * t * t * b + t * t * t
    const pendiente = 3 * u * u * a + 6 * u * t * (b - a) + 3 * t * t * (1 - b)
    if (Math.abs(pendiente) < 1e-4) break
    t = Math.min(1, Math.max(0, t - (valor - x) / pendiente))
  }
  const u = 1 - t
  return 3 * u * u * t * (y1 ?? 0.25) + 3 * u * t * t * (y2 ?? 0.75) + t * t * t
}

/** Valor de salida, de 0 a 1, para una entrada de 0 a 1. */
export function aplicarCurva(indice: number, x: number, personalizada = '') {
  switch (indice) {
    case 1: return x <= 0.5 ? x * 0.6 : 1.4 * x - 0.4
    case 2: return x * x
    case 3: return x * x * x
    case 4: return 1 - (1 - x) * (1 - x)
    case 5: return 1 - (1 - x) ** 3
    case PERSONALIZADA: return bezier(x, personalizada.split(',').map(Number))
    default: return x
  }
}

/** Trazo SVG de una curva dentro de un cuadrado de lado `lado`. */
export function trazoCurva(indice: number, lado: number, personalizada = '', muestras = 32) {
  const puntos: string[] = []
  for (let i = 0; i <= muestras; i++) {
    const x = i / muestras
    puntos.push(`${(x * lado).toFixed(1)},${((1 - aplicarCurva(indice, x, personalizada)) * lado).toFixed(1)}`)
  }
  return `M${puntos.join('L')}`
}

/** Aplica zona muerta, recorrido máximo y curva a un valor de 0 a 1. */
export function respuesta(entrada: number, muerta: number, maxima: number, curva: number, personalizada = '') {
  if (entrada <= muerta) return 0
  const util = Math.min(1, (entrada - muerta) / Math.max(0.01, maxima - muerta))
  return aplicarCurva(curva, util, personalizada)
}
