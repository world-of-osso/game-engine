extends SceneTree

const NAME := "Input Fixture"
const REMOTE_NAME := "Remote Fixture"
const REMOTE := Vector3(-8946.0, 114.245974, 0.0)
const UNEQUIPPED_NAME := "Unequipped Fixture"
const COLLECTION_NAME := "Collection Fixture"
const SELECTION_WAIT_MS := 30000
# Authored terrain height at this fixture's X/Z, not the transfer-only fixture's airborne Y.
const FIRST := Vector3(-8949.0, 112.879913, 0.0)
const WORLD_WAIT_MS := 60000
# Login starts once asset startup has read the CASC tables: 8.5 s warm, past 15 s on a
# loaded host or with the fixture's fresh user-data (empty CASC index cache).
const STARTUP_WAIT_MS := 120000
const LOADING_FRAMES := 24
const HELD_FRAMES := 60
const SETTLE_FRAMES := 20
const STOP_FRAMES := 45
const JUMP_WAIT_FRAMES := 360
const JUMP_HOLD_FRAMES := 6
const BACKGROUND_WAIT_MS := 30000
const AUTHORED_CHARACTER_POSITION := Vector3(-2981.82, 452.826, -457.35)

var focus_losses := 0

func _initialize() -> void:
	root.focus_exited.connect(func(): focus_losses += 1)
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
	var startup_screen := OS.get_environment("GODOT_TEST_STARTUP_SCREEN")
	var overlay_only := OS.get_environment("GODOT_TEST_OVERLAY_ONLY") == "1"
	var swimming := OS.get_environment("GODOT_TEST_SWIMMING") == "1"
	if startup_screen != "inworld":
		if not await enter_world_from_charselect(client):
			return
	if not await wait_for_screen(client, "Loading", STARTUP_WAIT_MS):
		return
	if client.get_node_or_null("CharacterSelectScene") != null:
		fail("Entering the world retained the character-selection preview")
		return
	if not client.get_node("LoadingUI").visible:
		fail("Native LoadingUI not visible while terrain is withheld")
		return
	clear_ui_focus()
	if not overlay_only:
		push_w(true)
	for frame in range(LOADING_FRAMES):
		await process_frame
		if client.account_state().screen != "Loading" or not client.get_node("LoadingUI").visible:
			fail("Loading ended before LoadTerrain at frame " + str(frame))
			return
	if not overlay_only:
		push_w(false)
	for frame in range(SETTLE_FRAMES):
		await process_frame
		if client.account_state().screen != "Loading":
			fail("Loading ended before withheld-terrain input observation at frame " + str(frame))
			return
	print("FIXTURE LOADING_OBSERVED")
	var world_wait_ms := 120000 if overlay_only else WORLD_WAIT_MS
	if not await wait_for_world(client, world_wait_ms, swimming):
		return
	if overlay_only:
		var overlay_probe = load("res://tests/wmo_shader7_authored_overlay.gd").new()
		var overlay_error: String = await overlay_probe.check(self, client)
		if overlay_error != "":
			fail(overlay_error)
			return
		print("FIXTURE OVERLAY_DONE")
		client.free()
		quit(0)
		return
	clear_ui_focus()
	var player := client.get_node_or_null("WorldUnits/" + NAME) as Node3D
	if player == null:
		fail("Native selected player missing after world readiness")
		return
	if swimming:
		var swim_probe = load("res://tests/swimming_input_probe.gd").new()
		var swim_error: String = await swim_probe.check(self, client, player)
		if swim_error != "":
			fail(swim_error)
			return
		var water: Node = client.get_node("WorldTerrain/Tile32_48/Water")
		client.free()
		if is_instance_valid(water):
			fail("Client teardown retained authored water nodes")
			return
		quit(0)
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
	var locomotion = load("res://tests/player_locomotion_probe.gd").new()
	var locomotion_error: String = locomotion.bind(player)
	if locomotion_error != "":
		fail(locomotion_error)
		return
	if locomotion.animation.current_animation_id() != 0:
		fail("Native player did not start in authored Stand 0: " + str(locomotion.animation.current_animation_id()))
		return
	var stand_pose: Array[Transform3D] = locomotion.capture_pose()
	var run_started_ms := -1
	var run_pose_changed := false
	push_w(true)
	for frame in range(HELD_FRAMES):
		await process_frame
		if client.account_state().screen != "InWorld":
			push_w(false)
			fail("World exited during held W at frame " + str(frame))
			return
		var animation_id: int = locomotion.animation.current_animation_id()
		if animation_id == 5 and run_started_ms < 0:
			run_started_ms = Time.get_ticks_msec()
		if run_started_ms >= 0 and animation_id != 5:
			push_w(false)
			fail("Held W left authored Run 5 for animation " + str(animation_id))
			return
		if run_started_ms >= 0 and Time.get_ticks_msec() - run_started_ms >= 150:
			run_pose_changed = run_pose_changed or locomotion.changed_from(stand_pose)
	if run_started_ms < 0 or locomotion.animation.current_animation_id() != 5:
		push_w(false)
		fail("Held W did not select authored Run 5: " + str(locomotion.animation.current_animation_id()))
		return
	if not run_pose_changed:
		push_w(false)
		fail("Authored Run 5 did not change PlayerModel bone pose after 150ms crossfade")
		return
	var pixels = load("res://tests/world_player_equipment_pixels.gd").new()
	var running_visual := pixels.find_visual(player) as Node3D
	var running_pixels: String = await pixels.capture_equipped(self, player, running_visual, "inworld-selected-player-running.png")
	if running_pixels != "":
		push_w(false)
		fail(running_pixels)
		return
	if locomotion.animation.current_animation_id() != 5:
		push_w(false)
		fail("Running screenshot did not retain authored Run 5")
		return
	push_w(false)
	print("FIXTURE RELEASED")
	var moved := player.position
	if moved.z > start.z - 0.1 or absf(moved.x - start.x) > 0.25:
		fail("Held W did not move native player along facing-PI direction: " + str(start) + " -> " + str(moved))
		return
	var returned_to_stand := false
	for frame in range(SETTLE_FRAMES):
		await process_frame
		if locomotion.animation.current_animation_id() == 0:
			returned_to_stand = true
	if not returned_to_stand or locomotion.animation.current_animation_id() != 0:
		fail("Released W did not return to authored Stand 0: " + str(locomotion.animation.current_animation_id()))
		return
	print("PASS: locomotion Stand 0 -> Run 5 -> Stand 0 with authored bone motion")
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
	# Z changes the original run/walk binding; restore running before the other directions.
	push_key(KEY_Z, true)
	await process_frame
	push_key(KEY_Z, false)
	await process_frame
	if not await check_locomotion_direction(client, locomotion, KEY_W, 4, "WALK"):
		return
	push_key(KEY_Z, true)
	await process_frame
	push_key(KEY_Z, false)
	await process_frame
	if not await check_locomotion_direction(client, locomotion, KEY_S, 13, "BACKWARD"):
		return
	if not await check_locomotion_direction(client, locomotion, KEY_A, 11, "LEFT"):
		return
	if not await check_locomotion_direction(client, locomotion, KEY_D, 12, "RIGHT"):
		return
	if locomotion.animation.current_animation_id() != 0:
		fail("Directional sequence did not finish at authored Stand 0")
		return
	print("FIXTURE FINAL_STAND")
	if not await check_idle_jump(client, player, locomotion):
		return
	if not await check_running_jump(client, player, locomotion):
		return
	print("FIXTURE STOPPED")
	var overlay_probe = load("res://tests/wmo_shader7_authored_overlay.gd").new()
	var overlay_error: String = await overlay_probe.check(self, client)
	if overlay_error != "":
		fail(overlay_error)
		return
	client.free()
	print("SHUTDOWN: client freed")
	quit(0)
	print("SHUTDOWN: quit requested")

