"""Builds the installer screens (760x460) as HTML; piezas.mjs renders them to images.

Usage: python generar.py <version>
"""
import re
import sys
from pathlib import Path

version = sys.argv[1]

aqui = Path(__file__).parent
salida = aqui / 'construido'
salida.mkdir(exist_ok=True)
mando = (aqui / 'mando.svg').read_text(encoding='utf-8')
mando = re.sub(r'<svg [^>]*>', '<svg class="dibujo" viewBox="0 0 1117 892" aria-hidden="true">', mando, count=1)
# The two light strips beside the touchpad are the lightbar.
for parte in ('border2', 'border3', 'touchpad'):
    mando = mando.replace(f'id="{parte}" d=', f'id="{parte}" class="luz" d=')
logo = (aqui / 'logo.svg').read_text(encoding='utf-8')
logo = re.sub(r'<\?xml[^>]*>', '', logo)
logo = re.sub(r'<svg ', '<svg class="logo" ', logo, count=1)

cabecera = f'''
<header>
  <span class="marca">{logo}DS5Control</span>
</header>'''


def pantalla(estado, izquierda, estado_id):
    return f'''
<section class="ventana {estado}" id="{estado_id}">
  <div class="izquierda">{cabecera}{izquierda}</div>
  <div class="derecha">{mando}<span class="pieza cerrar"><svg viewBox="0 0 12 12"><path d="M2 2l8 8M10 2l-8 8"/></svg></span></div>
</section>'''


bienvenida = pantalla('apagado', '''
  <div class="contenido">
    <h1>Tu DualSense,<br>listo para jugar.</h1>
    <p class="sub">Luz, vibración y gatillos adaptativos, y la app se abre desde el propio mando.</p>
    <p class="meta">Versión {version} · Solo para tu usuario, sin pedir permisos</p>
    <div class="carpeta">
      <span class="etiqueta">Se instala en</span>
      <span class="pieza ruta" data-pieza="ruta">C:\\Users\\julim\\AppData\\Local\\Programs\\DS5Control</span>
      <span class="pieza enlace" data-pieza="cambiar">Cambiar</span>
    </div>
  </div>
  <footer>
    <span class="pieza secundario" data-pieza="cancelar">Cancelar</span>
    <span class="pieza principal" data-pieza="instalar">Instalar</span>
  </footer>''', 'bienvenida')

instalando = pantalla('apagado', '''
  <div class="contenido">
    <h1>Instalando...</h1>
    <p class="sub">Tarda menos de un minuto. Si falta ViGEmBus, Windows te pedirá permiso para instalarlo.</p>
    <div class="progreso">
      <div class="linea"><span class="pieza" data-pieza="estado">Copiando archivos</span><span class="pieza cifra" data-pieza="cifra">62 %</span></div>
      <div class="pista" data-pieza="pista"><div class="pieza relleno" data-pieza="relleno" style="width: 62%"></div></div>
    </div>
  </div>
  <footer>
    <span class="pieza secundario" data-pieza="cancelar-instalando">Cancelar</span>
  </footer>''', 'instalando')

final = pantalla('encendido', '''
  <div class="contenido">
    <h1>Todo listo.</h1>
    <p class="sub">DS5Control ya está en el menú Inicio. Al abrirlo por primera vez te propondrá
      HidHide, para que ningún juego cuente cada botón dos veces.</p>
    <span class="pieza casilla" data-pieza="casilla"><span class="caja"><svg viewBox="0 0 12 12"><path d="M2.5 6.2l2.4 2.4 4.6-5"/></svg></span>Abrir DS5Control ahora</span>
  </div>
  <footer>
    <span class="pieza principal" data-pieza="terminar">Terminar</span>
  </footer>''', 'listo')

