import type { CanalWebView } from './webview'

interface TauriGlobal {
  core: { invoke(comando: string, argumentos: Record<string, unknown>): Promise<unknown> }
  event: { listen(evento: string, oyente: (evento: { payload: unknown }) => void): Promise<unknown> }
  window: { getCurrentWindow(): { startDragging(): Promise<void>; toggleMaximize(): Promise<void> } }
}

declare global {
  interface Window {
    __TAURI__?: TauriGlobal
  }
}

/**
 * La barra superior se arrastra con `app-region: drag`, que en la versión en C# resuelve WebView2.
 * Tauri no lo hace solo, así que aquí se hace lo mismo: arrastrar la ventana desde una zona
 * `drag` (salvo sus botones) y maximizar o restaurar con doble clic.
 */
function arrastrarConAppRegion(tauri: TauriGlobal) {
  const enZonaDeArrastre = (destino: EventTarget | null) => {
    for (let el = destino instanceof Element ? destino : null; el; el = el.parentElement) {
      const region = getComputedStyle(el).getPropertyValue('app-region')
      if (region === 'no-drag') return false
      if (region === 'drag') return true
    }
    return false
  }

  document.addEventListener('mousedown', (evento) => {
    if (evento.button !== 0 || !enZonaDeArrastre(evento.target)) return
    const ventana = tauri.window.getCurrentWindow()
    evento.preventDefault()
    void (evento.detail === 2 ? ventana.toggleMaximize() : ventana.startDragging())
  })
}

/** Dentro de la versión en Rust (Tauri) existe `window.__TAURI__`. */
export function canalTauri(): CanalWebView | null {
  const tauri = window.__TAURI__
  if (!tauri) return null

  arrastrarConAppRegion(tauri)

  // Los mensajes salientes esperan a que el oyente esté registrado: si no, la respuesta a
  // `listo` podría llegar antes y perderse.
  let oyendo: Promise<unknown> = Promise.resolve()

  // El mismo canal que WebView2: así el puente de webview.ts sirve tal cual.
  return {
    postMessage(mensaje) {
      void oyendo.then(() => tauri.core.invoke('mensaje', { mensaje }))
    },
    addEventListener(_tipo, oyente) {
      oyendo = tauri.event.listen('mensaje', (evento) => oyente({ data: evento.payload }))
    },
  }
}
