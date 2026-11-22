<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import type { AccionEspecial, TipoAccion } from '../bridge'
import {
  acciones,
  activarAccion,
  archivoElegido,
  borrarAccion,
  cargarAcciones,
  conectado,
  elegirPrograma,
  estadoApp,
  guardarAccion,
  mando,
} from '../estado/app'
import { nombreTecla } from '../estado/teclas'
import ControlNumero from './ControlNumero.vue'
import GrabadorMacro from './GrabadorMacro.vue'
import IconoMando from './IconoMando.vue'
import InterruptorFila from './InterruptorFila.vue'
import { t } from '../i18n'

const TIPOS: { id: TipoAccion; clave: string }[] = [
  { id: 'macro', clave: 'secciones.acciones.tipo.macro' },
  { id: 'tecla', clave: 'secciones.acciones.tipo.tecla' },
  { id: 'programa', clave: 'secciones.acciones.tipo.programa' },
  { id: 'perfil', clave: 'secciones.acciones.tipo.perfil' },
  { id: 'bateria', clave: 'secciones.acciones.tipo.bateria' },
  { id: 'desconectar', clave: 'secciones.acciones.tipo.desconectar' },
  { id: 'giroscopio', clave: 'secciones.acciones.tipo.giroscopio' },
]

const CONTROLES = [
  ['Cross', 'Cruz'], ['Circle', 'Círculo'], ['Square', 'Cuadrado'], ['Triangle', 'Triángulo'],
  ['L1', 'L1'], ['R1', 'R1'], ['L2', 'L2'], ['R2', 'R2'], ['L3', 'L3'], ['R3', 'R3'],
  ['Up', 'Arriba'], ['Down', 'Abajo'], ['Left', 'Izquierda'], ['Right', 'Derecha'],
  ['Options', 'Options'], ['Share', 'Crear'], ['PS', 'PS'], ['Mute', 'Mute'],
  ['Left Touch', 'Panel izquierda'], ['Right Touch', 'Panel derecha'],
  ['Upper Touch', 'Panel arriba'], ['Multitouch', 'Dos dedos'],
] as const

const NOMBRES = new Map<string, string>(CONTROLES)

interface Formulario {
  original: string | null
  nombre: string
  tipo: TipoAccion
  controles: string[]
  detalles: string
  extra: string
  retardo: number
  pasos: number[]
  repetir: boolean
}

const VACIO: Formulario = {
  original: null,
  nombre: '',
  tipo: 'macro',
  controles: [],
  detalles: '',
  extra: '',
  retardo: 3,
  pasos: [],
  repetir: false,
}

const form = ref<Formulario | null>(null)
const USAN_RETARDO: TipoAccion[] = ['programa', 'bateria', 'desconectar', 'giroscopio']

onMounted(cargarAcciones)
watch(() => [mando.value?.id, mando.value?.perfil], cargarAcciones)
watch(archivoElegido, (ruta) => {
  if (ruta && form.value) form.value.detalles = ruta
})

const tipoDe = (a: AccionEspecial) => t(TIPOS.find((x) => x.id === a.tipo)?.clave ?? 'secciones.acciones.tipo.otra')
const editable = (a: AccionEspecial) => a.tipo !== 'otra'

const valido = computed(() => {
  const f = form.value
  if (!f || !f.nombre.trim() || f.nombre.includes('/') || !f.controles.length) return false
  if (f.tipo === 'macro') return f.pasos.length > 0
  if (f.tipo === 'tecla' || f.tipo === 'programa' || f.tipo === 'perfil') return f.detalles !== ''
  return true
})

const teclaElegida = computed(() => {
  const codigo = Number(form.value?.detalles.split(' ')[0])
  return Number.isFinite(codigo) && codigo > 0 ? [codigo] : []
})

function nueva() {
  form.value = { ...VACIO, controles: [], pasos: [] }
}

function editar(a: AccionEspecial) {
  form.value = {
    original: a.nombre,
    nombre: a.nombre,
    tipo: a.tipo,
    controles: [...a.controles],
    detalles: a.detalles,
    extra: a.extra,
    retardo: a.retardo || 3,
    pasos: [...a.pasos],
    repetir: a.repetir,
  }
}

function alternarControl(id: string) {
  const f = form.value
  if (!f) return
  f.controles = f.controles.includes(id) ? f.controles.filter((c) => c !== id) : [...f.controles, id]
}

function guardar() {
  if (!form.value || !valido.value) return
  guardarAccion({ ...form.value, controles: [...form.value.controles], pasos: [...form.value.pasos] })
  form.value = null
}
</script>

