<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { EfectoGatillo, Gatillo } from '../bridge'
import {
  ajustesLocales as ajustes,
  cambiarArcoiris,
  cambiarBrillo,
  cambiarLuzSegunBateria,
  cambiarVibracion,
  conectado,
  elegirEfecto,
  elegirLuz,
  mando,
  pestana,
  probar,
  prueba,
} from '../estado/app'
import type { Pestana } from '../estado/app'
import { modeloDe } from '../mandos/modelos'
import ControlNivel from './ControlNivel.vue'
import { coloresGuardados, guardarColor } from '../estado/colores'
import InterruptorFila from './InterruptorFila.vue'
import SelectorColor from './SelectorColor.vue'

/** `curva` dibuja la resistencia del gatillo a lo largo de su recorrido. */
const EFECTOS: { id: EfectoGatillo; nombre: string; detalle: string; curva: string }[] = [
  { id: 'ninguno', nombre: 'Ninguno', detalle: 'Recorrido libre', curva: 'M2 20H42' },
  {
    id: 'clicCompleto',
    nombre: 'Clic completo',
    detalle: 'Resistencia con un clic al final',
    curva: 'M2 20H12L30 5V20H42',
  },
  { id: 'rigido', nombre: 'Rígido', detalle: 'Resistencia firme', curva: 'M2 20H10V5H42' },
  {
    id: 'pulso',
    nombre: 'Pulso',
    detalle: 'Resistencia que vibra',
    curva: 'M2 20H8L12 6L17 20L22 6L27 20L32 6L37 20H42',
  },
]

const GATILLOS: { id: Gatillo; nombre: string; clave: 'gatilloL2' | 'gatilloR2' }[] = [
  { id: 'l2', nombre: 'L2', clave: 'gatilloL2' },
  { id: 'r2', nombre: 'R2', clave: 'gatilloR2' },
]

const pestanas = computed(() => {
  const lista: { id: Pestana; nombre: string }[] = [
    { id: 'luz', nombre: 'Luz' },
    { id: 'vibracion', nombre: 'Vibración' },
  ]
  if (mando.value && modeloDe(mando.value.tipo).gatillosAdaptativos) {
    lista.push({ id: 'gatillos', nombre: 'Gatillos' })
  }
  return lista
})

const bloqueado = computed(() => !conectado.value)

const selectorAbierto = ref(false)
let colorPrevio = ''

function abrirSelector() {
  colorPrevio = ajustes.value?.luz ?? ''
  selectorAbierto.value = !selectorAbierto.value
}

function guardarSelector(hex: string) {
  guardarColor(hex)
  selectorAbierto.value = false
}

function cancelarSelector() {
  if (colorPrevio) elegirLuz(colorPrevio)
  selectorAbierto.value = false
}

const gatilloActivo = ref<Gatillo>('r2')
const gatilloElegido = computed(() => GATILLOS.find((g) => g.id === gatilloActivo.value)!)
const efectoDe = (clave: 'gatilloL2' | 'gatilloR2') =>
  EFECTOS.find((e) => e.id === ajustes.value?.[clave])?.nombre ?? ''
const recorrido = (id: Gatillo) => (conectado.value ? (mando.value?.gatillos[id] ?? 0) : 0)

// Apretar un gatillo en el mando lo selecciona aquí.
watch(
  () => mando.value?.gatillos,
  (gatillos) => {
    if (!gatillos) return
    if (gatillos.l2 > 0.15 && gatillos.l2 >= gatillos.r2) gatilloActivo.value = 'l2'
    else if (gatillos.r2 > 0.15) gatilloActivo.value = 'r2'
  },
)
const colorNivelBrillo = computed(() => {
  const luz = ajustes.value?.luzSegunBateria ? mando.value?.luzActual : ajustes.value?.luz
  return !luz || luz === '#FFFFFF' ? 'var(--ink)' : luz
})
</script>

