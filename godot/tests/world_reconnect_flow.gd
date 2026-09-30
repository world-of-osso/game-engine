extends SceneTree

const FIRST := Vector3(-8949.0, 112.87991, 0.0)
const SECOND := Vector3(-8940.0, 117.38283, 0.0)
const NAME := "Reconnect Fixture"
const WAIT_MS := 60000

func _initialize() -> void:
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
		fail("Initial credentials connection: " + error)
		return
	if not await wait_for_screen(client, "CharacterSelect", 15000):
		return
	var ui = client.get_node_or_null("CharacterSelectUI")
	var card = ui.find_child("CharCard_0", true, false) if ui != null else null
	var enter = ui.find_child("EnterWorld", true, false) if ui != null else null
	if not card is Control or not enter is Button or not card.visible or not enter.visible:
		fail("Authenticated selected character card and Enter World action missing")
		return
	await click_control(card)
	await click_control(enter)
	if not await wait_for_world(client, FIRST, WAIT_MS):
		return
	var first_terrain = client.get_node_or_null("WorldTerrain")
	var first_player = client.get_node_or_null("WorldUnits/" + NAME)
	if first_terrain == null or first_player == null:
		fail("Initial world lacks terrain or selected player")
		return
	var first_terrain_id: int = first_terrain.get_instance_id()
	var first_player_id: int = first_player.get_instance_id()
	var first_water := water_identity(client)
	if first_water.is_empty():
		return
	print("FIXTURE INITIAL_READY")
	if not await wait_for_world_reset(client, first_water, 90000):
		return
	print("FIXTURE WORLD_RESET")
	if not await wait_for_terrain_refresh(client, WAIT_MS):
		return
	print("FIXTURE TERRAIN_REFRESHED")
	if not await wait_for_world(client, SECOND, WAIT_MS):
		return
	var state: Dictionary = client.account_state()
	var terrain = client.get_node_or_null("WorldTerrain")
	var player = client.get_node_or_null("WorldUnits/" + NAME)
	if state.get("reconnect_phase") != "Inactive" or state.get("gameplay_input_allowed") != true:
		fail("Reconnect did not restore input after terrain and local player: " + str(state))
		return
	if terrain == null or terrain.get_instance_id() == first_terrain_id or terrain.get_child_count() == 0:
		fail("Reconnect reused old terrain root or has no refreshed tile")
		return
	if player == null or player.get_instance_id() == first_player_id:
		fail("Reconnect retained old local player node")
		return
	var refreshed_water := water_identity(client)
	if refreshed_water.is_empty():
		return
	if refreshed_water.node_id == first_water.node_id or refreshed_water.material_id == first_water.material_id:
		fail("Reconnect reused old water node or shader material")
		return
	if not await wait_for_water_clock(client):
		return
	if OS.get_environment("GODOT_TEST_VISUAL") == "1":
		var water_pixels = load("res://tests/adt_water_pixels.gd").new()
		var water_error: String = await water_pixels.check(self, client, "reconnect-water-isolated.png")
		if water_error != "":
			fail(water_error)
			return
		print("PASS: RECONNECT_WATER_PIXELS")
	if client.get_node("LoadingUI").visible:
		fail("Loading screen remained visible after reconnect")
		return
	print("FIXTURE RECONNECTED")
	client.free()
	quit(0)

func water_identity(client: Node) -> Dictionary:
	var water = client.get_node_or_null("WorldTerrain/Tile32_48/Water")
	if water == null:
		fail("Loaded tile 32_48 has no authored Water node")
		return {}
	for child in water.get_children():
		if child is MeshInstance3D and child.mesh != null and child.mesh.get_surface_count() > 0:
			var material = child.get_surface_override_material(0)
			if material is ShaderMaterial:
				return {
					"node_ref": weakref(water), "node_id": water.get_instance_id(),
					"material_ref": weakref(material), "material_id": material.get_instance_id()
				}
	fail("Authored Water has no nonempty mesh with a ShaderMaterial")
	return {}

