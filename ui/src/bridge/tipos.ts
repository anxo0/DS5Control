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

export interface AjustesMando {
  /** Color elegido, en hexadecimal. */
  luz: string
  /** 5 a 100. */
  brillo: number
  luzSegunBateria: boolean
  /** 0 a 100. */
  vibracion: number
  gatilloL2: EfectoGatillo
  gatilloR2: EfectoGatillo
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
  ajustes: AjustesMando
}

export interface EstadoApp {
  mandos: EstadoMando[]
  abrirConMute: boolean
  ventanaVisible: boolean
  servicios: { vigemBus: boolean; hidHide: boolean }
}

export type Comando =
  | { tipo: 'luz'; mandoId: string; color: string }
  | { tipo: 'brillo'; mandoId: string; valor: number }
  | { tipo: 'luzSegunBateria'; mandoId: string; activo: boolean }
  | { tipo: 'vibracion'; mandoId: string; fuerza: number }
  | { tipo: 'probarVibracion'; mandoId: string }
  | { tipo: 'gatillo'; mandoId: string; gatillo: Gatillo; efecto: EfectoGatillo }
  | { tipo: 'probarGatillos'; mandoId: string }
  | { tipo: 'abrirConMute'; activo: boolean }
  | { tipo: 'ocultar' }

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
  | { tipo: 'navegacion'; accion: AccionNavegacion }

export interface Eventos {
  alEstado(estado: EstadoApp): void
  alMute(): void
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
