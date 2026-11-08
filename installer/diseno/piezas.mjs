// Renders every installer piece at each Windows scale through Edge's DevTools protocol.
import { spawn } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const [edge, carpeta] = process.argv.slice(2)
const salida = join(carpeta, 'piezas')
mkdirSync(salida, { recursive: true })
const url = 'file:///' + carpeta.replaceAll('\\', '/') + '/index.html'
const perfil = join(carpeta, 'perfil-edge')
const proceso = spawn(edge, ['--headless=new', '--disable-gpu', '--hide-scrollbars', '--remote-debugging-port=9339',
  `--user-data-dir=${perfil}`, 'about:blank'], { stdio: 'ignore' })

const espera = (ms) => new Promise((r) => setTimeout(r, ms))
let objetivos
for (let i = 0; i < 40 && !objetivos; i++) {
  await espera(250)
  try { objetivos = await (await fetch('http://127.0.0.1:9339/json')).json() } catch {}
}
const pagina = objetivos.find((t) => t.type === 'page')
const ws = new WebSocket(pagina.webSocketDebuggerUrl)
await new Promise((r) => (ws.onopen = r))
let id = 0
const pendientes = new Map()
ws.onmessage = (e) => {
  const m = JSON.parse(e.data)
  if (pendientes.has(m.id)) { pendientes.get(m.id)(m.result); pendientes.delete(m.id) }
}
const cdp = (method, params = {}) => new Promise((r) => { const n = ++id; pendientes.set(n, r); ws.send(JSON.stringify({ id: n, method, params })) })
const evaluar = async (expression) => (await cdp('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true })).result.value

const captura = async (nombre, clip) => {
  const r = await cdp('Page.captureScreenshot', { format: 'png', clip: { ...clip, scale: 1 }, captureBeyondViewport: true })
  writeFileSync(join(salida, nombre + '.png'), Buffer.from(r.data, 'base64'))
}

await cdp('Page.enable')
const posiciones = {}
for (const escala of [100, 125, 150, 200]) {
  await cdp('Emulation.setDeviceMetricsOverride', { width: 840, height: 1660, deviceScaleFactor: escala / 100, mobile: false })
  await cdp('Page.navigate', { url })
  await espera(900)
  await evaluar('document.fonts.ready.then(() => true)')

  const ventanas = await evaluar(`[...document.querySelectorAll('.ventana')].map(v => { const r = v.getBoundingClientRect(); return { id: v.id, x: r.x, y: r.y, w: r.width, h: r.height } })`)
  // Pieces, with their position inside their window in CSS pixels.
  const piezas = await evaluar(`[...document.querySelectorAll('[data-pieza], .cerrar')].map(e => {
    const v = e.closest('.ventana').getBoundingClientRect(); const r = e.getBoundingClientRect()
    return { nombre: (e.dataset.pieza || 'cerrar') + (e.classList.contains('cerrar') ? '-' + e.closest('.ventana').id : ''),
      x: r.x, y: r.y, w: r.width, h: r.height, rx: r.x - v.x, ry: r.y - v.y } })`)

  for (const p of piezas) {
    const clave = p.nombre.startsWith('cerrar') ? 'cerrar' : p.nombre
    if (escala === 100 && !posiciones[clave]) posiciones[clave] = { x: Math.round(p.rx), y: Math.round(p.ry), w: Math.round(p.w), h: Math.round(p.h) }
  }
  const clip = (p) => ({ x: p.x, y: p.y, width: p.w, height: p.h })
  for (const nombre of ['instalar', 'cancelar', 'terminar', 'cambiar']) {
    const p = piezas.find((q) => q.nombre === nombre)
    await captura(`${nombre}-${escala}`, clip(p))
    await evaluar(`document.querySelector('[data-pieza="${nombre}"]').classList.add('hover')`)
    await espera(50)
    await captura(`${nombre}-hover-${escala}`, clip(p))
    await evaluar(`document.querySelector('[data-pieza="${nombre}"]').classList.remove('hover')`)
  }
  const cerrar = piezas.find((q) => q.nombre === 'cerrar-bienvenida')
  await captura(`cerrar-${escala}`, clip(cerrar))
  await evaluar(`document.querySelector('#bienvenida .cerrar').classList.add('hover')`)
  await espera(50)
  await captura(`cerrar-hover-${escala}`, clip(cerrar))
  const casilla = piezas.find((q) => q.nombre === 'casilla')
  await captura(`casilla-si-${escala}`, clip(casilla))
  await evaluar(`document.querySelector('.casilla').classList.add('no')`)
  await espera(50)
  await captura(`casilla-no-${escala}`, clip(casilla))
  // The full-width fill of the progress bar; the installer reveals as much as it needs.
  await evaluar(`document.querySelector('.relleno').style.width = '100%'`)
  await espera(50)
  await captura(`relleno-${escala}`, clip(piezas.find((q) => q.nombre === 'relleno' ) && { ...piezas.find((q) => q.nombre === 'pista') }))

  await evaluar(`document.body.classList.add('fondo')`)
  await espera(80)
  for (const v of ventanas) await captura(`fondo-${v.id}-${escala}`, { x: v.x, y: v.y, width: v.w, height: v.h })
}
writeFileSync(join(salida, 'posiciones.json'), JSON.stringify(posiciones, null, 1))
ws.close()
proceso.kill()
console.log('piezas listas', Object.keys(posiciones).join(', '))
