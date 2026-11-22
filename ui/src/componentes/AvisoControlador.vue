<script setup lang="ts">
import { estadoApp, instalarVigem } from '../estado/app'
import { t } from '../i18n'
</script>

<template>
  <div v-if="!estadoApp.servicios.vigemBus" class="aviso" role="alert">
    <svg class="icono" width="20" height="20" viewBox="0 0 20 20" aria-hidden="true">
      <path d="M10 3.2 17.4 16a.6.6 0 0 1-.5.9H3.1a.6.6 0 0 1-.5-.9L10 3.2Z" />
      <path d="M10 8.3v3.6" />
      <circle cx="10" cy="14.2" r="0.4" />
    </svg>
    <div class="texto">
      <strong>{{ t('general.vigem.titulo') }}</strong>
      <span>{{ t('general.vigem.texto') }}</span>
    </div>
    <button :disabled="estadoApp.servicios.instalandoVigem" @click="instalarVigem">
      {{ estadoApp.servicios.instalandoVigem ? t('general.vigem.instalando') : t('general.vigem.instalar') }}
    </button>
  </div>
</template>

<style scoped>
.aviso {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 0 12px 12px;
  padding: 10px 12px 10px 14px;
  border-radius: var(--radio-control);
  background: var(--surface);
}

/* El icono es lo único con el color de aviso. */
.icono {
  flex: none;
  fill: none;
  stroke: var(--aviso);
  stroke-width: 1.6;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.texto {
  display: flex;
  flex: 1;
  flex-wrap: wrap;
  gap: 2px 10px;
  min-width: 0;
}

span {
  color: var(--muted);
}

button {
  flex: none;
  min-height: 44px;
  padding: 0 16px;
  border-radius: var(--radio-control);
  background: var(--ink);
  color: var(--surface);
  font-weight: 600;
}

button:disabled {
  opacity: 0.6;
}
</style>
