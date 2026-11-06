<script setup lang="ts">
import { seccion } from '../estado/app'
import type { Seccion } from '../estado/app'

const SECCIONES: { id: Seccion; nombre: string }[] = [
  { id: 'mando', nombre: 'Mando' },
  { id: 'botones', nombre: 'Botones' },
  { id: 'perfiles', nombre: 'Perfiles' },
  { id: 'registro', nombre: 'Registro' },
  { id: 'ajustes', nombre: 'Ajustes' },
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
      {{ s.nombre }}
    </button>
  </nav>
</template>

<style scoped>
.nav {
  flex: 0 0 104px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.seccion {
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