<template>
  <aside aria-label="Ajustes del mando" class="ajustes">
    <template v-if="!mando">
      <h2>Cómo conectarlo</h2>
      <div class="pasos">
        <div>
          <h3>Por USB</h3>
          <p>Conecta el cable y enciende el mando con el botón PS.</p>
        </div>
        <div>
          <h3>Por Bluetooth</h3>
          <p>
            Mantén pulsados Crear y PS hasta que la luz parpadee. Después, emparéjalo desde
            Bluetooth en la configuración de Windows.
          </p>
        </div>
      </div>
    </template>

    <template v-else-if="ajustes">
      <div
        role="tablist"
        aria-label="Qué ajustar"
        class="pestanas"
        data-atras="[aria-current='page']"
      >
        <button
          v-for="p in pestanas"
          :id="`pestana-${p.id}`"
          :key="p.id"
          role="tab"
          class="pestana"
          :class="{ activa: pestana === p.id }"
          :aria-selected="pestana === p.id"
          :aria-controls="`panel-${p.id}`"
          @click="pestana = p.id"
        >
          {{ p.nombre }}
        </button>
      </div>

      <p v-if="bloqueado" class="nota">Conecta el mando para cambiar estos ajustes.</p>

      <section
        v-if="pestana === 'luz'"
        id="panel-luz"
        role="tabpanel"
        aria-labelledby="pestana-luz"
        class="panel"
        data-atras="[role='tab'][aria-selected='true']"
      >
        <div class="colores">
          <button
            v-for="(hex, i) in coloresGuardados"
            :key="hex"
            class="color"
            :class="{ activo: !ajustes.arcoiris && !ajustes.luzSegunBateria && ajustes.luz === hex }"
            :style="{ background: hex }"
            :aria-label="`Color ${hex}`"
            :aria-pressed="!ajustes.arcoiris && !ajustes.luzSegunBateria && ajustes.luz === hex"
            :disabled="bloqueado"
            :data-foco-inicial="i === 0 ? '' : undefined"
            @click="elegirLuz(hex)"
          />
          <button
            class="color personalizado"
            :class="{ activo: selectorAbierto }"
            aria-label="Elegir un color personalizado"
            :aria-expanded="selectorAbierto"
            :disabled="bloqueado"
            @click="abrirSelector"
          />
        </div>
        <SelectorColor
          v-if="selectorAbierto"
          :color="ajustes.luz"
          @cambiar="elegirLuz"
          @guardar="guardarSelector"
          @cancelar="cancelarSelector"
        />
        <ControlNivel
          id="brillo"
          etiqueta="Brillo"
          :valor="ajustes.brillo"
          :min="5"
          :color="colorNivelBrillo"
          :desactivado="bloqueado"
          @cambiar="cambiarBrillo"
        />
        <InterruptorFila
          etiqueta="Modo arcoíris"
          :activo="ajustes.arcoiris"
          :desactivado="bloqueado"
          @cambiar="cambiarArcoiris"
        />
        <InterruptorFila
          etiqueta="Cambiar color según batería"
          :activo="ajustes.luzSegunBateria"
          :desactivado="bloqueado"
          @cambiar="cambiarLuzSegunBateria"
        />
      </section>

      <section
        v-else-if="pestana === 'vibracion'"
        id="panel-vibracion"
        role="tabpanel"
        aria-labelledby="pestana-vibracion"
        class="panel"
        data-atras="[role='tab'][aria-selected='true']"
      >
        <ControlNivel
          id="fuerza"
          etiqueta="Fuerza"
          :valor="ajustes.vibracion"
          :min="0"
          color="var(--ink)"
          :desactivado="bloqueado"
          @cambiar="cambiarVibracion"
        />
        <button class="probar" :disabled="bloqueado" @click="probar('vibracion')">
          {{ prueba === 'vibracion' ? 'Probando...' : 'Probar en el mando' }}
        </button>
      </section>

      <section
        v-else
        id="panel-gatillos"
        role="tabpanel"
        aria-labelledby="pestana-gatillos"
        class="panel"
        data-atras="[role='tab'][aria-selected='true']"
      >
        <div class="selector" role="group" aria-label="Gatillo que se ajusta">
          <button
            v-for="(g, i) in GATILLOS"
            :key="g.id"
            class="lado"
            :class="{ activa: gatilloActivo === g.id }"
            :aria-pressed="gatilloActivo === g.id"
            :disabled="bloqueado"
            :data-foco-inicial="i === 0 ? '' : undefined"
            @click="gatilloActivo = g.id"
          >
            <span class="lado-nombre">{{ g.nombre }}</span>
            <span class="lado-efecto">{{ efectoDe(g.clave) }}</span>
            <span class="recorrido" aria-hidden="true">
              <span :style="{ transform: `scaleX(${recorrido(g.id)})` }" />
            </span>
          </button>
        </div>
        <div class="efectos" role="group" :aria-label="`Efecto de ${gatilloElegido.nombre}`">
          <button
            v-for="e in EFECTOS"
            :key="e.id"
            class="efecto"
            :class="{ activa: ajustes[gatilloElegido.clave] === e.id }"
            :aria-pressed="ajustes[gatilloElegido.clave] === e.id"
            :disabled="bloqueado"
            @click="elegirEfecto(gatilloActivo, e.id)"
          >
            <svg viewBox="0 0 44 24" aria-hidden="true"><path :d="e.curva" /></svg>
            <span class="efecto-texto">
              <span class="efecto-nombre">{{ e.nombre }}</span>
              <span class="efecto-detalle">{{ e.detalle }}</span>
            </span>
          </button>
        </div>
        <p class="pista">Aprieta {{ gatilloElegido.nombre }} en el mando para notarlo.</p>
      </section>
    </template>
  </aside>