func wait_for_water_clock(client: Node) -> bool:
	var clock = root.get_node("M2MaterialClock")
	var previous_time := -1.0
	var previous_sample := -1.0
	for frame in range(12):
		await process_frame
		var water = client.get_node_or_null("WorldTerrain/Tile32_48/Water")
		if water == null:
			fail("Refreshed Water disappeared during clock sampling")
			return false
		var material = water.get_child(0).get_surface_override_material(0) as ShaderMaterial
		if material == null:
			fail("Refreshed Water lost its shader material")
			return false
		var expected: float = fmod(clock.elapsed_time_ms() / 1000.0, 3600.0)
		# LiquidSurface::set_time writes milliseconds wrapped at one hour.
		var sampled: float = material.get_shader_parameter("animation_time_ms") / 1000.0
		if frame > 0 and absf(sampled - expected) > 0.2:
			fail("Refreshed Water clock diverged: sampled=%s expected=%s" % [sampled, expected])
			return false
		if frame > 0 and expected > previous_time and sampled > previous_sample and sampled > 0.0:
			return true
		previous_time = expected
		previous_sample = sampled
	fail("Refreshed Water did not advance with shared material clock")
	return false

func wait_for_world_reset(client: Node, first_water: Dictionary, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.get("screen") == "Login":
			fail("Ordinary transport loss returned to Login rather than automatic reconnect: " + str(state))
			return false
		if state.get("reconnect_phase") != "PendingConnect":
			continue
		var terrain: Dictionary = state.terrain
		if state.unit_count != 0 or not terrain.map.is_empty() or terrain.pending_count != 0 or not terrain.parsed_tiles.is_empty():
			fail("Reconnect retained stale unit or world terrain: " + str(state))
			return false
		if client.get_node_or_null("WorldTerrain") != null or client.get_node_or_null("WorldUnits/" + NAME) != null:
			fail("Reconnect retained stale world nodes")
			return false
		if client.get_node_or_null("WorldTerrain/Tile32_48/Water") != null:
			fail("Reconnect retained old authored Water in scene tree")
			return false
		for frame in range(120):
			if first_water.node_ref.get_ref() == null and first_water.material_ref.get_ref() == null:
				break
			await process_frame
		if first_water.node_ref.get_ref() != null or first_water.material_ref.get_ref() != null:
			fail("World reset retained old Water node or cached shader material")
			return false
		if state.get("gameplay_input_allowed") != false or state.selected_character_name != NAME:
			fail("Reconnect lost selected character or accepted gameplay input: " + str(state))
			return false
		return true
	fail("No automatic reconnect phase/world reset after ordinary UDP timeout (missing phase key means old DLL): " + str(client.account_state()))
	return false

func wait_for_terrain_refresh(client: Node, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.get("reconnect_phase") != "AwaitingWorld":
			continue
		var terrain: Dictionary = state.terrain
		if terrain.map != "azeroth":
			continue
		if state.selected_character_name != NAME or state.selected_character_id != 17:
			fail("Reordered roster changed selected character: " + str(state))
			return false
		if state.unit_count != 0 or state.get("gameplay_input_allowed") != false:
			fail("Gameplay resumed before reconnect local unit: " + str(state))
			return false
		return true
	fail("No automatic token authentication, selected character, and terrain refresh: " + str(client.account_state()))
	return false

func wait_for_screen(client: Node, wanted: String, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == wanted:
			if not state.reply_received or state.character_count != 2:
				fail("Initial credentials login did not populate two-character roster: " + str(state))
				return false
			return true
	fail("Timed out waiting for " + wanted + ": " + str(client.account_state()))
	return false

func wait_for_world(client: Node, position: Vector3, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen != "InWorld" or state.selected_character_name != NAME or state.unit_count != 1:
			continue
		var terrain: Dictionary = state.terrain
		if terrain.map != "azeroth" or terrain.pending_count != 0 or not terrain.failures.is_empty() or terrain.parsed_tiles.is_empty():
			continue
		var tile_root = client.get_node_or_null("WorldTerrain")
		var player = client.get_node_or_null("WorldUnits/" + NAME) as Node3D
		if tile_root == null or tile_root.get_child_count() == 0 or player == null:
			continue
		if Vector2(player.position.x, player.position.z).distance_to(Vector2(position.x, position.z)) > 0.5:
			continue
		var ground = client.terrain_height_at(player.position.x, player.position.z)
		if ground == null or absf(player.position.y - float(ground)) >= 0.3:
			continue
		for tile in terrain.parsed_tiles:
			if tile.chunk_count <= 0 or not FileAccess.file_exists(tile.root_path):
				fail("World tile lacks parsed geometry or asset cache file: " + str(tile))
				return false
		return true
	fail("Timed out waiting for selected grounded player and refreshed world near " + str(position) + ": " + str(client.account_state()))
	return false

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
