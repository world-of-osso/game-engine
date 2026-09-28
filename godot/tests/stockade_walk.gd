extends SceneTree

## Live walk from Theron's room-floor spawn to the Stockade entrance (area trigger 101) on the
## dev server, driven by real arrow/W key events. Logs client and server-replicated position.

const NAME := "Theron"
## Bevy (x, z) = WoW (x, -y). The last point is area trigger 101's box center.
const WAYPOINTS := [Vector2(-8786.0, -836.0), Vector2(-8772.0, -836.0), Vector2(-8761.85, -848.557)]
const ARRIVE := 0.8
const YAW_TOLERANCE := 0.06
const LEG_TIMEOUT_MS := 20000
const SHOT_DIR := "/tmp/claude"

var turn_sign := 0.0
var shot := 0
var log_next := 0

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5000":
		fail("GODOT_TEST_SERVER must explicitly select 127.0.0.1:5000")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, "admin", "admin", false)
	if error != "":
		fail("connect: " + error)
		return
	if not await enter_world(client):
		return
	var player := client.get_node("WorldUnits/" + NAME) as Node3D
	await snapshot("spawn")
	trace(client, player, "spawn")
	for index in WAYPOINTS.size():
		var result: String = await walk_to(client, player, WAYPOINTS[index])
		trace(client, player, "leg%d %s" % [index, result])
		await snapshot("leg%d" % index)
		if result == "transfer":
			await follow_transfer(client)
			return
		if result != "arrived":
			fail("leg %d stopped: %s" % [index, result])
			return
	var deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.account_state().screen != "InWorld":
			await follow_transfer(client)
			return
	trace(client, player, "at trigger, no transfer")
	fail("Reached trigger 101 center without a transfer")

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
		return state.screen == "InWorld" and state.selected_character_name == NAME \
			and state.local_player_position != null and state.local_server_position != null \
			and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty() \
			and state.world_objects.pending == 0 and state.world_objects.spawned > 0
	if not await wait_until(client, ready, 120000, "InWorld with terrain and WMOs"):
		return false
	# Let the placed WMOs settle and the first server snapshots arrive.
	for _frame in 60:
		await process_frame
	return true

func walk_to(client: Node, player: Node3D, target: Vector2) -> String:
	var deadline := Time.get_ticks_msec() + LEG_TIMEOUT_MS
	var best := INF
	var best_at := Time.get_ticks_msec()
	var running := false
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen != "InWorld":
			release_all()
			return "transfer"
		var here := Vector2(player.position.x, player.position.z)
		var distance := here.distance_to(target)
		trace_throttled(client, player)
		if distance < ARRIVE:
			release_all()
			return "arrived"
		if distance < best - 0.05:
			best = distance
			best_at = Time.get_ticks_msec()
		elif Time.get_ticks_msec() - best_at > 4000:
			release_all()
			return "stuck %.2f yd from %s" % [distance, target]
		var error := angle_difference(facing(player), bearing(here, target))
		if not await steer(player, error):
			return "turn keys do not change facing"
		var aligned: bool = abs(error) < 0.4
		if aligned != running:
			running = aligned
			push_key(KEY_W, running)
	release_all()
	return "leg timeout"

## Yaw whose forward (sin, cos) points from `from` to `to`.
func bearing(from: Vector2, to: Vector2) -> float:
	var d := to - from
	return atan2(d.x, d.y)

func facing(player: Node3D) -> float:
	return player.rotation.y + PI / 2.0

## Hold the turn key that reduces `error` (target - facing), learning its sign once.
func steer(player: Node3D, error: float) -> bool:
	if turn_sign == 0.0:
		var before := facing(player)
		push_key(KEY_RIGHT, true)
		for _frame in 10:
			await process_frame
		push_key(KEY_RIGHT, false)
		await process_frame
		var turned := angle_difference(before, facing(player))
		if abs(turned) < 0.01:
			return false
		turn_sign = sign(turned)
		print("TRACE turn-right changes yaw by ", turned)
		return true
	var right: bool = abs(error) > YAW_TOLERANCE and sign(error) == turn_sign
	var left: bool = abs(error) > YAW_TOLERANCE and sign(error) == -turn_sign
	push_key(KEY_RIGHT, right)
	push_key(KEY_LEFT, left)
	return true

func release_all() -> void:
	for key in [KEY_W, KEY_LEFT, KEY_RIGHT]:
		push_key(key, false)

func follow_transfer(client: Node) -> void:
	print("TRACE transfer began: ", client.account_state().screen)
	await snapshot("transfer")
	var arrived := func(state):
		return state.screen == "InWorld" and state.terrain.map == "stormwindjail" and state.local_server_position != null
	if not await wait_until(client, arrived, 90000, "InWorld on stormwindjail"):
		return
	for _frame in 90:
		await process_frame
	var state: Dictionary = client.account_state()
	print("TRACE arrived map=", state.terrain.map, " client=", state.local_player_position, " server=", state.local_server_position)
	await snapshot("stockade")
	print("PASS: entered the Stockade")
	quit(0)

func trace_throttled(client: Node, player: Node3D) -> void:
	if Time.get_ticks_msec() < log_next:
		return
	log_next = Time.get_ticks_msec() + 250
	trace(client, player, "")

func trace(client: Node, player: Node3D, label: String) -> void:
	var state: Dictionary = client.account_state()
	var ground = client.terrain_height_at(player.position.x, player.position.z)
	print("TRACE t=%d %s client=%s server=%s yaw=%.3f terrain=%s screen=%s" % [
		Time.get_ticks_msec(), label, player.position, state.local_server_position,
		facing(player), ground, state.screen])

func snapshot(label: String) -> void:
	await RenderingServer.frame_post_draw
	var path := "%s/stockade-walk-%02d-%s.png" % [SHOT_DIR, shot, label]
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
