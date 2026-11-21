# Estado de la mudanza a Rust

`estado.py` dibuja `estado.svg`, un mapa de cuadros con lo que ya está mudado de C# (`DS5Control/`) a Rust.

- Arriba, insignias: el porcentaje mudado y los `dato` de `mudanza.txt`.
- Debajo, un bloque por pieza (motor de mapeo, perfiles y ajustes, mando físico, mando virtual y HidHide, la app). Dentro, un rectángulo por archivo `.cs`, de tamaño proporcional a sus líneas y relleno de cuadraditos: verdes los mudados, grises los pendientes. Al pasar el ratón sale la ruta, las líneas y el porcentaje.
- Al final, en gris apagado, lo que no se muda (no cuenta en el porcentaje).

El porcentaje de cada pieza y el global es la media de los porcentajes de sus archivos ponderada por líneas.

## Cómo se ejecuta

```
python tools/estado-rust/estado.py
```

Solo necesita Python 3, sin dependencias. Escribe `tools/estado-rust/estado.svg` (o la ruta que se le pase como argumento) y muestra un resumen por pieza en la consola.

## Cómo se actualiza `mudanza.txt`

Cada línea es `<patrón> <pieza> <estado>`:

- `patrón`: ruta dentro de `DS5Control/` con barras `/`; `*` vale como comodín (también cruza carpetas). Gana la primera línea que encaje, así que lo concreto va antes que lo general.
- `pieza`: `mapeo`, `perfiles`, `mando`, `virtual`, `app` o `fuera`.
- `estado`: `hecho` (100), `pendiente` (0) o un número de 0 a 100.

Las líneas `dato <nombre> <valor>` son insignias (`dato fase 5 de 9`). `dato estados a+b+c` suma los números y los muestra como estados iguales que C#. Las líneas vacías y las que empiezan por `#` no cuentan.

Cuando una prueba de equivalencia confirma que un archivo ya funciona igual en Rust, se sube su porcentaje (o se pone `hecho`) y se vuelve a ejecutar el script.