func enter_world_from_charselect(client: Node) -> bool:
	if not await wait_for_screen(client, "CharacterSelect", 15000):
		return false
	var ui = client.get_node_or_null("CharacterSelectUI")
	var card = ui.find_child("CharCard_0", true, false) if ui != null else null
	if not card is Control or not card.visible:
		fail("Authenticated equipped character card missing")
		return false
	await click_control(card)
	ui = client.get_node_or_null("CharacterSelectUI")
	var initial_name = ui.find_child("CharSelectCharacterName", true, false) if ui != null else null
	var initial_highlight = ui.find_child("CharCard_0Selected", true, false) if ui != null else null
	if not initial_name is Label or initial_name.text != NAME or not initial_highlight is Control or not initial_highlight.visible:
		fail("Equipped roster character 17 is not selected")
		return false
	if not await inspect_character_preview(client):
		return false
	var equipped_model := client.get_node("CharacterSelectScene/SelectedCharacter") as Node3D
	if not await select_roster_preview(client, 1, UNEQUIPPED_NAME, weakref(equipped_model), []):
		return false
	var unequipped_model := client.get_node("CharacterSelectScene/SelectedCharacter") as Node3D
	if not await select_roster_preview(client, 0, NAME, weakref(unequipped_model), ["EquipmentMainHand", "EquipmentOffHand"]):
		return false
	var restored_model := client.get_node("CharacterSelectScene/SelectedCharacter") as Node3D
	if not await select_roster_preview(client, 2, COLLECTION_NAME, weakref(restored_model), ["EquipmentChest"]):
		return false
	var collection_model := client.get_node("CharacterSelectScene/SelectedCharacter") as Node3D
	var pose_probe = load("res://tests/equipment_pose_pixels.gd").new()
	var pose_error: String = await pose_probe.check(self, collection_model)
	if pose_error != "":
		fail(pose_error)
		return false
	if not await select_roster_preview(client, 0, NAME, weakref(collection_model), ["EquipmentMainHand", "EquipmentOffHand"]):
		return false
	var current_ui = client.get_node_or_null("CharacterSelectUI")
	var enter = current_ui.find_child("EnterWorld", true, false) if current_ui != null else null
	if not enter is Button or not enter.visible:
		fail("Enter World action missing after roster preview replacement")
		return false
	await click_control(enter)
	return true

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
	var remote := client.get_node_or_null("WorldUnits/" + REMOTE_NAME) as Node3D
	var remote_visual: Node3D = null
	deadline = Time.get_ticks_msec() + SELECTION_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if remote == null:
			remote = client.get_node_or_null("WorldUnits/" + REMOTE_NAME) as Node3D
		if remote != null:
			remote_visual = probe.find_visual(remote) as Node3D
			if remote_visual != null and probe.inspect_visual(remote_visual, true) == "":
				break
	if remote_visual == null or probe.inspect_visual(remote_visual, true) != "" or remote.position.distance_to(REMOTE) > 0.5:
		fail("Replicated remote human female lacks positioned authored body, skeleton or starter hands")
		return false
	var remote_ground = client.terrain_height_at(remote.position.x, remote.position.z)
	if remote_ground == null or absf(remote.position.y - float(remote_ground)) > 0.3:
		fail("Remote fixture is not on its authored terrain floor: " + str(remote.position) + " ground=" + str(remote_ground))
		return false
	var remote_id := remote.get_instance_id()
	var remote_hands := equipment_hands(remote_visual)
	var remote_pixels: String = await probe.capture_equipped(self, remote, remote_visual, "inworld-remote-female-player.png")
	if remote_pixels != "":
		fail(remote_pixels)
		return false
	var remote_poses: Array[Transform3D] = probe.pause_and_capture_pose(remote_visual)
	if remote_poses.is_empty():
		fail("Replicated remote human female lacks advancing authored animation")
		return false
	var pixel_error: String = await probe.capture_equipped(self, player, visual, "inworld-selected-player.png")
	if pixel_error != "":
		fail(pixel_error)
		return false
	var removed_hands := equipment_hands(visual)
	var paused_poses: Array[Transform3D] = probe.pause_and_capture_pose(visual)
	if paused_poses.is_empty():
		fail("Replicated selected Player lacks advancing authored animation")
		return false
	print("FIXTURE WORLD_READY")
	if not await wait_world_equipment(client, player, unit_id, removed_hands, false, probe, remote, remote_id, remote_visual, remote_hands, remote_poses):
		return false
	var empty_visual := probe.find_visual(player) as Node3D
	var pose_error: String = probe.compare_pose(empty_visual, paused_poses)
	if pose_error != "":
		fail(pose_error)
		return false
	print("FIXTURE EQUIPMENT_REMOVED")
	if not await wait_world_equipment(client, player, unit_id, [], true, probe, remote, remote_id, remote_visual, remote_hands, remote_poses):
		return false
	var restored := probe.find_visual(player) as Node3D
	pose_error = probe.compare_pose(restored, paused_poses)
	if pose_error != "":
		fail(pose_error)
		return false
	pixel_error = await probe.capture_equipped(self, player, restored, "inworld-selected-player-restored.png")
	if pixel_error != "":
		fail(pixel_error)
		return false
	print("FIXTURE EQUIPMENT_RESTORED")
	var local_hands := equipment_hands(restored)
	if not await wait_world_equipment(client, remote, remote_id, remote_hands, false, probe, player, unit_id, restored, local_hands, paused_poses):
		return false
	pose_error = probe.compare_pose(probe.find_visual(remote), remote_poses)
	if pose_error != "":
		fail("Remote removal: " + pose_error)
		return false
	print("FIXTURE REMOTE_EQUIPMENT_REMOVED")
	if not await wait_world_equipment(client, remote, remote_id, [], true, probe, player, unit_id, restored, local_hands, paused_poses):
		return false
	var remote_restored := probe.find_visual(remote) as Node3D
	pose_error = probe.compare_pose(remote_restored, remote_poses)
	if pose_error != "":
		fail("Remote restoration: " + pose_error)
		return false
	remote_pixels = await probe.capture_equipped(self, remote, remote_restored, "inworld-remote-female-player-restored.png")
	if remote_pixels != "":
		fail(remote_pixels)
		return false
	var local_error := unchanged_equipment(client, player, unit_id, restored, local_hands, paused_poses, probe)
	if local_error != "":
		fail("Local player changed during remote pixel proof: " + local_error)
		return false
	probe.resume_animation(restored)
	probe.resume_animation(remote_restored)
	print("FIXTURE REMOTE_EQUIPMENT_RESTORED")
	return true

