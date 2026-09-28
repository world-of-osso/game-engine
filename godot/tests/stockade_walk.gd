extends SceneTree

## Live walk from the room-floor spawn of `sw_magicdistrict` to the Stockade entrance (area trigger
## 101) on the dev server, driven by real arrow/W key events, until the client is in the Stockade.
## Logs client and server-replicated position. A character saved in the Stockade first walks out
## through its exit (area trigger 503), then back up the stairwell and in again. Account and
## character (card 0) come from STOCKADE_WALK_ACCOUNT, STOCKADE_WALK_PASSWORD, STOCKADE_WALK_CHARACTER;
## place the character with `game-server-admin set-position` while it is offline.

var NAME := OS.get_environment("STOCKADE_WALK_CHARACTER")
## Bevy (x, z) = WoW (x, -y). The last point is area trigger 101's box center.
const WAYPOINTS := [Vector2(-8786.0, -836.0), Vector2(-8772.0, -836.0), Vector2(-8761.85, -848.557)]
## Area trigger 503's box center inside the Stockade.
const STOCKADE_EXIT := Vector2(48.0937, -0.933267)
const STOCKADE := "stormwindjail"
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
	var account := OS.get_environment("STOCKADE_WALK_ACCOUNT")
	var password := OS.get_environment("STOCKADE_WALK_PASSWORD")
	if account == "" or password == "" or NAME == "":
		fail("STOCKADE_WALK_ACCOUNT, STOCKADE_WALK_PASSWORD and STOCKADE_WALK_CHARACTER are required")
		return
	var error = client.connect_account(server, account, password, false)
	if error != "":
		fail("connect: " + error)
		return
	if not await enter_world(client):
		return
	if not check_on_server_ground(client, "entered"):
		return
	var player := client.get_node("WorldUnits/" + NAME) as Node3D
	await snapshot("spawn")
	trace(client, "spawn")
	if client.account_state().terrain.map == STOCKADE:
		var exit: String = await walk_to(client, player, STOCKADE_EXIT)
		trace(client, "exit " + exit)
		if exit != "transfer":
			fail("Stockade exit stopped: " + exit)
			return
		if not await wait_until(client, func(state): return in_world(state, "azeroth"), 120000, "InWorld on azeroth"):
			return
		for _frame in 60:
			await process_frame
		player = client.get_node("WorldUnits/" + NAME) as Node3D
		if not check_on_server_ground(client, "back in Stormwind"):
			return
		await snapshot("stormwind")
		trace(client, "back in Stormwind")
	for index in WAYPOINTS.size():
		var result: String = await walk_to(client, player, WAYPOINTS[index])
		trace(client, "leg%d %s" % [index, result])
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
	trace(client, "at trigger, no transfer")
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
	if not await wait_until(client, func(state): return state.screen == "InWorld", 120000, "InWorld"):
		return false
	var entered: Dictionary = client.account_state()
	print("TRACE entered world: objects=", entered.world_objects, " tiles=", entered.terrain.parsed_tiles.size())
	trace(client, "entered")
	var ready := func(state):
		return in_world(state, "azeroth") or in_world(state, STOCKADE)
	if not await wait_until(client, ready, 120000, "InWorld with terrain and WMOs"):
		return false
	# Let the placed WMOs settle and the first server snapshots arrive.
	for _frame in 60:
		await process_frame
	return true

## In the world on `map` with its terrain parsed. Placed WMOs may still be spawning.
func in_world(state: Dictionary, map: String) -> bool:
	if state.screen != "InWorld" or state.selected_character_name != NAME or state.terrain.map != map:
		return false
	if state.local_player_position == null or state.local_server_position == null or state.terrain.pending_count != 0:
		return false
	return map == STOCKADE or not state.terrain.parsed_tiles.is_empty()

## The predicted feet match the server's once settled: placed WMO floors are ground before their
## nodes spawn, so the client does not drop to the terrain under a building.
func check_on_server_ground(client: Node, phase: String) -> bool:
	var state: Dictionary = client.account_state()
	trace(client, "%s, objects=%s" % [phase, state.world_objects])
	if state.local_player_health == 0.0:
		fail("%s is dead; the server drops a corpse's movement" % NAME)
		return false
	if absf(state.local_player_position.y - state.local_server_position.y) > 0.2:
		fail("%s: client feet %s left the server's %s" % [phase, state.local_player_position, state.local_server_position])
		return false
	return true

func walk_to(client: Node, player: Node3D, target: Vector2) -> String:
	var deadline := Time.get_ticks_msec() + LEG_TIMEOUT_MS
	var best := INF
	var best_at := Time.get_ticks_msec()
	var running := false
	var map: String = client.account_state().terrain.map
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		# A transfer to a WMO-only map can load between two script frames.
		if state.screen != "InWorld" or state.terrain.map != map:
			release_all()
			return "transfer"
		var here := Vector2(player.position.x, player.position.z)
		var distance := here.distance_to(target)
		trace_throttled(client)
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
	if not await wait_until(client, func(state): return in_world(state, STOCKADE), 90000, "InWorld on stormwindjail"):
		return
	for _frame in 90:
		await process_frame
	var state: Dictionary = client.account_state()
	print("TRACE arrived map=", state.terrain.map, " client=", state.local_player_position, " server=", state.local_server_position)
	dump_scene(client)
	await snapshot("stockade")
	var camera := client.get_viewport().get_camera_3d()
	var player := client.get_node("WorldUnits/" + NAME) as Node3D
	if camera == null or camera.global_position.distance_to(player.global_position) > 20.0:
		fail("World camera did not follow the player into the Stockade")
		return
	print("PASS: entered the Stockade")
	quit(0)

func dump_scene(client: Node) -> void:
	var camera := client.get_viewport().get_camera_3d()
	print("TRACE camera=", camera.get_path() if camera else "none", " at ", camera.global_position if camera else Vector3.ZERO, " far=", camera.far if camera else 0.0)
	var player := client.get_node_or_null("WorldUnits/" + NAME) as Node3D
	print("TRACE player visible=", player.is_visible_in_tree() if player else false, " at ", player.global_position if player else Vector3.ZERO)
	var wmos := client.get_node_or_null("WorldWmos") as Node3D
	if wmos == null:
		print("TRACE no WorldWmos")
		return
	for placement in wmos.get_children():
		var meshes := placement.find_children("*", "MeshInstance3D", true, false)
		var aabb := AABB()
		for mesh in meshes:
			var box: AABB = mesh.global_transform * mesh.get_aabb()
			aabb = box if aabb.size == Vector3.ZERO else aabb.merge(box)
		print("TRACE wmo ", placement.name, " visible=", placement.is_visible_in_tree(), " meshes=", meshes.size(), " at ", placement.global_transform, " aabb=", aabb)

func trace_throttled(client: Node) -> void:
	if Time.get_ticks_msec() < log_next:
		return
	log_next = Time.get_ticks_msec() + 250
	trace(client, "")

func trace(client: Node, label: String) -> void:
	var state: Dictionary = client.account_state()
	var player := client.get_node_or_null("WorldUnits/" + NAME) as Node3D
	if player == null:
		print("TRACE t=%d %s no player node server=%s screen=%s" % [Time.get_ticks_msec(), label, state.local_server_position, state.screen])
		return
	var ground = client.terrain_height_at(player.position.x, player.position.z)
	print("TRACE t=%d %s map=%s client=%s server=%s yaw=%.3f terrain=%s health=%s screen=%s" % [
		Time.get_ticks_msec(), label, state.terrain.map, player.position, state.local_server_position,
		facing(player), ground, state.local_player_health, state.screen])

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
