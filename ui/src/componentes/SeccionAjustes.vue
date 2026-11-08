<script setup lang="ts">
import { onMounted } from 'vue'
import { cambiarAbrirConMute, cambiarControlConMando, cambiarEmulacion, cambiarGeneral, cargarGenerales, conectado, estadoApp, generales, mando } from '../estado/app'
import { PALETAS, cambiarPaleta, paleta } from '../estado/paleta'
import { cambiarTema, tema } from '../estado/tema'
import InterruptorFila from './InterruptorFila.vue'
import ListaAjustes from './ListaAjustes.vue'

const EMULACIONES = [
  { id: 'xbox', nombre: 'Xbox 360', detalle: 'La opción más compatible' },
  { id: 'ds4', nombre: 'DualShock 4', detalle: 'Muestra botones de PlayStation' },
  { id: 'nativo', nombre: 'DualSense real', detalle: 'Gatillos y vibración del juego. Con cable' },
] as const

onMounted(cargarGenerales)

function fijarTema(valor: 'claro' | 'oscuro') {
  if (tema.value !== valor) cambiarTema()
}
</script>

<template>
  <main class="seccion" data-atras="[aria-current='page']">
    <h1>Ajustes</h1>

    <section class="fila">
      <div class="etiqueta">
        <h2>Mando en los juegos</h2>
        <p>Cómo ven los juegos tu mando.</p>
      </div>
      <div class="opciones tres">
        <button
          v-for="(e, i) in EMULACIONES"
          :key="e.id"
          class="opcion"
          :class="{ activa: mando?.ajustes.emulacion === e.id }"
          :aria-pressed="mando?.ajustes.emulacion === e.id"
          :disabled="!conectado"
          :data-foco-inicial="i === 0 ? '' : undefined"
          @click="cambiarEmulacion(e.id)"
        >
          <strong>{{ e.nombre }}</strong>
          <span>{{ e.detalle }}</span>
        </button>
      </div>
    </section>

    <section class="fila">
      <div class="etiqueta">
        <h2>Tema</h2>
        <p>Aspecto de la aplicación.</p>
      </div>
      <div class="opciones dos">
        <button class="opcion" :class="{ activa: tema === 'claro' }" :aria-pressed="tema === 'claro'" @click="fijarTema('claro')">
          <strong>Claro</strong>
        </button>
        <button class="opcion" :class="{ activa: tema === 'oscuro' }" :aria-pressed="tema === 'oscuro'" @click="fijarTema('oscuro')">
          <strong>Oscuro</strong>
        </button>
      </div>
    </section>

    <section class="fila">
      <div class="etiqueta">
        <h2>Color del icono</h2>
        <p>Se aplica a la ventana y a la barra superior.</p>
      </div>
      <div class="opciones cuatro">
        <button
          v-for="p in PALETAS"
          :key="p.id"
          class="opcion paleta"
          :class="{ activa: paleta === p.id }"
          :aria-pressed="paleta === p.id"
          @click="cambiarPaleta(p.id)"
        >
          <img :src="p.logo" alt="" width="56" height="56" />
          <strong>{{ p.nombre }}</strong>
        </button>
      </div>
    </section>

    <section class="fila">
      <div class="etiqueta">
        <h2>Botón de silencio</h2>
        <p>Trae la app al frente o la oculta desde el mando.</p>
      </div>
      <div class="control">
        <InterruptorFila
          etiqueta="Abrir con el botón de silencio"
          :activo="estadoApp.abrirConMute"
          @cambiar="cambiarAbrirConMute"
        />
        <InterruptorFila
          etiqueta="Manejar la app con el mando"
          :activo="estadoApp.controlConMando"
          @cambiar="cambiarControlConMando"
        />
      </div>
    </section>

    <ListaAjustes class="generales" :secciones="generales" @cambiar="cambiarGeneral" />
    <p class="creditos">Iconos de botones por Zacksly (CC BY 3.0, zacksly.itch.io).</p>
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
  overflow-y: auto;
}

h1 {
  font-size: 28px;
  font-weight: 800;
  line-height: 1.15;
  margin-bottom: 8px;
}

.fila {
  display: grid;
  grid-template-columns: minmax(180px, 280px) minmax(0, 1fr);
  gap: 24px;
  padding: 20px 0;
  border-bottom: 1px solid var(--linea);
}

h2 {
  font-size: 16px;
  font-weight: 600;
}

.etiqueta p {
  margin-top: 2px;
  color: var(--muted);
  font-size: 14px;
}

.opciones {
  display: grid;
  gap: 10px;
  max-width: 640px;
}

.dos,
.tres {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.cuatro {
  grid-template-columns: repeat(4, minmax(0, 1fr));
}

.control {
  max-width: 640px;
}

.opcion {
  display: grid;
  justify-items: start;
  align-content: center;
  gap: 2px;
  min-height: 60px;
  padding: 10px 14px;
  border-radius: var(--radio-control);
  background: var(--ground);
  text-align: left;
  transition: background-color 120ms var(--salida-rapida), color 120ms var(--salida-rapida);
}

.opcion strong {
  font-weight: 600;
}

.opcion span {
  font-size: 13px;
  color: var(--muted);
}

.opcion:hover:not(:disabled):not(.activa) {
  background: var(--linea);
}

.opcion:disabled {
  opacity: 0.45;
}

.opcion.activa {
  background: var(--ink);
  color: var(--surface);
}

.opcion.activa span {
  color: var(--surface);
}

.paleta {
  justify-items: center;
  gap: 6px;
  text-align: center;
}

.generales {
  margin-top: 24px;
}

.creditos {
  margin-top: auto;
  padding-top: 32px;
  font-size: 13px;
  color: var(--muted);
}

@media (max-width: 860px) {
  .fila {
    grid-template-columns: minmax(0, 1fr);
    gap: 10px;
  }
}
</style>
