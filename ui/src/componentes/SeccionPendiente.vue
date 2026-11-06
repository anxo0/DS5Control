<script setup lang="ts">
import type { Seccion } from '../estado/app'

defineProps<{ seccion: Seccion }>()

const TITULOS: Partial<Record<Seccion, string>> = {
  botones: 'Botones',
  perfiles: 'Perfiles',
  registro: 'Registro',
}
</script>

<template>
  <main class="seccion">
    <h1>{{ TITULOS[seccion] }}</h1>
    <p>Esta pantalla todavía no está disponible.</p>
  </main>
</template>

<style scoped>
.seccion {
  flex: 999 1 520px;
  min-width: 0;
  padding: 24px 28px;
  background: var(--surface);
  border-radius: var(--radio-panel);
}

h1 {
  font-size: 28px;
  font-weight: 800;
  line-height: 1.15;
}

p {
  margin-top: 4px;
  color: var(--muted);
}
</style>
