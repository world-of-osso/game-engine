extends SceneTree

## Live swim on the dev server with real key events: from the seabed of the deep water at
## `azeroth(32,48)` X=-8558 Z=500 (the swimming fixture's measured spot), held Space rises
## to the surface and bobs there, held X sinks back to the seabed. With the head under water
## the server's breath mirror timer (`MirrorTimerStart`, 180 s draining at 1 ms per ms) must
## show its bar counting down at that scale, and surfacing must refill and hide it. The
## server-replicated height (what other clients draw) must follow the swimmer at the
## surface and on the seabed. SWIM_DROWN=1 also stays down 180 s for one drowning hit.
## Account and character (card 0) come from SWIM_ACCOUNT, SWIM_PASSWORD, SWIM_CHARACTER;
## place the character offline with
## `game-server-admin teleport <name> 0 -8558 -500 140.3` (WoW coordinates).

var NAME := OS.get_environment("SWIM_CHARACTER")
const DEEP := Vector2(-8558.0, 500.0)
## `SWIM_DEPTH`: a swimmer floats with its feet this far under the surface.
const SWIM_DEPTH := 1.25
## `SWIM_SPEED` yards/second, the ascend and descend rate.
const SWIM_SPEED := 4.7222
## The server's head-under-water rule: the surface more than `DEFAULT_COLLISION_HEIGHT`
## (TrinityCore Object.h) over the feet.
const HEAD_UNDER := 2.03128
## Breath lasts 180 000 ms, draining 1 ms per ms: this bar fraction per second.
const BREATH_DRAIN := 1.0 / 180.0
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

	error = await server_follows(client, player, "floating")
	if error != "":
		fail(error)
		return
	if client.mirror_timer_fraction(BREATH) != null:
		fail("Breath bar shown while floating with the head out: %s" % client.mirror_timer_fraction(BREATH))
		return

	error = await sink_with_breath(client, player, float(surface), float(seabed))
	if error != "":
		fail(error)
		return
	if OS.get_environment("SWIM_DROWN") == "1":
		error = await drown(client, player)
		if error != "":
			fail(error)
			return

	error = await rise_to_surface(client, player, top)
	if error != "":
		fail(error)
		return
	# Head out: the server refills breath at 10 ms per ms, then stops the timer.
	var hidden := func(_state): return client.mirror_timer_fraction(BREATH) == null and not breath_bar(client).is_visible_in_tree()
	if not await wait_until(client, hidden, 30000, "breath bar hidden after surfacing"):
		return
	trace(client, player, "surfaced, breath stopped")
	await snapshot("surfaced-no-breath")
	print("PASS: Space rose to the surface, X sank to the seabed, server breath bar shown under water and hidden after surfacing")
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
	error = await server_follows(client, player, "surfacing")
	if error != "":
		return error
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

