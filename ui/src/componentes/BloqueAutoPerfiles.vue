<script setup lang="ts">
import { onMounted } from 'vue'
import {
  anadirAutoPerfil,
  autoPerfiles,
  cambiarAutoPerfil,
  cargarAutoPerfiles,
  estadoApp,
  quitarAutoPerfil,
} from '../estado/app'

onMounted(cargarAutoPerfiles)
</script>

<template>
  <section class="auto">
    <h2>Cambiar de perfil al abrir un juego</h2>
    <p>Elige el juego y el perfil que se pone solo. Al cerrarlo, vuelve el perfil anterior.</p>

    <ul v-if="autoPerfiles.length">
      <li v-for="a in autoPerfiles" :key="a.ruta">
        <div class="cabecera">
          <strong :title="a.ruta">{{ a.nombre }}</strong>
          <button class="quitar" @click="quitarAutoPerfil(a.ruta)">Quitar</button>
        </div>
        <div class="chips">
          <button
            v-for="p in estadoApp.perfiles"
            :key="p"
            :class="{ activa: a.perfil === p }"
            :aria-pressed="a.perfil === p"
            @click="cambiarAutoPerfil(a.ruta, p)"
          >
            {{ p }}
          </button>
        </div>
      </li>
    </ul>

    <button class="anadir" @click="anadirAutoPerfil">Añadir juego</button>
  </section>
</template>

<style scoped>
.auto {
  display: grid;
  gap: 10px;
  max-width: 640px;
}

h2 {
  font-size: 16px;
  font-weight: 700;
}

p {
  color: var(--muted);
  font-size: 14px;
}

ul {
  display: grid;
  gap: 14px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.cabecera {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-bottom: 6px;
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.chips button,
.quitar,
.anadir {
  min-height: 44px;
  padding: 0 14px;
  border-radius: var(--radio-control);
  background: var(--ground);
  font-weight: 600;
}

.chips button {
  font-size: 14px;
}

.chips button:hover:not(.activa),
.quitar:hover,
.anadir:hover {
  background: var(--linea);
}

.chips .activa {
  background: var(--ink);
  color: var(--surface);
}

.anadir {
  justify-self: start;
}
</style>
