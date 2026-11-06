extends Node2D
# Banco de pruebas: un juego sencillo y una lista de comprobacion del mando.
# Godot lee el mando virtual (Xbox 360), igual que lo haria un juego real.

const FONDO := Color("101114")
const PANEL := Color("191B1F")
const LINEA := Color("2C2F35")
const TINTA := Color("F2F4F7")
const SUAVE := Color("A9B0BC")
const ACENTO := Color("7C9BFF")
const BIEN := Color("7ED321")
const MAL := Color("FF6B5E")
const AVISO := Color("FFC400")

const ARENA := Rect2(20, 64, 760, 636)
const PANEL_X := 800.0
const INFORME := "user://informe.txt"

const BOTONES := [
	[JOY_BUTTON_A, "Cruz"], [JOY_BUTTON_B, "Circulo"],
	[JOY_BUTTON_X, "Cuadrado"], [JOY_BUTTON_Y, "Triangulo"],
	[JOY_BUTTON_LEFT_SHOULDER, "L1"], [JOY_BUTTON_RIGHT_SHOULDER, "R1"],
	[JOY_BUTTON_LEFT_STICK, "L3"], [JOY_BUTTON_RIGHT_STICK, "R3"],
	[JOY_BUTTON_BACK, "Share"], [JOY_BUTTON_START, "Options"],
	[JOY_BUTTON_GUIDE, "PS"],
	[JOY_BUTTON_DPAD_UP, "Arriba"], [JOY_BUTTON_DPAD_DOWN, "Abajo"],
	[JOY_BUTTON_DPAD_LEFT, "Izquierda"], [JOY_BUTTON_DPAD_RIGHT, "Derecha"],
]

# Lo que el juego no puede medir: lo confirmas tu con el teclado.
const MANUALES := [
	[KEY_1, "La luz cambia al elegir color"],
	[KEY_2, "Gatillo rigido se nota"],
	[KEY_3, "Gatillo pulso se nota"],
	[KEY_4, "Mute abre y oculta la app"],
	[KEY_5, "Mute no afecta al juego"],
]

var fuente: Font
var dispositivo := -1
var botones_vistos := {}
var manuales_ok := {}
var max_l2 := 0.0
var max_r2 := 0.0
var sectores_izq := {}
var sectores_der := {}
var vibracion_debil := false
var vibracion_fuerte := false
var eventos := 0
var eventos_seg := 0
var reloj_eventos := 0.0
var mensaje := ""
var reloj_mensaje := 0.0

var jugador := Vector2.ZERO
var apunte := Vector2.RIGHT
var reloj_disparo := 0.0
var reloj_impulso := 0.0
var reloj_golpe := 0.0
var reloj_enemigo := 1.0
var balas: Array[Dictionary] = []
var enemigos: Array[Dictionary] = []
var puntos := 0
var golpes := 0


func _ready() -> void:
	fuente = ThemeDB.fallback_font
	jugador = ARENA.get_center()
	Input.joy_connection_changed.connect(_al_cambiar_mandos)
	_elegir_dispositivo()


func _al_cambiar_mandos(_id: int, _conectado: bool) -> void:
	_elegir_dispositivo()


func _elegir_dispositivo() -> void:
	var lista := Input.get_connected_joypads()
	dispositivo = -1
	for id in lista:
		var nombre := Input.get_joy_name(id).to_lower()
		if nombre.contains("xbox") or nombre.contains("xinput"):
			dispositivo = id
			return
	if not lista.is_empty():
		dispositivo = lista[0]


func _input(evento: InputEvent) -> void:
	if evento is InputEventJoypadMotion or evento is InputEventJoypadButton:
		if evento.device == dispositivo:
			eventos += 1
	if evento is InputEventKey and evento.pressed and not evento.echo:
		for fila in MANUALES:
			if evento.keycode == fila[0]:
				manuales_ok[fila[0]] = not manuales_ok.get(fila[0], false)
		if evento.keycode == KEY_F12:
			_guardar_informe()


func _notification(aviso: int) -> void:
	if aviso == NOTIFICATION_WM_CLOSE_REQUEST:
		_guardar_informe()


func _process(delta: float) -> void:
	reloj_eventos += delta
	if reloj_eventos >= 1.0:
		eventos_seg = eventos
		eventos = 0
		reloj_eventos -= 1.0
	reloj_mensaje = max(reloj_mensaje - delta, 0.0)
	if dispositivo >= 0:
		_medir()
		_jugar(delta)
	queue_redraw()


