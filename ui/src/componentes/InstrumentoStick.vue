<script setup lang="ts">
import { computed } from 'vue'
import { respuesta } from '../mandos/curvas'
import type { PosicionStick } from '../bridge'
import InstrumentoRespuesta from './InstrumentoRespuesta.vue'

const LADO = 200
const CENTRO = LADO / 2
const RADIO = 92

const props = defineProps<{
  posicion: PosicionStick
  muerta: number
  maxima: number
  curva: number
  personalizada: string
}>()

const entrada = computed(() => Math.min(1, Math.hypot(props.posicion.x, props.posicion.y)))
const salida = computed(() => respuesta(entrada.value, props.muerta, props.maxima, props.curva, props.personalizada))
const pantalla = (v: number) => CENTRO + v * RADIO
const escala = computed(() => (entrada.value > 0 ? salida.value / entrada.value : 0))
</script>

<template>
  <div class="instrumento">
    <svg :viewBox="`0 0 ${LADO} ${LADO}`" role="img" aria-label="Posición del stick y zonas ajustadas">
      <circle class="aro" :cx="CENTRO" :cy="CENTRO" :r="RADIO" />
      <circle class="limite" :cx="CENTRO" :cy="CENTRO" :r="RADIO * maxima" />
      <circle class="muerta" :cx="CENTRO" :cy="CENTRO" :r="RADIO * muerta" />
      <path class="cruz" :d="`M${CENTRO} ${CENTRO - RADIO}V${CENTRO + RADIO}M${CENTRO - RADIO} ${CENTRO}H${CENTRO + RADIO}`" />
      <line class="enlace" :x1="pantalla(posicion.x)" :y1="pantalla(posicion.y)" :x2="pantalla(posicion.x * escala)" :y2="pantalla(posicion.y * escala)" />
      <circle class="real" :cx="pantalla(posicion.x)" :cy="pantalla(posicion.y)" r="5" />
      <circle class="juego" :cx="pantalla(posicion.x * escala)" :cy="pantalla(posicion.y * escala)" r="7" />
    </svg>
    <ul class="leyenda">
      <li><span class="m real" />Tu dedo</li>
      <li><span class="m juego" />Lo que ve el juego</li>
      <li><span class="m muerta" />Zona muerta</li>
    </ul>
    <InstrumentoRespuesta :entrada="entrada" :muerta="muerta" :maxima="maxima" :curva="curva" :personalizada="personalizada" />
  </div>
</template>

<style scoped>
.instrumento {
  display: grid;
  gap: 12px;
}

svg {
  display: block;
  width: 100%;
  height: 196px;
  border-radius: var(--radio-control);
  background: var(--ground);
}

.aro,
.limite {
  fill: none;
  stroke: var(--guia);
}

.limite {
  stroke-dasharray: 3 3;
}

circle.muerta {
  fill: var(--accent);
  fill-opacity: 0.18;
  stroke: var(--accent);
  stroke-opacity: 0.5;
}

.cruz {
  stroke: var(--linea);
}

.enlace {
  stroke: var(--accent);
  stroke-opacity: 0.5;
}

circle.real {
  fill: var(--muted);
}

circle.juego {
  fill: var(--accent);
}

.leyenda {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 12px;
  margin: 0;
  padding: 0;
  list-style: none;
  color: var(--muted);
  font-size: 13px;
}

.leyenda li {
  display: flex;
  align-items: center;
  gap: 6px;
}

.m {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}

.m.real {
  background: var(--muted);
}

.m.juego {
  background: var(--accent);
}

.m.muerta {
  background: var(--accent);
  opacity: 0.3;
}
</style>
