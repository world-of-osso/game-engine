extends SceneTree

## Live swim on the dev server with real key events: from the seabed of the deep water at
## `azeroth(32,48)` X=-8558 Z=500 (the swimming fixture's measured spot), held Space rises
## to the surface and bobs there, held X sinks back to the seabed. While the head is under
## water the fixture starts the retail breath mirror timer (the server sends none yet) and
## checks the bar is drawn and counts down. Logs client and server-replicated height.
## Account and character (card 0) come from SWIM_ACCOUNT, SWIM_PASSWORD, SWIM_CHARACTER;
## place the character offline with
## `game-server-admin teleport <name> 0 -8558 -500 140.3` (WoW coordinates).

var NAME := OS.get_environment("SWIM_CHARACTER")
const DEEP := Vector2(-8558.0, 500.0)
## `SWIM_DEPTH`: a swimmer floats with its feet this far under the surface.
const SWIM_DEPTH := 1.25
## `SWIM_SPEED` yards/second, the ascend and descend rate.
const SWIM_SPEED := 4.7222
## Human standing height; the head is under water once the feet are this far below the
## floating height.
const HEAD_UNDER := 1.0
const PHASE_TIMEOUT_MS := 8000
const SHOT_DIR := "/tmp/claude"
const BREATH := 1

var shot := 0

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5000":
		fail("GODOT_TEST_SERVER must explicitly select 127.0.0.1:5000")
		return
	var account := OS.get_environment("SWIM_ACCOUNT")
	var password := OS.get_environment("SWIM_PASSWORD")
	if account == "" or password == "" or NAME == "":
		fail("SWIM_ACCOUNT, SWIM_PASSWORD and SWIM_CHARACTER are required")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, password, false)
	if error != "":
		fail("connect: " + error)
		return
	if not await enter_world(client):
		return
	var player := client.get_node("WorldUnits/" + NAME) as Node3D
	var here := Vector2(player.position.x, player.position.z)
	if here.distance_to(DEEP) > 1.0:
		fail("%s is not at the deep-water spot: %s" % [NAME, player.position])
		return
	var surface = client.water_surface_at(player.position.x, player.position.z)
	var seabed = client.terrain_height_at(player.position.x, player.position.z)
	if surface == null or seabed == null or float(surface) - float(seabed) < 3.0:
		fail("No deep water under %s: surface=%s seabed=%s" % [player.position, surface, seabed])
		return
	var top := float(surface) - SWIM_DEPTH
	trace(client, player, "entered surface=%s seabed=%s top=%.3f" % [surface, seabed, top])
	await snapshot("start")

	# Idle in water: no gravity, no drift.
	var idle := player.position
	for _frame in 60:
		await process_frame
	if player.position.distance_to(idle) > 0.02:
		fail("Idle swimmer drifted: %s -> %s" % [idle, player.position])
		return

	# Get to the seabed first when the spawn floats.
	if player.position.y > float(seabed) + 0.05:
		error = await hold_until(client, player, KEY_X, func(y): return y <= float(seabed) + 0.02, "sink to the seabed")
		if error != "":
			fail(error)
			return

	error = await rise_to_surface(client, player, top)
	if error != "":
		fail(error)
		return
	await snapshot("surface")
	var floating := player.position
	for _frame in 60:
		await process_frame
	if absf(player.position.y - top) > 0.01 or player.position.distance_to(floating) > 0.02:
		fail("Released Space did not stay at the surface: %s" % player.position)
		return
	trace(client, player, "floating")

	error = await sink_with_breath(client, player, top, float(seabed))
	if error != "":
		fail(error)
		return

	error = await rise_to_surface(client, player, top)
	if error != "":
		fail(error)
		return
	client.stop_mirror_timer(BREATH)
	await process_frame
	await process_frame
	if client.mirror_timer_fraction(BREATH) != null or breath_bar(client).is_visible_in_tree():
		fail("Stopped breath timer is still shown")
		return
	await snapshot("surfaced-no-breath")
	print("PASS: Space rose to the surface, X sank to the seabed, breath bar shown under water")
	quit(0)

## Hold Space from below the surface: rise at swim speed to `top` and no higher.
func rise_to_surface(client: Node, player: Node3D, top: float) -> String:
	var start := player.position.y
	var started := Time.get_ticks_msec()
	var error: String = await hold_until(client, player, KEY_SPACE, func(y): return y >= top - 0.001, "rise to the surface")
	if error != "":
		return error
	var seconds := (Time.get_ticks_msec() - started) / 1000.0
	var rate := (player.position.y - start) / seconds
	print("TRACE rose %.3f yd in %.2f s (%.2f yd/s)" % [player.position.y - start, seconds, rate])
	if rate < SWIM_SPEED * 0.7 or rate > SWIM_SPEED * 1.2:
		return "Ascend rate %.2f yd/s is not the swim speed" % rate
	# Keep holding at the surface: bob there, never above it.
	push_key(KEY_SPACE, true)
	for _frame in 60:
		await process_frame
		if player.position.y > top + 0.01:
			push_key(KEY_SPACE, false)
			return "Held Space rose above the surface: %s > %.3f" % [player.position, top]
	push_key(KEY_SPACE, false)
	trace(client, player, "at surface")
	return ""

