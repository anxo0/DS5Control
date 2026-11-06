<script setup lang="ts">
import { computed } from 'vue'
import { conectado, controlarVentana, enVentana, mando, ocultar } from '../estado/app'
import { PALETAS, paleta } from '../estado/paleta'

const NOMBRE_PRODUCTO = 'DS5Control'

const CONEXIONES = { bluetooth: 'Bluetooth', usb: 'USB' } as const

const resumen = computed(() => {
  const m = mando.value
  if (!m) return 'Ningún mando conectado'
  if (!m.conectado) return `${m.nombre} · Desconectado`
  const partes = [m.nombre, CONEXIONES[m.conexion]]
  if (m.bateria !== null) partes.push(`${m.bateria}%`)
  return partes.join(' · ')
})

const luz = computed(() => (conectado.value ? mando.value?.luzActual : undefined))
const logo = computed(() => PALETAS.find((p) => p.id === paleta.value)!.logo)
</script>

<template>
  <header class="barra" data-atras="[aria-current='page']">
    <div class="marca">
      <img :src="logo" alt="" width="32" height="32" />
      {{ NOMBRE_PRODUCTO }}
    </div>
    <div class="resumen" role="status">
      <span class="chip" :style="{ background: luz ?? 'var(--linea)' }" />
      <span>{{ resumen }}</span>
    </div>
    <div v-if="enVentana" class="ventana">
      <button aria-label="Minimizar" @click="controlarVentana('minimizar')">
        <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true"><path d="M2 7h10" /></svg>
      </button>
      <button aria-label="Maximizar" @click="controlarVentana('maximizar')">
        <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true"><rect x="2.5" y="2.5" width="9" height="9" rx="1.5" /></svg>
      </button>
      <button class="cerrar" aria-label="Ocultar en segundo plano" @click="ocultar">
        <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true"><path d="M3 3l8 8M11 3l-8 8" /></svg>
      </button>
    </div>
    <button v-else class="ocultar" @click="ocultar">Ocultar</button>
  </header>
</template>

<style scoped>
.barra {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px 16px;
  padding: 8px 8px 8px 20px;
  app-region: drag;
}

.barra button {
  app-region: no-drag;
}

.marca {
  display: flex;
  align-items: center;
  gap: 10px;
  font-weight: 800;
  font-size: 18px;
}

.resumen {
  flex: 1 1 200px;
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 44px;
  padding: 0 14px;
  color: var(--muted);
}

.chip {
  width: 22px;
  height: 6px;
  border-radius: 3px;
  box-shadow: 0 0 0 1px var(--linea);
  transition: background-color 200ms var(--salida-rapida);
}

.ventana {
  display: flex;
  gap: 2px;
}

.ventana button {
  display: grid;
  place-items: center;
  width: 46px;
  height: 44px;
  border-radius: var(--radio-control);
  color: var(--muted);
  transition: background-color 120ms var(--salida-rapida), color 120ms var(--salida-rapida);
}

.ventana svg {
  fill: none;
  stroke: currentColor;
  stroke-width: 1.5;
  stroke-linecap: round;
}

.ventana button:hover {
  background: var(--linea);
  color: var(--ink);
}

.ventana .cerrar:hover {
  background: #c4163c;
  color: #ffffff;
}

.ocultar {
  min-height: 44px;
  border-radius: var(--radio-control);
  font-weight: 600;
  transition: transform 120ms var(--salida-rapida), background-color 120ms var(--salida-rapida);
  padding: 0 18px;
  background: var(--ink);
  color: var(--ground);
}

.ocultar:hover {
  background: var(--accent);
  color: var(--surface);
}
</style>
