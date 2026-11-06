<script setup lang="ts">
import { cambiarAbrirConMute, estadoApp } from '../estado/app'
import InterruptorFila from './InterruptorFila.vue'
</script>

<template>
  <main class="seccion" data-atras="[aria-current='page']">
    <h1>Ajustes</h1>
    <div class="opcion">
      <InterruptorFila
        data-foco-inicial
        etiqueta="Abrir con el botón de silencio"
        :activo="estadoApp.abrirConMute"
        @cambiar="cambiarAbrirConMute"
      />
      <p>Púlsalo en el mando para traer la app al frente o volver a ocultarla.</p>
    </div>
  </main>
</template>

<style scoped>
.seccion {
  flex: 999 1 520px;
  min-width: 0;
  padding: 24px 28px;
  background: var(--surface);
  border-radius: var(--radio-panel);
  display: flex;
  flex-direction: column;
  gap: 18px;
}

h1 {
  font-size: 28px;
  font-weight: 800;
  line-height: 1.15;
}

.opcion {
  max-width: 480px;
}

p {
  color: var(--muted);
}
</style>
