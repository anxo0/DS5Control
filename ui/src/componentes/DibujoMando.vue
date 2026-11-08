<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { ControlMando, EfectoGatillo, EstadoMando } from '../bridge'
import { ajustesLocales, estadoApp, prueba } from '../estado/app'
import { modeloDe } from '../mandos/modelos'

const props = defineProps<{
  mando: EstadoMando | null
  conectado: boolean
  /** Sin rótulos alrededor, para pantallas que explican el mando por su cuenta. */
  limpio?: boolean
  /** Botón del dibujo marcado como elegido; su presencia hace pulsable el mando. */
  seleccion?: string
}>()
const emit = defineEmits<{ elegir: [id: string] }>()

function alPulsar(e: MouseEvent) {
  const id = (e.target as Element).closest('[id]')?.id
  if (id) emit('elegir', id.replace(/-(simbolo|border|icon)$/, '').replace(/group$/, ''))
}

/** Lo que se desplaza la seta del stick dentro de su aro, en unidades del dibujo. */
const RECORRIDO_STICK = 28

const EFECTOS: Record<EfectoGatillo, string> = {
  ninguno: 'Ninguno',
  clicCompleto: 'Clic completo',
  rigido: 'Rígido',
  pulso: 'Pulso',
}

const modelo = computed(() => modeloDe(props.mando?.tipo ?? 'dualsense'))
const lienzo = ref<SVGGElement>()
const ajustes = ajustesLocales

const luz = computed(() => (props.conectado ? (props.mando?.luzActual ?? null) : null))
const opacidadLuz = computed(() => 0.3 + 0.7 * ((ajustes.value?.brillo ?? 100) / 100))

const descripcion = computed(() => {
  if (!props.mando) return 'Dibujo del mando, sin ningún mando conectado'
  return 'Mando con su barra de luz, gatillos y botón de silencio'
})

const resaltados = computed<ControlMando[]>(() => {
  const pulsados = props.conectado ? (props.mando?.pulsados ?? []) : []
  const lista = prueba.value === 'gatillos' ? ([...pulsados, 'l2', 'r2'] as ControlMando[]) : pulsados
  return props.seleccion ? [...lista, props.seleccion as ControlMando] : lista
})

// Posiciones directas sobre el SVG: cambian decenas de veces por segundo.
watch(
  [() => props.mando?.sticks, () => props.mando?.gatillos, () => props.conectado, lienzo],
  ([sticks, gatillos, activo, g]) => {
    if (!g) return
    const buscar = (id: string) => g.querySelector<SVGElement>(`[id="${id}"]`)
    const mover = (id: string, x = 0, y = 0) => {
      const el = buscar(id)
      if (el) el.style.transform = `translate(${x * RECORRIDO_STICK}px, ${y * RECORRIDO_STICK}px)`
    }
    const llenar = (id: string, valor = 0) => {
      const el = buscar(id)
      if (!el) return
      el.style.fill = valor > 0 ? 'var(--accent)' : 'none'
      el.style.fillOpacity = valor > 0 ? String(valor) : ''
    }
    mover('l3', activo ? sticks?.izq.x : 0, activo ? sticks?.izq.y : 0)
    mover('r3', activo ? sticks?.der.x : 0, activo ? sticks?.der.y : 0)
    llenar('l2', activo ? gatillos?.l2 : 0)
    llenar('r2', activo ? gatillos?.r2 : 0)
  },
  { flush: 'post' },
)

let anteriores: Element[] = []
watch(
  [resaltados, lienzo],
  ([controles, g]) => {
    anteriores.forEach((el) => el.classList.remove('pulsado'))
    if (!g) return
    anteriores = controles.flatMap((id) => [
      ...g.querySelectorAll(`[id="${id}"], [id="${id}-simbolo"]`),
    ])
    anteriores.forEach((el) => el.classList.add('pulsado'))
  },
  { flush: 'post' },
)
</script>