## Hold X to the seabed: the head goes under water on the way down (feet more than
## `HEAD_UNDER` below the surface). The server must start the breath bar, full and counting
## down at 1/180 of the bar per second.
func sink_with_breath(client: Node, player: Node3D, surface: float, seabed: float) -> String:
	if seabed > surface - HEAD_UNDER - 0.1:
		return "Seabed %.3f is too shallow to put the head under %.3f" % [seabed, surface]
	push_key(KEY_X, true)
	var deadline := Time.get_ticks_msec() + PHASE_TIMEOUT_MS
	var last_y := player.position.y
	var under_at := -1
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var y := player.position.y
		if y > last_y + 0.001:
			push_key(KEY_X, false)
			return "Held X rose: %.3f -> %.3f" % [last_y, y]
		last_y = y
		if under_at < 0 and y < surface - HEAD_UNDER:
			under_at = Time.get_ticks_msec()
			trace(client, player, "head under water")
		if y <= seabed + 0.02:
			break
	push_key(KEY_X, false)
	if player.position.y > seabed + 0.02:
		return "Held X did not reach the seabed: %s seabed=%.3f" % [player.position, seabed]
	trace(client, player, "on seabed")
	var error: String = await server_follows(client, player, "seabed")
	if error != "":
		return error
	var shown := func(_state): return client.mirror_timer_fraction(BREATH) != null
	if not await wait_until(client, shown, 3000, "server breath timer after the head went under"):
		return "Server sent no breath timer"
	print("TRACE breath shown %d ms after the head went under" % (Time.get_ticks_msec() - under_at))
	var first_ms := Time.get_ticks_msec()
	var first := float(client.mirror_timer_fraction(BREATH))
	if first < 0.97:
		return "Breath timer did not start full: %.4f" % first
	for _frame in 120:
		await process_frame
	var seconds := (Time.get_ticks_msec() - first_ms) / 1000.0
	var fraction := float(client.mirror_timer_fraction(BREATH))
	var drain := (first - fraction) / seconds
	print("TRACE breath %.4f -> %.4f in %.2f s: %.5f/s (server scale %.5f/s)" % [first, fraction, seconds, drain, BREATH_DRAIN])
	if absf(drain - BREATH_DRAIN) > BREATH_DRAIN * 0.1:
		return "Breath bar drains %.5f/s, not the server's %.5f/s" % [drain, BREATH_DRAIN]
	var bar := breath_bar(client)
	if not bar.is_visible_in_tree():
		return "Breath bar is not drawn"
	var width := bar.get_global_rect().size.x
	if absf(width - 195.0 * fraction) > 1.5:
		return "Breath bar fill %.1f px does not match %.4f" % [width, fraction]
	var label = client.get_node("MirrorTimers").find_child("MirrorTimer1Text", true, false)
	if label == null or label.text != "Breath":
		return "Breath bar label: %s" % (label.text if label != null else "missing")
	print("TRACE breath fraction=%.4f fill=%.1f px at %s" % [fraction, width, bar.get_global_rect()])
	await snapshot("seabed-breath")
	return ""

## Stay on the seabed until breath runs out: the server deals `DAMAGE_DROWNING` (20% of
## maximum health) every second at 0.
func drown(client: Node, player: Node3D) -> String:
	var before = client.account_state().local_player_health
	var empty := func(_state): return client.mirror_timer_fraction(BREATH) != null and float(client.mirror_timer_fraction(BREATH)) <= 0.0
	if not await wait_until(client, empty, 190000, "breath at 0"):
		return "Breath did not run out"
	trace(client, player, "out of breath, health=%s" % before)
	var deadline := Time.get_ticks_msec() + 10000
	var next_trace := 0
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var health = client.account_state().local_player_health
		if Time.get_ticks_msec() >= next_trace:
			next_trace = Time.get_ticks_msec() + 500
			trace(client, player, "drowning? health=%s" % health)
		if health != null and before != null and int(health) < int(before):
			break
	var after = client.account_state().local_player_health
	if after == null or before == null or int(after) >= int(before):
		return "No drowning damage within 10 s at 0 breath: health %s -> %s" % [before, after]
	print("TRACE drowning: health %s -> %s" % [before, client.account_state().local_player_health])
	await snapshot("drowning")
	return ""

## The server adopts the swimmer's height: its replicated position (what other clients draw)
## reaches the client's within two seconds.
func server_follows(client: Node, player: Node3D, what: String) -> String:
	var started := Time.get_ticks_msec()
	var deadline := started + 2000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var server = client.account_state().local_server_position
		if server != null and absf(float(server.y) - player.position.y) < 0.02:
			print("TRACE server height %.3f follows client %.3f (%s) after %d ms" % [float(server.y), player.position.y, what, Time.get_ticks_msec() - started])
			return ""
	return "Server height %s does not follow the client %.3f (%s)" % [client.account_state().local_server_position, player.position.y, what]

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
			next_trace = Time.get_ticks_msec() + 50
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
