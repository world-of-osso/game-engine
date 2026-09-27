extends SceneTree

const NAME := "Input Fixture"
# Authored terrain height at this fixture's X/Z, not the transfer-only fixture's airborne Y.
const FIRST := Vector3(-8949.0, 112.879913, 0.0)
const WORLD_WAIT_MS := 60000
const LOADING_FRAMES := 24
const HELD_FRAMES := 60
const SETTLE_FRAMES := 20
const STOP_FRAMES := 45
const BACKGROUND_WAIT_MS := 30000
const AUTHORED_CHARACTER_POSITION := Vector3(-2981.82, 452.826, -457.35)

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":0"):
		fail("Fixture requires its owned ephemeral loopback UDP endpoint")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, "fixture", "fixture", false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await wait_for_screen(client, "CharacterSelect", 15000):
		return
	var ui = client.get_node_or_null("CharacterSelectUI")
	var card = ui.find_child("CharCard_0", true, false) if ui != null else null
	var enter = ui.find_child("EnterWorld", true, false) if ui != null else null
	if not card is Control or not enter is Button or not card.visible or not enter.visible:
		fail("Authenticated character card and Enter World action missing")
		return
	await click_control(card)
	if not await inspect_character_preview(client):
		return
	await click_control(enter)
	if not await wait_for_screen(client, "Loading", 15000):
		return
	if client.get_node_or_null("CharacterSelectScene") != null:
		fail("Entering the world retained the character-selection preview")
		return
	if not client.get_node("LoadingUI").visible:
		fail("Native LoadingUI not visible while terrain is withheld")
		return
	clear_ui_focus()
	push_w(true)
	for frame in range(LOADING_FRAMES):
		await process_frame
		if client.account_state().screen != "Loading" or not client.get_node("LoadingUI").visible:
			fail("Loading ended before LoadTerrain at frame " + str(frame))
			return
	push_w(false)
	for frame in range(SETTLE_FRAMES):
		await process_frame
		if client.account_state().screen != "Loading":
			fail("Loading ended before withheld-terrain input observation at frame " + str(frame))
			return
	print("FIXTURE LOADING_OBSERVED")
	if not await wait_for_world(client, WORLD_WAIT_MS):
		return
	clear_ui_focus()
	var player := client.get_node_or_null("WorldUnits/" + NAME) as Node3D
	if player == null:
		fail("Native selected player missing after world readiness")
		return
	var start := player.position
	var ground_height = client.terrain_height_at(start.x, start.z)
	if ground_height == null or absf(start.y - float(ground_height)) >= 0.3:
		fail("Input fixture must start grounded on its authored terrain: " + str(start) + " ground=" + str(ground_height))
		return
	print("FIXTURE WORLD_READY")
	for frame in range(6):
		await process_frame
		if client.account_state().screen != "InWorld":
			fail("World exited before input injection at frame " + str(frame))
			return
	if not await focus_loss_stops_held_input(player):
		return
	push_w(true)
	for frame in range(HELD_FRAMES):
		await process_frame
		if client.account_state().screen != "InWorld":
			fail("World exited during held W at frame " + str(frame))
			return
	push_w(false)
	print("FIXTURE RELEASED")
	var moved := player.position
	if moved.z > start.z - 0.1 or absf(moved.x - start.x) > 0.25:
		fail("Held W did not move native player along facing-PI direction: " + str(start) + " -> " + str(moved))
		return
	for frame in range(SETTLE_FRAMES):
		await process_frame
	var stopped_at := player.position
	for frame in range(STOP_FRAMES):
		await process_frame
		if client.account_state().screen != "InWorld":
			fail("World exited during release observation at frame " + str(frame))
			return
	if player.position.distance_to(stopped_at) > 0.05:
		fail("Native player continued moving after W release: " + str(stopped_at) + " -> " + str(player.position))
		return
	if not await camera_controls_change_orbit(client, player):
		return
	print("FIXTURE STOPPED")
	client.free()
	print("SHUTDOWN: client freed")
	quit(0)
	print("SHUTDOWN: quit requested")

