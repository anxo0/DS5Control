<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, watch } from 'vue'
import { conectado, diagnostico, mando, pedirDiagnostico, pedirTelemetria } from '../estado/app'
import { iconoDe } from '../mandos/iconos'
import IconoMando from './IconoMando.vue'
import VistaDatos from './VistaDatos.vue'

/** Voltajes orientativos de una batería de litio de 3,7 V. */
const VOLTIOS_VACIA = 3.0
const VOLTIOS_LLENA = 4.2
const VOLTIOS_BAJA = 3.5
const VOLTIOS_MEDIA = 3.7

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
const nivelVoltios = computed(() => {
  const v = voltios.value
  return v === null ? 0 : Math.max(0, Math.min(1, (v - VOLTIOS_VACIA) / (VOLTIOS_LLENA - VOLTIOS_VACIA)))
})
const lecturaVoltios = computed(() => {
  const v = voltios.value
  if (v === null) return ''
  if (v < VOLTIOS_BAJA) return 'Muy baja'
  if (v < VOLTIOS_MEDIA) return 'Baja'
  return 'Bien'
})

const uso = computed(() => diagnostico.value?.uso ?? null)
const maxPulsaciones = computed(() => Math.max(1, ...(uso.value?.pulsaciones ?? []).map((p) => p.valor)))
const pulsaciones = computed(() => [...(uso.value?.pulsaciones ?? [])].sort((a, b) => b.valor - a.valor))
const totalErrores = computed(() => (uso.value?.errores ?? []).reduce((s, e) => s + e.valor, 0))

function duracion(segundos: number) {
  const dias = Math.floor(segundos / 86400)
  const horas = Math.floor((segundos % 86400) / 3600)
  const minutos = Math.floor((segundos % 3600) / 60)
  return dias ? `${dias} d ${horas} h` : horas ? `${horas} h ${minutos} min` : `${minutos} min`
}

const numero = (v: number) => v.toLocaleString('es-ES')
</script>

<template>
  <main class="seccion" data-atras="[aria-current='page']">
    <header>
      <h1>Datos</h1>
      <button data-foco-inicial :disabled="!conectado" @click="pedirDiagnostico()">Actualizar</button>
    </header>

    <p v-if="!conectado" class="nota">Conecta un mando para ver sus datos.</p>

    <div v-else class="columnas">
      <section class="tarjeta destacada">
        <h2>Batería</h2>
        <div class="voltaje">
          <strong>{{ voltios === null ? '...' : `${voltios.toFixed(2)} V` }}</strong>
          <span>{{ lecturaVoltios }}</span>
        </div>
        <div class="carril"><i :style="{ transform: `scaleX(${nivelVoltios})` }" /></div>
        <dl v-if="uso">
          <div><dt>Ciclos de carga</dt><dd>{{ numero(uso.ciclosCarga) }}</dd></div>
          <div><dt>Cargas completas</dt><dd>{{ numero(uso.cargasCompletas) }}</dd></div>
          <div><dt>Tiempo total cargando</dt><dd>{{ duracion(uso.carga) }}</dd></div>
        </dl>
      </section>

      <section class="tarjeta">
        <h2>Ahora mismo</h2>
        <VistaDatos />
      </section>

      <section v-if="uso" class="tarjeta">
        <h2>Tiempo de uso</h2>
        <p class="enorme">{{ duracion(uso.activo) }}</p>
        <dl>
          <div><dt>Registros</dt><dd>{{ numero(uso.registros) }}</dd></div>
          <div><dt>Motores hápticos</dt><dd>{{ duracion(uso.haptico) }}</dd></div>
          <div><dt>Gatillo izquierdo</dt><dd>{{ duracion(uso.gatilloIzq) }}</dd></div>
          <div><dt>Gatillo derecho</dt><dd>{{ duracion(uso.gatilloDer) }}</dd></div>
        </dl>
      </section>

      <section v-if="uso" class="tarjeta">
        <h2>Errores registrados: {{ numero(totalErrores) }}</h2>
        <dl>
          <div v-for="f in uso.errores" :key="f.nombre"><dt>{{ f.nombre }}</dt><dd>{{ numero(f.valor) }}</dd></div>
        </dl>
      </section>

      <section v-if="uso" class="tarjeta">
        <h2>Pulsaciones</h2>
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
        <h2>Sticks</h2>
        <dl>
          <div v-for="f in uso.sticks" :key="f.nombre"><dt>{{ f.nombre }}</dt><dd>{{ numero(f.valor) }}</dd></div>
        </dl>
      </section>

      <section v-if="uso" class="tarjeta">
        <h2>Conexión</h2>
        <dl>
          <div v-for="f in uso.conexion" :key="f.nombre"><dt>{{ f.nombre }}</dt><dd>{{ numero(f.valor) }}</dd></div>
        </dl>
      </section>

      <section class="tarjeta">
        <h2>Fábrica</h2>
        <p v-if="!diagnostico" class="nota">Leyendo el mando...</p>
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
  font-weight: 700;
}

.carril {
  height: 10px;
  overflow: hidden;
  border-radius: 5px;
  background: var(--ground);
}

.carril i {
  display: block;
  height: 100%;
  background: var(--accent);
  transform-origin: left;
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
</style>
