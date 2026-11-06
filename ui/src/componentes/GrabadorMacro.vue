<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import { nombreTecla } from '../estado/teclas'

const props = defineProps<{ pasos: number[]; soloUnaTecla?: boolean }>()
const emit = defineEmits<{ cambiar: [pasos: number[]] }>()

/** En una macro, las esperas se guardan sumando 300 a los milisegundos. */
const BASE_ESPERA = 300
const MAX_ESPERA_MS = 10000
const BOTONES_RATON = [256, 258, 257, 259, 260]

const grabando = ref(false)
let ultimo = 0
let pulsadas = new Set<number>()
let grabados: number[] = []
const boton = ref<HTMLButtonElement>()

const resumen = computed(() =>
  props.pasos.map((p) => (p >= BASE_ESPERA ? `${p - BASE_ESPERA} ms` : nombreTecla(p))).join(', '),
)

function anotar(codigo: number) {
  const ahora = performance.now()
  if (grabados.length) grabados.push(BASE_ESPERA + Math.min(Math.round(ahora - ultimo), MAX_ESPERA_MS))
  grabados.push(codigo)
  ultimo = ahora
}

function alBajar(codigo: number) {
  if (pulsadas.has(codigo)) return
  pulsadas.add(codigo)
  if (props.soloUnaTecla) {
    emit('cambiar', [codigo])
    parar()
    return
  }
  anotar(codigo)
}

function alSubir(codigo: number) {
  if (!pulsadas.delete(codigo)) return
  anotar(codigo)
}

function teclaAbajo(e: KeyboardEvent) {
  e.preventDefault()
  e.stopImmediatePropagation()
  if (!e.repeat) alBajar(e.keyCode)
}

function teclaArriba(e: KeyboardEvent) {
  e.preventDefault()
  e.stopImmediatePropagation()
  alSubir(e.keyCode)
}

function ratonAbajo(e: MouseEvent) {
  if (boton.value?.contains(e.target as Node)) return
  e.preventDefault()
  alBajar(BOTONES_RATON[e.button] ?? 256)
}

function ratonArriba(e: MouseEvent) {
  if (boton.value?.contains(e.target as Node)) return
  alSubir(BOTONES_RATON[e.button] ?? 256)
}

function escuchar(activo: boolean) {
  const accion = activo ? window.addEventListener : window.removeEventListener
  accion('keydown', teclaAbajo as EventListener, true)
  accion('keyup', teclaArriba as EventListener, true)
  accion('mousedown', ratonAbajo as EventListener, true)
  accion('mouseup', ratonArriba as EventListener, true)
}

function empezar() {
  grabados = []
  pulsadas = new Set()
  grabando.value = true
  escuchar(true)
}

function parar() {
  if (!grabando.value) return
  grabando.value = false
  escuchar(false)
  if (!props.soloUnaTecla) {
    // Lo que siga pulsado se suelta: una tecla enganchada bloquearía el teclado.
    pulsadas.forEach(anotar)
    emit('cambiar', grabados)
  }
}

onBeforeUnmount(parar)
</script>

<template>
  <div class="grabador">
    <button ref="boton" class="grabar" :class="{ activa: grabando }" @click="grabando ? parar() : empezar()">
      <template v-if="soloUnaTecla">{{ grabando ? 'Pulsa una tecla...' : 'Elegir tecla' }}</template>
      <template v-else>{{ grabando ? 'Parar grabación' : pasos.length ? 'Volver a grabar' : 'Grabar con teclado y ratón' }}</template>
    </button>
    <p v-if="resumen">{{ resumen }}</p>
  </div>
</template>

<style scoped>
.grabador {
  display: grid;
  gap: 8px;
}

.grabar {
  min-height: 44px;
  padding: 0 14px;
  border-radius: var(--radio-control);
  background: var(--ground);
  font-weight: 600;
}

.grabar:hover:not(.activa) {
  background: var(--linea);
}

.grabar.activa {
  background: var(--accent);
  color: var(--surface);
}

p {
  max-height: 96px;
  overflow-y: auto;
  color: var(--muted);
  font-size: 13px;
}
</style>