html = f'''<!doctype html>
<html lang="es"><head><meta charset="utf-8"><title>Instalador DS5Control</title>
<style>
@font-face {{ font-family: Figtree; src: url(../../../ui/node_modules/@fontsource-variable/figtree/files/figtree-latin-wght-normal.woff2) format('woff2'); font-weight: 300 900; }}
:root {{
  --ground: #101114; --panel: #15171b; --linea: rgba(255,255,255,.07); --ink: #f2f4f7;
  --muted: #a9b0bc; --guia: #6a7079; --accent: #7c9bff; --accent-tinta: #0b1230;
}}
* {{ box-sizing: border-box; margin: 0; }}
body {{ background: #2a2d33; font-family: Figtree, sans-serif; color: var(--ink);
  display: grid; gap: 40px; padding: 40px; justify-content: center;
  -webkit-font-smoothing: antialiased; }}
.ventana {{ width: 760px; height: 460px; display: grid; grid-template-columns: 440px 1fr;
  background: var(--ground); border-radius: 12px; overflow: hidden;
  box-shadow: 0 0 0 1px var(--linea), 0 30px 60px -20px rgba(0,0,0,.6); }}
.izquierda {{ display: flex; flex-direction: column; padding: 22px 36px 28px 40px; }}
header {{ display: flex; justify-content: space-between; align-items: center; }}
.marca {{ display: flex; align-items: center; gap: 10px; font-size: 14px; font-weight: 600; }}
.logo {{ width: 22px; height: 22px; }}
.derecha {{ position: relative; background: var(--panel); display: grid; place-items: center; }}
.derecha::before--sin-uso {{ content: ''; position: absolute; top: 22px; right: 22px; width: 12px; height: 12px;
  background: no-repeat center / 12px url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12'%3E%3Cpath d='M2 2l8 8M10 2l-8 8' stroke='%23a9b0bc' stroke-width='1.4' stroke-linecap='round'/%3E%3C/svg%3E"); }}
.dibujo {{ width: 296px; height: auto; transform: translateX(6px); }}
#border2, #border3 {{ stroke-width: 9px !important; stroke-linecap: round; }}
.encendido .luz {{ stroke: var(--accent) !important; filter: drop-shadow(0 0 14px var(--accent)) drop-shadow(0 0 4px var(--accent)); }}
.contenido {{ flex: 1; display: flex; flex-direction: column; justify-content: center; gap: 0; }}
h1 {{ font-size: 36px; line-height: 1.08; font-weight: 750; letter-spacing: -0.025em; }}
.sub {{ margin-top: 14px; font-size: 15px; line-height: 1.5; color: var(--muted); max-width: 34ch; }}
.meta {{ display: flex; flex-wrap: wrap; gap: 4px 14px; margin-top: 18px; font-size: 12.5px; color: var(--guia); }}
.meta span + span::before {{ content: ''; display: inline-block; width: 3px; height: 3px; border-radius: 50%;
  background: var(--guia); margin: 0 14px 3px -8px; }}
.carpeta {{ display: grid; grid-template-columns: 1fr auto; gap: 4px 12px; margin-top: 26px; padding-top: 18px;
  border-top: 1px solid var(--linea); }}
.etiqueta {{ grid-column: 1 / -1; font-size: 12px; color: var(--guia); }}
.ruta {{ font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}
.enlace {{ font-size: 13px; font-weight: 600; color: var(--accent); }}
footer {{ display: flex; justify-content: flex-end; align-items: center; gap: 8px; }}
.principal {{ display: inline-grid; place-items: center; min-width: 132px; height: 44px; padding: 0 24px;
  border-radius: 10px; background: var(--accent); color: var(--accent-tinta); font-size: 15px; font-weight: 700; }}
.secundario {{ display: inline-grid; place-items: center; height: 44px; padding: 0 18px; border-radius: 10px;
  color: var(--muted); font-size: 14px; font-weight: 600; }}
.progreso {{ margin-top: 30px; display: grid; gap: 10px; }}
.linea {{ display: flex; justify-content: space-between; font-size: 13px; color: var(--muted); }}
.cifra {{ color: var(--ink); font-variant-numeric: tabular-nums; font-weight: 600; }}
.pista {{ height: 6px; border-radius: 3px; background: #24272d; overflow: hidden; }}
.relleno {{ height: 100%; border-radius: 3px; background: var(--accent); box-shadow: 0 0 12px var(--accent); }}
.casilla {{ display: inline-flex; align-items: center; gap: 12px; margin-top: 26px; font-size: 14px; font-weight: 600; }}
.caja {{ display: grid; place-items: center; width: 22px; height: 22px; border-radius: 6px; background: var(--accent); }}
.caja svg {{ width: 12px; height: 12px; fill: none; stroke: var(--accent-tinta); stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }}
.cerrar {{ position: absolute; top: 14px; right: 14px; display: grid; place-items: center; width: 28px; height: 28px; border-radius: 8px; }}
.cerrar svg {{ width: 12px; height: 12px; fill: none; stroke: var(--muted); stroke-width: 1.4; stroke-linecap: round; }}
.hover.cerrar {{ background: rgba(255,255,255,.08); }}
.hover.cerrar svg {{ stroke: var(--ink); }}
.hover.principal {{ background: #93adff; }}
.hover.secundario {{ background: rgba(255,255,255,.06); color: var(--ink); }}
.hover.enlace {{ color: #a9bcff; text-decoration: underline; text-underline-offset: 3px; }}
.casilla.no .caja {{ background: transparent; box-shadow: inset 0 0 0 1.5px var(--guia); }}
.casilla.no .caja svg {{ visibility: hidden; }}
.enlace {{ padding: 2px 0; }}
.fondo .pieza {{ visibility: hidden !important; }}
.fondo .ventana {{ border-radius: 0; box-shadow: none; }}
</style></head><body>
{bienvenida}
{instalando}
{final}
</body></html>'''
html = html.replace('{version}', version)
(salida / 'index.html').write_text(html, encoding='utf-8')
print('OK')
