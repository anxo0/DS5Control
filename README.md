<p align="center">
  <img src="assets/logo/final/ds5control-fuego-128.png" alt="Logo de DS5Control" width="128" height="128">
</p>

<h1 align="center">DS5Control</h1>

<p align="center">Usa tu mando de PlayStation en Windows y ajusta su luz, vibración y gatillos en segundos.</p>

DS5Control es un fork de [DS4Windows](https://github.com/schmaldeo/DS4Windows) con una interfaz nueva, pensada para el DualSense y para manejarse con el propio mando.

## Qué hace

- **Emula un mando de Xbox 360 o DualShock 4** para que los juegos lo reconozcan.
- **Barra de luz:** colores, selector personalizado, brillo, modo arcoíris y color según batería.
- **Vibración y gatillos adaptativos:** ninguno, clic completo, rígido y pulso.
- **Mando en vivo:** el dibujo muestra botones, sticks y gatillos mientras los usas.
- **Botón de silencio:** ábrela y ocúltala desde el mando.
- **Tema claro y oscuro**, con cuatro colores de icono.

## Descarga

1. Descarga el ZIP de la [última versión](https://github.com/anxo0/DS5Control/releases/latest).
2. Descomprímelo en la carpeta que quieras.
3. Ejecuta `DS5Control.exe`.

## Requisitos

- Windows 10 u 11 de 64 bits.
- [.NET 8 Desktop Runtime x64](https://dotnet.microsoft.com/download/dotnet/8.0).
- [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases).
- Runtime de WebView2, incluido en Windows 11.
- Recomendado: [HidHide](https://github.com/nefarius/HidHide/releases), para que los juegos no reciban el mando dos veces.

## Estado

Esta es una versión temprana, probada con un DualSense por Bluetooth. Botones, Perfiles y Registro aún no están disponibles en la interfaz nueva. La interfaz anterior sigue disponible con `DS5Control.exe -uiantigua`.

Algunos sistemas antitrampas pueden reaccionar a los mandos emulados. Úsalo con precaución en juegos en línea.

## Compilar

```bash
cd ui
npm install
npm run build
cd ..
dotnet build DS4WindowsWPF.sln -c Debug -p:Platform=x64
```

## Créditos

- Basado en DS4Windows, de Jays2Kings, Ryochan7, schmaldeo y sus colaboradores.
- Dibujos del mando: [dualsense-tester](https://github.com/daidr/dualsense-tester), de daidr (MIT).
- Iconos de botones: [PS5 Button Icons and Controls](https://zacksly.itch.io/ps5-button-icons-and-controls), de Zacksly (CC BY 3.0).

Consulta [ui/THIRD_PARTY.md](ui/THIRD_PARTY.md) y [contributors.txt](contributors.txt).

## Licencia

GNU General Public License v3.0. Consulta [COPYING](COPYING).

Proyecto independiente, sin relación con Sony, Microsoft ni los autores de DS4Windows. PlayStation, DualSense y DualShock son marcas de Sony Interactive Entertainment. Windows y Xbox son marcas de Microsoft.
