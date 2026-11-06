# Plan del fork de DS4Windows

Estado: planificación. Aún no hay fork ni código. Base: `schmaldeo/DS4Windows` v3.9.9 (GPL-3.0, archivado).

## Decisiones tomadas

- Motor: se conserva el C# / .NET 8 de DS4Windows.
- Interfaz: nueva, web, dentro de WebView2. Propuesta de framework: Vue 3 + Vite + TypeScript, porque los componentes de los mandos de dualsense-tester ya están en Vue.
- Diseño: layout A con los colores de C. Modo claro por defecto y modo oscuro.
- Control de nivel propio, sin sliders por defecto: barra de 20 segmentos crecientes sobre un `input range` real.
- Prioridad de la pantalla principal: estado y batería, luz, vibración, gatillos. Botones y perfiles en segundo nivel.
- Función propia: el botón de mute abre la app y se maneja con el mando. Se activa y desactiva en Ajustes.
- Mando de referencia: DualSense estándar (PID 0x0CE6) por Bluetooth. DualShock 3 como prueba secundaria.

Diseño: https://claude.ai/artifact/KoogTzSkb5sZuVYE4QZX6i

## Fases

### 0. Base
- Crear repositorio propio y comprobar que compila y funciona con tu mando.
- Elegir nombre. Mantener licencia, avisos y créditos.
- Cambiar las URL del actualizador, que apuntan a schmaldeo: `DS4Control/ScpUtil.cs:3295`, `DS4Forms/MainWindow.xaml.cs:277` y `:339`, `DS4Forms/ViewModels/MainWindowsViewModel.cs:51`, `:76` y `:107`.

### 1. Estabilidad
Antes de tocar la interfaz, porque son fallos del motor:
- #57: `Convert.ToInt32` desborda en `Mapping.CalculateControllerAngle` y tira la app. Validar el valor antes de convertir.
- #52: `NullReferenceException` en `DS4Device.performDs4Input`, en el hilo de lectura.
- #65: cierres aleatorios por Bluetooth desde 3.9.2, cuando se pasó a la API nativa de Win32. Es el error más grave y puede afectarte: DS4 por Bluetooth en los informes.
- #101: la app desaparece sin registrar nada útil.
- #104: cierre al editar acciones especiales. Se resuelve si esa pantalla se reescribe.
- Añadir: una excepción en el hilo de entrada no debe cerrar la app, solo reconectar ese mando, y debe quedar registrada.

### 2. Separar motor e interfaz
- `DS4Control` no debería conocer la interfaz. Hoy dependen de `DS4Forms`: `ControlService.cs`, `Mapping.cs`, `ScpUtil.cs`, `ProfileDTO.cs` y `WindowPlacementHelper.cs`.
- Crear una capa de servicio única: estado de mandos, eventos y comandos.
- Pruebas: ahora solo hay 7 archivos de test. Cubrir lo que se toque.

### 3. Puente con WebView2
- Host mínimo: ventana, icono en la bandeja y arranque en segundo plano.
- Mensajes JSON entre C# y web: estado, batería, comandos.
- La vista recibe el estado a frecuencia limitada: el bucle de entrada nunca espera a la interfaz.

### 4. Pantalla Mando
- Layout A, temas claro y oscuro, SVG de DualSense y DualShock 4 interactivos.
- Luz, brillo, color según batería, vibración, y efectos de gatillo: ninguno, clic completo, rígido y pulso.
- Probar cada ajuste en el mando real.

### 5. Control con el mando y botón de mute
- `DualSenseDevice` ya lee `cState.Mute`. Falta conectar la pulsación a abrir y ocultar la ventana.
- Con la app abierta, la entrada navega por la interfaz y no llega al juego.
- Opción en Ajustes, y foco visible en todos los controles.

### 6. Resto de pantallas
- Botones, Perfiles, Automático, Macros, Registro y Ajustes, sin perder funciones.
- Retirar la interfaz WPF cuando todo esté cubierto.

### 7. Publicación
- Instalador, comprobación de drivers y actualizador propio.
- Créditos de dualsense-tester (MIT) y código fuente disponible (GPL-3.0).

## Mejoras candidatas

Pedidas en issues abiertas, sin fase asignada:
- #91 hápticos del DualSense por Bluetooth.
- #100 gestión integrada de HidHide.
- #73 y #88 añadir mandos compatibles desde un archivo.
- #51 comportamiento al cerrar y minimizar.
- PR abiertos que se pueden aprovechar: #102 (panel táctil en Passthru), #89 (dispositivos personalizados), #86 y #97 (más mandos).

## Riesgos

- ViGEmBus está archivado desde 2023. #85 es un pantallazo azul `WDF_VIOLATION` al salir de hibernación atribuido a ese driver. No se puede arreglar desde aquí sin otro driver.
- Antitrampas: #79 informa de baneos en Apex y FC25 usando emulación. Sin confirmar la causa. Avisar en la documentación.
- DualShock 3 necesita DsHidMini o Sixaxis, y BthPS3 por Bluetooth. No tiene barra de luz ni gatillos adaptativos.
- Bluetooth y mando reales: los puede probar solo el mantenedor, cada fase necesita esa prueba.
- Marcas: los dibujos de los mandos reproducen productos de Sony. Revisarlo antes de una publicación abierta.
