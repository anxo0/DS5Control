<script setup lang="ts">
import { computed, ref } from 'vue'
import { asignarBoton, asignarMacro, asignarTecla, conectado, mando } from '../estado/app'
import { nombreTecla } from '../estado/teclas'
import { iconoDe } from '../mandos/iconos'
import DibujoMando from './DibujoMando.vue'
import GrabadorMacro from './GrabadorMacro.vue'
import IconoMando from './IconoMando.vue'
import { t } from '../i18n'

/** `dibujo` es el identificador del botón en el SVG del mando; `nombre`, la clave de su texto. */
const GRUPOS = [
  {
    nombre: 'Botones',
    controles: [
      { id: 'Cross', nombre: 'mando.control.cruz', base: 'A', dibujo: 'cross' },
      { id: 'Circle', nombre: 'mando.control.circulo', base: 'B', dibujo: 'circle' },
      { id: 'Square', nombre: 'mando.control.cuadrado', base: 'X', dibujo: 'rect' },
      { id: 'Triangle', nombre: 'mando.control.triangulo', base: 'Y', dibujo: 'triangle' },
    ],
  },
  {
    nombre: 'Gatillos',
    controles: [
      { id: 'L1', nombre: 'mando.control.l1', base: 'LB', dibujo: 'l1' },
      { id: 'R1', nombre: 'mando.control.r1', base: 'RB', dibujo: 'r1' },
      { id: 'L2', nombre: 'mando.control.l2', base: 'LT', dibujo: 'l2' },
      { id: 'R2', nombre: 'mando.control.r2', base: 'RT', dibujo: 'r2' },
    ],
  },
  {
    nombre: 'Cruceta',
    controles: [
      { id: 'DpadUp', nombre: 'mando.control.crucetaArriba', base: 'DpadUp', dibujo: 'dpad-up' },
      { id: 'DpadDown', nombre: 'mando.control.crucetaAbajo', base: 'DpadDown', dibujo: 'dpad-down' },
      { id: 'DpadLeft', nombre: 'mando.control.crucetaIzquierda', base: 'DpadLeft', dibujo: 'dpad-left' },
      { id: 'DpadRight', nombre: 'mando.control.crucetaDerecha', base: 'DpadRight', dibujo: 'dpad-right' },
    ],
  },
  {
    nombre: 'Centro',
    controles: [
      { id: 'L3', nombre: 'mando.control.l3', base: 'LS', dibujo: 'l3' },
      { id: 'R3', nombre: 'mando.control.r3', base: 'RS', dibujo: 'r3' },
      { id: 'Share', nombre: 'mando.control.crear', base: 'Back', dibujo: 'create' },
      { id: 'Options', nombre: 'mando.control.options', base: 'Start', dibujo: 'options' },
      { id: 'PS', nombre: 'mando.control.ps', base: 'Guide', dibujo: 'ps' },
    ],
  },
]

type Control = (typeof GRUPOS)[number]['controles'][number]
const CONTROLES = GRUPOS.flatMap((g) => g.controles)

/** Cada salida con la clave de su texto. */
const SALIDAS = [
  ['A', 'mando.control.cruz'],
  ['B', 'mando.control.circulo'],
  ['X', 'mando.control.cuadrado'],
  ['Y', 'mando.control.triangulo'],
  ['LB', 'mando.control.l1'],
  ['RB', 'mando.control.r1'],
  ['LT', 'mando.control.l2'],
  ['RT', 'mando.control.r2'],
  ['LS', 'mando.control.l3'],
  ['RS', 'mando.control.r3'],
  ['DpadUp', 'mando.control.arriba'],
  ['DpadDown', 'mando.control.abajo'],
  ['DpadLeft', 'mando.control.izquierda'],
  ['DpadRight', 'mando.control.derecha'],
  ['Back', 'mando.control.crear'],
  ['Start', 'mando.control.options'],
  ['Guide', 'mando.control.ps'],
  ['Unbound', 'mando.sinFuncion'],
] as const

const NOMBRES = new Map<string, string>(SALIDAS)
const elegido = ref<Control>(CONTROLES[0]!)

const asignadas = computed(
  () => new Map((mando.value?.mapeo ?? []).map((m) => [m.control, m.salida])),
)

const salidaDe = (id: string) => asignadas.value.get(id) ?? null
const cambiado = (c: Control) => {
  const s = salidaDe(c.id)
  return s !== null && s !== c.base
}
const etiqueta = (c: Control) => {
  const s = salidaDe(c.id) ?? c.base
  if (s === 'Macro') return t('mando.botones.macro')
  if (s.startsWith('Key:')) return nombreTecla(Number(s.slice(4)))
  const clave = NOMBRES.get(s)
  return clave ? t(clave) : t('mando.botones.personalizado')
}
const salidaVisible = (c: Control) => {
  const s = salidaDe(c.id) ?? c.base
  return iconoDe(s) ? s : etiqueta(c).slice(0, 3)
}
const actual = computed(() => salidaDe(elegido.value.id) ?? elegido.value.base)