func equipment_hands(visual: Node3D) -> Array:
	return [weakref(visual.find_child("EquipmentMainHand", true, false)), weakref(visual.find_child("EquipmentOffHand", true, false))]

func unchanged_equipment(client: Node, unit: Node3D, unit_id: int, visual: Node3D, hands: Array, poses: Array[Transform3D], probe) -> String:
	if client.get_node_or_null("WorldUnits/" + str(unit.name)) != unit or unit.get_instance_id() != unit_id:
		return "replicated unit identity changed: " + str(unit.name)
	if probe.find_visual(unit) != visual or probe.inspect_visual(visual, true) != "":
		return "replicated body or gear changed: " + str(unit.name)
	var hand_names := ["EquipmentMainHand", "EquipmentOffHand"]
	for index in hand_names.size():
		var hand: WeakRef = hands[index]
		if hand.get_ref() == null or visual.find_child(hand_names[index], true, false) != hand.get_ref():
			return "replicated hand node changed: " + str(unit.name) + "/" + hand_names[index]
	return probe.compare_pose(visual, poses)

func wait_world_equipment(client: Node, player: Node3D, unit_id: int, removed_hands: Array, equipped: bool, probe, other: Node3D, other_id: int, other_visual: Node3D, other_hands: Array, other_poses: Array[Transform3D]) -> bool:
	var deadline := Time.get_ticks_msec() + SELECTION_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.get_node_or_null("WorldUnits/" + str(player.name)) != player or player.get_instance_id() != unit_id:
			fail("Replicated equipment update replaced server unit node: " + str(player.name))
			return false
		var other_error := unchanged_equipment(client, other, other_id, other_visual, other_hands, other_poses, probe)
		if other_error != "":
			fail("Equipment update affected other player: " + other_error)
			return false
		var visual := probe.find_visual(player) as Node3D
		if visual == null:
			continue
		var old_hands_freed := removed_hands.all(func(hand): return hand.get_ref() == null)
		if not old_hands_freed:
			continue
		var error: String = probe.inspect_visual(visual, equipped)
		if error == "":
			print("PASS: replicated selected equipment stage equipped=", equipped, " retained unit=", unit_id)
			return true
	fail("Timed out waiting for replicated equipment stage equipped=" + str(equipped))
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

