<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'

/** Versión a la que se está actualizando, o null si no hay actualización en marcha. */
const props = defineProps<{ version: string | null }>()

const dialogo = ref<HTMLDialogElement>()

watch(
  () => props.version,
  async (version) => {
    await nextTick()
    const el = dialogo.value
    if (el && version && !el.open) el.showModal()
  },
  { immediate: true },
)
</script>

<template>
  <!-- Sin @cancel que cierre: la ventana se cerrará sola y no hay nada que decidir. -->
  <dialog
    ref="dialogo"
    class="aviso"
    aria-labelledby="aviso-titulo"
    aria-describedby="aviso-texto"
    @cancel.prevent
  >
    <h2 id="aviso-titulo">Actualizando a la versión {{ version }}</h2>
    <p id="aviso-texto">
      DS5Control se cerrará y volverá a abrirse solo en unos segundos. No hace falta que hagas nada.
    </p>
    <span class="barra" aria-hidden="true"><span /></span>
  </dialog>
</template>

<style scoped>
.aviso {
  width: min(420px, calc(100vw - 32px));
  margin: auto;
  padding: 22px 24px 20px;
  border: none;
  border-radius: var(--radio-panel);
  background: var(--surface);
  color: var(--ink);
  box-shadow:
    0 0 0 1px color-mix(in srgb, var(--ink) 8%, transparent),
    0 24px 48px -16px rgba(10, 14, 22, 0.35),
    0 4px 12px -4px rgba(10, 14, 22, 0.18);
  transition:
    opacity 220ms var(--salida-rapida),
    transform 220ms var(--salida-rapida);
}

@starting-style {
  .aviso[open] {
    opacity: 0;
    transform: scale(0.96);
  }
}

.aviso::backdrop {
  background: rgba(10, 14, 22, 0.5);
}

h2 {
  font-size: 18px;
  font-weight: 800;
  line-height: 1.25;
}

p {
  margin-top: 6px;
  color: var(--muted);
  font-size: 14px;
  line-height: 1.5;
}

/* Se llena en lo que tarda la ventana en cerrarse: el tiempo de espera es real, no adorno. */
.barra {
  display: block;
  height: 4px;
  margin-top: 18px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--ground);
}

.barra span {
  display: block;
  height: 100%;
  background: var(--accent);
  transform-origin: left;
  animation: llenar 2500ms linear forwards;
}

@keyframes llenar {
  from {
    transform: scaleX(0);
  }
  to {
    transform: scaleX(1);
  }
}

@media (prefers-reduced-motion: reduce) {
  .barra span {
    animation: none;
    transform: none;
  }
}
</style>