## Hold X to the seabed; once the head is under water start the breath timer and check
## its bar is drawn and counting down.
func sink_with_breath(client: Node, player: Node3D, top: float, seabed: float) -> String:
	push_key(KEY_X, true)
	var deadline := Time.get_ticks_msec() + PHASE_TIMEOUT_MS
	var breathing := false
	var last_y := player.position.y
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var y := player.position.y
		if y > last_y + 0.001:
			push_key(KEY_X, false)
			return "Held X rose: %.3f -> %.3f" % [last_y, y]
		last_y = y
		if not breathing and y < top - HEAD_UNDER:
			var started: String = client.start_mirror_timer(BREATH, 180000, 180000, -1.0, false)
			if started != "":
				push_key(KEY_X, false)
				return "Breath timer: " + started
			breathing = true
			trace(client, player, "head under water, breath started")
		if y <= seabed + 0.02:
			break
	push_key(KEY_X, false)
	if player.position.y > seabed + 0.02:
		return "Held X did not reach the seabed: %s seabed=%.3f" % [player.position, seabed]
	trace(client, player, "on seabed")
	for _frame in 60:
		await process_frame
	var fraction = client.mirror_timer_fraction(BREATH)
	if fraction == null or float(fraction) >= 1.0 or float(fraction) < 0.95:
		return "Breath timer did not count down one second: %s" % fraction
	var bar := breath_bar(client)
	if not bar.is_visible_in_tree():
		return "Breath bar is not drawn"
	var width := bar.get_global_rect().size.x
	if absf(width - 195.0 * float(fraction)) > 1.5:
		return "Breath bar fill %.1f px does not match %.3f" % [width, float(fraction)]
	print("TRACE breath fraction=%.4f fill=%.1f px at %s" % [float(fraction), width, bar.get_global_rect()])
	await snapshot("seabed-breath")
	return ""

func breath_bar(client: Node) -> Control:
	return client.get_node("MirrorTimers").find_child("MirrorTimer1StatusBar", true, false) as Control

## Hold `key` until `reached(y)`, failing on a phase timeout.
func hold_until(client: Node, player: Node3D, key: Key, reached: Callable, what: String) -> String:
	push_key(key, true)
	var deadline := Time.get_ticks_msec() + PHASE_TIMEOUT_MS
	var next_trace := 0
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if Time.get_ticks_msec() >= next_trace:
			next_trace = Time.get_ticks_msec() + 250
			trace(client, player, what)
		if reached.call(player.position.y):
			push_key(key, false)
			return ""
	push_key(key, false)
	return "Timed out: %s at %s" % [what, player.position]

func enter_world(client: Node) -> bool:
	if not await wait_until(client, func(state): return state.screen == "CharacterSelect" and state.reply_received, 20000, "CharacterSelect"):
		return false
	var ui = client.get_node("CharacterSelectUI")
	await click_control(ui.find_child("CharCard_0", true, false))
	await process_frame
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != NAME:
		fail("Card 0 is %s, not %s" % [selected.text, NAME])
		return false
	await click_control(ui.find_child("EnterWorld", true, false))
	var ready := func(state):
		return state.screen == "InWorld" and state.selected_character_name == NAME and state.terrain.map == "azeroth" \
			and state.local_player_position != null and state.local_server_position != null \
			and state.terrain.pending_count == 0 and client.water_surface_at(DEEP.x, DEEP.y) != null
	if not await wait_until(client, ready, 180000, "InWorld on azeroth_32_48"):
		return false
	for _frame in 60:
		await process_frame
	return true

func trace(client: Node, player: Node3D, label: String) -> void:
	var state: Dictionary = client.account_state()
	print("TRACE t=%d %s client=%s server=%s breath=%s" % [
		Time.get_ticks_msec(), label, player.position, state.local_server_position,
		client.mirror_timer_fraction(BREATH)])

func snapshot(label: String) -> void:
	await RenderingServer.frame_post_draw
	var path := "%s/swimming-live-%02d-%s.png" % [SHOT_DIR, shot, label]
	shot += 1
	root.get_texture().get_image().save_png(path)
	print("TRACE screenshot ", path)

func wait_until(client: Node, predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(client.account_state()):
			return true
	fail("Timed out waiting for %s: %s" % [what, client.account_state()])
	return false

func push_key(keycode: Key, pressed: bool) -> void:
	var key := InputEventKey.new()
	key.physical_keycode = keycode
	key.keycode = keycode
	key.pressed = pressed
	root.push_input(key, true)

func click_control(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
