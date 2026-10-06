extends SceneTree

# Requires GODOT_TEST_SERVER (private endpoint, never shared :5000),
# GODOT_TEST_ACCOUNT, GODOT_TEST_PASSWORD and GODOT_TEST_SHOTS (output directory).
# In-world World Map (docs/specs/world-map.md): M opens the
# player's zone, right-click zooms out zone -> continent -> world, clicking the
# player's spot zooms back in, the arrow sits at the player's map position and
# points where forward movement goes, Escape and M close it.

const WORLD_WAIT_MS := 90000
const MAP_WAIT_MS := 20000
const MOVE_FRAMES := 60
var client: Node
var shots: String

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("GODOT_TEST_ACCOUNT")
	var password := OS.get_environment("GODOT_TEST_PASSWORD")
	shots = OS.get_environment("GODOT_TEST_SHOTS")
	if server.is_empty() or account.is_empty() or password.is_empty() or shots.is_empty():
		fail("GODOT_TEST_SERVER, GODOT_TEST_ACCOUNT, GODOT_TEST_PASSWORD and GODOT_TEST_SHOTS are required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, password, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if client.get_node_or_null("WorldMapUI") != null:
		fail("World map open before M")
		return
	await tap(KEY_M)
	if not await wait_map_open():
		return
	var zone: Dictionary = client.world_map_state()
	print("FIXTURE ZONE ", zone.map_name, " ", zone.map_id, " kind ", zone.map_kind, " player ", zone.player_map_id)
	if zone.map_kind != 3 or zone.map_id != zone.player_map_id or zone.map_name == "":
		fail("M did not open the player's zone: " + str(zone))
		return
	# Every art tile is drawn or reported missing from the local CASC install.
	if zone.tile_count + zone.missing_tiles == 0:
		fail("Zone map has no art tiles: " + str(zone))
		return
	if not arrow_at_player(zone):
		return
	await capture("worldmap-zone.png")
	if not await arrow_follows_forward_movement():
		return

	await click_canvas(MOUSE_BUTTON_RIGHT, Vector2(0.5, 0.5))
	var continent: Dictionary = client.world_map_state()
	print("FIXTURE CONTINENT ", continent.map_name, " ", continent.map_id)
	if continent.map_kind != 2 or continent.breadcrumbs.size() != zone.breadcrumbs.size() - 1 or continent.breadcrumbs[-1] != continent.map_name:
		fail("Right-click did not zoom zone out to its continent: " + str(continent))
		return
	if not arrow_at_player(continent):
		return
	await capture("worldmap-continent.png")

	await click_canvas(MOUSE_BUTTON_RIGHT, Vector2(0.5, 0.5))
	var world: Dictionary = client.world_map_state()
	print("FIXTURE WORLD ", world.map_name, " ", world.map_id)
	if world.map_kind != 1 or not arrow_at_player(world):
		fail("Second right-click did not reach the world map: " + str(world))
		return
	await hover_canvas(world.player_uv)
	var hovered: Dictionary = client.world_map_state()
	if hovered.highlight != continent.map_name:
		fail("Hovering the player's spot did not highlight " + continent.map_name + ": " + str(hovered.highlight))
		return
	await capture("worldmap-world.png")

	await click_canvas(MOUSE_BUTTON_LEFT, world.player_uv)
	if client.world_map_state().map_id != continent.map_id:
		fail("Clicking the player's continent did not zoom in: " + str(client.world_map_state()))
		return
	await click_canvas(MOUSE_BUTTON_LEFT, client.world_map_state().player_uv)
	if client.world_map_state().map_id != zone.map_id:
		fail("Clicking the player's zone did not zoom in: " + str(client.world_map_state()))
		return
	await click_named("WorldMapNav1")
	if client.world_map_state().map_id != zone.breadcrumb_ids[1]:
		fail("Breadcrumb did not navigate to its map: " + str(client.world_map_state()))
		return

	await tap(KEY_ESCAPE)
	if not await wait_map_closed():
		return
	if client.get_node_or_null("GameMenuUI") != null:
		fail("Escape closing the map also opened the game menu")
		return
	await tap(KEY_M)
	if not await wait_map_open():
		return
	if client.world_map_state().map_id != zone.map_id:
		fail("Reopening did not return to the player's zone: " + str(client.world_map_state()))
		return
	await tap(KEY_M)
	if not await wait_map_closed():
		return
	print("FIXTURE WORLD_MAP_DONE")
	client.free()
	quit(0)

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received:
			if state.screen != "CharacterSelect" or state.character_count < 1:
				fail("Fixture needs an authenticated character: " + str(state))
				return false
			break
	var ui = client.get_node_or_null("CharacterSelectUI")
	var enter = ui.find_child("EnterWorld", true, false) if ui != null else null
	if not enter is Button:
		fail("Enter World button missing")
		return false
	await click_control(enter)
	deadline = Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		var terrain: Dictionary = state.terrain
		if state.screen == "InWorld" and state.local_player_position != null and not terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD ", state.selected_character_name, " at ", state.local_player_position)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func map_ui() -> Node:
	return client.get_node_or_null("WorldMapUI")

func wait_map_open() -> bool:
	var deadline := Time.get_ticks_msec() + MAP_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var ui := map_ui()
		if ui != null and ui.find_child("WorldMapCanvas", true, false) != null:
			for frame in range(3):
				await process_frame
			return true
	fail("Timed out opening WorldMapUI")
	return false

func wait_map_closed() -> bool:
	var deadline := Time.get_ticks_msec() + MAP_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if map_ui() == null:
			return true
	fail("World map did not close")
	return false

func canvas_rect() -> Rect2:
	return (map_ui().find_child("WorldMapCanvas", true, false) as Control).get_global_rect()

func arrow() -> Control:
	return map_ui().find_child("WorldMapPlayerArrow", true, false) as Control

# The arrow's drawn part, which carries the facing rotation.
func arrow_part() -> Control:
	var node := arrow()
	return node.find_child("Part0", true, false) as Control if node != null else null

func arrow_at_player(state: Dictionary) -> bool:
	var node := arrow()
	if node == null or state.player_uv == null:
		fail("No player arrow on " + str(state.map_name) + ": " + str(state))
		return false
	var rect := canvas_rect()
	var expected := rect.position + rect.size * (state.player_uv as Vector2)
	var center := node.get_global_rect().get_center()
	if center.distance_to(expected) > 1.5:
		fail("Arrow at %s, player map position %s on %s" % [center, expected, state.map_name])
		return false
	return true

func arrow_screen_direction() -> Vector2:
	var angle := arrow_part().rotation
	return Vector2(sin(angle), -cos(angle))

func arrow_follows_forward_movement() -> bool:
	var before := arrow().get_global_rect().get_center()
	var pointing := arrow_screen_direction()
	var uv_before: Vector2 = client.world_map_state().player_uv
	push_key(KEY_W, true)
	for frame in range(MOVE_FRAMES):
		await process_frame
	push_key(KEY_W, false)
	for frame in range(10):
		await process_frame
	var state: Dictionary = client.world_map_state()
	var after := arrow().get_global_rect().get_center()
	var moved := after - before
	print("FIXTURE MOVE uv ", uv_before, " -> ", state.player_uv, " arrow ", before, " -> ", after, " pointing ", pointing)
	if moved.length() < 1.0:
		fail("Walking forward with the map open did not move the arrow: " + str(moved))
		return false
	if moved.normalized().dot(pointing) < 0.95:
		fail("Arrow points %s but the player moved %s on the map" % [pointing, moved.normalized()])
		return false
	return arrow_at_player(state)

func click_canvas(button: MouseButton, uv: Vector2) -> void:
	var rect := canvas_rect()
	var point := rect.position + rect.size * uv
	await hover_point(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	for frame in range(4):
		await process_frame

func hover_canvas(uv: Vector2) -> void:
	var rect := canvas_rect()
	await hover_point(rect.position + rect.size * uv)
	for frame in range(3):
		await process_frame

func hover_point(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame

func click_named(name: String) -> void:
	var control = map_ui().find_child(name, true, false)
	if not control is Control:
		fail("Missing world map control " + name)
		return
	await click_control(control)
	for frame in range(4):
		await process_frame

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(shots.path_join(file))
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func tap(code: Key) -> void:
	push_key(code, true)
	await process_frame
	push_key(code, false)
	await process_frame
	await process_frame

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func click_control(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	await hover_point(point)
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
