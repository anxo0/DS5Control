<script setup lang="ts">
import { ref } from 'vue'
import type { AjusteAvanzado, SeccionAvanzada } from '../bridge'
import { trazoCurva } from '../mandos/curvas'
import { iconoDe } from '../mandos/iconos'
import ControlNumero from './ControlNumero.vue'
import EditorCurva from './EditorCurva.vue'
import IconoMando from './IconoMando.vue'
import InterruptorFila from './InterruptorFila.vue'

defineProps<{ secciones: SeccionAvanzada[]; columnas?: boolean }>()
const emit = defineEmits<{ cambiar: [id: string, valor: AjusteAvanzado['valor']] }>()

/** Lo que casi nadie toca empieza plegado para no competir con lo importante. */
const PLEGADAS = /precisión|deriva|sensores|curva personalizada$/i
const abiertas = ref(new Set<string>())

const plegada = (s: SeccionAvanzada) => PLEGADAS.test(s.nombre) && !s.nombre.endsWith('curva personalizada')
const cerrada = (s: SeccionAvanzada) => plegada(s) && !abiertas.value.has(s.nombre)
const esCurva = (a: AjusteAvanzado) => a.id.endsWith('OutputCurveIndex') || a.id.endsWith('CurveIndex')

function alternar(nombre: string) {
  const siguiente = new Set(abiertas.value)
  if (!siguiente.delete(nombre)) siguiente.add(nombre)
  abiertas.value = siguiente
}

function alternarBoton(ajuste: AjusteAvanzado, indice: number) {
  const lista = ajuste.valor as number[]
  emit('cambiar', ajuste.id, lista.includes(indice) ? lista.filter((v) => v !== indice) : [...lista, indice])
}
</script>

<template>
  <div class="tarjetas" :class="{ columnas }">
    <section v-for="s in secciones" :key="s.nombre" class="tarjeta">
      <button v-if="plegada(s)" class="cabecera plegable" :aria-expanded="!cerrada(s)" @click="alternar(s.nombre)">
        <h2>{{ s.nombre }}</h2>
        <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true" :class="{ abierta: !cerrada(s) }"><path d="M3 5l4 4 4-4" /></svg>
      </button>
      <h2 v-else class="cabecera">
        <IconoMando v-if="iconoDe(s.nombre.split(':')[0]!)" :control="s.nombre.split(':')[0]!" :tamano="24" />
        {{ s.nombre }}
      </h2>

      <div v-if="!cerrada(s)" class="ajustes">
        <template v-for="a in s.ajustes" :key="a.id">
          <InterruptorFila
            v-if="a.tipo === 'interruptor'"
            :etiqueta="a.nombre"
            :activo="a.valor as boolean"
            @cambiar="emit('cambiar', a.id, $event)"
          />
          <div v-else-if="a.tipo === 'numero'" class="fila">
            <span>{{ a.nombre }}</span>
            <ControlNumero
              :etiqueta="a.nombre"
              :valor="a.valor as number"
              :min="a.min"
              :max="a.max"
              :paso="a.paso"
              :porcentaje="a.porcentaje"
              @cambiar="emit('cambiar', a.id, $event)"
            />
          </div>
          <div v-else-if="a.tipo === 'opciones' && esCurva(a)" class="bloque">
            <span>{{ a.nombre }}</span>
            <div class="curvas">
              <button
                v-for="(o, i) in a.opciones"
                :key="o"
                :class="{ activa: a.valor === i }"
                :aria-pressed="a.valor === i"
                @click="emit('cambiar', a.id, i)"
              >
                <svg viewBox="-3 -3 46 46" aria-hidden="true">
                  <path :d="trazoCurva(i, 40, '0.25, 0.10, 0.75, 0.90', 16)" />
                </svg>
                {{ o }}
              </button>
            </div>
          </div>
          <div v-else-if="a.tipo === 'opciones'" class="bloque">
            <span>{{ a.nombre }}</span>
            <div class="chips">
              <button
                v-for="(o, i) in a.opciones"
                :key="o"
                :class="{ activa: a.valor === i }"
                :aria-pressed="a.valor === i"
                @click="emit('cambiar', a.id, i)"
              >
                {{ o }}
              </button>
            </div>
          </div>
          <div v-else-if="a.tipo === 'botones'" class="bloque">
            <span>{{ a.nombre }}{{ (a.valor as number[]).length ? '' : ': siempre activo' }}</span>
            <div class="chips">
              <button
                v-for="(o, i) in a.opciones"
                :key="o"
                :class="{ activa: (a.valor as number[]).includes(i) }"
                :aria-pressed="(a.valor as number[]).includes(i)"
                @click="alternarBoton(a, i)"
              >
                <IconoMando :control="o" />
              </button>
            </div>
          </div>
          <button v-else-if="a.tipo === 'accion'" class="accion" @click="emit('cambiar', a.id, true)">
            {{ a.nombre }}
          </button>
          <EditorCurva v-else :valor="a.valor as string" @cambiar="emit('cambiar', a.id, $event)" />
        </template>
      </div>
    </section>
  </div>
</template>

<style scoped>
.tarjetas {
  display: grid;
  gap: 12px;
  align-content: start;
}

.tarjetas.columnas {
  display: block;
  columns: 2 340px;
  column-gap: 12px;
}

.columnas .tarjeta {
  break-inside: avoid;
  margin-bottom: 12px;
}

.tarjeta {
  padding: 14px 16px;
  border: 1px solid var(--linea);
  border-radius: var(--radio-control);
}

.cabecera {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.plegable {
  justify-content: space-between;
  min-height: 28px;
  text-align: left;
}

.plegable path {
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.plegable svg {
  transition: transform 160ms var(--salida-rapida);
}

.plegable svg.abierta {
  transform: rotate(180deg);
}

h2 {
  font-size: 15px;
  font-weight: 700;
}

.ajustes {
  display: grid;
  gap: 2px;
  margin-top: 8px;
}

.fila {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  min-height: 44px;
  font-size: 14px;
  font-weight: 600;
}

.fila > span {
  flex: 1 1 140px;
  min-width: 0;
}

.fila :deep(.numero) {
  flex: 0 1 210px;
  grid-template-columns: 40px minmax(70px, 1fr) 40px;
}

.fila :deep(.numero button) {
  height: 40px;
}

.ajustes :deep(.fila[role='switch']) {
  min-height: 44px;
  font-size: 14px;
}

.bloque {
  display: grid;
  gap: 6px;
  padding: 6px 0;
  font-size: 14px;
  font-weight: 600;
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.chips button,
.accion {
  min-height: 40px;
  padding: 0 12px;
  border-radius: var(--radio-control);
  background: var(--ground);
  font-size: 13px;
  font-weight: 600;
}

.accion {
  justify-self: start;
  margin: 2px 0;
}

.curvas {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(84px, 1fr));
  gap: 4px;
}

.curvas button {
  display: grid;
  justify-items: center;
  gap: 4px;
  padding: 8px 4px;
  border-radius: var(--radio-control);
  background: var(--ground);
  font-size: 12px;
  font-weight: 600;
}

.curvas svg {
  width: 40px;
  height: 40px;
}

.curvas path {
  fill: none;
  stroke: currentColor;
  stroke-width: 3;
  stroke-linecap: round;
}

.chips button:hover:not(.activa),
.curvas button:hover:not(.activa),
.accion:hover {
  background: var(--linea);
}

.chips .activa,
.curvas .activa {
  background: var(--ink);
  color: var(--surface);
}
</style>
