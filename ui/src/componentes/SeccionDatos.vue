<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, watch } from 'vue'
import { cargandoDiagnostico, conectado, diagnostico, mando, pedirDiagnostico, pedirTelemetria } from '../estado/app'
import { iconoDe } from '../mandos/iconos'
import IconoMando from './IconoMando.vue'
import VistaDatos from './VistaDatos.vue'
import { local, t } from '../i18n'

onMounted(() => {
  pedirTelemetria(true)
  pedirDiagnostico()
})
onBeforeUnmount(() => pedirTelemetria(false))
watch(() => mando.value?.id, () => pedirDiagnostico())

const voltios = computed(() => {
  const mv = diagnostico.value?.milivoltios
  return mv == null ? null : mv / 1000
})

const uso = computed(() => diagnostico.value?.uso ?? null)
const maxPulsaciones = computed(() => Math.max(1, ...(uso.value?.pulsaciones ?? []).map((p) => p.valor)))
const pulsaciones = computed(() => [...(uso.value?.pulsaciones ?? [])].sort((a, b) => b.valor - a.valor))
const totalErrores = computed(() => (uso.value?.errores ?? []).reduce((s, e) => s + e.valor, 0))

function duracion(segundos: number) {
  const dias = Math.floor(segundos / 86400)
  const horas = Math.floor((segundos % 86400) / 3600)
  const minutos = Math.floor((segundos % 3600) / 60)
  return dias
    ? t('secciones.datos.duracion.dh', { d: dias, h: horas })
    : horas
      ? t('secciones.datos.duracion.hm', { h: horas, m: minutos })
      : t('secciones.datos.duracion.m', { m: minutos })
}

const numero = (v: number) => v.toLocaleString(local.value)

/** Mientras llega el diagnóstico, las tarjetas salen con su título y huecos donde irán los datos. */
const cargando = computed(() => cargandoDiagnostico.value && !diagnostico.value)
const esqueleto = [
  { titulo: 'secciones.datos.tiempoUso', filas: 4, grande: true },
  { titulo: 'secciones.datos.errores', filas: 3, grande: false },
  { titulo: 'secciones.datos.pulsaciones', filas: 6, grande: false },
  { titulo: 'secciones.datos.sticks', filas: 4, grande: false },
  { titulo: 'secciones.datos.conexion', filas: 3, grande: false },
]
</script>

<template>
  <main class="seccion" data-atras="[aria-current='page']">
    <header>
      <h1>{{ t('secciones.datos.titulo') }}</h1>
      <button data-foco-inicial :disabled="!conectado" @click="pedirDiagnostico()">{{ t('secciones.datos.actualizar') }}</button>
    </header>

    <p v-if="!conectado" class="nota">{{ t('secciones.datos.sinMando') }}</p>

    <div v-else class="columnas">
      <section class="tarjeta destacada">
        <h2>{{ t('secciones.datos.bateria') }}</h2>
        <div class="voltaje">
          <span v-if="voltios === null && cargando" class="hueso voltios" />
          <strong v-else>{{ voltios === null ? '...' : `${voltios.toFixed(2)} V` }}</strong>
          <span>{{ t('secciones.datos.voltajeAhora') }}</span>
        </div>
        <p class="nota">{{ t('secciones.datos.voltajeNota') }}</p>
        <dl v-if="cargando" aria-hidden="true">
          <div v-for="n in 3" :key="n"><span class="hueso" :style="{ width: `${48 + n * 9}%` }" /><span class="hueso dato" /></div>
        </dl>
        <dl v-if="uso">
          <div><dt>{{ t('secciones.datos.cargas') }}</dt><dd>{{ numero(uso.ciclosCarga) }}</dd></div>
          <div><dt>{{ t('secciones.datos.cargasCompletas') }}</dt><dd>{{ numero(uso.cargasCompletas) }}</dd></div>
          <div><dt>{{ t('secciones.datos.tiempoCargando') }}</dt><dd>{{ duracion(uso.carga) }}</dd></div>
        </dl>
      </section>

      <section class="tarjeta">
        <h2>{{ t('secciones.datos.ahora') }}</h2>
        <VistaDatos />
      </section>

      <template v-if="cargando">
        <section v-for="e in esqueleto" :key="e.titulo" class="tarjeta" aria-busy="true">
          <h2>{{ t(e.titulo) }}</h2>
          <span v-if="e.grande" class="hueso enorme-hueco" />
          <dl aria-hidden="true">
            <div v-for="n in e.filas" :key="n">
              <span class="hueso" :style="{ width: `${40 + ((n * 17) % 35)}%` }" /><span class="hueso dato" />
            </div>
          </dl>
        </section>
      </template>

      <section v-if="uso" class="tarjeta">
        <h2>{{ t('secciones.datos.tiempoUso') }}</h2>
        <p class="enorme">{{ duracion(uso.activo) }}</p>
        <dl>
          <div><dt>{{ t('secciones.datos.registros') }}</dt><dd>{{ numero(uso.registros) }}</dd></div>
          <div><dt>{{ t('secciones.datos.hapticos') }}</dt><dd>{{ duracion(uso.haptico) }}</dd></div>
          <div><dt>{{ t('secciones.datos.gatilloIzq') }}</dt><dd>{{ duracion(uso.gatilloIzq) }}</dd></div>
          <div><dt>{{ t('secciones.datos.gatilloDer') }}</dt><dd>{{ duracion(uso.gatilloDer) }}</dd></div>
        </dl>
      </section>

      <section v-if="uso" class="tarjeta">
        <h2>{{ t('secciones.datos.erroresTotal', { n: numero(totalErrores) }) }}</h2>
        <dl>
          <div v-for="f in uso.errores" :key="f.nombre"><dt>{{ f.nombre }}</dt><dd>{{ numero(f.valor) }}</dd></div>
        </dl>
      </section>

      <section v-if="uso" class="tarjeta">
        <h2>{{ t('secciones.datos.pulsaciones') }}</h2>
        <ul class="pulsaciones">
          <li v-for="p in pulsaciones" :key="p.nombre">
            <IconoMando v-if="iconoDe(p.nombre)" :control="p.nombre" :tamano="20" />
            <span v-else class="texto">{{ p.nombre }}</span>
            <div class="barra"><i :style="{ transform: `scaleX(${p.valor / maxPulsaciones})` }" /></div>
            <output>{{ numero(p.valor) }}</output>
          </li>
        </ul>
      </section>

      <section v-if="uso" class="tarjeta">
        <h2>{{ t('secciones.datos.sticks') }}</h2>
        <dl>
          <div v-for="f in uso.sticks" :key="f.nombre"><dt>{{ f.nombre }}</dt><dd>{{ numero(f.valor) }}</dd></div>
        </dl>
      </section>

      <section v-if="uso" class="tarjeta">
        <h2>{{ t('secciones.datos.conexion') }}</h2>
        <dl>
          <div v-for="f in uso.conexion" :key="f.nombre"><dt>{{ f.nombre }}</dt><dd>{{ numero(f.valor) }}</dd></div>
        </dl>
      </section>

      <section class="tarjeta">
        <h2>{{ t('secciones.datos.fabrica') }}</h2>
        <dl v-if="!diagnostico && cargando" aria-busy="true">
          <div v-for="n in 6" :key="n"><span class="hueso" :style="{ width: `${36 + ((n * 13) % 30)}%` }" /><span class="hueso dato largo" /></div>
        </dl>
        <p v-else-if="!diagnostico" class="nota">{{ t('secciones.datos.sinFabrica') }}</p>
        <dl v-else class="largos">
          <div v-for="f in diagnostico.fabrica" :key="f.nombre"><dt>{{ f.nombre }}</dt><dd>{{ f.valor }}</dd></div>
        </dl>
      </section>
    </div>
  </main>
