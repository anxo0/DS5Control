<script setup lang="ts">
import { ref } from 'vue'
import { mando, puente } from '../estado/app'

const abierto = ref(false)
const d = puente.depuracion!
</script>

<template>
  <div class="depuracion" data-sin-foco>
    <div v-if="abierto" class="acciones">
      <button @click="d.mute()">Simular mute</button>
      <button @click="d.fijarBateria(12)">Batería baja</button>
      <button @click="d.fijarCargando(!mando?.cargando)">
        {{ mando?.cargando ? 'Dejar de cargar' : 'Cargando' }}
      </button>
      <button @click="d.fijarConectado(!mando?.conectado)">
        {{ mando?.conectado ? 'Desconectar' : 'Reconectar' }}
      </button>
      <button @click="d.quitarMandos()">Sin mando</button>
      <button @click="d.restablecer()">Restablecer</button>
      <button @click="d.actualizar()">Aviso de actualización</button>
    </div>
    <button class="abrir" :aria-expanded="abierto" @click="abierto = !abierto">Depuración</button>
  </div>
</template>

<style scoped>
.depuracion {
  position: fixed;
  left: 50%;
  bottom: 6px;
  transform: translateX(-50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: var(--muted);
}

.abrir {
  padding: 4px 8px;
  border-radius: 6px;
  opacity: 0.6;
}

.abrir:hover {
  opacity: 1;
}

.acciones {
  display: flex;
  gap: 4px;
  padding: 6px;
  background: var(--surface);
  border-radius: var(--radio-control);
  box-shadow: 0 0 0 1px var(--linea), 0 4px 12px rgba(0, 0, 0, 0.12);
}

.acciones button {
  padding: 6px 8px;
  border-radius: 6px;
  white-space: nowrap;
}

.acciones button:hover {
  background: var(--ground);
  color: var(--ink);
}
</style>
