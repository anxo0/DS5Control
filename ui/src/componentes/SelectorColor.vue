<script setup lang="ts">
import { computed, ref, watch } from 'vue'

const props = defineProps<{ color: string }>()
const emit = defineEmits<{ cambiar: [hex: string]; guardar: [hex: string]; cancelar: [] }>()

type Rgb = [number, number, number]

const rgbDeHex = (hex: string): Rgb => {
  const valor = parseInt(hex.slice(1), 16)
  return [(valor >> 16) & 255, (valor >> 8) & 255, valor & 255]
}

const hexDeRgb = (rgb: Rgb) =>
  `#${rgb.map((c) => c.toString(16).padStart(2, '0')).join('').toUpperCase()}`

function hsvDeRgb([r, g, b]: Rgb): Rgb {
  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  const d = max - min
  let h = 0
  if (d > 0) {
    if (max === r) h = ((g - b) / d + 6) % 6
    else if (max === g) h = (b - r) / d + 2
    else h = (r - g) / d + 4
  }
  return [h * 60, max === 0 ? 0 : d / max, max / 255]
}

function rgbDeHsv(h: number, s: number, v: number): Rgb {
  const canal = (n: number) => {
    const k = (n + h / 60) % 6
    return Math.round(255 * (v - v * s * Math.max(0, Math.min(k, 4 - k, 1))))
  }
  return [canal(5), canal(3), canal(1)]
}

const inicial = hsvDeRgb(rgbDeHex(props.color))
const tono = ref(inicial[0])
const saturacion = ref(inicial[1])
const valor = ref(inicial[2])

const rgb = computed(() => rgbDeHsv(tono.value, saturacion.value, valor.value))
const hex = computed(() => hexDeRgb(rgb.value))
const textoHex = ref(hex.value)
const puroTono = computed(() => hexDeRgb(rgbDeHsv(tono.value, 1, 1)))

watch(hex, (nuevo) => {
  textoHex.value = nuevo
  emit('cambiar', nuevo)
})

function fijarRgb(nuevo: Rgb) {
  const [h, s, v] = hsvDeRgb(nuevo)
  // El gris y el negro no tienen tono: se conserva el que había.
  if (s > 0 && v > 0) tono.value = h
  if (v > 0) saturacion.value = s
  valor.value = v
}

function escribirHex(evento: Event) {
  const texto = (evento.target as HTMLInputElement).value.trim()
  textoHex.value = texto
  const limpio = texto.replace(/^#/, '')
  if (/^[0-9a-fA-F]{6}$/.test(limpio)) fijarRgb(rgbDeHex(`#${limpio}`))
}

function escribirCanal(indice: number, evento: Event) {
  const numero = Number((evento.target as HTMLInputElement).value)
  if (!Number.isFinite(numero)) return
  const nuevo: Rgb = [...rgb.value]
  nuevo[indice] = Math.max(0, Math.min(255, Math.round(numero)))
  fijarRgb(nuevo)
}

const plano = ref<HTMLElement>()

function moverEnPlano(evento: PointerEvent) {
  const caja = plano.value?.getBoundingClientRect()
  if (!caja) return
  saturacion.value = Math.max(0, Math.min(1, (evento.clientX - caja.left) / caja.width))
  valor.value = 1 - Math.max(0, Math.min(1, (evento.clientY - caja.top) / caja.height))
}

function pulsarPlano(evento: PointerEvent) {
  plano.value?.setPointerCapture(evento.pointerId)
  moverEnPlano(evento)
}

const CANALES = ['R', 'G', 'B']
</script>

<template>
  <section class="selector-color" aria-label="Color personalizado">
    <div
      ref="plano"
      class="plano"
      :style="{ backgroundColor: puroTono }"
      aria-hidden="true"
      @pointerdown="pulsarPlano"
      @pointermove="(e) => e.buttons && moverEnPlano(e)"
    >
      <span
        class="tirador"
        :style="{
          left: `${saturacion * 100}%`,
          top: `${(1 - valor) * 100}%`,
          background: hex,
        }"
      />
    </div>

    <label class="tono">
      <span>Tono</span>
      <input v-model.number="tono" type="range" min="0" max="359" step="3" />
    </label>

    <div class="campos">
      <label class="campo hex">
        <span>Hex</span>
        <input
          :value="textoHex"
          maxlength="7"
          spellcheck="false"
          autocomplete="off"
          @input="escribirHex"
        />
      </label>
      <label v-for="(canal, i) in CANALES" :key="canal" class="campo">
        <span>{{ canal }}</span>
        <input
          type="number"
          min="0"
          max="255"
          :value="rgb[i]"
          @input="escribirCanal(i, $event)"
        />
      </label>
    </div>

    <div class="acciones">
      <button class="cancelar" @click="emit('cancelar')">Cancelar</button>
      <button class="guardar" @click="emit('guardar', hex)">Guardar color</button>
    </div>
  </section>
</template>

<style scoped>
.selector-color {
  display: grid;
  gap: 12px;
  padding: 12px;
  border-radius: var(--radio-control);
  background: var(--ground);
}

.plano {
  position: relative;
  height: 130px;
  border-radius: 8px;
  background-image:
    linear-gradient(to top, #000000, transparent),
    linear-gradient(to right, #ffffff, transparent);
  cursor: crosshair;
  touch-action: none;
}

.tirador {
  position: absolute;
  width: 16px;
  height: 16px;
  margin: -8px 0 0 -8px;
  border: 2px solid #ffffff;
  border-radius: 50%;
  box-shadow: 0 0 0 1px #000000;
  pointer-events: none;
}

.tono {
  display: grid;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
}

.tono input {
  width: 100%;
  height: 14px;
  margin: 0;
  border-radius: 7px;
  appearance: none;
  background: linear-gradient(
    to right,
    #ff0000,
    #ffff00,
    #00ff00,
    #00ffff,
    #0000ff,
    #ff00ff,
    #ff0000
  );
  cursor: ew-resize;
}

.tono input::-webkit-slider-thumb {
  appearance: none;
  width: 18px;
  height: 18px;
  border: 2px solid #ffffff;
  border-radius: 50%;
  box-shadow: 0 0 0 1px #000000;
  background: transparent;
}

.campos {
  display: grid;
  grid-template-columns: 2fr repeat(3, minmax(0, 1fr));
  gap: 6px;
}

.campo {
  display: grid;
  gap: 4px;
  font-size: 12px;
  color: var(--muted);
}

.campo input {
  width: 100%;
  min-width: 0;
  height: 36px;
  box-sizing: border-box;
  padding: 0 6px;
  border: 1px solid var(--linea);
  border-radius: 6px;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
  font-size: 13px;
  font-variant-numeric: tabular-nums;
  appearance: textfield;
}

.campo input::-webkit-inner-spin-button {
  appearance: none;
}

.acciones {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.acciones button {
  min-height: 44px;
  border-radius: 8px;
  font-weight: 600;
}

.cancelar {
  background: var(--linea);
}

.guardar {
  background: var(--ink);
  color: var(--surface);
}
</style>
