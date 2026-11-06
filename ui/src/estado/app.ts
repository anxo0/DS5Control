import { computed, ref, shallowRef } from 'vue'
import { crearPuente } from '../bridge'
import type { Diagnostico, Emulacion, AccionEspecial, AccionNueva, AjusteAvanzado, AjustesMando, AutoPerfil, SeccionAvanzada, Comando, EfectoGatillo, EstadoApp, EstadoMando, Gatillo, GrupoAvanzado } from '../bridge'

export type Seccion = 'mando' | 'botones' | 'perfiles' | 'acciones' | 'avanzado' | 'datos' | 'registro' | 'ajustes'
export type Pestana = 'luz' | 'vibracion' | 'gatillos' | 'sticks'
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
  controlConMando: false,
  ventanaVisible: true,
  perfiles: [],
  registro: [],
  servicios: { vigemBus: true, hidHide: true, instalandoVigem: false },
})
export const yaHuboMando = ref(false)
export const seccion = ref<Seccion>('mando')
export const pestana = ref<Pestana>('luz')
export const prueba = ref<Prueba | null>(null)
export const grupos = shallowRef<GrupoAvanzado[]>([])
export const grupoActivo = ref('sticks')
export const acciones = shallowRef<AccionEspecial[]>([])
export const archivoElegido = ref('')
export const generales = shallowRef<SeccionAvanzada[]>([])
export const autoPerfiles = shallowRef<AutoPerfil[]>([])
export const diagnostico = shallowRef<Diagnostico | null>(null)

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
  // Si el motor rechaza el cambio no llega eco: la pantalla no puede quedarse con un valor falso.
  setTimeout(() => {
    if (performance.now() - ultimoCambio >= MARGEN_ECO_MS && mando.value) {
      ajustesLocales.value = mando.value.ajustes
    }
  }, MARGEN_ECO_MS + 20)
}

const idMando = () => mando.value?.id ?? ''

export const elegirLuz = (color: string) => {
  // Elegir un color a mano deja de seguir a la batería.
  if (ajustesLocales.value?.luzSegunBateria) cambiarLuzSegunBateria(false)
  if (ajustesLocales.value?.arcoiris) cambiarArcoiris(false)
  ajustar({ luz: color }, { tipo: 'luz', mandoId: idMando(), color })
}

export const cambiarBrillo = (valor: number) =>
  ajustar({ brillo: valor }, { tipo: 'brillo', mandoId: idMando(), valor })

export const cambiarLuzSegunBateria = (activo: boolean) => {
  if (activo && ajustesLocales.value?.arcoiris) {
    ajustar({ arcoiris: false }, { tipo: 'arcoiris', mandoId: idMando(), activo: false })
  }
  ajustar({ luzSegunBateria: activo }, { tipo: 'luzSegunBateria', mandoId: idMando(), activo })
}

export const cambiarArcoiris = (activo: boolean) => {
  if (activo && ajustesLocales.value?.luzSegunBateria) {
    ajustar({ luzSegunBateria: false }, { tipo: 'luzSegunBateria', mandoId: idMando(), activo: false })
  }
  ajustar({ arcoiris: activo }, { tipo: 'arcoiris', mandoId: idMando(), activo })
}

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

export const cambiarEmulacion = (valor: Emulacion) =>
  ajustar({ emulacion: valor }, { tipo: 'emulacion', mandoId: idMando(), valor })

export const cargarOpciones = () => {
  if (mando.value) puente.enviar({ tipo: 'pedirOpciones', mandoId: mando.value.id })
}

export function cambiarOpcion(id: string, valor: AjusteAvanzado['valor']) {
  // Se pinta al momento: la respuesta del host llega unos milisegundos después.
  grupos.value = grupos.value.map((g) => ({
    ...g,
    secciones: g.secciones.map((s) => ({
      ...s,
      ajustes: s.ajustes.map((a) => (a.id === id ? { ...a, valor } : a)),
    })),
  }))
  puente.enviar({ tipo: 'opcion', mandoId: idMando(), id, valor })
}

export const instalarVigem = () => puente.enviar({ tipo: 'instalarVigem' })
export const pedirDiagnostico = () => puente.enviar({ tipo: 'pedirDiagnostico' })
export const pedirTelemetria = (activo: boolean) => puente.enviar({ tipo: 'telemetria', activo })

export const cargarGenerales = () => puente.enviar({ tipo: 'pedirGenerales' })
export const cambiarGeneral = (id: string, valor: AjusteAvanzado['valor']) =>
  puente.enviar({ tipo: 'general', id, valor })

export const cargarAutoPerfiles = () => puente.enviar({ tipo: 'pedirAuto' })
export const anadirAutoPerfil = () => puente.enviar({ tipo: 'autoAnadir' })
export const cambiarAutoPerfil = (ruta: string, perfil: string) =>
  puente.enviar({ tipo: 'autoPerfil', ruta, perfil })
export const quitarAutoPerfil = (ruta: string) => puente.enviar({ tipo: 'autoQuitar', ruta })

export const cargarAcciones = () => {
  if (mando.value) puente.enviar({ tipo: 'pedirAcciones', mandoId: mando.value.id })
}

export function guardarAccion({ tipo, ...resto }: AccionNueva) {
  puente.enviar({ ...resto, mandoId: idMando(), tipo: 'guardarAccion', tipoAccion: tipo })
}

export const borrarAccion = (nombre: string) =>
  puente.enviar({ tipo: 'borrarAccion', mandoId: idMando(), nombre })

export const activarAccion = (nombre: string, activa: boolean) =>
  puente.enviar({ tipo: 'activarAccion', mandoId: idMando(), nombre, activa })

export const elegirPrograma = () => puente.enviar({ tipo: 'elegirPrograma', mandoId: idMando() })

export const asignarTecla = (control: string, tecla: number) =>
  puente.enviar({ tipo: 'mapearTecla', mandoId: idMando(), control, tecla })

export const asignarMacro = (control: string, pasos: number[]) =>
  puente.enviar({ tipo: 'mapearMacro', mandoId: idMando(), control, pasos })

export const usarPerfil = (nombre: string) =>
  puente.enviar({ tipo: 'perfil', mandoId: idMando(), nombre })

export const crearPerfil = (nombre: string) =>
  puente.enviar({ tipo: 'nuevoPerfil', mandoId: mando.value?.id ?? '', nombre })

export const borrarPerfil = (nombre: string) => puente.enviar({ tipo: 'borrarPerfil', nombre })

export const asignarBoton = (control: string, salida: string | null) =>
  puente.enviar({ tipo: 'mapear', mandoId: idMando(), control, salida })

export const cambiarControlConMando = (activo: boolean) =>
  puente.enviar({ tipo: 'controlConMando', activo })

export const cambiarAbrirConMute = (activo: boolean) =>
  puente.enviar({ tipo: 'abrirConMute', activo })

export const ocultar = () => puente.enviar({ tipo: 'ocultar' })

/** Dentro de la app, la ventana no tiene barra del sistema: los botones son nuestros. */
export const enVentana = puente.entrada === 'host'

export const controlarVentana = (accion: 'minimizar' | 'maximizar') =>
  puente.enviar({ tipo: 'ventana', accion })
