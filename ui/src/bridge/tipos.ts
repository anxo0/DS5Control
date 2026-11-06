/** Contrato entre la interfaz y el motor. Todo viaja como JSON. */

export type TipoMando = 'dualsense' | 'dualshock4'
export type TipoConexion = 'bluetooth' | 'usb'
export type EfectoGatillo = 'ninguno' | 'clicCompleto' | 'rigido' | 'pulso'
export type Gatillo = 'l2' | 'r2'

/** Coinciden con los id de los SVG de `assets/mandos`. */
export type ControlMando =
  | 'cross' | 'circle' | 'rect' | 'triangle'
  | 'dpad-up' | 'dpad-down' | 'dpad-left' | 'dpad-right'
  | 'l1' | 'r1' | 'l2' | 'r2' | 'l3' | 'r3'
  | 'create' | 'options' | 'ps' | 'touchpad' | 'mute'

/** De -1 a 1. Positivo es derecha y abajo, como en el dibujo. */
export interface PosicionStick {
  x: number
  y: number
}

export interface AjustesMando {
  /** Color elegido, en hexadecimal. */
  luz: string
  /** 5 a 100. */
  brillo: number
  luzSegunBateria: boolean
  /** La luz recorre todos los colores. */
  arcoiris: boolean
  /** Mando virtual que ven los juegos. */
  emulacion: Emulacion
  /** 0 a 100. */
  vibracion: number
  gatilloL2: EfectoGatillo
  gatilloR2: EfectoGatillo
}

export type Emulacion = 'xbox' | 'ds4' | 'nativo'

export interface EntradaRegistro {
  hora: string
  texto: string
  aviso: boolean
}

export interface BotonMapeado {
  /** Nombre interno del control en el motor. */
  control: string
  /** Salida asignada. null es la función original del botón. */
  salida: string | null
}

export interface DatosMando {
  bateria: { nivel: number; estado: number }
  audio: { cascos: boolean; micro: boolean }
  giro: { x: number; y: number; z: number }
  acel: { x: number; y: number; z: number }
  toques: { x: number; y: number }[]
}

export interface FilaDato<T = number> {
  nombre: string
  valor: T
}

export interface UsoMando {
  registros: number
  activo: number
  carga: number
  haptico: number
  gatilloIzq: number
  gatilloDer: number
  ciclosCarga: number
  cargasCompletas: number
  conexion: FilaDato[]
  errores: FilaDato[]
  pulsaciones: FilaDato[]
  sticks: FilaDato[]
}

export interface Diagnostico {
  milivoltios: number | null
  fabrica: FilaDato<string>[]
  uso: UsoMando | null
}

export interface EstadoMando {
  id: string
  nombre: string
  tipo: TipoMando
  conexion: TipoConexion
  conectado: boolean
  /** 0 a 100, o null si no hay lectura. */
  bateria: number | null
  cargando: boolean
  /** Mando virtual que se está emulando, por ejemplo "Xbox 360". */
  emulando: string | null
  /** Color real de la barra: difiere de `ajustes.luz` si manda la batería. */
  luzActual: string
  pulsados: ControlMando[]
  perfil: string
  mapeo: BotonMapeado[]
  /** Solo llega con la sección Datos abierta. */
  datos?: DatosMando | null
  sticks: { izq: PosicionStick; der: PosicionStick }
  /** Recorrido de cada gatillo, de 0 a 1. */
  gatillos: { l2: number; r2: number }
  ajustes: AjustesMando
}

export interface AjusteAvanzado {
  id: string
  nombre: string
  tipo: 'interruptor' | 'numero' | 'opciones' | 'botones' | 'curva' | 'accion'
  min: number
  max: number
  paso: number
  porcentaje: boolean
  opciones: string[] | null
  valor: boolean | number | string | number[] | null
}

export interface SeccionAvanzada {
  nombre: string
  ajustes: AjusteAvanzado[]
}

export interface GrupoAvanzado {
  id: string
  nombre: string
  secciones: SeccionAvanzada[]
}

export type TipoAccion = 'macro' | 'tecla' | 'programa' | 'perfil' | 'bateria' | 'desconectar' | 'giroscopio' | 'otra'

export interface AccionEspecial {
  nombre: string
  tipo: TipoAccion
  controles: string[]
  detalles: string
  extra: string
  retardo: number
  pasos: number[]
  repetir: boolean
  activa: boolean
}

export type AccionNueva = Omit<AccionEspecial, 'activa'> & { original: string | null }

export interface AutoPerfil {
  ruta: string
  nombre: string
  perfil: string
}

export interface EstadoApp {
  mandos: EstadoMando[]
  abrirConMute: boolean
  ventanaVisible: boolean
  perfiles: string[]
  registro: EntradaRegistro[]
  servicios: { vigemBus: boolean; hidHide: boolean }
}

