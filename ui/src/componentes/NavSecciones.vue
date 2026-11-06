<script setup lang="ts">
import { seccion } from '../estado/app'
import type { Seccion } from '../estado/app'

/** Trazos propios sobre una rejilla de 20 x 20. */
const SECCIONES: { id: Seccion; nombre: string; icono: string }[] = [
  {
    id: 'mando',
    nombre: 'Mando',
    icono: 'M6 5h8c2 0 3 1.500 3.500 4l.8 4.500c.3 2-.9 3-2 3-1 0-1.600-.7-2.300-2l-.8-1.500H6.800L6 15c-.7 1.300-1.300 2-2.300 2-1.100 0-2.300-1-2-3L2.500 9C3 6.500 4 5 6 5Z M6.500 8v3 M5 9.500h3 M13.500 9.500h.01',
  },
  {
    id: 'botones',
    nombre: 'Botones',
    icono: 'M10 2.500a2.500 2.500 0 1 1 0 5 2.500 2.500 0 0 1 0-5Z M10 12.500a2.500 2.500 0 1 1 0 5 2.500 2.500 0 0 1 0-5Z M5 7.500a2.500 2.500 0 1 1 0 5 2.500 2.500 0 0 1 0-5Z M15 7.500a2.500 2.500 0 1 1 0 5 2.500 2.500 0 0 1 0-5Z',
  },
  {
    id: 'perfiles',
    nombre: 'Perfiles',
    icono: 'M10 2.500 17.500 6.500 10 10.500 2.500 6.500Z M2.500 10 10 14 17.500 10 M2.500 13.500 10 17.500 17.500 13.500',
  },
  {
    id: 'acciones',
    nombre: 'Acciones',
    icono: 'M11 2.500 4 11.500h5l-1 6 7-9h-5Z',
  },
  {
    id: 'avanzado',
    nombre: 'Avanzado',
    icono: 'M10 6.500a3.500 3.500 0 1 1 0 7 3.500 3.500 0 0 1 0-7Z M10 2.500v2 M10 15.500v2 M2.500 10h2 M15.500 10h2 M4.700 4.700l1.400 1.400 M13.900 13.900l1.400 1.400 M15.300 4.700l-1.400 1.400 M6.100 13.900l-1.400 1.400',
  },
  {
    id: 'registro',
    nombre: 'Registro',
    icono: 'M4.500 2.500h11v15h-11Z M7.500 6.500h5 M7.500 10h5 M7.500 13.500h3',
  },
  {
    id: 'ajustes',
    nombre: 'Ajustes',
    icono: 'M3 6h6 M13 6h4 M3 14h3 M10 14h7 M11 3.500v5 M8 11.500v5',
  },
]
</script>

<template>
  <nav aria-label="Secciones" class="nav">
    <button
      v-for="s in SECCIONES"
      :key="s.id"
      class="seccion"
      :class="{ activa: seccion === s.id, final: s.id === 'ajustes' }"
      :aria-current="seccion === s.id ? 'page' : undefined"
      @click="seccion = s.id"
    >
      <svg width="20" height="20" viewBox="0 0 20 20" aria-hidden="true"><path :d="s.icono" /></svg>
      {{ s.nombre }}
    </button>
  </nav>
</template>

<style scoped>
.nav {
  flex: 0 0 136px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.seccion svg {
  flex: none;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.5;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.seccion {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 48px;
  text-align: left;
  padding: 0 12px;
  border-radius: var(--radio-control);
  color: var(--muted);
  transition: transform 120ms var(--salida-rapida), color 120ms var(--salida-rapida),
    background-color 120ms var(--salida-rapida);
}

.seccion:hover {
  color: var(--ink);
}

.activa {
  background: var(--surface);
  color: var(--ink);
  font-weight: 600;
}

.final {
  margin-top: auto;
}
</style>
