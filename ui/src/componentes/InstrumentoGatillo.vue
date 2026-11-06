<script setup lang="ts">
import InstrumentoRespuesta from './InstrumentoRespuesta.vue'

defineProps<{
  recorrido: number
  muerta: number
  maxima: number
  curva: number
  personalizada: string
}>()
</script>

<template>
  <div class="instrumento">
    <div class="carril" role="img" aria-label="Recorrido del gatillo y zonas ajustadas">
      <span class="muerta" :style="{ width: `${muerta * 100}%` }" />
      <span class="fuera" :style="{ left: `${maxima * 100}%` }" />
      <span class="lleno" :style="{ transform: `scaleX(${recorrido})` }" />
    </div>
    <ul class="leyenda">
      <li><span class="m muerta" />No hace nada</li>
      <li><span class="m lleno" />Lo que pulsas</li>
    </ul>
    <InstrumentoRespuesta :entrada="recorrido" :muerta="muerta" :maxima="maxima" :curva="curva" :personalizada="personalizada" />
  </div>
</template>

<style scoped>
.instrumento {
  display: grid;
  gap: 12px;
}

.carril {
  position: relative;
  height: 72px;
  overflow: hidden;
  border-radius: var(--radio-control);
  background: var(--ground);
}

.carril span {
  position: absolute;
  top: 0;
  bottom: 0;
}

.carril .muerta {
  left: 0;
  background: var(--guia);
  opacity: 0.5;
}

.fuera {
  right: 0;
  background: var(--guia);
  opacity: 0.5;
}

.carril .lleno {
  left: 0;
  width: 100%;
  background: var(--accent);
  opacity: 0.75;
  transform-origin: left;
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
  border-radius: 3px;
}

.m.muerta {
  background: var(--guia);
}

.m.lleno {
  background: var(--accent);
}
</style>
