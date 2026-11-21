#!/usr/bin/env python3
"""Dibuja el mapa de cuadros (treemap) con el estado de la mudanza de DS5Control de C# a Rust.

Lee los *.cs de DS5Control/ (sin obj ni bin), cuenta sus líneas, les asigna pieza y porcentaje
según tools/estado-rust/mudanza.txt y escribe un SVG con insignias arriba y un treemap por pieza.

Uso:
    python tools/estado-rust/estado.py [salida.svg]

Solo usa la biblioteca estándar de Python 3.
"""

import fnmatch
import os
import subprocess
import sys
from xml.sax.saxutils import escape

AQUI = os.path.dirname(os.path.abspath(__file__))
RAIZ = os.path.normpath(os.path.join(AQUI, "..", ".."))
CARPETA_CS = os.path.join(RAIZ, "DS5Control")
LISTA = os.path.join(AQUI, "mudanza.txt")
SALIDA = os.path.join(AQUI, "estado.svg")

PIEZAS = [
    ("mapeo", "Motor de mapeo"),
    ("perfiles", "Perfiles y ajustes"),
    ("mando", "Mando físico"),
    ("virtual", "Mando virtual y HidHide"),
    ("app", "La app"),
    ("fuera", "No se muda"),
]
NOMBRE_PIEZA = dict(PIEZAS)

# Colores (fondo oscuro tipo GitHub)
FONDO = "#0d1117"
BORDE = "#30363d"
TEXTO = "#e6edf3"
TEXTO_SUAVE = "#8b949e"
VERDE = "#2ea043"
GRIS = "#4b5563"
GRIS_APAGADO = "#2d333b"
ETIQUETA = "#161b22"

ANCHO = 1000
MARGEN = 16
CUADRO = 6      # lado de cada cuadradito
HUECO = 1       # separación entre cuadraditos
PASO = CUADRO + HUECO


# ---------------------------------------------------------------- formato español

def miles(n):
    """Entero con punto de millar: 11513672 -> 11.513.672."""
    return f"{int(n):,}".replace(",", ".")


def decimal(x, cifras=1):
    """Número con coma decimal: 95.1 -> 95,1."""
    return f"{x:.{cifras}f}".replace(".", ",")


# ---------------------------------------------------------------- datos

def leer_lista(ruta):
    """Devuelve (reglas, datos). reglas: [(patrón, pieza, porcentaje)]; datos: [(nombre, valor)]."""
    reglas, datos = [], []
    with open(ruta, encoding="utf-8") as f:
        for num, linea in enumerate(f, 1):
            linea = linea.strip()
            if not linea or linea.startswith("#"):
                continue
            partes = linea.split()
            if partes[0] == "dato":
                if len(partes) < 3:
                    raise SystemExit(f"mudanza.txt:{num}: falta el valor del dato")
                nombre, valor = partes[1], " ".join(partes[2:])
                if nombre == "estados":
                    total = sum(int(t) for t in valor.split("+"))
                    valor = f"{miles(total)} estados iguales que C#"
                datos.append((nombre, valor))
                continue
            if len(partes) != 3:
                raise SystemExit(f"mudanza.txt:{num}: se esperaba '<patrón> <pieza> <estado>'")
            patron, pieza, estado = partes
            if pieza not in NOMBRE_PIEZA:
                raise SystemExit(f"mudanza.txt:{num}: pieza desconocida '{pieza}'")
            if estado == "hecho":
                pct = 100.0
            elif estado == "pendiente":
                pct = 0.0
            else:
                try:
                    pct = float(estado.replace(",", "."))
                except ValueError:
                    raise SystemExit(f"mudanza.txt:{num}: estado no válido '{estado}'")
                if not 0 <= pct <= 100:
                    raise SystemExit(f"mudanza.txt:{num}: el porcentaje debe ir de 0 a 100")
            reglas.append((patron, pieza, pct))
    return reglas, datos


def contar_lineas(ruta):
    with open(ruta, "rb") as f:
        contenido = f.read()
    if not contenido:
        return 0
    return contenido.count(b"\n") + (0 if contenido.endswith(b"\n") else 1)