function elegirEnDibujo(id: string) {
  const control = CONTROLES.find((c) => c.dibujo === id)
  if (control) elegido.value = control
}
</script>

<template>
  <main class="seccion" data-atras="[aria-current='page']">
    <h1>{{ t('mando.botones.titulo') }}</h1>
    <div class="cuerpo">
      <section class="mando" :aria-label="t('mando.botones.delMando')">
        <DibujoMando
          class="dibujo"
          :mando="mando"
          :conectado="conectado"
          limpio
          :seleccion="elegido.dibujo"
          @elegir="elegirEnDibujo"
        />
        <div class="grupos">
          <div v-for="(g, n) in GRUPOS" :key="g.nombre" class="grupo">
            <button
              v-for="(c, i) in g.controles"
              :key="c.id"
              class="control"
              :class="{ activa: elegido.id === c.id }"
              :aria-label="`${t(c.nombre)}: ${etiqueta(c)}`"
              :aria-pressed="elegido.id === c.id"
              :disabled="!conectado"
              :data-foco-inicial="n === 0 && i === 0 ? '' : undefined"
              @click="elegido = c"
            >
              <IconoMando :control="c.id" :tamano="28" />
              <span v-if="cambiado(c)" class="sale"><IconoMando :control="salidaVisible(c)" :tamano="16" /></span>
            </button>
          </div>
        </div>
      </section>

      <section class="panel" :aria-label="t('mando.botones.funcionDe', { boton: t(elegido.nombre) })">
        <h2 class="titulo"><IconoMando :control="elegido.id" /> {{ t('mando.botones.funcionaComo') }}</h2>
        <div class="salidas">
          <button
            v-for="[id, nombre] in SALIDAS"
            :key="id"
            class="salida"
            :class="{ activa: actual === id }"
            :aria-pressed="actual === id"
            :aria-label="t(nombre)"
            :disabled="!conectado"
            @click="asignarBoton(elegido.id, id)"
          >
            <IconoMando :control="id === 'Unbound' ? t(nombre) : id" />
          </button>
        </div>
        <h2>{{ t('mando.botones.comoTeclado') }}</h2>
        <GrabadorMacro solo-una-tecla :pasos="[]" @cambiar="asignarTecla(elegido.id, $event[0] ?? 0)" />
        <GrabadorMacro :key="elegido.id" :pasos="[]" @cambiar="asignarMacro(elegido.id, $event)" />
        <button
          class="restablecer"
          :disabled="!conectado || salidaDe(elegido.id) === null"
          @click="asignarBoton(elegido.id, null)"
        >
          {{ t('mando.botones.restablecer') }}
        </button>
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
  gap: 12px;
  overflow-y: auto;
}

h1 {
  font-size: 28px;
  font-weight: 800;
  line-height: 1.15;
}

.cuerpo {
  flex: 1 1 auto;
  display: flex;
  align-items: flex-start;
  gap: 24px;
}

.mando {
  flex: 1 1 0;
  min-width: 0;
  display: grid;
  gap: 16px;
}

.dibujo {
  width: 100%;
  max-width: 560px;
  justify-self: center;
}

.grupos {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 10px 20px;
}

.grupo {
  display: flex;
  gap: 4px;
}

.control {
  position: relative;
  display: grid;
  place-items: center;
  width: 48px;
  height: 48px;
  border-radius: var(--radio-control);
  background: var(--ground);
}

.sale {
  position: absolute;
  right: -3px;
  bottom: -3px;
  display: grid;
  place-items: center;
  min-width: 20px;
  height: 20px;
  padding: 0 2px;
  border-radius: 10px;
  background: var(--accent);
  color: var(--surface);
  font-size: 10px;
  font-weight: 700;
}

.control:hover:not(:disabled):not(.activa),
.salida:hover:not(:disabled):not(.activa),
.restablecer:hover:not(:disabled) {
  background: var(--linea);
}

.control.activa,
.salida.activa {
  background: var(--ink);
  color: var(--surface);
}

button:disabled {
  opacity: 0.45;
}

.panel {
  flex: 0 1 420px;
  min-width: 0;
  display: grid;
  gap: 10px;
}

h2 {
  font-size: 15px;
  font-weight: 700;
}

.titulo {
  display: flex;
  align-items: center;
  gap: 8px;
}

.salidas {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 4px;
}

.salida {
  display: grid;
  place-items: center;
  min-height: 48px;
  padding: 0 4px;
  border-radius: var(--radio-control);
  background: var(--ground);
  font-size: 11px;
  font-weight: 600;
}

.restablecer {
  min-height: 44px;
  border-radius: var(--radio-control);
  color: var(--muted);
  font-weight: 600;
}
</style>
