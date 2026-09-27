extends SceneTree

const NAME := "Input Fixture"
const UNEQUIPPED_NAME := "Unequipped Fixture"
const COLLECTION_NAME := "Collection Fixture"
const SELECTION_WAIT_MS := 30000
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
	if not card is Control or not card.visible:
		fail("Authenticated equipped character card missing")
		return
	await click_control(card)
	ui = client.get_node_or_null("CharacterSelectUI")
	var initial_name = ui.find_child("CharSelectCharacterName", true, false) if ui != null else null
	var initial_highlight = ui.find_child("CharCard_0Selected", true, false) if ui != null else null
	if not initial_name is Label or initial_name.text != NAME or not initial_highlight is Control or not initial_highlight.visible:
		fail("Equipped roster character 17 is not selected")
		return
	if not await inspect_character_preview(client):
		return
	var equipped_model := client.get_node("CharacterSelectScene/SelectedCharacter") as Node3D
	if not await select_roster_preview(client, 1, UNEQUIPPED_NAME, weakref(equipped_model), []):
		return
	var unequipped_model := client.get_node("CharacterSelectScene/SelectedCharacter") as Node3D
	if not await select_roster_preview(client, 0, NAME, weakref(unequipped_model), ["EquipmentMainHand", "EquipmentOffHand"]):
		return
	var restored_model := client.get_node("CharacterSelectScene/SelectedCharacter") as Node3D
	if not await select_roster_preview(client, 2, COLLECTION_NAME, weakref(restored_model), ["EquipmentChest"]):
		return
	var collection_model := client.get_node("CharacterSelectScene/SelectedCharacter") as Node3D
	var pose_probe = load("res://tests/equipment_pose_pixels.gd").new()
	var pose_error: String = await pose_probe.check(self, collection_model)
	if pose_error != "":
		fail(pose_error)
		return
	if not await select_roster_preview(client, 0, NAME, weakref(collection_model), ["EquipmentMainHand", "EquipmentOffHand"]):
		return
	var current_ui = client.get_node_or_null("CharacterSelectUI")
	var enter = current_ui.find_child("EnterWorld", true, false) if current_ui != null else null
	if not enter is Button or not enter.visible:
		fail("Enter World action missing after roster preview replacement")
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
	if not await inspect_ground_collision(client, start):
		return
	if not await inspect_world_equipment(client, player):
		return
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

func inspect_world_equipment(client: Node, player: Node3D) -> bool:
	var probe = load("res://tests/world_player_equipment_pixels.gd").new()
	var unit_id := player.get_instance_id()
	var visual: Node3D = null
	var deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		visual = probe.find_visual(player) as Node3D
		if visual != null and probe.inspect_visual(visual, true) == "":
			break
	if visual == null or probe.inspect_visual(visual, true) != "":
		fail("Replicated selected Player lacks visible authored body, skeleton or starter hands")
		return false
	var pixel_error: String = await probe.capture_initial(self, player, visual)
	if pixel_error != "":
		fail(pixel_error)
		return false
	var old_visual := weakref(visual)
	print("FIXTURE WORLD_READY")
	if not await wait_world_equipment(client, player, unit_id, old_visual, false, probe):
		return false
	var empty_visual := probe.find_visual(player) as Node3D
	var empty_ref := weakref(empty_visual)
	print("FIXTURE EQUIPMENT_REMOVED")
	if not await wait_world_equipment(client, player, unit_id, empty_ref, true, probe):
		return false
	print("FIXTURE EQUIPMENT_RESTORED")
	return true

func wait_world_equipment(client: Node, player: Node3D, unit_id: int, old_visual: WeakRef, equipped: bool, probe) -> bool:
	var deadline := Time.get_ticks_msec() + SELECTION_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.get_node_or_null("WorldUnits/" + NAME) != player or player.get_instance_id() != unit_id:
			fail("Replicated equipment update replaced selected server unit node")
			return false
		var visual := probe.find_visual(player) as Node3D
		if visual == null or old_visual.get_ref() != null:
			continue
		var error: String = probe.inspect_visual(visual, equipped)
		if error == "":
			print("PASS: replicated selected equipment stage equipped=", equipped, " retained unit=", unit_id)
			return true
	fail("Timed out waiting for replicated equipment stage equipped=" + str(equipped) + " old_valid=" + str(old_visual.get_ref() != null))
	return false

