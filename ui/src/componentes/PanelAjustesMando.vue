<script setup lang="ts">
import { computed } from 'vue'
import type { EfectoGatillo, Gatillo } from '../bridge'
import {
  ajustesLocales as ajustes,
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
import InterruptorFila from './InterruptorFila.vue'

const COLORES = [
  ['Azul', '#1F4FE0'],
  ['Cian', '#00A3FF'],
  ['Verde agua', '#00B894'],
  ['Lima', '#7ED321'],
  ['Amarillo', '#FFC400'],
  ['Naranja', '#FF6B2C'],
  ['Rojo', '#E8254F'],
  ['Blanco', '#FFFFFF'],
] as const

const EFECTOS: { id: EfectoGatillo; nombre: string }[] = [
  { id: 'ninguno', nombre: 'Ninguno' },
  { id: 'clicCompleto', nombre: 'Clic completo' },
  { id: 'rigido', nombre: 'Rígido' },
  { id: 'pulso', nombre: 'Pulso' },
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
            v-for="([nombre, hex], i) in COLORES"
            :key="hex"
            class="color"
            :class="{ activo: !ajustes.luzSegunBateria && ajustes.luz === hex }"
            :style="{ background: hex }"
            :aria-label="nombre"
            :aria-pressed="!ajustes.luzSegunBateria && ajustes.luz === hex"
            :disabled="bloqueado"
            :data-foco-inicial="i === 0 ? '' : undefined"
            @click="elegirLuz(hex)"
          />
        </div>
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
        <div v-for="g in GATILLOS" :key="g.id" role="group" :aria-label="g.nombre">
          <div class="titulo-gatillo">{{ g.nombre }}</div>
          <div class="opciones">
            <button
              v-for="e in EFECTOS"
              :key="e.id"
              class="opcion"
              :class="{ activa: ajustes[g.clave] === e.id }"
              :aria-pressed="ajustes[g.clave] === e.id"
              :disabled="bloqueado"
              @click="elegirEfecto(g.id, e.id)"
            >
              {{ e.nombre }}
            </button>
          </div>
        </div>
        <button class="probar" :disabled="bloqueado" @click="probar('gatillos')">
          {{ prueba === 'gatillos' ? 'Probando...' : 'Probar en el mando' }}
        </button>
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

.titulo-gatillo {
  font-weight: 600;
  margin-bottom: 8px;
}

.opciones {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.opcion,
.probar {
  min-height: 48px;
  border-radius: var(--radio-control);
  background: var(--ground);
  font-weight: 600;
  transition: transform 120ms var(--salida-rapida), background-color 120ms var(--salida-rapida);
}

.opcion:hover:not(:disabled):not(.activa),
.probar:hover:not(:disabled) {
  background: var(--linea);
}

.opcion.activa {
  background: var(--ink);
  color: var(--surface);
}
</style>