func wait_for_world(client: Node, timeout_ms: int, swimming: bool = false) -> bool:
	print("TRACE WORLD_WAIT_START elapsed_ms=", Time.get_ticks_msec())
	var deadline := Time.get_ticks_msec() + timeout_ms
	var next_report := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if Time.get_ticks_msec() >= next_report:
			print("TRACE WORLD_WAIT elapsed_ms=", Time.get_ticks_msec(), " screen=", state.screen, " units=", state.unit_count, " pending=", state.terrain.pending_count)
			next_report = Time.get_ticks_msec() + 5000
		if state.screen != "InWorld" or state.selected_character_name != NAME or state.unit_count != 2:
			continue
		var terrain: Dictionary = state.terrain
		if terrain.map != "azeroth" or terrain.pending_count != 0 or not terrain.failures.is_empty() or terrain.parsed_tiles.is_empty():
			continue
		var terrain_root = client.get_node_or_null("WorldTerrain")
		var player = client.get_node_or_null("WorldUnits/" + NAME) as Node3D
		var expected_start := Vector3(-8558.0, 144.96008, 522.0) if swimming else FIRST
		if terrain_root == null or terrain_root.get_child_count() == 0 or player == null or player.position.distance_to(expected_start) > 0.5:
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
	var locomotion = load("res://tests/player_locomotion_probe.gd").new()
	var locomotion_error: String = locomotion.bind(player)
	if locomotion_error != "":
		fail(locomotion_error)
		return false
	if not await check_idle_mouse_turn(client, player, camera, locomotion, MOUSE_BUTTON_LEFT, 12, "LEFT_ORBIT"):
		return false
	if not await check_idle_mouse_turn(client, player, camera, locomotion, MOUSE_BUTTON_RIGHT, 12, "RIGHT_DRAG_POSITIVE"):
		return false
	if not await check_idle_mouse_turn(client, player, camera, locomotion, MOUSE_BUTTON_RIGHT, -12, "RIGHT_DRAG_NEGATIVE"):
		return false
	print("CAMERA: native mouse orbit, facing, wheel zoom, and idle turns observed")
	return true

