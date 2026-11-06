import dualsenseSvg from '../assets/mandos/dualsense.svg?raw'
import type { TipoMando } from '../bridge'

interface Rotulo {
  x: number
  y: number
}

export interface ModeloMando {
  /** Contenido del SVG, sin la etiqueta raíz. */
  dibujo: string
  /** Lienzo con margen a los lados para los rótulos. */
  viewBox: string
  proporcion: string
  barraLuz: string
  guias: string[]
  rotulos: { l2: Rotulo; luz: Rotulo; r2: Rotulo; vibracion: Rotulo; silencio: Rotulo | null }
  gatillosAdaptativos: boolean
}

const interior = (svg: string) => svg.replace(/^[\s\S]*?<svg[^>]*>/, '').replace(/<\/svg>\s*$/, '')

const dualsense: ModeloMando = {
  dibujo: interior(dualsenseSvg),
  viewBox: '-320 0 1757 892',
  proporcion: '1757 / 892',
  barraLuz: 'M302 180L334 345M815 180L783 345',
  guias: ['M-40 60H130', 'M-40 262H312', 'M985 60H1157', 'M1040 560H1157', 'M559 610V740'],
  rotulos: {
    l2: { x: -52, y: 52 },
    luz: { x: -52, y: 254 },
    r2: { x: 1169, y: 52 },
    vibracion: { x: 1169, y: 552 },
    silencio: { x: 559, y: 782 },
  },
  gatillosAdaptativos: true,
}

/** Para añadir otro mando: su SVG con los mismos id y una entrada aquí. */
const modelos: Partial<Record<TipoMando, ModeloMando>> = { dualsense }

export const modeloDe = (tipo: TipoMando): ModeloMando => modelos[tipo] ?? dualsense
