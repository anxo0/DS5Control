# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

La interfaz es web y se muestra en WebView2 dentro de una app de escritorio de Windows 11. El motor es el código C# de DS4Windows (.NET 8).

## Stack

C# / .NET 8 para el motor, heredado del fork. Interfaz web en WebView2. Framework web sin decidir.

## Users

El mantenedor y un grupo de amigos, jugadores de PC con mandos de PlayStation. A futuro podría abrirse a más gente, sin compromiso de soporte. Mando de referencia: DualSense estándar por Bluetooth. Segundo mando posible: DualShock 3.

## Product Purpose

Fork de DS4Windows con la interfaz rehecha por completo. Permite usar un mando de PlayStation en Windows y ajustar en segundos lo que importa: luz, batería, vibración y gatillos adaptativos.

## Positioning

La app se abre desde el propio mando: el botón de mute del DualSense la trae al frente y se maneja entera con el mando, sin tocar teclado ni ratón.

## Operating Context

Se configura la primera vez. Después solo se abre al conectar el mando, se minimiza y queda en segundo plano en la bandeja del sistema. Las visitas son breves, normalmente justo antes de jugar. La app debe seguir siendo cómoda también con ratón.

## Capabilities and Constraints

- Prioridad de uso: 1) estado del mando y batería, 2) color de la barra de luz, 3) vibración, 4) gatillos adaptativos. Remapeo como función secundaria. Los perfiles no los usa el mantenedor: existen, pero no ocupan la pantalla principal.
- Abrir con el botón de mute es una opción de Ajustes que se activa y desactiva.
- Hay que conservar las funciones existentes de DS4Windows, aunque queden en segundo nivel.
- Efectos de gatillo disponibles hoy en el código: ninguno, clic completo, rígido y pulso.
- DualShock 3 necesita los drivers DsHidMini o Sixaxis, y BthPS3 por Bluetooth. No tiene barra de luz ni gatillos adaptativos.
- Dependencias de terceros: ViGEmBus (archivado), HidHide, FakerInput. Licencia: GPL-3.0, heredada.
- Nombre del producto: sin decidir. No debe confundirse con DS4Windows.

## Evidence on Hand

- SVG de los mandos de `daidr/dualsense-tester` (MIT), con atribución obligatoria.
- No hay marca, logotipo, usuarios ni métricas propias. No inventarlos.

## Product Principles

- Lo que importa antes de jugar se ve al abrir, sin navegar.
- Todo se puede manejar con el mando; el ratón es un extra, no un requisito.
- Probar en el mando real al momento vale más que describir un ajuste.
- La app molesta lo mínimo: aparece, cumple y se va.
- Las funciones avanzadas siguen existiendo, pero no compiten con las diarias.

## Accessibility & Inclusion

Foco siempre visible, navegable con mando y con teclado. Texto legible a distancia cuando se maneje desde el sofá.
