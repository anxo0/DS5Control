<script setup lang="ts">
import { paleta } from './estado/paleta'
import { tema } from './estado/tema'
import { defineAsyncComponent, nextTick, onMounted, ref, watch } from 'vue'
import type { Pestana } from './estado/app'
import {
  conectado,
  estadoApp,
  mando,
  ocultar,
  pestana,
  puente,
  recibirEstado,
  seccion,
} from './estado/app'
import { GestorFoco } from './nav/gestorFoco'
import { escucharGamepad, escucharTeclado } from './nav/entrada'
import type { Salidas } from './nav/entrada'
import { modeloDe } from './mandos/modelos'
import BarraSuperior from './componentes/BarraSuperior.vue'
import NavSecciones from './componentes/NavSecciones.vue'
import PanelMando from './componentes/PanelMando.vue'
import PanelAjustesMando from './componentes/PanelAjustesMando.vue'
import SeccionAjustes from './componentes/SeccionAjustes.vue'
import SeccionPendiente from './componentes/SeccionPendiente.vue'
import PiePagina from './componentes/PiePagina.vue'
import VentanaOculta from './componentes/VentanaOculta.vue'

const PanelDepuracion =
  import.meta.env.DEV && puente.depuracion
    ? defineAsyncComponent(() => import('./componentes/PanelDepuracion.vue'))
    : null

const raiz = ref<HTMLElement>()
let gestor: GestorFoco | undefined

const cambiarPestana = async (delta: 1 | -1) => {
  if (seccion.value !== 'mando' || !conectado.value || !mando.value) return
  const lista: Pestana[] = modeloDe(mando.value.tipo).gatillosAdaptativos
    ? ['luz', 'vibracion', 'gatillos']
    : ['luz', 'vibracion']
  const enPanel = !!document.activeElement?.closest('[role="tabpanel"]')
  const enPestanas = !!document.activeElement?.closest('[role="tablist"]')
  const i = lista.indexOf(pestana.value)
  pestana.value = lista[(i + delta + lista.length) % lista.length]!
  await nextTick()
  if (enPanel) gestor?.enfocar('[role="tabpanel"]')
  else if (enPestanas) gestor?.enfocar('[role="tab"][aria-selected="true"]')
  gestor?.asegurarFoco()
}

const alLlegarARaiz = async () => {
  if (seccion.value !== 'mando') {
    seccion.value = 'mando'
    await nextTick()
    gestor?.enfocar('[aria-current="page"]')
  } else if (estadoApp.value.abrirConMute) {
    // Sin el botón de silencio no habría forma de volver con el mando.
    ocultar()
  }
}

onMounted(() => {
  gestor = new GestorFoco(raiz.value!, {
    alLlegarARaiz,
    alCambiarPestana: cambiarPestana,
  })

  const salidas: Salidas = {
    accion: (accion) => {
      if (estadoApp.value.ventanaVisible) gestor?.accion(accion)
    },
    pulsar: (control, activo) => puente.depuracion?.pulsar(control, activo),
  }

  escucharTeclado(salidas)
  if (puente.entrada === 'gamepad') escucharGamepad(salidas)

  puente.iniciar({
    alEstado: recibirEstado,
    alMute: () => {
      // Al volver del segundo plano, la ventana debe recuperar el foco de entrada.
      window.focus()
      gestor?.mostrarFoco(true)
    },
    alNavegar: (accion) => salidas.accion(accion),
  })
  puente.enviar({ tipo: 'tema', valor: tema.value })
  puente.enviar({ tipo: 'paleta', valor: paleta.value })
})

watch(tema, (valor) => puente.enviar({ tipo: 'tema', valor }))
watch(paleta, (valor) => puente.enviar({ tipo: 'paleta', valor }))

// Un cambio de estado puede retirar el control que tenía el foco.
watch([estadoApp, seccion, pestana], () => nextTick(() => gestor?.asegurarFoco()), { flush: 'post' })
</script>

<template>
  <div ref="raiz" class="app">
    <BarraSuperior />
    <div class="cuerpo">
      <NavSecciones />
      <template v-if="seccion === 'mando'">
        <PanelMando />
        <PanelAjustesMando />
      </template>
      <SeccionAjustes v-else-if="seccion === 'ajustes'" />
      <SeccionPendiente v-else :seccion="seccion" />
    </div>
    <PiePagina />
    <VentanaOculta v-if="!estadoApp.ventanaVisible" />
    <component :is="PanelDepuracion" v-if="PanelDepuracion" />
  </div>
</template>

<style scoped>
.app {
  min-height: 100vh;
  background: var(--ground);
  color: var(--ink);
  display: flex;
  flex-direction: column;
}

.cuerpo {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  flex: 1 1 auto;
  min-width: 0;
  padding: 0 12px 12px;
}
</style>
