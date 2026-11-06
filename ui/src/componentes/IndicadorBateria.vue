<script setup lang="ts">
import { computed } from 'vue'
import type { EstadoMando } from '../bridge'

const props = defineProps<{ mando: EstadoMando; baja: boolean }>()

const conLectura = computed(() => props.mando.conectado && props.mando.bateria !== null)
const nivel = computed(() => (conLectura.value ? (props.mando.bateria ?? 0) : 0))
const etiqueta = computed(() => {
  if (props.mando.cargando && conLectura.value) {
    return nivel.value === 100 ? 'Carga completa' : 'Cargando'
  }
  return 'Batería'
})
</script>

<template>
  <div class="bateria" :class="{ baja, cargando: mando.cargando && conLectura }">
    <div class="fila">
      <span class="etiqueta">
        <svg
          v-if="mando.cargando && conLectura"
          class="rayo"
          width="12"
          height="16"
          viewBox="0 0 12 16"
          aria-hidden="true"
        >
          <path d="M7 0 0 9h4.5L3.5 16 12 6.5H7.2L7 0Z" fill="currentColor" />
        </svg>
        {{ etiqueta }}
      </span>
      <span v-if="conLectura" class="valor">{{ nivel }}%</span>
      <span v-else class="sin-datos">Sin datos</span>
    </div>
    <div class="pista">
      <div class="carga" :style="{ width: `${nivel}%` }" />
    </div>
  </div>
</template>

<style scoped>
.bateria {
  min-width: 150px;
}

.fila {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 16px;
}

.etiqueta {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--muted);
}

.valor {
  font-size: 32px;
  font-weight: 800;
  line-height: 1;
}

.sin-datos {
  font-size: 18px;
  font-weight: 600;
  line-height: 32px;
  color: var(--muted);
}

.pista {
  height: 8px;
  margin-top: 8px;
  border-radius: 4px;
  background: var(--linea);
  overflow: hidden;
}

.carga {
  height: 100%;
  background: var(--ink);
  transition: width 300ms var(--salida-rapida), background-color 200ms var(--salida-rapida);
}

.baja .valor,
.baja .etiqueta {
  color: var(--aviso);
}

.baja .carga {
  background: var(--aviso);
}

.cargando .etiqueta {
  color: var(--ink);
}

.cargando .carga {
  background: var(--accent);
  animation: latido 1.6s ease-in-out infinite;
}

@keyframes latido {
  50% {
    opacity: 0.55;
  }
}
</style>
