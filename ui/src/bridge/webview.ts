import type { Comando, Eventos, MensajeEntrante, MensajeSaliente, Puente } from './tipos'

export interface CanalWebView {
  postMessage(mensaje: unknown): void
  addEventListener(tipo: 'message', oyente: (evento: { data: unknown }) => void): void
}

declare global {
  interface Window {
    chrome?: { webview?: CanalWebView }
  }
}

/** Intervalo mínimo entre comandos continuos, para no saturar el motor al arrastrar. */
const INTERVALO_MS = 50

function clavesContinuas(comando: Comando): string | null {
  if (comando.tipo === 'brillo' || comando.tipo === 'vibracion' || comando.tipo === 'luz') {
    return `${comando.tipo}:${comando.mandoId}`
  }
  return null
}

function leerMensaje(data: unknown): MensajeEntrante | null {
  let valor = data
  if (typeof valor === 'string') {
    try {
      valor = JSON.parse(valor)
    } catch {
      return null
    }
  }
  if (typeof valor !== 'object' || valor === null || !('tipo' in valor)) return null
  return valor as MensajeEntrante
}

export function crearPuenteWebView(canal: CanalWebView): Puente {
  const enviarMensaje = (mensaje: MensajeSaliente) => canal.postMessage(mensaje)
  const pendientes = new Map<string, Comando>()
  const ultimos = new Map<string, number>()

  const vaciar = (clave: string) => {
    const comando = pendientes.get(clave)
    if (!comando) return
    pendientes.delete(clave)
    ultimos.set(clave, performance.now())
    enviarMensaje({ tipo: 'comando', comando })
  }

  return {
    entrada: 'host',
    iniciar(eventos: Eventos) {
      canal.addEventListener('message', ({ data }) => {
        const mensaje = leerMensaje(data)
        switch (mensaje?.tipo) {
          case 'estado':
            eventos.alEstado(mensaje.estado)
            break
          case 'mute':
            eventos.alMute()
            break
          case 'navegacion':
            eventos.alNavegar(mensaje.accion)
            break
        }
      })
      enviarMensaje({ tipo: 'listo' })
    },
    enviar(comando) {
      const clave = clavesContinuas(comando)
      if (!clave) {
        enviarMensaje({ tipo: 'comando', comando })
        return
      }
      const yaEsperando = pendientes.has(clave)
      pendientes.set(clave, comando)
      if (yaEsperando) return
      const espera = (ultimos.get(clave) ?? -Infinity) + INTERVALO_MS - performance.now()
      if (espera <= 0) vaciar(clave)
      else setTimeout(() => vaciar(clave), espera)
    },
  }
}