export type Comando =
  | { tipo: 'luz'; mandoId: string; color: string }
  | { tipo: 'brillo'; mandoId: string; valor: number }
  | { tipo: 'luzSegunBateria'; mandoId: string; activo: boolean }
  | { tipo: 'arcoiris'; mandoId: string; activo: boolean }
  | { tipo: 'vibracion'; mandoId: string; fuerza: number }
  | { tipo: 'probarVibracion'; mandoId: string }
  | { tipo: 'gatillo'; mandoId: string; gatillo: Gatillo; efecto: EfectoGatillo }
  | { tipo: 'probarGatillos'; mandoId: string }
  | { tipo: 'abrirConMute'; activo: boolean }
  | { tipo: 'ocultar' }
  | { tipo: 'pedirDiagnostico' }
  | { tipo: 'telemetria'; activo: boolean }
  | { tipo: 'pedirGenerales' }
  | { tipo: 'general'; id: string; valor: AjusteAvanzado['valor'] }
  | { tipo: 'pedirAuto' }
  | { tipo: 'autoAnadir' }
  | { tipo: 'autoPerfil'; ruta: string; perfil: string }
  | { tipo: 'autoQuitar'; ruta: string }
  | { tipo: 'pedirAcciones'; mandoId: string }
  | ({ tipo: 'guardarAccion'; mandoId: string } & Omit<AccionNueva, 'tipo'> & { tipoAccion: TipoAccion })
  | { tipo: 'borrarAccion'; mandoId: string; nombre: string }
  | { tipo: 'activarAccion'; mandoId: string; nombre: string; activa: boolean }
  | { tipo: 'elegirPrograma'; mandoId: string }
  | { tipo: 'mapearTecla'; mandoId: string; control: string; tecla: number }
  | { tipo: 'mapearMacro'; mandoId: string; control: string; pasos: number[] }
  | { tipo: 'pedirOpciones'; mandoId: string }
  | { tipo: 'opcion'; mandoId: string; id: string; valor: AjusteAvanzado['valor'] }
  | { tipo: 'perfil'; mandoId: string; nombre: string }
  | { tipo: 'nuevoPerfil'; mandoId: string; nombre: string }
  | { tipo: 'borrarPerfil'; nombre: string }
  | { tipo: 'mapear'; mandoId: string; control: string; salida: string | null }
  | { tipo: 'ventana'; accion: 'minimizar' | 'maximizar' }
  | { tipo: 'tema'; valor: 'claro' | 'oscuro' }
  | { tipo: 'paleta'; valor: string }
  | { tipo: 'emulacion'; mandoId: string; valor: Emulacion }

export type AccionNavegacion =
  | 'arriba' | 'abajo' | 'izquierda' | 'derecha'
  | 'elegir' | 'atras'
  | 'pestanaAnterior' | 'pestanaSiguiente'

/** Mensajes de la interfaz al host. */
export type MensajeSaliente =
  | { tipo: 'listo' }
  | { tipo: 'comando'; comando: Comando }

/** Mensajes del host a la interfaz. */
export type MensajeEntrante =
  | { tipo: 'estado'; estado: EstadoApp }
  | { tipo: 'mute' }
  | { tipo: 'diagnostico'; datos: Diagnostico | null }
  | { tipo: 'generales'; secciones: SeccionAvanzada[] }
  | { tipo: 'autoPerfiles'; lista: AutoPerfil[] }
  | { tipo: 'acciones'; acciones: AccionEspecial[] }
  | { tipo: 'archivo'; ruta: string }
  | { tipo: 'opciones'; grupos: GrupoAvanzado[] }
  | { tipo: 'navegacion'; accion: AccionNavegacion }

export interface Eventos {
  alEstado(estado: EstadoApp): void
  alMute(): void
  alDiagnostico(datos: Diagnostico | null): void
  alGenerales(secciones: SeccionAvanzada[]): void
  alAutoPerfiles(lista: AutoPerfil[]): void
  alAcciones(acciones: AccionEspecial[]): void
  alArchivo(ruta: string): void
  alOpciones(grupos: GrupoAvanzado[]): void
  alNavegar(accion: AccionNavegacion): void
}

/** Controles que solo existen fuera del host, para probar la interfaz. */
export interface Depuracion {
  pulsar(control: ControlMando, activo: boolean): void
  mute(): void
  fijarBateria(valor: number): void
  fijarCargando(activo: boolean): void
  fijarConectado(activo: boolean): void
  quitarMandos(): void
  restablecer(): void
}

export interface Puente {
  /** `gamepad`: la entrada del mando se lee aquí. `host`: llega por mensajes. */
  readonly entrada: 'gamepad' | 'host'
  readonly depuracion?: Depuracion
  iniciar(eventos: Eventos): void
  enviar(comando: Comando): void
}