func check_idle_mouse_turn(client: Node, player: Node3D, camera: Camera3D, locomotion: RefCounted, button: MouseButton, motion_x: float, phase: String) -> bool:
	var position := player.position
	var facing := player.rotation.y
	var facing_before := facing
	var camera_before := camera.global_transform.basis.z
	var stand_pose: Array[Transform3D] = locomotion.capture_pose()
	var press := InputEventMouseButton.new()
	press.position = Vector2(640, 360)
	press.button_index = button
	press.pressed = true
	root.push_input(press, true)
	var sampled_frames := 0
	var selected_at := -1
	var changed_pose := false
	var error := ""
	for frame in range(HELD_FRAMES):
		var motion := InputEventMouseMotion.new()
		motion.position = press.position
		motion.relative = Vector2(motion_x, 0)
		root.push_input(motion, true)
		await process_frame
		var delta := wrapf(player.rotation.y - facing, -PI, PI)
		facing = player.rotation.y
		if client.account_state().screen != "InWorld" or player.position.distance_to(position) > 0.05:
			error = phase + " exited world or moved while only mouse was held"
			break
		var expected_id := 0
		if button == MOUSE_BUTTON_RIGHT and absf(delta) >= 0.02:
			expected_id = 11 if delta > 0.0 else 12
			sampled_frames += 1
		if button == MOUSE_BUTTON_LEFT and (absf(delta) >= 0.02 or locomotion.animation.current_animation_id() != 0):
			error = "Left-button orbit changed character facing or authored Stand 0: yaw delta=" + str(delta)
			break
		if expected_id != 0 and frame >= 3 and locomotion.animation.current_animation_id() != expected_id:
			error = phase + " yaw delta " + str(delta) + " expected authored turn " + str(expected_id) + " got " + str(locomotion.animation.current_animation_id()) + " at frame " + str(frame)
			break
		if expected_id != 0 and locomotion.animation.current_animation_id() == expected_id:
			if selected_at < 0:
				selected_at = Time.get_ticks_msec()
			if Time.get_ticks_msec() - selected_at >= 150:
				changed_pose = changed_pose or locomotion.changed_from(stand_pose)
	if error == "":
		for frame in range(4):
			await process_frame
		if locomotion.animation.current_animation_id() != 0:
			error = phase + " stopped mouse motion while held but did not return to Stand 0: " + str(locomotion.animation.current_animation_id())
	var release := InputEventMouseButton.new()
	release.position = press.position
	release.button_index = button
	release.pressed = false
	root.push_input(release, true)
	if error != "":
		fail(error)
		return false
	if camera_before.angle_to(camera.global_transform.basis.z) < 0.01:
		fail(phase + " mouse drag did not orbit native camera")
		return false
	if button == MOUSE_BUTTON_LEFT and absf(wrapf(player.rotation.y - facing_before, -PI, PI)) >= 0.01:
		fail("Left-button orbit changed character facing: " + str(facing_before) + " -> " + str(player.rotation.y))
		return false
	if button == MOUSE_BUTTON_RIGHT and (sampled_frames < 10 or selected_at < 0 or not changed_pose):
		fail(phase + " did not sustain yaw >= 0.02/frame, authored turn, and changed bones after 150ms: samples=" + str(sampled_frames) + " selected_at=" + str(selected_at))
		return false
	for frame in range(SETTLE_FRAMES):
		await process_frame
	if locomotion.animation.current_animation_id() != 0 or player.position.distance_to(position) > 0.05:
		fail(phase + " release did not restore Stand 0 without movement: " + str(locomotion.animation.current_animation_id()))
		return false
	print("PASS: " + phase + " samples=" + str(sampled_frames) + " selected_at_ms=" + str(selected_at) + " -> Stand 0")
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