def archivos_de_git(carpeta):
    """Rutas (relativas a `carpeta`) que git sigue, o None si no se puede preguntar a git."""
    try:
        salida = subprocess.run(["git", "ls-files", "-z", "--", "."], cwd=carpeta,
                                capture_output=True, check=True).stdout
    except (OSError, subprocess.CalledProcessError):
        return None
    return {r.decode("utf-8") for r in salida.split(bytes(1)) if r}


def leer_archivos(carpeta, reglas):
    """Lista de dicts {ruta, nombre, lineas, pieza, pct}."""
    archivos = []
    publicados = archivos_de_git(carpeta)
    for base, dirs, ficheros in os.walk(carpeta):
        dirs[:] = sorted(d for d in dirs if d.lower() not in ("obj", "bin"))
        for nombre in sorted(ficheros):
            if not nombre.endswith(".cs"):
                continue
            completa = os.path.join(base, nombre)
            rel = os.path.relpath(completa, carpeta).replace(os.sep, "/")
            # Lo que no está en git (pruebas locales, partes sin publicar) no sale en el mapa.
            if publicados is not None and rel not in publicados:
                continue
            pieza, pct = "app", 0.0
            for patron, p, v in reglas:
                if fnmatch.fnmatchcase(rel, patron):
                    pieza, pct = p, v
                    break
            lineas = contar_lineas(completa)
            if lineas == 0:
                continue
            archivos.append({
                "ruta": rel,
                "nombre": nombre[:-3],
                "lineas": lineas,
                "pieza": pieza,
                "pct": pct,
            })
    return archivos


# ---------------------------------------------------------------- treemap squarified

def _peor(fila, lado):
    s = sum(fila)
    return max(max(lado * lado * r / (s * s), (s * s) / (lado * lado * r)) for r in fila)


def squarify(valores, x, y, w, h):
    """Reparte el rectángulo (x, y, w, h) en rectángulos de área proporcional a valores.
    Devuelve una lista de (x, y, w, h) en el mismo orden que valores (que deben ir de mayor a menor)."""
    total = sum(valores)
    if total <= 0 or w <= 0 or h <= 0:
        return [(x, y, 0, 0) for _ in valores]
    escala = w * h / total
    areas = [v * escala for v in valores]
    rects = []
    i = 0
    while i < len(areas):
        lado = min(w, h)
        fila = [areas[i]]
        i += 1
        while i < len(areas) and _peor(fila + [areas[i]], lado) <= _peor(fila, lado):
            fila.append(areas[i])
            i += 1
        s = sum(fila)
        if w >= h:
            # columna a la izquierda
            ancho = s / h if h else 0
            yy = y
            for a in fila:
                alto = a / ancho if ancho else 0
                rects.append((x, yy, ancho, alto))
                yy += alto
            x += ancho
            w -= ancho
        else:
            # fila arriba
            alto = s / w if w else 0
            xx = x
            for a in fila:
                ancho = a / alto if alto else 0
                rects.append((xx, y, ancho, alto))
                xx += ancho
            y += alto
            h -= alto
    return rects


# ---------------------------------------------------------------- dibujo

def ancho_texto(texto, px=11):
    return len(texto) * px * 0.62


def insignia(x, y, izquierda, derecha, color):
    """Insignia tipo shields.io. Devuelve (svg, ancho)."""
    a1 = ancho_texto(izquierda) + 12
    a2 = ancho_texto(derecha) + 12
    alto = 20
    partes = [
        f'<g transform="translate({x:.0f},{y:.0f})">',
        f'<rect width="{a1 + a2:.1f}" height="{alto}" rx="3" fill="{color}"/>',
        f'<rect width="{a1:.1f}" height="{alto}" rx="3" fill="#555"/>',
        f'<rect x="{a1 - 4:.1f}" width="4" height="{alto}" fill="#555"/>',
        f'<text x="{a1 / 2:.1f}" y="14" fill="#fff" text-anchor="middle" '
        f'font-size="11">{escape(izquierda)}</text>',
        f'<text x="{a1 + a2 / 2:.1f}" y="14" fill="#fff" text-anchor="middle" '
        f'font-size="11">{escape(derecha)}</text>',
        "</g>",
    ]
    return "".join(partes), a1 + a2