func _eje(eje: JoyAxis) -> float:
	return Input.get_joy_axis(dispositivo, eje)


func _stick_izq() -> Vector2:
	return Vector2(_eje(JOY_AXIS_LEFT_X), _eje(JOY_AXIS_LEFT_Y))


func _stick_der() -> Vector2:
	return Vector2(_eje(JOY_AXIS_RIGHT_X), _eje(JOY_AXIS_RIGHT_Y))


func _medir() -> void:
	for fila in BOTONES:
		if Input.is_joy_button_pressed(dispositivo, fila[0]):
			botones_vistos[fila[0]] = true
	max_l2 = max(max_l2, _eje(JOY_AXIS_TRIGGER_LEFT))
	max_r2 = max(max_r2, _eje(JOY_AXIS_TRIGGER_RIGHT))
	_marcar_sector(_stick_izq(), sectores_izq)
	_marcar_sector(_stick_der(), sectores_der)
	if Input.is_joy_button_pressed(dispositivo, JOY_BUTTON_X):
		Input.start_joy_vibration(dispositivo, 1.0, 0.0, 0.1)
		vibracion_debil = true
	if Input.is_joy_button_pressed(dispositivo, JOY_BUTTON_B):
		Input.start_joy_vibration(dispositivo, 0.0, 1.0, 0.1)
		vibracion_fuerte = true


func _marcar_sector(stick: Vector2, sectores: Dictionary) -> void:
	if stick.length() > 0.7:
		sectores[int(posmod(stick.angle(), TAU) / TAU * 8.0) % 8] = true


func _jugar(delta: float) -> void:
	var izq := _stick_izq()
	var der := _stick_der()
	var r2 := _eje(JOY_AXIS_TRIGGER_RIGHT)
	var l2 := _eje(JOY_AXIS_TRIGGER_LEFT)
	var ritmo: float = 1.0 - l2 * 0.7

	reloj_impulso = max(reloj_impulso - delta, 0.0)
	if Input.is_joy_button_pressed(dispositivo, JOY_BUTTON_A) and reloj_impulso <= 0.0:
		reloj_impulso = 0.6
		jugador += izq.limit_length(1.0) * 110.0
		Input.start_joy_vibration(dispositivo, 0.3, 0.6, 0.12)

	jugador += izq.limit_length(1.0) * 420.0 * delta
	jugador = jugador.clamp(ARENA.position + Vector2(16, 16), ARENA.end - Vector2(16, 16))
	if der.length() > 0.25:
		apunte = der.normalized()
	elif izq.length() > 0.25:
		apunte = izq.normalized()

	reloj_disparo -= delta
	if r2 > 0.05 and reloj_disparo <= 0.0:
		reloj_disparo = lerpf(0.35, 0.06, r2)
		balas.append({"p": jugador, "v": apunte * 900.0})
		Input.start_joy_vibration(dispositivo, 0.25 + r2 * 0.4, 0.0, 0.05)

	reloj_enemigo -= delta * ritmo
	if reloj_enemigo <= 0.0:
		reloj_enemigo = randf_range(0.35, 0.9)
		enemigos.append({"p": _punto_borde()})

	for bala in balas:
		bala["p"] += bala["v"] * delta
	balas = balas.filter(func(b): return ARENA.has_point(b["p"]))

	reloj_golpe = max(reloj_golpe - delta, 0.0)
	for enemigo in enemigos:
		var hacia: Vector2 = (jugador - enemigo["p"]).normalized()
		enemigo["p"] += hacia * 130.0 * ritmo * delta
	_resolver_choques()


func _punto_borde() -> Vector2:
	var lado := randi() % 4
	var x := randf_range(ARENA.position.x, ARENA.end.x)
	var y := randf_range(ARENA.position.y, ARENA.end.y)
	match lado:
		0: return Vector2(x, ARENA.position.y)
		1: return Vector2(x, ARENA.end.y)
		2: return Vector2(ARENA.position.x, y)
	return Vector2(ARENA.end.x, y)


