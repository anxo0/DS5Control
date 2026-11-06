# Banco Mando

Juego sencillo de Godot 4 para comprobar el mando con la app en marcha. Godot lee el mando virtual, igual que un juego real, así que prueba toda la cadena.

## Cómo usarlo

1. Abre la app y conecta el DualSense.
2. Abre esta carpeta en Godot y pulsa F5.
3. Juega y completa la lista del panel derecho.
4. Cierra el juego o pulsa F12. El informe queda en `%APPDATA%\Godot\app_userdata\Banco Mando\informe.txt`.

## Qué comprueba

- Todos los botones, sticks y gatillos, con su valor exacto.
- Los dos motores de vibración, y los eventos por segundo.
- Si Godot ve más de un mando: entonces el juego recibe entrada doble y hay que ocultar el DualSense físico con HidHide.

El juego no puede medir la luz, los gatillos adaptativos ni el botón de mute, porque no llegan por el mando virtual. Se confirman a mano con las teclas 1 a 5.

Los motores se prueban con Cuadrado y Círculo. PS puede no llegar si Windows u otra app lo capturan.