func check_locomotion_direction(client: Node, locomotion: RefCounted, keycode: Key, animation_id: int, phase: String) -> bool:
	var stand_pose: Array[Transform3D] = locomotion.capture_pose()
	push_key(keycode, true)
	print("FIXTURE " + phase + "_START")
	var selected_at := -1
	var changed_pose := false
	for frame in range(HELD_FRAMES):
		await process_frame
		if client.account_state().screen != "InWorld":
			push_key(keycode, false)
			fail("World exited during " + phase + " at frame " + str(frame))
			return false
		var current_id: int = locomotion.animation.current_animation_id()
		if current_id == animation_id and selected_at < 0:
			selected_at = Time.get_ticks_msec()
		if selected_at >= 0 and current_id != animation_id:
			push_key(keycode, false)
			fail(phase + " left authored animation " + str(animation_id) + " for " + str(current_id) + " frame=" + str(frame) + " focus_losses=" + str(focus_losses) + " window_focused=" + str(root.has_focus()) + " focus_owner=" + str(root.gui_get_focus_owner()) + " key_pressed=" + str(Input.is_physical_key_pressed(keycode)))
			return false
		if selected_at >= 0 and Time.get_ticks_msec() - selected_at >= 150:
			changed_pose = changed_pose or locomotion.changed_from(stand_pose)
	push_key(keycode, false)
	print("FIXTURE " + phase + "_END")
	if selected_at < 0 or not changed_pose:
		fail(phase + " did not play authored " + str(animation_id) + " with changed PlayerModel bones after crossfade")
		return false
	for frame in range(STOP_FRAMES):
		await process_frame
		if client.account_state().screen != "InWorld":
			fail("World exited after " + phase + " at frame " + str(frame))
			return false
	if locomotion.animation.current_animation_id() != 0:
		fail(phase + " release did not return to authored Stand 0: " + str(locomotion.animation.current_animation_id()))
		return false
	print("PASS: " + phase + " authored " + str(animation_id) + " -> Stand 0 with changed bones")
	return true