func _resolver_choques() -> void:
	var vivos: Array[Dictionary] = []
	for enemigo in enemigos:
		var tocado := false
		for bala in balas:
			if bala["p"].distance_to(enemigo["p"]) < 20.0:
				bala["p"] = Vector2(-999, -999)
				tocado = true
				break
		if tocado:
			puntos += 1
			continue
		if enemigo["p"].distance_to(jugador) < 28.0:
			golpes += 1
			reloj_golpe = 0.35
			Input.start_joy_vibration(dispositivo, 0.6, 1.0, 0.35)
			continue
		vivos.append(enemigo)
	enemigos = vivos


func _draw() -> void:
	draw_rect(Rect2(Vector2.ZERO, Vector2(1280, 720)), FONDO)
	_texto(Vector2(20, 42), "Banco Mando", TINTA, 26)
	if dispositivo < 0:
		_texto(Vector2(240, 42), "Sin mando: abre la app y conecta el DualSense", AVISO, 18)
	else:
		_texto(Vector2(240, 42), Input.get_joy_name(dispositivo), SUAVE, 16)
	_dibujar_arena()
	_dibujar_panel()


func _dibujar_arena() -> void:
	draw_rect(ARENA, PANEL)
	if reloj_golpe > 0.0:
		draw_rect(ARENA, Color(MAL, reloj_golpe), false, 6.0)
	for bala in balas:
		draw_circle(bala["p"], 4.0, TINTA)
	for enemigo in enemigos:
		draw_circle(enemigo["p"], 14.0, MAL)
	draw_circle(jugador, 16.0, ACENTO)
	draw_line(jugador, jugador + apunte * 30.0, TINTA, 3.0)
	_texto(ARENA.position + Vector2(14, 28), "Puntos %d    Golpes %d" % [puntos, golpes], TINTA, 18)
	var ayuda := "Stick izq: mover   Stick der: apuntar   R2: disparar   L2: frenar enemigos   Cruz: impulso"
	_texto(Vector2(ARENA.position.x + 14, ARENA.end.y - 14), ayuda, SUAVE, 14)


func _dibujar_panel() -> void:
	var y := 70.0
	var mandos := Input.get_connected_joypads().size()
	if mandos > 1:
		_texto(Vector2(PANEL_X, y), "%d mandos visibles: posible entrada doble" % mandos, AVISO, 15)
	else:
		_texto(Vector2(PANEL_X, y), "%d mando visible · %d eventos/s" % [mandos, eventos_seg], SUAVE, 15)
	y += 26.0
	if dispositivo >= 0:
		_dibujar_stick(Vector2(PANEL_X + 58, y + 58), _stick_izq(), sectores_izq, "Stick izq.")
		_dibujar_stick(Vector2(PANEL_X + 186, y + 58), _stick_der(), sectores_der, "Stick der.")
		_dibujar_gatillo(Vector2(PANEL_X + 268, y), "L2", _eje(JOY_AXIS_TRIGGER_LEFT), max_l2)
		_dibujar_gatillo(Vector2(PANEL_X + 354, y), "R2", _eje(JOY_AXIS_TRIGGER_RIGHT), max_r2)
	y += 156.0

	_texto(Vector2(PANEL_X, y), "Botones %d de %d" % [botones_vistos.size(), BOTONES.size()], TINTA, 16)
	y += 8.0
	for i in BOTONES.size():
		var fila: Array = BOTONES[i]
		var celda := Rect2(PANEL_X + (i % 5) * 92.0, y + (i / 5) * 34.0, 88, 30)
		var pulsado := dispositivo >= 0 and Input.is_joy_button_pressed(dispositivo, fila[0])
		var color := ACENTO if pulsado else (PANEL if not botones_vistos.has(fila[0]) else Color(BIEN, 0.25))
		draw_rect(celda, color)
		_texto(celda.position + Vector2(8, 21), fila[1], TINTA, 14)
	y += 118.0

	_marca(Vector2(PANEL_X, y), vibracion_debil, "Cuadrado: motor debil")
	_marca(Vector2(PANEL_X + 230, y), vibracion_fuerte, "Circulo: motor fuerte")
	y += 24.0
	_marca(Vector2(PANEL_X, y), max_l2 > 0.98, "L2 llega al 100%")
	_marca(Vector2(PANEL_X + 230, y), max_r2 > 0.98, "R2 llega al 100%")
	y += 24.0
	_marca(Vector2(PANEL_X, y), sectores_izq.size() == 8, "Stick izq. vuelta completa")
	_marca(Vector2(PANEL_X + 230, y), sectores_der.size() == 8, "Stick der. vuelta completa")
	y += 34.0

	_texto(Vector2(PANEL_X, y), "Confirma tu con el teclado", TINTA, 16)
	y += 24.0
	for i in MANUALES.size():
		var fila: Array = MANUALES[i]
		_marca(Vector2(PANEL_X, y), manuales_ok.get(fila[0], false), "%d: %s" % [i + 1, fila[1]])
		y += 24.0
	if reloj_mensaje > 0.0:
		_texto(Vector2(PANEL_X, 708), mensaje, BIEN, 14)
	else:
		_texto(Vector2(PANEL_X, 708), "F12: guardar informe (tambien al cerrar)", SUAVE, 14)


