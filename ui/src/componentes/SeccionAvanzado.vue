<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import type { AjusteAvanzado, SeccionAvanzada } from '../bridge'
import { cambiarOpcion, cargarOpciones, conectado, grupoActivo, grupos, mando } from '../estado/app'
import IconoMando from './IconoMando.vue'
import InstrumentoGatillo from './InstrumentoGatillo.vue'
import InstrumentoStick from './InstrumentoStick.vue'
import ListaAjustes from './ListaAjustes.vue'

onMounted(cargarOpciones)
watch(() => [mando.value?.id, mando.value?.perfil], cargarOpciones)

/** Los grupos con dos controles iguales enseñan uno cada vez, junto a su instrumento. */
const LADOS: Record<string, { id: string; prefijo: string; titulo: string; icono: string }[]> = {
  sticks: [
    { id: 'izq', prefijo: 'LS', titulo: 'Stick izquierdo', icono: 'Stick izquierdo' },
    { id: 'der', prefijo: 'RS', titulo: 'Stick derecho', icono: 'Stick derecho' },
  ],
  gatillos: [
    { id: 'l2', prefijo: 'L2', titulo: 'L2', icono: 'L2' },
    { id: 'r2', prefijo: 'R2', titulo: 'R2', icono: 'R2' },
  ],
}

const elegido = ref<Record<string, number>>({ sticks: 0, gatillos: 0 })
const grupo = computed(() => grupos.value.find((g) => g.id === grupoActivo.value) ?? grupos.value[0])
const lados = computed(() => LADOS[grupo.value?.id ?? ''] ?? null)
const lado = computed(() => lados.value?.[elegido.value[grupo.value?.id ?? ''] ?? 0] ?? null)

function nombreCorto(nombre: string, titulo: string) {
  if (nombre === titulo) return 'Respuesta'
  const resto = nombre.slice(titulo.length + 2)
  return resto.charAt(0).toUpperCase() + resto.slice(1)
}

const secciones = computed<SeccionAvanzada[]>(() => {
  const todas = grupo.value?.secciones ?? []
  const actual = lado.value
  if (!actual) return todas
  return todas
    .filter((s) => s.nombre.startsWith(actual.titulo))
    .map((s) => ({ ...s, nombre: nombreCorto(s.nombre, actual.titulo) }))
})

function valorDe(id: string): AjusteAvanzado['valor'] | undefined {
  for (const s of grupo.value?.secciones ?? []) {
    const a = s.ajustes.find((x) => x.id === id)
    if (a) return a.valor
  }
  return undefined
}

const medidas = computed(() => {
  const p = lado.value?.prefijo ?? ''
  return {
    muerta: (valorDe(`${p}DeadZone`) as number) ?? 0,
    maxima: (valorDe(`${p}MaxZone`) as number) ?? 1,
    curva: (valorDe(`${p}OutputCurveIndex`) as number) ?? 0,
    personalizada: (valorDe(`${p}CustomCurve`) as string) ?? '',
  }
})

const posicion = computed(() => {
  const sticks = mando.value?.sticks
  return (lado.value?.id === 'der' ? sticks?.der : sticks?.izq) ?? { x: 0, y: 0 }
})
const recorrido = computed(() => (lado.value?.id === 'r2' ? mando.value?.gatillos.r2 : mando.value?.gatillos.l2) ?? 0)
</script>

<template>
  <main class="seccion" data-atras="[aria-current='page']">
    <header>
      <h1>Avanzado</h1>
      <p>Se guarda en el perfil {{ mando?.perfil ?? '' }}.</p>
    </header>

    <p v-if="!conectado" class="vacio">Conecta un mando para ver estos ajustes.</p>

    <template v-else>
      <div class="grupos" role="tablist" aria-label="Grupos de ajustes">
        <button
          v-for="(g, i) in grupos"
          :key="g.id"
          role="tab"
          :aria-selected="grupo?.id === g.id"
          :class="{ activa: grupo?.id === g.id }"
          :data-foco-inicial="i === 0 ? '' : undefined"
          @click="grupoActivo = g.id"
        >
          {{ g.nombre }}
        </button>
      </div>

      <div class="banco" role="tabpanel" data-atras="[role='tab'][aria-selected='true']">
        <aside v-if="lados && lado && grupo" class="mesa">
          <div class="lados" role="group" aria-label="Control que se ajusta">
            <button
              v-for="(l, i) in lados"
              :key="l.id"
              :class="{ activa: lado.id === l.id }"
              :aria-pressed="lado.id === l.id"
              @click="elegido = { ...elegido, [grupo.id]: i }"
            >
              <IconoMando :control="l.icono" />
              <span>{{ l.titulo }}</span>
            </button>
          </div>
          <InstrumentoStick
            v-if="grupo.id === 'sticks'"
            :posicion="posicion"
            :muerta="medidas.muerta"
            :maxima="medidas.maxima"
            :curva="medidas.curva"
            :personalizada="medidas.personalizada"
          />
          <InstrumentoGatillo
            v-else
            :recorrido="recorrido"
            :muerta="medidas.muerta"
            :maxima="medidas.maxima"
            :curva="medidas.curva"
            :personalizada="medidas.personalizada"
          />
        </aside>

        <div class="desliza">
          <ListaAjustes :secciones="secciones" :columnas="!lados" @cambiar="cambiarOpcion" />
        </div>
      </div>
    </template>
  </main>
</template>

<style scoped>
.seccion {
  flex: 999 1 520px;
  min-width: 0;
  padding: 20px 24px 0;
  background: var(--surface);
  border-radius: var(--radio-panel);
  display: flex;
  flex-direction: column;
  gap: 14px;
  overflow: hidden;
}

header {
  display: flex;
  align-items: baseline;
  gap: 12px;
}

h1 {
  font-size: 28px;
  font-weight: 800;
  line-height: 1.15;
}

header p,
.vacio {
  color: var(--muted);
  font-size: 14px;
}

.grupos {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.grupos button {
  min-height: 40px;
  padding: 0 14px;
  border-radius: var(--radio-control);
  font-size: 14px;
  font-weight: 600;
  color: var(--muted);
}

.grupos button:hover:not(.activa) {
  background: var(--ground);
}

.grupos .activa {
  background: var(--ink);
  color: var(--surface);
}

.banco {
  flex: 1 1 0;
  min-height: 0;
  display: flex;
  gap: 16px;
}

.mesa {
  flex: 0 0 300px;
  display: grid;
  align-content: start;
  gap: 12px;
  padding-bottom: 16px;
  overflow-y: auto;
}

.lados {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
}

.lados button {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: 48px;
  border-radius: var(--radio-control);
  background: var(--ground);
  font-size: 13px;
  font-weight: 600;
}

.lados .activa {
  background: var(--ink);
  color: var(--surface);
}

.desliza {
  flex: 1 1 0;
  min-width: 0;
  padding: 3px 3px 16px;
  overflow-y: auto;
}
</style>
