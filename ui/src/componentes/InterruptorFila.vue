<script setup lang="ts">
defineProps<{ etiqueta: string; activo: boolean; desactivado?: boolean }>()
defineEmits<{ cambiar: [activo: boolean] }>()
</script>

<template>
  <button
    class="fila"
    role="switch"
    :aria-checked="activo"
    :disabled="desactivado"
    @click="$emit('cambiar', !activo)"
  >
    {{ etiqueta }}
    <span aria-hidden="true" class="pista" :class="{ activo }"><span class="bola" /></span>
  </button>
</template>

<style scoped>
.fila {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  width: 100%;
  min-height: 48px;
  font-weight: 600;
  text-align: left;
}

.fila:disabled {
  opacity: 0.45;
}

.fila:active:not(:disabled) {
  transform: none;
}

.pista {
  position: relative;
  flex: none;
  width: 44px;
  height: 26px;
  border-radius: 13px;
  background: var(--guia);
  transition: background-color 160ms var(--salida-rapida);
}

.pista.activo {
  background: var(--accent);
}

.bola {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--surface);
  transition: transform 160ms var(--salida-rapida);
}

.activo .bola {
  transform: translateX(18px);
}
</style>
