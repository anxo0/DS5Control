<script setup lang="ts">
import { computed } from 'vue'
import { estadoApp } from '../estado/app'

const servicios = computed(() => {
  const { vigemBus, hidHide } = estadoApp.value.servicios
  return `ViGEmBus ${vigemBus ? 'instalado' : 'no instalado'} · HidHide ${hidHide ? 'activo' : 'inactivo'}`
})
</script>

<template>
  <footer class="pie">
    <span>Cruz: elegir</span>
    <span>Círculo: atrás</span>
    <span>L1 y R1: cambiar pestaña</span>
    <span>Cruceta: ajustar</span>
    <span class="servicios">{{ servicios }}</span>
  </footer>
</template>

<style scoped>
.pie {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 28px;
  padding: 0 20px 14px;
  font-size: 13px;
  color: var(--muted);
}

.servicios {
  margin-left: auto;
}
</style>
