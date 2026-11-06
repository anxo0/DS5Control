<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import type { PosicionStick } from '../bridge'
import { conectado, mando } from '../estado/app'
import IconoMando from './IconoMando.vue'

const TAMANO = 120
const CENTRO = TAMANO / 2
const RADIO = 52
const LARGO_ESTELA = 40
/** Por debajo de esto suele ser ruido normal del sensor; es una orientación, no un estándar. */
const UMBRAL_LEVE = 0.03
const UMBRAL_DERIVA = 0.08
const DURACION_MS = 3000
const MUESTREO_MS = 50

type Lado = 'izq' | 'der'
interface Medida {
  media: number
  maxima: number
}

const LADOS: { id: Lado; nombre: string }[] = [
  { id: 'izq', nombre: 'Izquierdo' },
  { id: 'der', nombre: 'Derecho' },
]

const posicion = (lado: Lado): PosicionStick => mando.value?.sticks[lado] ?? { x: 0, y: 0 }
const estelas = ref<Record<Lado, string[]>>({ izq: [], der: [] })
const aPantalla = (v: number) => (CENTRO + v * RADIO).toFixed(1)

watch(
  () => mando.value?.sticks,
  (sticks) => {
    if (!sticks) return
    for (const { id } of LADOS) {
      const punto = `${aPantalla(sticks[id].x)},${aPantalla(sticks[id].y)}`
      const lista = estelas.value[id]
      if (lista[lista.length - 1] !== punto) {
        estelas.value[id] = [...lista, punto].slice(-LARGO_ESTELA)
      }
    }
  },
)

const magnitud = (p: PosicionStick) => Math.hypot(p.x, p.y)
const porcentaje = (v: number) => `${Math.round(v * 1000) / 10}%`

const midiendo = ref(false)
const progreso = ref(0)
const resultado = ref<Record<Lado, Medida> | null>(null)
let temporizador: ReturnType<typeof setInterval> | undefined

function medir() {
  clearInterval(temporizador)
  midiendo.value = true
  resultado.value = null
  progreso.value = 0
  const muestras: Record<Lado, number[]> = { izq: [], der: [] }
  const inicio = performance.now()
  temporizador = setInterval(() => {
    for (const { id } of LADOS) muestras[id].push(magnitud(posicion(id)))
    const pasado = performance.now() - inicio
    progreso.value = Math.min(pasado / DURACION_MS, 1)
    if (pasado < DURACION_MS) return
    clearInterval(temporizador)
    midiendo.value = false
    const resumen = (lista: number[]): Medida => ({
      media: lista.reduce((a, b) => a + b, 0) / lista.length,
      maxima: Math.max(...lista),
    })
    resultado.value = { izq: resumen(muestras.izq), der: resumen(muestras.der) }
  }, MUESTREO_MS)
}

onBeforeUnmount(() => clearInterval(temporizador))

function veredicto(m: Medida) {
  if (m.media >= UMBRAL_DERIVA) return { texto: 'Tiene deriva', clase: 'mal' }
  if (m.media >= UMBRAL_LEVE) return { texto: 'Deriva leve', clase: 'leve' }
  return { texto: 'Sin deriva', clase: 'bien' }
}

const textoBoton = computed(() => {
  if (midiendo.value) return 'No toques los sticks...'
  return resultado.value ? 'Repetir prueba' : 'Comprobar deriva'
})
</script>

<template>
  <div class="vista">
    <div class="sticks">
      <figure v-for="s in LADOS" :key="s.id">
        <svg :viewBox="`0 0 ${TAMANO} ${TAMANO}`" role="img" :aria-label="`Posición del stick ${s.nombre}`">
          <circle class="aro" :cx="CENTRO" :cy="CENTRO" :r="RADIO" />
          <circle class="zona" :cx="CENTRO" :cy="CENTRO" :r="RADIO * UMBRAL_DERIVA" />
          <path class="cruz" :d="`M${CENTRO} ${CENTRO - RADIO}V${CENTRO + RADIO}M${CENTRO - RADIO} ${CENTRO}H${CENTRO + RADIO}`" />
          <polyline class="estela" :points="estelas[s.id].join(' ')" />
          <circle class="punto" :cx="aPantalla(posicion(s.id).x)" :cy="aPantalla(posicion(s.id).y)" r="5" />
        </svg>
        <figcaption>
          <strong><IconoMando :control="s.nombre" :tamano="22" /></strong>
          <span>X {{ posicion(s.id).x.toFixed(2) }} · Y {{ posicion(s.id).y.toFixed(2) }}</span>
        </figcaption>
      </figure>
    </div>

    <div class="prueba">
      <button class="medir" :disabled="!conectado || midiendo" @click="medir">
        <span v-if="midiendo" class="avance" :style="{ transform: `scaleX(${progreso})` }" />
        <span class="texto">{{ textoBoton }}</span>
      </button>
      <p v-if="!resultado && !midiendo" class="ayuda">Suelta los sticks y pulsa. Mide durante 3 segundos.</p>
      <ul v-if="resultado" class="resultados">
        <li v-for="s in LADOS" :key="s.id">
          <span>{{ s.nombre }}</span>
          <strong :class="veredicto(resultado[s.id]).clase">{{ veredicto(resultado[s.id]).texto }}</strong>
          <small>Media {{ porcentaje(resultado[s.id].media) }}, máximo {{ porcentaje(resultado[s.id].maxima) }}</small>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.vista {
  display: grid;
  gap: 18px;
}

.sticks {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

figure {
  margin: 0;
}

svg {
  display: block;
  width: 100%;
  border-radius: var(--radio-control);
  background: var(--ground);
}

.aro,
.zona,
.estela {
  fill: none;
}

.aro {
  stroke: var(--guia);
  stroke-width: 1.5;
}

.zona {
  stroke: var(--guia);
  stroke-width: 1;
  stroke-dasharray: 2 2;
}

.cruz {
  stroke: var(--linea);
  stroke-width: 1;
}

.estela {
  stroke: var(--accent);
  stroke-width: 1.5;
  stroke-linejoin: round;
  opacity: 0.55;
}

.punto {
  fill: var(--accent);
}

figcaption {
  display: grid;
  gap: 1px;
  margin-top: 6px;
  font-size: 13px;
}

figcaption span {
  color: var(--muted);
  font-variant-numeric: tabular-nums;
}

.prueba {
  display: grid;
  gap: 10px;
}

.medir {
  position: relative;
  overflow: hidden;
  min-height: 48px;
  border-radius: var(--radio-control);
  background: var(--ground);
  font-weight: 600;
}

.medir:disabled {
  cursor: default;
}

.avance {
  position: absolute;
  inset: 0;
  background: var(--accent);
  opacity: 0.25;
  transform-origin: left;
}

.texto {
  position: relative;
}

.ayuda {
  font-size: 13px;
  color: var(--muted);
}

.resultados {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.resultados li {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0 8px;
}

.resultados small {
  grid-column: 1 / -1;
  color: var(--muted);
  font-size: 13px;
  font-variant-numeric: tabular-nums;
}

.bien {
  color: var(--ink);
}

.leve {
  color: var(--accent);
}

.mal {
  color: var(--aviso);
}
</style>
