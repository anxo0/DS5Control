<script setup lang="ts">
import { estadoApp, instalarVigem } from '../estado/app'
</script>

<template>
  <div v-if="!estadoApp.servicios.vigemBus" class="aviso" role="alert">
    <div>
      <strong>Falta ViGEmBus</strong>
      <span>Sin este controlador, los juegos no ven el mando virtual.</span>
    </div>
    <button :disabled="estadoApp.servicios.instalandoVigem" @click="instalarVigem">
      {{ estadoApp.servicios.instalandoVigem ? 'Instalando...' : 'Instalar ahora' }}
    </button>
  </div>
</template>

<style scoped>
.aviso {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  margin: 0 12px 12px;
  padding: 10px 12px 10px 16px;
  border-radius: var(--radio-control);
  background: var(--surface);
  box-shadow: inset 3px 0 0 var(--aviso);
}

.aviso div {
  display: flex;
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
