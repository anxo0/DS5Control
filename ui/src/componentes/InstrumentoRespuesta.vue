<script setup lang="ts">
import { computed } from 'vue'
import { respuesta, trazoCurva } from '../mandos/curvas'

const LADO = 100

const props = defineProps<{
  /** Lo que hace el usuario, de 0 a 1. */
  entrada: number
  muerta: number
  maxima: number
  curva: number
  personalizada: string
}>()

const salida = computed(() => respuesta(props.entrada, props.muerta, props.maxima, props.curva, props.personalizada))
const trazo = computed(() => trazoCurva(props.curva, LADO, props.personalizada))
const util = computed(() => {
  const { entrada, muerta, maxima } = props
  return entrada <= muerta ? 0 : Math.min(1, (entrada - muerta) / Math.max(0.01, maxima - muerta))
})
const porcentaje = (v: number) => `${Math.round(v * 100)}%`
</script>

<template>
  <div class="respuesta">
    <dl>
      <div>
        <dt>Mueves</dt>
        <dd>{{ porcentaje(entrada) }}</dd>
      </div>
      <div class="juego">
        <dt>Le llega al juego</dt>
        <dd>{{ porcentaje(salida) }}</dd>
      </div>
    </dl>
    <svg :viewBox="`-2 -2 ${LADO + 4} ${LADO + 4}`" preserveAspectRatio="none" role="img" aria-label="Curva de respuesta">
      <path class="rejilla" :d="`M0 ${LADO}H${LADO}V0M${LADO / 2} 0V${LADO}M0 ${LADO / 2}H${LADO}`" />
      <path class="ideal" :d="`M0 ${LADO}L${LADO} 0`" />
      <path class="curva" :d="trazo" />
      <circle class="punto" :cx="util * LADO" :cy="(1 - salida) * LADO" r="3.5" />
    </svg>
  </div>
</template>

<style scoped>
.respuesta {
  display: grid;
  gap: 12px;
}

dl {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin: 0;
}

dt {
  color: var(--muted);
  font-size: 13px;
}

dd {
  margin: 0;
  font-size: 28px;
  font-weight: 800;
  line-height: 1.15;
  font-variant-numeric: tabular-nums;
}

.juego dd {
  color: var(--accent);
}

svg {
  display: block;
  width: 100%;
  height: 130px;
  border-radius: var(--radio-control);
  background: var(--ground);
}

path {
  fill: none;
  vector-effect: non-scaling-stroke;
}

.rejilla {
  stroke: var(--linea);
}

.ideal {
  stroke: var(--guia);
  stroke-dasharray: 3 3;
}

.curva {
  stroke: var(--accent);
  stroke-width: 2.5;
}

.punto {
  fill: var(--ink);
}
</style>
