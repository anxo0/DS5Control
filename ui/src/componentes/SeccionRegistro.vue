<script setup lang="ts">
import { computed, ref } from 'vue'
import { estadoApp } from '../estado/app'

const entradas = computed(() => [...estadoApp.value.registro].reverse())
const copiado = ref(false)

async function copiar() {
  const texto = estadoApp.value.registro.map((e) => `${e.hora} ${e.texto}`).join('\n')
  try {
    await navigator.clipboard.writeText(texto)
    copiado.value = true
    setTimeout(() => (copiado.value = false), 1500)
  } catch {
    copiado.value = false
  }
}
</script>

<template>
  <main class="seccion" data-atras="[aria-current='page']">
    <div class="cabecera">
      <h1>Registro</h1>
      <button data-foco-inicial :disabled="entradas.length === 0" @click="copiar">
        {{ copiado ? 'Copiado' : 'Copiar' }}
      </button>
    </div>
    <p v-if="entradas.length === 0" class="vacio">Todavía no hay mensajes en esta sesión.</p>
    <ol v-else class="lista">
      <li v-for="(e, i) in entradas" :key="i" :class="{ aviso: e.aviso }">
        <time>{{ e.hora }}</time>
        <span>{{ e.texto }}</span>
      </li>
    </ol>
  </main>
</template>

<style scoped>
.seccion {
  flex: 999 1 520px;
  min-width: 0;
  min-height: 0;
  padding: 24px 28px;
  background: var(--surface);
  border-radius: var(--radio-panel);
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.cabecera {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

h1 {
  font-size: 28px;
  font-weight: 800;
  line-height: 1.15;
}

button {
  min-height: 44px;
  padding: 0 18px;
  border-radius: var(--radio-control);
  background: var(--ground);
  font-weight: 600;
}

button:hover:not(:disabled) {
  background: var(--linea);
}

button:disabled {
  opacity: 0.45;
}

.vacio {
  color: var(--muted);
}

.lista {
  flex: 1 1 0;
  min-height: 0;
  max-height: 560px;
  margin: 0;
  padding: 0;
  list-style: none;
  overflow-y: auto;
}

li {
  display: grid;
  grid-template-columns: 72px minmax(0, 1fr);
  gap: 12px;
  padding: 8px 0;
  border-bottom: 1px solid var(--linea);
  font-size: 14px;
  line-height: 1.4;
}

time {
  color: var(--muted);
  font-variant-numeric: tabular-nums;
}

span {
  overflow-wrap: anywhere;
}

.aviso span {
  color: var(--aviso);
}
</style>
