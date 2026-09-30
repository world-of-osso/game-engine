extends SceneTree

const FIRST := Vector3(-8949.0, 112.87991, 0.0)
const SECOND := Vector3(-8940.0, 117.38283, 0.0)
const ERROR_TEXT := "Transfer Aborted: instance is full"

var transfer_requested := false
var transfer_loading_seen := false
var transfer_loading_error := ""

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
	client.screen_requested.connect(on_screen_requested.bind(client))
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
	await click_control(enter)
	if not await wait_for_world(client, FIRST, 60000):
		return
	var first_terrain = client.get_node_or_null("WorldTerrain")
	var first_player = client.get_node_or_null("WorldUnits/Transfer Fixture")
	if first_terrain == null or first_player == null:
		fail("Initial world has no native terrain and selected player")
		return
	var first_terrain_id: int = first_terrain.get_instance_id()
	var first_tile_id: int = first_terrain.get_child(0).get_instance_id()
	var first_player_id: int = first_player.get_instance_id()
	var first_water := water_identity(client)
	if first_water.is_empty():
		return
	transfer_requested = true
	print("FIXTURE INITIAL_READY")
	var deadline := Time.get_ticks_msec() + 60000
	var water_released := false
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if transfer_loading_error != "":
			fail(transfer_loading_error)
			return
		if not transfer_loading_seen:
			continue
		if not water_released:
			if not await wait_for_water_release(first_water):
				return
			water_released = true
		var state: Dictionary = client.account_state()
		var overlay = client.get_node_or_null("UIErrors")
		var label = overlay.find_child("UIErrorsFrameLine1", true, false) if overlay != null else null
		if state.screen != "InWorld":
			continue
		if not world_ready(client, SECOND):
			continue
		var terrain = client.get_node_or_null("WorldTerrain")
		var player = client.get_node_or_null("WorldUnits/Transfer Fixture")
		if terrain == null or terrain.get_instance_id() == first_terrain_id or terrain.get_child_count() == 0 or terrain.get_child(0).get_instance_id() == first_tile_id:
			fail("Same-map transfer reused old terrain root or tile")
			return
		if player == null or player.get_instance_id() != first_player_id:
			fail("Transfer lost or duplicated the selected player Node3D")
			return
		if absf(player.rotation.y - 0.5) > 0.001:
			fail("Transfer did not apply server-authored model facing")
			return
		if overlay == null or not label is Label or not label.is_visible_in_tree() or label.text != ERROR_TEXT:
			fail("Native UIErrors overlay lost authored transfer error")
			return
		if client.get_node("LoadingUI").visible:
			fail("Loading screen remained visible after terrain readiness")
			return
		var refreshed_water := water_identity(client)
		if refreshed_water.is_empty():
			return
		if refreshed_water.node_id == first_water.node_id or refreshed_water.material_id == first_water.material_id:
			fail("Transfer reused old Water node or shader material")
			return
		if not await wait_for_water_clock(refreshed_water):
			return
		if OS.get_environment("GODOT_TEST_VISUAL") == "1":
			var water_pixels = load("res://tests/adt_water_pixels.gd").new()
			var water_error: String = await water_pixels.check(self, client, "transfer-water-isolated.png")
			if water_error != "":
				fail(water_error)
				return
			print("PASS: TRANSFER_WATER_PIXELS")
		for _frame in range(60):
			await process_frame
			if client.account_state().screen != "InWorld" or client.account_state().unit_count != 1:
				fail("Transfer did not retain the live connected world")
				return
		print("FIXTURE TRANSFER_WATER_READY")
		print("FIXTURE TRANSFER_READY")
		client.free()
		quit(0)
		return
	fail("Timed out waiting for transfer/loading/error/readiness: " + str(client.account_state()))

func on_screen_requested(screen: String, client: Node) -> void:
	if not transfer_requested or screen != "Loading" or transfer_loading_seen:
		return
	transfer_loading_seen = true
	var ui = client.get_node_or_null("LoadingUI")
	if ui == null or not ui.visible:
		transfer_loading_error = "Transfer did not show native LoadingUI"
		return
	var old_water = client.get_node_or_null("WorldTerrain/Tile32_48/Water")
	if old_water != null:
		transfer_loading_error = "NewWorld retained old authored Water at Loading boundary"
		return
	print("FIXTURE TRANSFER_LOADING")

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

func wait_for_water_release(first_water: Dictionary) -> bool:
	for _frame in range(120):
		if first_water.node_ref.get_ref() == null and first_water.material_ref.get_ref() == null:
			return true
		await process_frame
	fail("NewWorld retained old Water node or cached shader material")
	return false

func wait_for_water_clock(water: Dictionary) -> bool:
	var clock = root.get_node("M2MaterialClock")
	var previous_time := -1.0
	var previous_sample := -1.0
	for frame in range(12):
		await process_frame
		var node = water.node_ref.get_ref()
		var material = water.material_ref.get_ref() as ShaderMaterial
		if node == null or material == null:
			fail("Destination Water or its shader material disappeared during clock sampling")
			return false
		var expected: float = fmod(clock.elapsed_time_ms() / 1000.0, 3600.0)
		# LiquidSurface::set_time writes milliseconds wrapped at one hour.
		var sampled: float = material.get_shader_parameter("animation_time_ms") / 1000.0
		if frame > 0 and absf(sampled - expected) > 0.2:
			fail("Destination Water clock diverged: sampled=%s expected=%s" % [sampled, expected])
			return false
		if frame > 0 and expected > previous_time and sampled > previous_sample and sampled > 0.0:
			return true
		previous_time = expected
		previous_sample = sampled
	fail("Destination Water did not advance with shared material clock")
	return false

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

func wait_for_world(client: Node, position: Vector3, timeout_ms: int) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if world_ready(client, position):
			return true
	fail("Timed out waiting for native world at " + str(position) + ": " + str(client.account_state()))
	return false

func world_ready(client: Node, position: Vector3) -> bool:
	var state: Dictionary = client.account_state()
	if state.screen != "InWorld" or state.selected_character_name != "Transfer Fixture" or state.unit_count != 1:
		return false
	var terrain: Dictionary = state.terrain
	if terrain.map != "azeroth" or terrain.pending_count != 0 or not terrain.failures.is_empty() or terrain.parsed_tiles.size() == 0:
		return false
	var root = client.get_node_or_null("WorldTerrain")
	var player = client.get_node_or_null("WorldUnits/Transfer Fixture") as Node3D
	if root == null or root.get_child_count() == 0 or player == null or player.position.distance_to(position) > 0.5:
		return false
	for tile in terrain.parsed_tiles:
		if tile.chunk_count <= 0 or not FileAccess.file_exists(tile.root_path):
			return false
	return true

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
