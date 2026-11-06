<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  etiqueta: string
  valor: number
  min: number
  max: number
  paso: number
  porcentaje?: boolean
  desactivado?: boolean
}>()

const emit = defineEmits<{ cambiar: [valor: number] }>()

const decimales = computed(() => (String(props.paso).split('.')[1] ?? '').length)
const relleno = computed(() => (props.valor - props.min) / (props.max - props.min || 1))
const texto = computed(() =>
  props.porcentaje
    ? `${Math.round(props.valor * 100)}%`
    : Number(props.valor.toFixed(Math.max(decimales.value, 0))).toString(),
)

const limitar = (valor: number) => Math.min(props.max, Math.max(props.min, valor))

function mover(direccion: 1 | -1) {
  emit('cambiar', Number(limitar(props.valor + direccion * props.paso).toFixed(decimales.value + 2)))
}

function arrastrar(e: PointerEvent) {
  if (props.desactivado || (e.type === 'pointermove' && e.buttons === 0)) return
  const barra = e.currentTarget as HTMLElement
  const { left, width } = barra.getBoundingClientRect()
  const relativo = Math.min(1, Math.max(0, (e.clientX - left) / width))
  const pasos = Math.round((relativo * (props.max - props.min)) / props.paso)
  const valor = Number(limitar(props.min + pasos * props.paso).toFixed(decimales.value + 2))
  if (valor !== props.valor) emit('cambiar', valor)
}

function empezar(e: PointerEvent) {
  ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  arrastrar(e)
}
</script>

<template>
  <div class="numero" role="group" :aria-label="etiqueta">
    <button :disabled="desactivado || valor <= min" :aria-label="`Bajar ${etiqueta}`" @click="mover(-1)">
      <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true"><path d="M2 7h10" /></svg>
    </button>
    <output @pointerdown="empezar" @pointermove="arrastrar">
      <span class="relleno" :style="{ transform: `scaleX(${relleno})` }" />
      <span class="cifra">{{ texto }}</span>
    </output>
    <button :disabled="desactivado || valor >= max" :aria-label="`Subir ${etiqueta}`" @click="mover(1)">
      <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true"><path d="M2 7h10M7 2v10" /></svg>
    </button>
  </div>
</template>

<style scoped>
.numero {
  display: grid;
  grid-template-columns: 44px minmax(84px, 1fr) 44px;
  gap: 4px;
  width: 100%;
  max-width: 240px;
}

button {
  display: grid;
  place-items: center;
  height: 44px;
  border-radius: var(--radio-control);
  background: var(--ground);
}

button:hover:not(:disabled) {
  background: var(--linea);
}

button:disabled {
  opacity: 0.4;
}

path {
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
}

output {
  position: relative;
  display: grid;
  place-items: center;
  overflow: hidden;
  border-radius: var(--radio-control);
  background: var(--ground);
  cursor: ew-resize;
  touch-action: none;
  user-select: none;
}

.relleno {
  position: absolute;
  inset: 0;
  background: var(--accent);
  opacity: 0.22;
  transform-origin: left;
}

.cifra {
  position: relative;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}
</style>
