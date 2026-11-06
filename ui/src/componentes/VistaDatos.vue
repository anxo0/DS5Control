<script setup lang="ts">
import { computed } from 'vue'
import { mando } from '../estado/app'

/** Tope de las barras: los giros rápidos pasan de aquí y las barras simplemente se llenan. */
const MAX_GIRO = 20000
const MAX_ACEL = 2
const ANCHO_PANEL = 1920
const ALTO_PANEL = 1080

const datos = computed(() => mando.value?.datos ?? null)

const ESTADOS: Record<number, string> = {
  0: 'Descargando',
  1: 'Cargando',
  2: 'Carga completa',
  10: 'No carga: voltaje anómalo',
  11: 'No carga: temperatura anómala',
  15: 'Error de carga',
}

const ejes = (v: { x: number; y: number; z: number }, max: number) =>
  (['x', 'y', 'z'] as const).map((e) => ({
    eje: e.toUpperCase(),
    valor: v[e],
    relleno: Math.max(-1, Math.min(1, v[e] / max)),
  }))

const fmt = (v: number) => (Math.abs(v) >= 100 ? Math.round(v) : v.toFixed(2))
</script>

<template>
  <div v-if="datos" class="vista">
    <section>
      <h3>Batería</h3>
      <div class="bateria" role="img" :aria-label="`Nivel ${datos.bateria.nivel} de 10`">
        <span v-for="i in 10" :key="i" :class="{ lleno: i <= datos.bateria.nivel }" />
      </div>
      <p :class="{ alerta: datos.bateria.estado >= 10 }">
        {{ ESTADOS[datos.bateria.estado] ?? 'Estado desconocido' }}, nivel {{ datos.bateria.nivel }} de 10
      </p>
    </section>

    <section>
      <h3>Panel táctil</h3>
      <svg :viewBox="`0 0 ${ANCHO_PANEL} ${ALTO_PANEL}`" role="img" aria-label="Dedos sobre el panel">
        <circle v-for="(t, i) in datos.toques" :key="i" :cx="t.x" :cy="t.y" r="70" />
      </svg>
    </section>

    <section>
      <h3>Giroscopio</h3>
      <div v-for="e in ejes(datos.giro, MAX_GIRO)" :key="e.eje" class="eje">
        <span>{{ e.eje }}</span>
        <div class="barra"><i :style="{ transform: `scaleX(${e.relleno})` }" /></div>
        <output>{{ fmt(e.valor) }}</output>
      </div>
    </section>

    <section>
      <h3>Acelerómetro</h3>
      <div v-for="e in ejes(datos.acel, MAX_ACEL)" :key="e.eje" class="eje">
        <span>{{ e.eje }}</span>
        <div class="barra"><i :style="{ transform: `scaleX(${e.relleno})` }" /></div>
        <output>{{ fmt(e.valor) }}</output>
      </div>
    </section>

    <section>
      <h3>Audio</h3>
      <dl>
        <div><dt>Cascos</dt><dd>{{ datos.audio.cascos ? 'Conectados' : 'No' }}</dd></div>
        <div><dt>Micrófono</dt><dd>{{ datos.audio.micro ? 'Conectado' : 'No' }}</dd></div>
      </dl>
    </section>

  </div>
  <p v-else class="nada">Estos datos solo están disponibles con un DualSense.</p>
</template>

<style scoped>
.vista {
  display: grid;
  gap: 16px;
}

section {
  display: grid;
  gap: 6px;
}

h3 {
  font-size: 14px;
  font-weight: 700;
}

p {
  color: var(--muted);
  font-size: 13px;
}

.alerta {
  color: var(--aviso);
  font-weight: 600;
}

.bateria {
  display: grid;
  grid-template-columns: repeat(10, 1fr);
  gap: 3px;
}

.bateria span {
  height: 16px;
  border-radius: 3px;
  background: var(--linea);
}

.bateria .lleno {
  background: var(--accent);
}

svg {
  display: block;
  width: 100%;
  aspect-ratio: 16 / 9;
  border-radius: var(--radio-control);
  background: var(--ground);
}

circle {
  fill: var(--accent);
}

.eje {
  display: grid;
  grid-template-columns: 14px 1fr 56px;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.eje output {
  color: var(--muted);
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.barra {
  position: relative;
  height: 8px;
  border-radius: 4px;
  background: var(--ground);
}

.barra i {
  position: absolute;
  left: 50%;
  width: 50%;
  height: 100%;
  background: var(--accent);
  transform-origin: left;
}

dl {
  display: grid;
  gap: 2px;
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

.nada {
  font-size: 14px;
}
</style>
