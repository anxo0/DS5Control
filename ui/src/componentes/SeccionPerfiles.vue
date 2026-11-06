<script setup lang="ts">
import { computed, ref } from 'vue'
import BloqueAutoPerfiles from './BloqueAutoPerfiles.vue'
import { borrarPerfil, conectado, crearPerfil, estadoApp, mando, usarPerfil } from '../estado/app'

const nombre = ref('')
const porBorrar = ref<string | null>(null)

const repetido = computed(() =>
  estadoApp.value.perfiles.some((p) => p.toLowerCase() === nombre.value.trim().toLowerCase()),
)
const invalido = computed(() => /[\\/:*?"<>|]/.test(nombre.value))
const puedeCrear = computed(() => nombre.value.trim() !== '' && !repetido.value && !invalido.value)

function crear() {
  if (!puedeCrear.value) return
  crearPerfil(nombre.value.trim())
  nombre.value = ''
}

function pedirBorrado(perfil: string) {
  if (porBorrar.value === perfil) {
    borrarPerfil(perfil)
    porBorrar.value = null
  } else {
    porBorrar.value = perfil
  }
}
</script>

<template>
  <main class="seccion" data-atras="[aria-current='page']">
    <h1>Perfiles</h1>
    <p class="ayuda">Cada perfil guarda luz, vibración, gatillos y botones.</p>

    <ul class="lista">
      <li v-for="(p, i) in estadoApp.perfiles" :key="p">
        <span class="nombre">{{ p }}</span>
        <span v-if="mando?.perfil === p" class="uso">En uso</span>
        <button
          v-else
          :disabled="!conectado"
          :data-foco-inicial="i === 0 ? '' : undefined"
          @click="usarPerfil(p)"
        >
          Usar
        </button>
        <button
          class="borrar"
          :class="{ confirmar: porBorrar === p }"
          :disabled="mando?.perfil === p"
          @click="pedirBorrado(p)"
        >
          {{ porBorrar === p ? 'Confirmar' : 'Borrar' }}
        </button>
      </li>
    </ul>

    <form class="nuevo" @submit.prevent="crear">
      <label for="nombre-perfil">Nuevo perfil, a partir del actual</label>
      <div class="fila">
        <input id="nombre-perfil" v-model="nombre" maxlength="60" autocomplete="off" />
        <button type="submit" class="crear" :disabled="!puedeCrear">Crear</button>
      </div>
      <p v-if="repetido" class="error">Ya hay un perfil con ese nombre.</p>
      <p v-else-if="invalido" class="error">El nombre no puede llevar \ / : * ? " &lt; &gt; |</p>
    </form>

    <BloqueAutoPerfiles />
  </main>
</template>

<style scoped>
.seccion {
  flex: 999 1 520px;
  min-width: 0;
  padding: 24px 28px;
  background: var(--surface);
  border-radius: var(--radio-panel);
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
}

h1 {
  font-size: 28px;
  font-weight: 800;
  line-height: 1.15;
}

.ayuda {
  color: var(--muted);
}

.lista {
  margin: 8px 0 0;
  padding: 0;
  list-style: none;
  max-width: 720px;
}

li {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 60px;
  border-bottom: 1px solid var(--linea);
}

.nombre {
  flex: 1 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
}

.uso {
  min-width: 64px;
  padding: 0 14px;
  color: var(--muted);
  font-size: 14px;
  text-align: center;
}

button {
  flex: none;
  min-height: 44px;
  padding: 0 16px;
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

.borrar {
  background: transparent;
  color: var(--muted);
}

.borrar.confirmar {
  background: var(--aviso);
  color: #ffffff;
}

.nuevo {
  display: grid;
  gap: 8px;
  max-width: 720px;
  margin-top: 16px;
}

label {
  font-weight: 600;
}

.fila {
  display: flex;
  gap: 8px;
}

input {
  flex: 1 1 auto;
  min-width: 0;
  min-height: 44px;
  box-sizing: border-box;
  padding: 0 12px;
  border: 1px solid var(--linea);
  border-radius: var(--radio-control);
  background: var(--ground);
  color: var(--ink);
  font: inherit;
}

.crear {
  background: var(--ink);
  color: var(--surface);
}

.crear:hover:not(:disabled) {
  background: var(--accent);
}

.error {
  color: var(--aviso);
  font-size: 13px;
}
</style>
