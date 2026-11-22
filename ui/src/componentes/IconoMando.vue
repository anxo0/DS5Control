<script setup lang="ts">
import { computed } from 'vue'
import { iconoDe } from '../mandos/iconos'
import { t } from '../i18n'

const props = withDefaults(defineProps<{ control: string; tamano?: number }>(), { tamano: 26 })
const src = computed(() => iconoDe(props.control))

/** Los nombres en español que llegan como control, con su texto en cada idioma. */
const CLAVES: Record<string, string> = {
  Cruz: 'mando.control.cruz',
  Círculo: 'mando.control.circulo',
  Cuadrado: 'mando.control.cuadrado',
  Triángulo: 'mando.control.triangulo',
  'Stick izquierdo': 'mando.control.stickIzquierdo',
  'Stick derecho': 'mando.control.stickDerecho',
  Izquierdo: 'mando.sticks.izquierdo',
  Derecho: 'mando.sticks.derecho',
  Arriba: 'mando.control.arriba',
  Abajo: 'mando.control.abajo',
  Izquierda: 'mando.control.izquierda',
  Derecha: 'mando.control.derecha',
  'Cruceta arriba': 'mando.control.crucetaArriba',
  'Cruceta abajo': 'mando.control.crucetaAbajo',
  'Cruceta izquierda': 'mando.control.crucetaIzquierda',
  'Cruceta derecha': 'mando.control.crucetaDerecha',
  Crear: 'mando.control.crear',
  'Clic del panel': 'mando.control.clicPanel',
  'Panel táctil': 'mando.control.panelTactil',
  'Sin función': 'mando.sinFuncion',
}
const nombre = computed(() => {
  const clave = CLAVES[props.control]
  return clave ? t(clave) : props.control
})
</script>

<template>
  <span
    v-if="src"
    class="icono"
    role="img"
    :aria-label="nombre"
    :title="nombre"
    :style="{ maskImage: `url(&quot;${src}&quot;)`, width: `${tamano}px`, height: `${tamano}px` }"
  />
  <span v-else>{{ nombre }}</span>
</template>

<style scoped>
.icono {
  display: inline-block;
  flex: none;
  vertical-align: middle;
  background: currentColor;
  mask-size: contain;
  mask-repeat: no-repeat;
  mask-position: center;
}
</style>
