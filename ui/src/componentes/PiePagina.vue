<script setup lang="ts">
import { computed } from 'vue'
import circulo from '../assets/iconos/Circle.svg'
import cruz from '../assets/iconos/Cross.svg'
import cruceta from '../assets/iconos/D-Pad.svg'
import l1 from '../assets/iconos/L1.svg'
import r1 from '../assets/iconos/R1.svg'
import { estadoApp } from '../estado/app'

/** Cada icono se pinta como máscara para que tome el color del tema. */
const AYUDAS = [
  { iconos: [{ src: cruz, nombre: 'Cruz' }], texto: 'Elegir' },
  { iconos: [{ src: circulo, nombre: 'Círculo' }], texto: 'Atrás' },
  { iconos: [{ src: l1, nombre: 'L1' }, { src: r1, nombre: 'R1' }], texto: 'Cambiar pestaña' },
  { iconos: [{ src: cruceta, nombre: 'Cruceta' }], texto: 'Moverse y ajustar' },
]

const servicios = computed(() => {
  const { vigemBus, hidHide } = estadoApp.value.servicios
  return `ViGEmBus ${vigemBus ? 'instalado' : 'no instalado'} · HidHide ${hidHide ? 'activo' : 'inactivo'}`
})
</script>

<template>
  <footer class="pie">
    <span v-for="ayuda in estadoApp.controlConMando ? AYUDAS : []" :key="ayuda.texto" class="ayuda">
      <span
        v-for="icono in ayuda.iconos"
        :key="icono.nombre"
        class="icono"
        role="img"
        :aria-label="icono.nombre"
        :style="{ maskImage: `url(&quot;${icono.src}&quot;)` }"
      />
      {{ ayuda.texto }}
    </span>
    <span class="servicios">{{ servicios }}</span>
  </footer>
</template>

<style scoped>
.pie {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 24px;
  padding: 0 20px 12px;
  font-size: 13px;
  color: var(--muted);
}

.ayuda {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.icono {
  width: 26px;
  height: 26px;
  background: currentColor;
  mask-size: contain;
  mask-repeat: no-repeat;
  mask-position: center;
}

.servicios {
  margin-left: auto;
}
</style>