</template>

<style scoped>
.ajustes {
  flex: 1 1 340px;
  max-width: 100%;
  padding: 16px 24px 24px;
  background: var(--surface);
  border-radius: var(--radio-panel);
  display: flex;
  flex-direction: column;
  gap: 22px;
}

h2 {
  font-size: 18px;
  font-weight: 800;
  padding-top: 12px;
}

h3 {
  margin: 0 0 4px;
  font-size: 15px;
  font-weight: 600;
}

.pasos {
  display: flex;
  flex-direction: column;
  gap: 18px;
  color: var(--muted);
  line-height: 1.45;
  text-wrap: pretty;
}

.pasos h3 {
  color: var(--ink);
}

.nota {
  color: var(--muted);
  margin-bottom: -6px;
}

.pestanas {
  display: flex;
  gap: 4px;
  border-bottom: 1px solid var(--linea);
}

.pestana {
  flex: 1 1 auto;
  min-height: 48px;
  font-weight: 600;
  color: var(--muted);
  border-bottom: 3px solid transparent;
  transition: color 120ms var(--salida-rapida), border-color 120ms var(--salida-rapida);
}

.pestana:hover {
  color: var(--ink);
}

.pestana.activa {
  color: var(--ink);
  border-bottom-color: var(--accent);
}

.panel {
  display: flex;
  flex-direction: column;
  gap: 22px;
}

.colores {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
}

.color {
  height: 52px;
  border-radius: 12px;
  border: 1px solid var(--linea);
  transition: transform 120ms var(--salida-rapida), box-shadow 120ms var(--salida-rapida);
}

.color.personalizado {
  background: conic-gradient(
    #ff3b30,
    #ffcc00,
    #34c759,
    #00c7ff,
    #2f5bff,
    #d633ff,
    #ff3b30
  );
}

.color.activo {
  box-shadow: 0 0 0 2px var(--surface), 0 0 0 5px var(--ink);
}

.color:focus-visible {
  outline-offset: 7px;
}

[data-foco-activo] .color:focus {
  outline-offset: 7px;
}

.color:disabled,
.opcion:disabled,
.probar:disabled {
  opacity: 0.45;
}

.selector {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.lado {
  display: grid;
  gap: 2px;
  padding: 12px 14px;
  border-radius: var(--radio-control);
  background: var(--ground);
  text-align: left;
  transition: background-color 120ms var(--salida-rapida), color 120ms var(--salida-rapida);
}

.lado-nombre {
  font-size: 24px;
  font-weight: 800;
  line-height: 1.1;
}

.lado-efecto {
  font-size: 13px;
  color: var(--muted);
}

.recorrido {
  height: 6px;
  margin-top: 8px;
  border-radius: 3px;
  background: var(--linea);
  overflow: hidden;
}

.recorrido span {
  display: block;
  height: 100%;
  background: var(--accent);
  transform-origin: left;
}

.lado.activa {
  background: var(--ink);
  color: var(--surface);
}

.lado.activa .lado-efecto {
  color: var(--surface);
}

.efectos {
  display: grid;
  gap: 2px;
}

.efecto {
  display: flex;
  align-items: center;
  gap: 14px;
  min-height: 60px;
  padding: 8px 12px;
  border-radius: var(--radio-control);
  text-align: left;
  transition: background-color 120ms var(--salida-rapida);
}

.efecto svg {
  flex: none;
  width: 44px;
  height: 24px;
  fill: none;
  stroke: var(--guia);
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.efecto-texto {
  display: grid;
}

.efecto-nombre {
  font-weight: 600;
}

.efecto-detalle {
  font-size: 13px;
  color: var(--muted);
}

.efecto:hover:not(:disabled):not(.activa),
.lado:hover:not(:disabled):not(.activa) {
  background: var(--linea);
}

.efecto.activa {
  background: var(--ground);
}

.efecto.activa svg {
  stroke: var(--accent);
}

.pista {
  font-size: 13px;
  color: var(--muted);
}

.probar {
  min-height: 48px;
  border-radius: var(--radio-control);
  background: var(--ground);
  font-weight: 600;
  transition: transform 120ms var(--salida-rapida), background-color 120ms var(--salida-rapida);
}

.probar:hover:not(:disabled) {
  background: var(--linea);
}

</style>