func wait_for_screen(client: Node, wanted: String, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == wanted:
			if wanted == "CharacterSelect" and (not state.reply_received or state.character_count != 3):
				fail("Fixture authentication did not populate character selection: " + str(state))
				return false
			return true
	fail("Timed out waiting for " + wanted + ": " + str(client.account_state()))
	return false

func select_roster_preview(client: Node, index: int, expected_name: String, old_model: WeakRef, required_equipment: Array[String]) -> bool:
	var ui = client.get_node_or_null("CharacterSelectUI")
	var card = ui.find_child("CharCard_" + str(index), true, false) if ui != null else null
	if not card is Control or not card.visible:
		fail("Character card missing for " + expected_name)
		return false
	await click_control(card)
	var deadline := Time.get_ticks_msec() + SELECTION_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		ui = client.get_node_or_null("CharacterSelectUI")
		var selected_name = ui.find_child("CharSelectCharacterName", true, false) if ui != null else null
		var selected_card = ui.find_child("CharCard_" + str(index) + "Selected", true, false) if ui != null else null
		var preview := client.get_node_or_null("CharacterSelectScene/SelectedCharacter") as Node3D
		if not selected_name is Label or selected_name.text != expected_name or not selected_card is Control or not selected_card.visible or old_model.get_ref() != null or preview == null:
			continue
		await RenderingServer.frame_post_draw
		var equipment := preview.find_children("Equipment*", "Node3D", true, false)
		if equipment.size() != required_equipment.size():
			continue
		var names := []
		for item in equipment:
			names.append(str(item.name))
		if not required_equipment.all(func(item): return names.has(item)):
			continue
		print("PASS: selected ", expected_name, " replaced prior model; equipment nodes=", equipment.size())
		return true
	fail("Timed out replacing selected preview with " + expected_name + "; old_valid=" + str(old_model.get_ref() != null))
	return false

func wait_for_world(client: Node, timeout_ms: int) -> bool:
	print("TRACE WORLD_WAIT_START elapsed_ms=", Time.get_ticks_msec())
	var deadline := Time.get_ticks_msec() + timeout_ms
	var next_report := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if Time.get_ticks_msec() >= next_report:
			print("TRACE WORLD_WAIT elapsed_ms=", Time.get_ticks_msec(), " screen=", state.screen, " units=", state.unit_count, " pending=", state.terrain.pending_count)
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
	print("TRACE PREVIEW_ENTRY elapsed_ms=", Time.get_ticks_msec())
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
	print("TRACE TERRAIN_READY elapsed_ms=", Time.get_ticks_msec())
	var authored := AUTHORED_CHARACTER_POSITION
	var position := preview.global_position
	if absf(position.x - authored.x) > 0.1 or absf(position.z - authored.z) > 0.1 or position.y < authored.y - 0.1:
		fail("Selected character is not at the authored first-slot location or above its floor: " + str(position))
		return false
	if not await inspect_ground_collision(client, position):
		return false
	if absf(camera.fov - 55.0) > 0.1:
		fail("Solo character camera FOV is not 55 degrees: " + str(camera.fov))
		return false
	var objects := client.get_node_or_null("CharacterSelectScene/CampsiteObjects") as Node3D
	if objects == null:
		fail("Authored campsite props and waterfall placements are absent")
		return false
	if objects.find_children("Doodad*", "Node3D", false, false).size() != 118 or objects.get_node_or_null("Wmo48366671") == null:
		fail("Campsite lacks its 76 primary + 42 supplemental doodads or nearby WMO")
		return false
	var sky := client.get_node_or_null("CharacterSelectScene/AuthoredSky525142") as Node3D
	if sky == null:
		fail("Original campsite sky model is absent")
		return false
	var main_hand := preview.find_child("EquipmentMainHand", true, false) as Node3D
	var off_hand := preview.find_child("EquipmentOffHand", true, false) as Node3D
	if main_hand == null or off_hand == null:
		fail("Equipped starter sword and shield are missing from the selected character")
		return false
	if DisplayServer.get_name() == "headless":
		fail("Character background pixel probe requires a real GPU display")
		return false
	print("TRACE PIXEL_FREEZE elapsed_ms=", Time.get_ticks_msec())
	paused = true
	for _frame in range(4):
		await RenderingServer.frame_post_draw
	var shown := root.get_texture().get_image()
	print("TRACE PIXEL_CAPTURED elapsed_ms=", Time.get_ticks_msec())
	shown.save_png("res://../data/diagnostics/godot-conversion/character-select-preview.png")
	main_hand.visible = false
	off_hand.visible = false
	for _frame in range(2):
		await RenderingServer.frame_post_draw
	var without_weapons := root.get_texture().get_image()
	main_hand.visible = true
	off_hand.visible = true
	if count_changed_pixels(shown, without_weapons) < 5:
		fail("Equipped starter sword and shield do not contribute rendered pixels")
		return false
	print("PASS: equipped starter weapons change selected-character pixels")
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
	objects.visible = false
	for _frame in range(2):
		await RenderingServer.frame_post_draw
	var without_objects := root.get_texture().get_image()
	objects.visible = true
	if count_changed_pixels(shown, without_objects) < 200:
		fail("Authored campsite props and waterfall do not contribute visible pixels")
		return false
	print("TRACE SKY_HIDE elapsed_ms=", Time.get_ticks_msec())
	sky.visible = false
	for _frame in range(2):
		await RenderingServer.frame_post_draw
	var without_sky := root.get_texture().get_image()
	print("TRACE SKY_CAPTURED elapsed_ms=", Time.get_ticks_msec())
	sky.visible = true
	var sky_pixels := count_changed_pixels(shown, without_sky)
	# Authored mountains occlude almost all sky in the solo camera. Test its
	# contribution with foreground hidden rather than requiring a new framing.
	terrain.visible = false
	objects.visible = false
	preview.visible = false
	for _frame in range(2):
		await RenderingServer.frame_post_draw
	var isolated_sky := root.get_texture().get_image()
	isolated_sky.save_png("res://../data/diagnostics/godot-conversion/character-select-isolated-sky.png")
	sky.visible = false
	for _frame in range(2):
		await RenderingServer.frame_post_draw
	var isolated_baseline := root.get_texture().get_image()
	isolated_baseline.save_png("res://../data/diagnostics/godot-conversion/character-select-isolated-baseline.png")
	var isolated_pixels := count_changed_pixels(isolated_sky, isolated_baseline)
	sky.visible = true
	preview.visible = true
	objects.visible = true
	terrain.visible = true
	paused = false
	if isolated_pixels < 200:
		fail("Original campsite sky does not render with foreground hidden: " + str(isolated_pixels))
		return false
	print("PASS: selected body, terrain, props and isolated sky change GPU pixels independently; sky_full=", sky_pixels, " sky_isolated=", isolated_pixels, " elapsed_ms=", Time.get_ticks_msec())
	return true

func inspect_ground_collision(client: Node3D, point: Vector3) -> bool:
	await physics_frame
	await process_frame
	var query := PhysicsRayQueryParameters3D.create(point + Vector3.UP * 2.0, point - Vector3.UP * 2.0)
	var hit := client.get_world_3d().direct_space_state.intersect_ray(query)
	if hit.is_empty() or absf(hit.position.y - point.y) > 0.3 or hit.normal.y <= 0.0:
		fail("Authored terrain collision misses the grounded character: " + str(point) + " hit=" + str(hit))
		return false
	print("PASS: authored terrain ray hit at grounded character ", point)
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