func wait_for_screen(client: Node, wanted: String, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == wanted:
			if wanted == "CharacterSelect" and (not state.reply_received or state.character_count != 1):
				fail("Fixture authentication did not populate character selection: " + str(state))
				return false
			return true
	fail("Timed out waiting for " + wanted + ": " + str(client.account_state()))
	return false

func wait_for_world(client: Node, timeout_ms: int) -> bool:
	print("FIXTURE WORLD_WAIT_START elapsed_ms=", Time.get_ticks_msec())
	var deadline := Time.get_ticks_msec() + timeout_ms
	var next_report := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if Time.get_ticks_msec() >= next_report:
			print("FIXTURE WORLD_WAIT elapsed_ms=", Time.get_ticks_msec(), " screen=", state.screen, " units=", state.unit_count, " pending=", state.terrain.pending_count)
			next_report = Time.get_ticks_msec() + 5000
		if state.screen != "InWorld" or state.selected_character_name != NAME or state.unit_count != 1:
			continue
		var terrain: Dictionary = state.terrain
		if terrain.map != "azeroth" or terrain.pending_count != 0 or not terrain.failures.is_empty() or terrain.parsed_tiles.is_empty():
			continue
		var terrain_root = client.get_node_or_null("WorldTerrain")
		var player = client.get_node_or_null("WorldUnits/" + NAME) as Node3D
		if terrain_root == null or terrain_root.get_child_count() == 0 or player == null or player.position.distance_to(FIRST) > 0.5:
			continue
		for tile in terrain.parsed_tiles:
			if tile.chunk_count <= 0 or not FileAccess.file_exists(tile.root_path):
				fail("World tile lacks parsed geometry or cached authored asset: " + str(tile))
				return false
		if client.get_node("LoadingUI").visible:
			fail("LoadingUI remained visible after native terrain readiness")
			return false
		return true
	fail("Timed out waiting for native player and authored terrain: " + str(client.account_state()))
	return false

func camera_controls_change_orbit(client: Node, player: Node3D) -> bool:
	var camera := client.get_node_or_null("WorldCamera") as Camera3D
	if camera == null:
		fail("Native world camera missing during input probe")
		return false
	var before := camera.global_transform.basis.z
	var facing_before := player.rotation.y
	var press := InputEventMouseButton.new()
	press.position = Vector2(640, 360)
	press.button_index = MOUSE_BUTTON_RIGHT
	press.pressed = true
	root.push_input(press, true)
	var motion := InputEventMouseMotion.new()
	motion.position = press.position
	motion.relative = Vector2(30, -12)
	root.push_input(motion, true)
	for frame in range(20):
		await process_frame
	var release := InputEventMouseButton.new()
	release.position = press.position
	release.button_index = MOUSE_BUTTON_RIGHT
	release.pressed = false
	root.push_input(release, true)
	if before.angle_to(camera.global_transform.basis.z) < 0.01 or absf(player.rotation.y - facing_before) < 0.01:
		fail("Right-mouse motion failed to rotate native camera and character facing")
		return false
	var distance_before := camera.global_position.distance_to(player.global_position)
	var wheel := InputEventMouseButton.new()
	wheel.position = press.position
	wheel.button_index = MOUSE_BUTTON_WHEEL_UP
	wheel.factor = 1.0
	wheel.pressed = true
	root.push_input(wheel, true)
	for frame in range(20):
		await process_frame
	if camera.global_position.distance_to(player.global_position) >= distance_before - 0.2:
		fail("Mouse wheel failed to zoom native world camera inward")
		return false
	print("CAMERA: native mouse orbit, facing, and wheel zoom observed")
	return true

func focus_loss_stops_held_input(player: Node3D) -> bool:
	var before := player.position
	push_w(true)
	for frame in range(6):
		await process_frame
	if player.position.z > before.z - 0.1:
		fail("Focus-reset probe did not first observe held movement")
		return false
	root.focus_exited.emit()
	for frame in range(2):
		await process_frame
	var stopped := player.position
	for frame in range(6):
		await process_frame
	if player.position.distance_to(stopped) > 0.05:
		fail("Held input survived window focus loss: " + str(stopped) + " -> " + str(player.position))
		return false
	push_w(false)
	await process_frame
	return true

func inspect_character_preview(client: Node) -> bool:
	var preview := client.get_node_or_null("CharacterSelectScene/SelectedCharacter") as Node3D
	if preview == null:
		fail("Authenticated character selection has no selected-character 3D preview")
		return false
	var meshes := preview.find_children("*", "MeshInstance3D", true, false)
	var visible_meshes := 0
	for node in meshes:
		var mesh := node as MeshInstance3D
		if mesh.is_visible_in_tree() and mesh.mesh != null and mesh.mesh.get_surface_count() > 0:
			visible_meshes += 1
	var camera := root.get_camera_3d()
	if visible_meshes == 0 or camera == null:
		fail("Character preview lacks visible mesh geometry or a current camera")
		return false
	var terrain := await wait_for_character_background(client)
	if terrain == null:
		return false
	var authored := AUTHORED_CHARACTER_POSITION
	var position := preview.global_position
	if absf(position.x - authored.x) > 0.1 or absf(position.z - authored.z) > 0.1 or position.y < authored.y - 0.1:
		fail("Selected character is not at the authored first-slot location or above its floor: " + str(position))
		return false
	if absf(camera.fov - 55.0) > 0.1:
		fail("Solo character camera FOV is not 55 degrees: " + str(camera.fov))
		return false
	if client.get_node_or_null("CharacterSelectScene/CampsiteObjects") == null:
		fail("Authored campsite props and waterfall placements are absent")
		return false
	if DisplayServer.get_name() == "headless":
		fail("Character background pixel probe requires a real GPU display")
		return false
	for _frame in range(4):
		await RenderingServer.frame_post_draw
	var shown := root.get_texture().get_image()
	shown.save_png("res://../data/diagnostics/godot-conversion/character-select-preview.png")
	preview.visible = false
	for _frame in range(2):
		await RenderingServer.frame_post_draw
	var hidden := root.get_texture().get_image()
	preview.visible = true
	var changed := count_changed_pixels(shown, hidden)
	if changed < 200:
		fail("Selected character does not change rendered pixels: " + str(changed))
		return false
	terrain.visible = false
	for _frame in range(2):
		await RenderingServer.frame_post_draw
	var without_terrain := root.get_texture().get_image()
	if not preview.is_visible_in_tree():
		fail("Hiding background also hid the selected character")
		terrain.visible = true
		return false
	var background_changed := count_changed_pixels(shown, without_terrain)
	if background_changed < 200:
		fail("Authored terrain does not change GPU background pixels: " + str(background_changed))
		terrain.visible = true
		return false
	preview.visible = false
	for _frame in range(2):
		await RenderingServer.frame_post_draw
	var without_body := root.get_texture().get_image()
	preview.visible = true
	terrain.visible = true
	if count_changed_pixels(without_terrain, without_body) < 200:
		fail("Selected body stopped contributing GPU pixels when background was hidden")
		return false
	print("PASS: authenticated selected character and authored terrain change GPU pixels independently; elapsed_ms=", Time.get_ticks_msec())
	return true

func count_changed_pixels(first: Image, second: Image) -> int:
	var changed := 0
	for y in range(0, first.get_height(), 4):
		for x in range(0, first.get_width(), 4):
			if not first.get_pixel(x, y).is_equal_approx(second.get_pixel(x, y)):
				changed += 1
	return changed

func wait_for_character_background(client: Node) -> Node3D:
	var deadline := Time.get_ticks_msec() + BACKGROUND_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var terrain := client.get_node_or_null("CharacterSelectScene/WorldTerrain") as Node3D
		if terrain == null:
			continue
		var primary := terrain.get_node_or_null("Tile31_37") as Node3D
		var supplemental := terrain.get_node_or_null("Tile31_36") as Node3D
		if primary == null or supplemental == null:
			continue
		if not has_visible_mesh(primary) or not has_visible_mesh(supplemental):
			continue
		return terrain
	fail("Authenticated character selection never attached both authored terrain tiles 31_37 and 31_36")
	return null

func has_visible_mesh(tile: Node3D) -> bool:
	for node in tile.find_children("*", "MeshInstance3D", true, false):
		var mesh := node as MeshInstance3D
		if mesh.is_visible_in_tree() and mesh.mesh != null and mesh.mesh.get_surface_count() > 0:
			return true
	return false

func clear_ui_focus() -> void:
	var focused := root.gui_get_focus_owner()
	if focused != null:
		focused.release_focus()

func push_w(pressed: bool) -> void:
	var key := InputEventKey.new()
	key.physical_keycode = KEY_W
	key.pressed = pressed
	root.push_input(key, true)

func click_control(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	var press := InputEventMouseButton.new()
	press.position = point
	press.button_index = MOUSE_BUTTON_LEFT
	press.pressed = true
	root.push_input(press, true)
	await process_frame
	var release := InputEventMouseButton.new()
	release.position = point
	release.button_index = MOUSE_BUTTON_LEFT
	release.pressed = false
	root.push_input(release, true)
	await process_frame
	await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