<template>
  <main class="seccion" data-atras="[aria-current='page']">
    <h1>{{ t('secciones.acciones.titulo') }}</h1>
    <p class="nota">{{ t('secciones.acciones.nota') }}</p>

    <p v-if="!conectado" class="nota">{{ t('secciones.acciones.sinMando') }}</p>

    <template v-else-if="!form">
      <button class="principal" data-foco-inicial @click="nueva">{{ t('secciones.acciones.nueva') }}</button>
      <p v-if="!acciones.length" class="nota">{{ t('secciones.acciones.vacio') }}</p>
      <ul class="lista">
        <li v-for="a in acciones" :key="a.nombre">
          <InterruptorFila :etiqueta="a.nombre" :activo="a.activa" @cambiar="activarAccion(a.nombre, $event)" />
          <p class="con">
            {{ tipoDe(a) }}:
            <template v-for="(c, i) in a.controles" :key="c">
              <span v-if="i">+</span>
              <IconoMando :control="NOMBRES.get(c) ?? c" :tamano="22" />
            </template>
          </p>
          <div class="botones">
            <button v-if="editable(a)" class="secundario" @click="editar(a)">{{ t('secciones.acciones.editar') }}</button>
            <button class="secundario" @click="borrarAccion(a.nombre)">{{ t('secciones.acciones.borrar') }}</button>
          </div>
        </li>
      </ul>
    </template>

    <form v-else class="formulario" @submit.prevent="guardar">
      <label class="campo">
        <span>{{ t('secciones.acciones.nombre') }}</span>
        <input v-model="form.nombre" type="text" maxlength="60" data-foco-inicial />
      </label>

      <div class="campo">
        <span>{{ t('secciones.acciones.queHace') }}</span>
        <div class="chips">
          <button
            v-for="tp in TIPOS"
            :key="tp.id"
            type="button"
            :class="{ activa: form.tipo === tp.id }"
            :aria-pressed="form.tipo === tp.id"
            @click="form.tipo = tp.id; form.detalles = ''"
          >
            {{ t(tp.clave) }}
          </button>
        </div>
      </div>

      <div class="campo">
        <span>{{ t('secciones.acciones.seLanzaCon') }}</span>
        <div class="chips">
          <button
            v-for="[id, nombre] in CONTROLES"
            :key="id"
            type="button"
            :class="{ activa: form.controles.includes(id) }"
            :aria-pressed="form.controles.includes(id)"
            @click="alternarControl(id)"
          >
            <IconoMando :control="nombre" />
          </button>
        </div>
      </div>

      <template v-if="form.tipo === 'macro'">
        <GrabadorMacro :pasos="form.pasos" @cambiar="form.pasos = $event" />
        <InterruptorFila :etiqueta="t('secciones.acciones.repetir')" :activo="form.repetir" @cambiar="form.repetir = $event" />
      </template>

      <div v-else-if="form.tipo === 'tecla'" class="campo">
        <span>{{ t('secciones.acciones.tecla') }}{{ teclaElegida[0] ? `: ${nombreTecla(teclaElegida[0])}` : '' }}</span>
        <GrabadorMacro solo-una-tecla :pasos="[]" @cambiar="form.detalles = String($event[0])" />
      </div>

      <template v-else-if="form.tipo === 'programa'">
        <label class="campo">
          <span>{{ t('secciones.acciones.programa') }}</span>
          <input v-model="form.detalles" type="text" />
        </label>
        <button type="button" class="secundario" @click="elegirPrograma">{{ t('secciones.acciones.buscarPrograma') }}</button>
        <label class="campo">
          <span>{{ t('secciones.acciones.argumentos') }}</span>
          <input v-model="form.extra" type="text" />
        </label>
      </template>

      <div v-else-if="form.tipo === 'perfil'" class="campo">
        <span>{{ t('secciones.acciones.perfil') }}</span>
        <div class="chips">
          <button
            v-for="p in estadoApp.perfiles"
            :key="p"
            type="button"
            :class="{ activa: form.detalles === p }"
            :aria-pressed="form.detalles === p"
            @click="form.detalles = p"
          >
            {{ p }}
          </button>
        </div>
      </div>

      <div v-if="USAN_RETARDO.includes(form.tipo)" class="fila">
        <span>{{ form.tipo === 'programa' ? t('secciones.acciones.esperaAbrir') : t('secciones.acciones.mantener') }} ({{ t('secciones.acciones.segundosMin') }})</span>
        <ControlNumero :etiqueta="t('secciones.acciones.segundos')" :valor="form.retardo" :min="0" :max="60" :paso="0.5" @cambiar="form.retardo = $event" />
      </div>

      <div class="botones">
        <button type="submit" class="principal" :disabled="!valido">{{ t('secciones.acciones.guardar') }}</button>
        <button type="button" class="secundario" @click="form = null">{{ t('secciones.acciones.cancelar') }}</button>
      </div>
    </form>
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
  gap: 16px;
  overflow-y: auto;
}

h1 {
  font-size: 28px;
  font-weight: 800;
  line-height: 1.15;
}

.nota {
  color: var(--muted);
  font-size: 14px;
}

/* Las acciones llenan el ancho en columnas: cada fila sigue siendo corta y fácil de leer. */
.lista {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 16px 32px;
  margin: 0;
  padding: 0;
  list-style: none;
}

li {
  display: grid;
  gap: 4px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--linea);
}

li p {
  color: var(--muted);
  font-size: 14px;
}

.con {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.principal,
.secundario,
.chips button {
  min-height: 44px;
  padding: 0 14px;
  border-radius: var(--radio-control);
  font-weight: 600;
}

.principal {
  align-self: flex-start;
  background: var(--ink);
  color: var(--surface);
}

.secundario,
.chips button {
  background: var(--ground);
}

.secundario:hover,
.chips button:hover:not(.activa) {
  background: var(--linea);
}

button:disabled {
  opacity: 0.45;
}

.botones,
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.chips button {
  font-size: 14px;
}

.chips .activa {
  background: var(--ink);
  color: var(--surface);
}

.formulario {
  display: grid;
  gap: 18px;
  max-width: 640px;
}

.campo {
  display: grid;
  gap: 8px;
  font-weight: 600;
}

input {
  min-height: 44px;
  padding: 0 12px;
  border: 0;
  border-radius: var(--radio-control);
  background: var(--ground);
  color: inherit;
  font: inherit;
}

.fila {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  font-weight: 600;
}
</style>
