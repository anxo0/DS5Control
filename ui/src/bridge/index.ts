import { crearPuenteSimulado } from './simulado'
import { canalTauri } from './tauri'
import { crearPuenteWebView } from './webview'
import type { Puente } from './tipos'

export type * from './tipos'

/**
 * En la versión en Rust existe `window.__TAURI__`; en la de C#, `window.chrome.webview`; en un
 * navegador normal, ninguno. Tauri va primero porque en Windows también trae `chrome.webview`.
 */
export function crearPuente(): Puente {
  const tauri = canalTauri()
  if (tauri) return crearPuenteWebView(tauri)
  const canal = window.chrome?.webview
  return typeof canal?.postMessage === 'function' ? crearPuenteWebView(canal) : crearPuenteSimulado()
}