func check_idle_jump(client: Node, player: Node3D, locomotion: RefCounted) -> bool:
	var stand_pose: Array[Transform3D] = locomotion.capture_pose()
	var ground := player.position
	var highest_y := ground.y
	var sequence := [37, 38, 39, 0]
	var next_id := 0
	var changed_pose := [false, false, false]
	var jump_frames := 0
	var released := false
	print("FIXTURE JUMP_START")
	push_key(KEY_SPACE, true)
	for frame in range(JUMP_WAIT_FRAMES):
		await process_frame
		if client.account_state().screen != "InWorld":
			push_key(KEY_SPACE, false)
			fail("World exited during idle jump at frame " + str(frame))
			return false
		var current_id: int = locomotion.animation.current_animation_id()
		if current_id != sequence[next_id]:
			if next_id == 0 and current_id == 0:
				if frame < 30:
					continue
				push_key(KEY_SPACE, false)
				fail("Idle Space did not select authored JumpStart 37: " + str(current_id))
				return false
			if next_id + 1 >= sequence.size() or current_id != sequence[next_id + 1]:
				push_key(KEY_SPACE, false)
				fail("Idle jump skipped authored animation " + str(sequence[next_id + 1]) + " for " + str(current_id))
				return false
			next_id += 1
			if current_id == 39:
				print("FIXTURE JUMP_LANDED")
		highest_y = maxf(highest_y, player.position.y)
		if next_id < 3:
			changed_pose[next_id] = changed_pose[next_id] or locomotion.changed_from(stand_pose)
		if not released:
			jump_frames += 1
			if jump_frames >= JUMP_HOLD_FRAMES:
				push_key(KEY_SPACE, false)
				released = true
				print("FIXTURE JUMP_RELEASED")
		if next_id == 3:
			break
	if not released:
		push_key(KEY_SPACE, false)
	if next_id != 3 or not changed_pose[0] or not changed_pose[1] or not changed_pose[2]:
		fail("Idle jump did not complete authored 37 -> 38 -> 39 -> 0 with changed PlayerModel bones: " + str(next_id) + " poses=" + str(changed_pose))
		return false
	if highest_y < ground.y + 0.3 or absf(player.position.y - ground.y) > 0.3 or Vector2(player.position.x, player.position.z).distance_to(Vector2(ground.x, ground.z)) > 0.1:
		fail("Idle Space did not rise and return to stationary ground: " + str(ground) + " peak=" + str(highest_y) + " final=" + str(player.position))
		return false
	print("FIXTURE JUMP_STAND")
	for frame in range(STOP_FRAMES):
		await process_frame
		if locomotion.animation.current_animation_id() != 0 or player.position.distance_to(ground) > 0.3:
			fail("Idle jump did not remain grounded in authored Stand 0")
			return false
	print("PASS: idle Space 37 -> 38 -> 39 -> 0, changing bones, rise and ground return")
	return true

