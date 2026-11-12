<script setup lang="ts">
const TOTAL = 20

const props = defineProps<{
  id: string
  etiqueta: string
  valor: number
  min: number
  color: string
  desactivado?: boolean
}>()

const emit = defineEmits<{ cambiar: [valor: number] }>()

const altura = (i: number) => 14 + i * 1.6
const encendido = (i: number) => (i + 1) * 5 <= props.valor

const alCambiar = (e: Event) => emit('cambiar', Number((e.target as HTMLInputElement).value))
</script>

<template>
  <div class="control" :class="{ desactivado }">
    <div class="cabecera">
      <label :for="id">{{ etiqueta }}</label>
      <span>{{ valor }}%</span>
    </div>
    <div class="nivel">
      <span
        v-for="i in TOTAL"
        :key="i"
        aria-hidden="true"
        class="segmento"
        :style="{
          height: `${altura(i - 1)}px`,
          background: encendido(i - 1) ? color : 'var(--linea)',
        }"
      />
      <input
        :id="id"
        type="range"
        :min="min"
        max="100"
        step="5"
        :value="valor"
        :disabled="desactivado"
        :aria-valuetext="`${valor}%`"
        @input="alCambiar"
      />
    </div>
  </div>
</template>

<style scoped>
.cabecera {
  display: flex;
  justify-content: space-between;
  font-weight: 600;
  margin-bottom: 6px;
}

.nivel {
  position: relative;
  display: flex;
  align-items: flex-end;
  gap: 3px;
  height: 44px;
  border-radius: 6px;
}

.segmento {
  flex: 1 1 0;
  border-radius: 2px;
  transition: background-color 100ms var(--salida-rapida);
}

input {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  margin: 0;
  opacity: 0;
  cursor: ew-resize;
}

.nivel:hover .segmento {
  filter: saturate(1.15);
}

.nivel:has(input:focus-visible) {
  outline: 3px solid var(--accent);
  outline-offset: 3px;
}

:global([data-foco-activo] .nivel:has(input:focus)) {
  outline: 3px solid var(--accent);
  outline-offset: 3px;
}

.desactivado {
  opacity: 0.45;
}

.desactivado input {
  cursor: not-allowed;
}
</style>
