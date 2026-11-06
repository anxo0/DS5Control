import { computed, ref, shallowRef } from 'vue'
import { crearPuente } from '../bridge'
import type { AjustesMando, Comando, EfectoGatillo, EstadoApp, EstadoMando, Gatillo } from '../bridge'

export type Seccion = 'mando' | 'botones' | 'perfiles' | 'registro' | 'ajustes'
export type Pestana = 'luz' | 'vibracion' | 'gatillos'
export type Prueba = 'vibracion' | 'gatillos'

/** Por debajo de esto se avisa de batería baja. */
export const UMBRAL_BATERIA_BAJA = 20
/** Mientras dura, el eco del host no pisa lo que el usuario acaba de mover. */
const MARGEN_ECO_MS = 500
const DURACION_PRUEBA_MS = 900

export const puente = crearPuente()

export const estadoApp = shallowRef<EstadoApp>({
  mandos: [],
  abrirConMute: true,
  ventanaVisible: true,
  servicios: { vigemBus: true, hidHide: true },
})
export const yaHuboMando = ref(false)
export const seccion = ref<Seccion>('mando')
export const pestana = ref<Pestana>('luz')
export const prueba = ref<Prueba | null>(null)

export const ajustesLocales = shallowRef<AjustesMando | null>(null)
let ultimoCambio = -Infinity

export const mando = computed<EstadoMando | null>(() => {
  const { mandos } = estadoApp.value
  return mandos.find((m) => m.conectado) ?? mandos[0] ?? null
})

export const conectado = computed(() => mando.value?.conectado === true)

export const bateriaBaja = computed(() => {
  const m = mando.value
  return !!m && m.conectado && !m.cargando && m.bateria !== null && m.bateria <= UMBRAL_BATERIA_BAJA
})

export function recibirEstado(estado: EstadoApp) {
  estadoApp.value = estado
  const actual = mando.value
  if (actual) yaHuboMando.value = true
  if (!actual) ajustesLocales.value = null
  else if (!ajustesLocales.value || performance.now() - ultimoCambio > MARGEN_ECO_MS) {
    ajustesLocales.value = actual.ajustes
  }
}

function ajustar(cambio: Partial<AjustesMando>, comando: Comando) {
  if (!ajustesLocales.value) return
  ajustesLocales.value = { ...ajustesLocales.value, ...cambio }
  ultimoCambio = performance.now()
  puente.enviar(comando)
}

const idMando = () => mando.value?.id ?? ''

export const elegirLuz = (color: string) => {
  // Elegir un color a mano deja de seguir a la batería.
  if (ajustesLocales.value?.luzSegunBateria) cambiarLuzSegunBateria(false)
  ajustar({ luz: color }, { tipo: 'luz', mandoId: idMando(), color })
}

export const cambiarBrillo = (valor: number) =>
  ajustar({ brillo: valor }, { tipo: 'brillo', mandoId: idMando(), valor })

export const cambiarLuzSegunBateria = (activo: boolean) =>
  ajustar({ luzSegunBateria: activo }, { tipo: 'luzSegunBateria', mandoId: idMando(), activo })

export const cambiarVibracion = (fuerza: number) =>
  ajustar({ vibracion: fuerza }, { tipo: 'vibracion', mandoId: idMando(), fuerza })

export const elegirEfecto = (gatillo: Gatillo, efecto: EfectoGatillo) =>
  ajustar(gatillo === 'l2' ? { gatilloL2: efecto } : { gatilloR2: efecto }, {
    tipo: 'gatillo',
    mandoId: idMando(),
    gatillo,
    efecto,
  })

let temporizadorPrueba: ReturnType<typeof setTimeout> | undefined

export function probar(tipo: Prueba) {
  puente.enviar({
    tipo: tipo === 'vibracion' ? 'probarVibracion' : 'probarGatillos',
    mandoId: idMando(),
  })
  prueba.value = tipo
  clearTimeout(temporizadorPrueba)
  temporizadorPrueba = setTimeout(() => (prueba.value = null), DURACION_PRUEBA_MS)
}

export const cambiarAbrirConMute = (activo: boolean) =>
  puente.enviar({ tipo: 'abrirConMute', activo })

export const ocultar = () => puente.enviar({ tipo: 'ocultar' })