func _dibujar_stick(centro: Vector2, valor: Vector2, sectores: Dictionary, nombre: String) -> void:
	draw_arc(centro, 52.0, 0.0, TAU, 48, LINEA, 2.0)
	for sector in sectores:
		var inicio: float = sector * TAU / 8.0
		draw_arc(centro, 52.0, inicio, inicio + TAU / 8.0, 12, BIEN, 3.0)
	draw_circle(centro + valor.limit_length(1.0) * 52.0, 6.0, ACENTO)
	_texto(centro + Vector2(-52, 72), nombre, SUAVE, 13)
	_texto(centro + Vector2(-52, 90), "%.3f %.3f" % [valor.x, valor.y], TINTA, 13)


func _dibujar_gatillo(origen: Vector2, nombre: String, valor: float, maximo: float) -> void:
	var caja := Rect2(origen, Vector2(26, 116))
	draw_rect(caja, PANEL)
	var alto := caja.size.y * valor
	draw_rect(Rect2(caja.position.x, caja.end.y - alto, caja.size.x, alto), ACENTO)
	var marca := caja.end.y - caja.size.y * maximo
	draw_line(Vector2(caja.position.x, marca), Vector2(caja.end.x, marca), BIEN, 2.0)
	_texto(origen + Vector2(32, 14), nombre, SUAVE, 13)
	_texto(origen + Vector2(0, 134), "%d%%" % int(round(valor * 100.0)), TINTA, 13)


func _marca(pos: Vector2, hecho: bool, texto: String) -> void:
	var caja := Rect2(pos + Vector2(0, -12), Vector2(14, 14))
	draw_rect(caja, BIEN if hecho else LINEA)
	_texto(pos + Vector2(22, 0), texto, TINTA if hecho else SUAVE, 14)


func _texto(pos: Vector2, texto: String, color: Color, tamano: int) -> void:
	draw_string(fuente, pos, texto, HORIZONTAL_ALIGNMENT_LEFT, -1, tamano, color)


func _guardar_informe() -> void:
	var lineas := PackedStringArray()
	lineas.append("Fecha: %s" % Time.get_datetime_string_from_system())
	for id in Input.get_connected_joypads():
		lineas.append("Mando %d: %s" % [id, Input.get_joy_name(id)])
	lineas.append("Eventos/s: %d" % eventos_seg)
	for fila in BOTONES:
		lineas.append("%s: %s" % [fila[1], "ok" if botones_vistos.has(fila[0]) else "sin probar"])
	lineas.append("L2 maximo: %.3f" % max_l2)
	lineas.append("R2 maximo: %.3f" % max_r2)
	lineas.append("Stick izq. sectores: %d de 8" % sectores_izq.size())
	lineas.append("Stick der. sectores: %d de 8" % sectores_der.size())
	lineas.append("Motor debil probado: %s" % vibracion_debil)
	lineas.append("Motor fuerte probado: %s" % vibracion_fuerte)
	for fila in MANUALES:
		lineas.append("%s: %s" % [fila[1], "ok" if manuales_ok.get(fila[0], false) else "sin confirmar"])
	lineas.append("Puntos: %d, golpes: %d" % [puntos, golpes])
	var archivo := FileAccess.open(INFORME, FileAccess.WRITE)
	if archivo == null:
		mensaje = "No se pudo guardar el informe"
	else:
		archivo.store_string("\n".join(lineas) + "\n")
		archivo.close()
		mensaje = "Informe guardado"
	reloj_mensaje = 3.0
