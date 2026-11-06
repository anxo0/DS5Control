import { crearPuenteSimulado } from './simulado'
import { crearPuenteWebView } from './webview'
import type { Puente } from './tipos'

export type * from './tipos'

/** Dentro de WebView2 existe `window.chrome.webview`; en un navegador normal, no. */
export function crearPuente(): Puente {
  const canal = window.chrome?.webview
  return typeof canal?.postMessage === 'function' ? crearPuenteWebView(canal) : crearPuenteSimulado()
}