func check_running_jump(client: Node, player: Node3D, locomotion: RefCounted) -> bool:
	var highest_y := player.position.y
	push_w(true)
	print("FIXTURE RUN_JUMP_RUN_START")
	for frame in range(30):
		await process_frame
		if client.account_state().screen != "InWorld" or (frame > 10 and locomotion.animation.current_animation_id() != 5):
			push_w(false)
			fail("Running jump did not establish authored Run 5 at frame " + str(frame))
			return false
	if locomotion.animation.current_animation_id() != 5:
		push_w(false)
		fail("Running jump did not begin in Run 5")
		return false
	var takeoff := player.position
	var facing := player.rotation.y + PI / 2.0
	var forward := Vector3(sin(facing), 0.0, cos(facing))
	var run_pose: Array[Transform3D] = locomotion.capture_pose()
	var sequence := [37, 38, 187, 5]
	var next_id := 0
	var changed_pose := [false, false, false]
	var hold_frames := 0
	var released := false
	push_key(KEY_SPACE, true)
	print("FIXTURE RUN_JUMP_START")
	for frame in range(JUMP_WAIT_FRAMES):
		await process_frame
		if client.account_state().screen != "InWorld":
			push_key(KEY_SPACE, false)
			push_w(false)
			fail("World exited during running jump at frame " + str(frame))
			return false
		var current_id: int = locomotion.animation.current_animation_id()
		if current_id != sequence[next_id]:
			if next_id == 0 and current_id == 5 and frame < 30:
				continue
			if next_id + 1 >= sequence.size() or current_id != sequence[next_id + 1]:
				push_key(KEY_SPACE, false)
				push_w(false)
				fail("Running jump skipped authored animation " + str(sequence[next_id + 1]) + " for " + str(current_id))
				return false
			next_id += 1
			if current_id == 187:
				print("FIXTURE RUN_JUMP_LANDED")
		highest_y = maxf(highest_y, player.position.y)
		if next_id < 3:
			changed_pose[next_id] = changed_pose[next_id] or locomotion.changed_from(run_pose)
		if not released:
			hold_frames += 1
			if hold_frames >= JUMP_HOLD_FRAMES:
				push_key(KEY_SPACE, false)
				released = true
				print("FIXTURE RUN_JUMP_RELEASED")
		if next_id == 3:
			break
	if not released:
		push_key(KEY_SPACE, false)
	if next_id != 3 or not changed_pose[0] or not changed_pose[1] or not changed_pose[2]:
		push_w(false)
		fail("Running jump did not complete 37 -> 38 -> 187 -> 5 with changing bones: " + str(next_id) + " poses=" + str(changed_pose))
		return false
	if highest_y < takeoff.y + 0.3 or (player.position - takeoff).dot(forward) <= 0.1:
		push_w(false)
		fail("Running jump did not rise and advance forward: " + str(takeoff) + " peak=" + str(highest_y) + " final=" + str(player.position))
		return false
	print("FIXTURE RUN_JUMP_RESUMED")
	var resumed_at := player.position
	for frame in range(30):
		await process_frame
		if client.account_state().screen != "InWorld" or locomotion.animation.current_animation_id() != 5:
			push_w(false)
			fail("Running jump did not retain Run 5 after landing at frame " + str(frame))
			return false
	if (player.position - resumed_at).dot(forward) <= 0.1:
		push_w(false)
		fail("Running jump did not resume forward movement: " + str(resumed_at) + " -> " + str(player.position))
		return false
	push_w(false)
	print("FIXTURE RUN_JUMP_W_RELEASED")
	for frame in range(STOP_FRAMES):
		await process_frame
		if client.account_state().screen != "InWorld":
			fail("World exited after running jump at frame " + str(frame))
			return false
	var landing_height = client.terrain_height_at(player.position.x, player.position.z)
	if landing_height == null or locomotion.animation.current_animation_id() != 0 or absf(player.position.y - float(landing_height)) > 0.3:
		fail("Running jump release did not return grounded to authored Stand 0")
		return false
	var stopped := player.position
	for frame in range(STOP_FRAMES):
		await process_frame
		if locomotion.animation.current_animation_id() != 0 or player.position.distance_to(stopped) > 0.05:
			fail("Running jump did not remain stationary in Stand 0")
			return false
	print("FIXTURE RUN_JUMP_STAND")
	print("PASS: running W+Space 37 -> 38 -> 187 -> 5 -> 0, changing bones, rise and resumed movement")
	return true

func push_w(pressed: bool) -> void:
	push_key(KEY_W, pressed)

func push_key(keycode: Key, pressed: bool) -> void:
	var key := InputEventKey.new()
	key.physical_keycode = keycode
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
