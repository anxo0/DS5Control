<script setup lang="ts">
import { computed } from 'vue'
import { bateriaBaja, conectado, mando } from '../estado/app'
import IndicadorBateria from './IndicadorBateria.vue'
import DibujoMando from './DibujoMando.vue'

const titulo = computed(() => mando.value?.nombre ?? 'Conecta tu mando')

const detalle = computed(() => {
  const m = mando.value
  if (!m) return 'Aún no hay ningún mando. En cuanto conectes uno, aparecerá aquí.'
  if (!m.conectado) return 'Se ha desconectado. Enciéndelo o conéctalo por USB para seguir.'
  if (bateriaBaja.value) return 'Batería baja. Conéctalo por USB pronto.'
  const emulacion = m.emulando ? `emulando un mando de ${m.emulando}` : 'sin emulación'
  if (m.cargando) return `Cargando y ${emulacion}`
  return `Conectado y ${emulacion}`
})
</script>

<template>
  <main class="panel">
    <div class="cabecera">
      <div>
        <h1>{{ titulo }}</h1>
        <p :class="{ aviso: bateriaBaja }" aria-live="polite">{{ detalle }}</p>
      </div>
      <IndicadorBateria v-if="mando" :mando="mando" :baja="bateriaBaja" />
    </div>
    <DibujoMando :mando="mando" :conectado="conectado" />
  </main>
</template>

<style scoped>
.panel {
  flex: 999 1 520px;
  min-width: 0;
  padding: 24px 28px;
  background: var(--surface);
  border-radius: var(--radio-panel);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.cabecera {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  align-items: flex-end;
  gap: 12px;
}

h1 {
  font-size: 28px;
  font-weight: 800;
  line-height: 1.15;
  text-wrap: balance;
}

p {
  margin-top: 4px;
  color: var(--muted);
  text-wrap: pretty;
}

p.aviso {
  color: var(--aviso);
  font-weight: 600;
}
</style>