<template>
  <div
    class="dibujo"
    :class="{ apagado: !conectado, vibrando: prueba === 'vibracion', pulsable: seleccion }"
    :style="{ aspectRatio: modelo.proporcion }"
  >
    <svg :viewBox="modelo.viewBox" role="img" :aria-label="descripcion">
      <g class="mando">
        <g ref="lienzo" class="trazo" @click="alPulsar" v-html="modelo.dibujo" />
        <path class="luz-fondo" :class="{ blanca: luz === '#FFFFFF' }" :d="modelo.barraLuz" />
        <path
          v-if="luz"
          class="luz"
          :d="modelo.barraLuz"
          :style="{ stroke: luz, opacity: opacidadLuz }"
        />
      </g>
      <template v-if="mando && ajustes && !limpio">
        <g class="guias">
          <path v-for="g in modelo.guias" :key="g" :d="g" />
        </g>
        <g text-anchor="end">
          <text class="nombre" :x="modelo.rotulos.l2.x" :y="modelo.rotulos.l2.y">L2</text>
          <text class="valor" :x="modelo.rotulos.l2.x" :y="modelo.rotulos.l2.y + 32">
            {{ EFECTOS[ajustes.gatilloL2] }}
          </text>
          <text class="nombre" :x="modelo.rotulos.luz.x" :y="modelo.rotulos.luz.y">Barra de luz</text>
          <text class="valor" :x="modelo.rotulos.luz.x" :y="modelo.rotulos.luz.y + 32">
            {{ luz ?? 'Apagada' }}
          </text>
        </g>
        <g>
          <text class="nombre" :x="modelo.rotulos.r2.x" :y="modelo.rotulos.r2.y">R2</text>
          <text class="valor" :x="modelo.rotulos.r2.x" :y="modelo.rotulos.r2.y + 32">
            {{ EFECTOS[ajustes.gatilloR2] }}
          </text>
          <text class="nombre" :x="modelo.rotulos.vibracion.x" :y="modelo.rotulos.vibracion.y">
            Vibración
          </text>
          <text class="valor" :x="modelo.rotulos.vibracion.x" :y="modelo.rotulos.vibracion.y + 32">
            {{ ajustes.vibracion }}%
          </text>
        </g>
        <g v-if="modelo.rotulos.silencio" text-anchor="middle">
          <text class="nombre" :x="modelo.rotulos.silencio.x" :y="modelo.rotulos.silencio.y">
            Silencio
          </text>
          <text class="valor" :x="modelo.rotulos.silencio.x" :y="modelo.rotulos.silencio.y + 32">
            {{ estadoApp.abrirConMute ? 'Abre esta app' : 'Sin función' }}
          </text>
        </g>
      </template>
    </svg>
  </div>
</template>

<style scoped>
.dibujo {
  position: relative;
  width: 100%;
  max-width: 860px;
  margin: auto;
}

svg {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  font-size: 28px;
}

.trazo {
  --mando-trazo: var(--ink);
  --mando-relleno: var(--linea);
  fill-rule: evenodd;
  clip-rule: evenodd;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-miterlimit: 1.5;
}

.apagado .trazo {
  --mando-trazo: var(--guia);
}

.luz-fondo,
.luz {
  fill: none;
  stroke-linecap: round;
}

.luz-fondo {
  stroke: var(--linea);
  stroke-width: 17;
}

/* La luz blanca desaparecería sobre el panel claro. */
.luz-fondo.blanca {
  stroke: var(--guia);
}

.luz {
  stroke-width: 13;
  transition: stroke 200ms var(--salida-rapida), opacity 200ms var(--salida-rapida);
}

.guias path {
  stroke: var(--guia);
  stroke-width: 2;
  fill: none;
}

.nombre {
  font-weight: 600;
  fill: var(--ink);
}

.valor {
  fill: var(--muted);
}

.vibrando .mando {
  animation: vibrar 90ms linear infinite alternate;
}

@keyframes vibrar {
  from {
    transform: translateX(-3px);
  }
  to {
    transform: translateX(3px);
  }
}
.pulsable .trazo :deep([id]) {
  cursor: pointer;
  pointer-events: all;
}
</style>

<style>
/* Sin scoped: estos nodos llegan por v-html. !important vence a sus estilos en línea. */
.trazo #t1,
.trazo #t2 {
  display: none;
}

.trazo .pulsado,
.trazo g.pulsado > path:first-child {
  fill: var(--accent) !important;
  stroke: var(--accent) !important;
}

.trazo g.pulsado > path:last-child,
.trazo [id$='-simbolo'].pulsado {
  fill: none !important;
  stroke: var(--surface) !important;
}

.trazo g.pulsado > path:last-child {
  fill: var(--surface) !important;
  stroke: none !important;
}

.trazo #touchpad.pulsado {
  fill-opacity: 0.3;
}
</style>
