<script setup lang="ts">
import { computed } from 'vue'
import ControlNumero from './ControlNumero.vue'

const props = defineProps<{ valor: string }>()
const emit = defineEmits<{ cambiar: [valor: string] }>()

const LINEAL = [0.25, 0.25, 0.75, 0.75]
const TAMANO = 120
const PUNTOS = [
  { nombre: 'Primer punto, entrada' },
  { nombre: 'Primer punto, salida' },
  { nombre: 'Segundo punto, entrada' },
  { nombre: 'Segundo punto, salida' },
]

const valores = computed(() => {
  const lista = (props.valor ?? '').split(',').map(Number)
  return lista.length === 4 && lista.every(Number.isFinite) ? lista : LINEAL
})

const x = (v: number) => v * TAMANO
const y = (v: number) => TAMANO - v * TAMANO
const trazo = computed(() => {
  const [a = 0, b = 0, c = 1, d = 1] = valores.value
  return `M0 ${TAMANO}C${x(a)} ${y(b)} ${x(c)} ${y(d)} ${TAMANO} 0`
})

function cambiar(indice: number, valor: number) {
  emit('cambiar', valores.value.map((v, i) => (i === indice ? valor : v).toFixed(2)).join(', '))
}
</script>

<template>
  <div class="curva">
    <svg :viewBox="`0 0 ${TAMANO} ${TAMANO}`" role="img" aria-label="Forma de la curva">
      <path class="guia" :d="`M0 ${TAMANO}L${TAMANO} 0`" />
      <path class="linea" :d="trazo" />
    </svg>
    <ul>
      <li v-for="(p, i) in PUNTOS" :key="p.nombre">
        <span>{{ p.nombre }}</span>
        <ControlNumero
          :etiqueta="p.nombre"
          :valor="valores[i] ?? 0"
          :min="0"
          :max="1"
          :paso="0.05"
          @cambiar="cambiar(i, $event)"
        />
      </li>
    </ul>
  </div>
</template>

<style scoped>
.curva {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
}

svg {
  flex: 0 0 150px;
  height: 150px;
  border-radius: var(--radio-control);
  background: var(--ground);
}

path {
  fill: none;
}

.guia {
  stroke: var(--guia);
  stroke-dasharray: 3 3;
}

.linea {
  stroke: var(--accent);
  stroke-width: 2.5;
}

ul {
  flex: 1 1 280px;
  display: grid;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}

li {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  font-size: 14px;
}
</style>