</template>

<style scoped>
.seccion {
  flex: 999 1 520px;
  min-width: 0;
  padding: 20px 24px;
  background: var(--surface);
  border-radius: var(--radio-panel);
  display: flex;
  flex-direction: column;
  gap: 14px;
  overflow-y: auto;
}

header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

h1 {
  font-size: 28px;
  font-weight: 800;
  line-height: 1.15;
}

header button {
  min-height: 40px;
  padding: 0 14px;
  border-radius: var(--radio-control);
  background: var(--ground);
  font-size: 14px;
  font-weight: 600;
}

header button:hover:not(:disabled) {
  background: var(--linea);
}

.nota {
  color: var(--muted);
  font-size: 14px;
}

.columnas {
  columns: 3 280px;
  column-gap: 12px;
}

.tarjeta {
  display: grid;
  gap: 10px;
  margin-bottom: 12px;
  padding: 14px 16px;
  border: 1px solid var(--linea);
  border-radius: var(--radio-control);
  break-inside: avoid;
}

h2 {
  font-size: 15px;
  font-weight: 700;
}

.voltaje {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
}

.voltaje strong,
.enorme {
  font-size: 34px;
  font-weight: 800;
  line-height: 1.1;
  color: var(--accent);
  font-variant-numeric: tabular-nums;
}

.voltaje span {
  color: var(--muted);
  font-size: 13px;
}

dl {
  display: grid;
  gap: 4px;
  margin: 0;
  font-size: 13px;
}

dl div {
  display: flex;
  justify-content: space-between;
  gap: 12px;
}

dt {
  color: var(--muted);
}

dd {
  margin: 0;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.largos dd {
  min-width: 0;
  overflow-wrap: anywhere;
}

.pulsaciones {
  display: grid;
  gap: 4px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.pulsaciones li {
  display: grid;
  grid-template-columns: 56px 1fr 52px;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}

.texto {
  color: var(--muted);
  line-height: 1.1;
}

.barra {
  height: 8px;
  overflow: hidden;
  border-radius: 4px;
  background: var(--ground);
}

.barra i {
  display: block;
  height: 100%;
  background: var(--accent);
  transform-origin: left;
}

output {
  text-align: right;
  font-variant-numeric: tabular-nums;
}

/* Esqueleto: huecos del tamaño del texto que va a ocupar su sitio, con un latido suave. */
.hueso {
  display: block;
  height: 0.9em;
  border-radius: 4px;
  background: var(--ground);
  animation: latido 1.4s ease-in-out infinite;
}

dl .hueso {
  align-self: center;
}

.hueso.dato {
  width: 3.5em;
}

.hueso.largo {
  width: 7em;
}

.hueso.voltios {
  width: 4.2em;
  height: 34px;
  border-radius: 6px;
}

.hueso.enorme-hueco {
  width: 6em;
  height: 34px;
  border-radius: 6px;
}

@keyframes latido {
  50% {
    opacity: 0.45;
  }
}

@media (prefers-reduced-motion: reduce) {
  .hueso {
    animation: none;
  }
}
</style>
