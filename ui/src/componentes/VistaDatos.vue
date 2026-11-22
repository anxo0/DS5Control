<script setup lang="ts">
import { computed } from 'vue'
import { diagnostico, mando } from '../estado/app'
import { t } from '../i18n'

/** Tope de las barras: los giros rápidos pasan de aquí y las barras simplemente se llenan. */
const MAX_GIRO = 20000
const MAX_ACEL = 2
const ANCHO_PANEL = 1920
const ALTO_PANEL = 1080

const datos = computed(() => mando.value?.datos ?? null)

/**
 * El mando no mide su capacidad real, así que esto es una orientación. Las horas por carga y
 * los límites de ciclos son supuestos habituales en baterías de litio, no datos de Sony.
 */
const HORAS_POR_CARGA = 3
const CICLOS_A_VIGILAR = 300
const CICLOS_DESGASTE = 500
const ERRORES_DE_CARGA = ['Voltaje', 'Temperatura', 'Circuito de carga (PMIC)']

const salud = computed(() => {
  const uso = diagnostico.value?.uso
  if (!uso) return null
  const ciclos = Math.round(uso.carga / 3600 / HORAS_POR_CARGA)
  const errores = uso.errores
    .filter((e) => ERRORES_DE_CARGA.includes(e.nombre))
    .reduce((s, e) => s + e.valor, 0)
  if (errores > 0) return { texto: t('secciones.vista.salud.revisar'), motivo: t('secciones.vista.salud.errores', { n: errores }) }
  const motivo = t('secciones.vista.salud.ciclos', { n: ciclos })
  if (ciclos >= CICLOS_DESGASTE) return { texto: t('secciones.vista.salud.desgastada'), motivo }
  if (ciclos >= CICLOS_A_VIGILAR) return { texto: t('secciones.vista.salud.vigilar'), motivo }
  return { texto: t('secciones.vista.salud.buena'), motivo: t('secciones.vista.salud.sinErrores', { motivo }) }
})

const ESTADOS: Record<number, string> = {
  0: 'secciones.vista.estado.descargando',
  1: 'secciones.vista.estado.cargando',
  2: 'secciones.vista.estado.completa',
  10: 'secciones.vista.estado.voltaje',
  11: 'secciones.vista.estado.temperatura',
  15: 'secciones.vista.estado.error',
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
      <h3>{{ t('secciones.vista.salud', { estado: salud?.texto ?? '...' }) }}</h3>
      <p v-if="salud">{{ t('secciones.vista.estimacion', { motivo: salud.motivo }) }}</p>
      <p :class="{ alerta: datos.bateria.estado >= 10 }">
        {{ t(ESTADOS[datos.bateria.estado] ?? 'secciones.vista.estado.desconocido') }}
      </p>
    </section>

    <section>
      <h3>{{ t('secciones.vista.panel') }}</h3>
      <svg :viewBox="`0 0 ${ANCHO_PANEL} ${ALTO_PANEL}`" role="img" :aria-label="t('secciones.vista.panel.dedos')">
        <circle v-for="(toque, i) in datos.toques" :key="i" :cx="toque.x" :cy="toque.y" r="70" />
      </svg>
    </section>

    <section>
      <h3>{{ t('secciones.vista.giroscopio') }}</h3>
      <div v-for="e in ejes(datos.giro, MAX_GIRO)" :key="e.eje" class="eje">
        <span>{{ e.eje }}</span>
        <div class="barra"><i :style="{ transform: `scaleX(${e.relleno})` }" /></div>
        <output>{{ fmt(e.valor) }}</output>
      </div>
    </section>

    <section>
      <h3>{{ t('secciones.vista.acelerometro') }}</h3>
      <div v-for="e in ejes(datos.acel, MAX_ACEL)" :key="e.eje" class="eje">
        <span>{{ e.eje }}</span>
        <div class="barra"><i :style="{ transform: `scaleX(${e.relleno})` }" /></div>
        <output>{{ fmt(e.valor) }}</output>
      </div>
    </section>

    <section>
      <h3>{{ t('secciones.vista.audio') }}</h3>
      <dl>
        <div><dt>{{ t('secciones.vista.cascos') }}</dt><dd>{{ datos.audio.cascos ? t('secciones.vista.cascos.si') : t('secciones.vista.no') }}</dd></div>
        <div><dt>{{ t('secciones.vista.micro') }}</dt><dd>{{ datos.audio.micro ? t('secciones.vista.micro.si') : t('secciones.vista.no') }}</dd></div>
      </dl>
    </section>

  </div>
  <p v-else class="nada">{{ t('secciones.vista.soloDualSense') }}</p>
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