def recortar(texto, ancho_max, px=10):
    if ancho_texto(texto, px) <= ancho_max:
        return texto
    caben = int(ancho_max / (px * 0.62)) - 1
    if caben < 2:
        return None
    return texto[:caben] + "…"


def dibujar_archivo(a, x, y, w, h, color_hecho, color_pendiente):
    """Rectángulo de un archivo relleno de cuadraditos."""
    salida = ['<g>']
    salida.append(
        f'<title>{escape(a["ruta"])}: {miles(a["lineas"])} líneas, '
        f'{decimal(a["pct"])} % mudado</title>'
    )
    salida.append(
        f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" '
        f'fill="{FONDO}" stroke="{BORDE}" stroke-width="1"/>'
    )
    columnas = int((w - 2 + HUECO) // PASO)
    filas = int((h - 2 + HUECO) // PASO)
    n = max(columnas, 0) * max(filas, 0)
    if n > 0:
        verdes = round(n * a["pct"] / 100)
        # centra la rejilla en el rectángulo
        x0 = x + (w - (columnas * PASO - HUECO)) / 2
        y0 = y + (h - (filas * PASO - HUECO)) / 2
        for color, desde, hasta in ((color_hecho, 0, verdes), (color_pendiente, verdes, n)):
            if desde >= hasta:
                continue
            caminos = []
            for k in range(desde, hasta):
                fila, col = divmod(k, columnas)
                caminos.append(
                    f"M{x0 + col * PASO:.1f} {y0 + fila * PASO:.1f}h{CUADRO}v{CUADRO}h-{CUADRO}z"
                )
            salida.append(f'<path fill="{color}" d="{"".join(caminos)}"/>')
    # etiqueta con el nombre
    if w >= 28 and h >= 18:
        texto = recortar(a["nombre"], w - 10)
        if texto:
            ancho = ancho_texto(texto, 10) + 8
            salida.append(
                f'<rect x="{x + 1:.1f}" y="{y + 1:.1f}" width="{min(ancho, w - 2):.1f}" '
                f'height="14" rx="2" fill="{ETIQUETA}" fill-opacity="0.92"/>'
            )
            salida.append(
                f'<text x="{x + 5:.1f}" y="{y + 11.5:.1f}" fill="{TEXTO}" '
                f'font-size="10">{escape(texto)}</text>'
            )
    salida.append("</g>")
    return "".join(salida)


def dibujar_archivos(lista, x, y, w, h, color_hecho, color_pendiente):
    lista = sorted(lista, key=lambda a: -a["lineas"])
    rects = squarify([a["lineas"] for a in lista], x, y, w, h)
    return "".join(
        dibujar_archivo(a, *r, color_hecho, color_pendiente)
        for a, r in zip(lista, rects) if r[2] > 0 and r[3] > 0
    )


def resumen_pieza(lista):
    lineas = sum(a["lineas"] for a in lista)
    hechas = sum(a["lineas"] * a["pct"] / 100 for a in lista)
    pct = 100 * hechas / lineas if lineas else 0.0
    return lineas, hechas, pct


def generar(archivos, datos):
    por_pieza = {clave: [a for a in archivos if a["pieza"] == clave] for clave, _ in PIEZAS}
    cuentan = [a for a in archivos if a["pieza"] != "fuera"]
    lineas_tot, hechas_tot, pct_global = resumen_pieza(cuentan)

    svg = []
    y = MARGEN

    # insignias
    x = MARGEN
    color = "#dfb317" if pct_global < 50 else "#4c1"
    trozo, ancho = insignia(x, y, "mudado", f"{decimal(pct_global)} %", color)
    svg.append(trozo)
    x += ancho + 6
    for nombre, valor in datos:
        trozo, ancho = insignia(x, y, nombre, valor, "#007ec6")
        if x + ancho > ANCHO - MARGEN and x > MARGEN:
            x = MARGEN
            y += 26
            trozo, ancho = insignia(x, y, nombre, valor, "#007ec6")
        svg.append(trozo)
        x += ancho + 6
    y += 20 + 14

    # treemap principal de piezas
    ancho_util = ANCHO - 2 * MARGEN
    # unas 0,2 celdas por línea de código, entre 420 y 900 px de alto
    alto_principal = round(lineas_tot * 0.2 * PASO * PASO / ancho_util)
    alto_principal = min(max(alto_principal, 420), 900)
    piezas = [(c, por_pieza[c]) for c, _ in PIEZAS if c != "fuera" and por_pieza[c]]
    piezas.sort(key=lambda p: -sum(a["lineas"] for a in p[1]))
    rects = squarify([sum(a["lineas"] for a in l) for _, l in piezas],
                     MARGEN, y, ancho_util, alto_principal)
    resumenes = []
    for (clave, lista), (px, py, pw, ph) in zip(piezas, rects):
        lineas, hechas, pct = resumen_pieza(lista)
        resumenes.append((clave, lineas, hechas, pct))
        titulo = (f"{NOMBRE_PIEZA[clave]}: {decimal(pct)} % "
                  f"({miles(round(hechas))}/{miles(lineas)} líneas)")
        pad = 3
        svg.append(
            f'<rect x="{px + 1:.1f}" y="{py + 1:.1f}" width="{pw - 2:.1f}" height="{ph - 2:.1f}" '
            f'rx="4" fill="{FONDO}" stroke="{BORDE}"/>'
        )
        texto = recortar(titulo, pw - 12, 12) or ""
        svg.append(
            f'<text x="{px + 7:.1f}" y="{py + 16:.1f}" fill="{TEXTO}" font-size="12" '
            f'font-weight="600"><title>{escape(titulo)}</title>{escape(texto)}</text>'
        )
        svg.append(dibujar_archivos(lista, px + pad + 1, py + 22, pw - 2 * pad - 2,
                                    ph - 22 - pad - 1, VERDE, GRIS))
    y += alto_principal + 18

    # franja de lo que no se muda
    fuera = por_pieza["fuera"]
    lineas_fuera = sum(a["lineas"] for a in fuera)
    if fuera:
        svg.append(
            f'<text x="{MARGEN}" y="{y + 12}" fill="{TEXTO_SUAVE}" font-size="12" '
            f'font-weight="600">No se muda (no cuenta en el porcentaje): '
            f'{miles(lineas_fuera)} líneas</text>'
        )
        y += 20
        alto_fuera = 140
        svg.append(dibujar_archivos(fuera, MARGEN, y, ancho_util, alto_fuera,
                                    GRIS_APAGADO, GRIS_APAGADO))
        y += alto_fuera
    y += MARGEN

    cabecera = (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{ANCHO}" height="{y:.0f}" '
        f'viewBox="0 0 {ANCHO} {y:.0f}" font-family="Verdana,DejaVu Sans,sans-serif">'
        f'<title>Estado de la mudanza a Rust: {decimal(pct_global)} %</title>'
        f'<rect width="100%" height="100%" fill="{FONDO}"/>'
    )
    return cabecera + "".join(svg) + "</svg>\n", resumenes, (lineas_tot, hechas_tot, pct_global), lineas_fuera


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except AttributeError:
        pass
    salida = sys.argv[1] if len(sys.argv) > 1 else SALIDA
    reglas, datos = leer_lista(LISTA)
    archivos = leer_archivos(CARPETA_CS, reglas)
    texto, resumenes, (lineas, hechas, pct), lineas_fuera = generar(archivos, datos)
    with open(salida, "w", encoding="utf-8", newline="\n") as f:
        f.write(texto)

    print("Estado de la mudanza a Rust")
    for clave, l, h, p in resumenes:
        print(f"  {NOMBRE_PIEZA[clave]:<26} {decimal(p):>6} %  ({miles(round(h))}/{miles(l)} líneas)")
    print(f"  {'Total':<26} {decimal(pct):>6} %  ({miles(round(hechas))}/{miles(lineas)} líneas)")
    print(f"  {NOMBRE_PIEZA['fuera']:<26} {miles(lineas_fuera)} líneas (no cuentan)")
    print(f"SVG escrito en {salida}")


if __name__ == "__main__":
    main()
