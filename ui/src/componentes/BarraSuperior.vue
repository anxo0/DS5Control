<script setup lang="ts">
import { computed } from 'vue'
import { conectado, mando, ocultar } from '../estado/app'
import { cambiarTema, etiquetaTema } from '../estado/tema'

/** Nombre del producto: aún sin decidir. */
const NOMBRE_PRODUCTO = '[NOMBRE]'

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
</script>

<template>
  <header class="barra" data-atras="[aria-current='page']">
    <div class="marca">
      <svg width="26" height="26" viewBox="0 0 26 26" fill="none" aria-hidden="true">
        <rect x="2" y="7" width="22" height="12" rx="6" stroke="var(--accent)" stroke-width="2.5" />
        <circle cx="9" cy="13" r="2.5" :fill="luz ?? 'var(--guia)'" />
      </svg>
      {{ NOMBRE_PRODUCTO }}
    </div>
    <div class="resumen" role="status">
      <span class="chip" :style="{ background: luz ?? 'var(--linea)' }" />
      <span>{{ resumen }}</span>
    </div>
    <button class="tema" @click="cambiarTema">{{ etiquetaTema }}</button>
    <button class="ocultar" @click="ocultar">Ocultar</button>
  </header>
</template>

<style scoped>
.barra {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px 16px;
  padding: 12px 20px;
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

.tema,
.ocultar {
  min-height: 44px;
  border-radius: var(--radio-control);
  font-weight: 600;
  transition: transform 120ms var(--salida-rapida), background-color 120ms var(--salida-rapida);
}

.tema {
  padding: 0 16px;
  background: var(--surface);
}

.tema:hover {
  background: var(--linea);
}

.ocultar {
  padding: 0 18px;
  background: var(--ink);
  color: var(--ground);
}

.ocultar:hover {
  background: var(--accent);
  color: var(--surface);
}
</style>
