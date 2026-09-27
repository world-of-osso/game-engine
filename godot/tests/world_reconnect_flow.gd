extends SceneTree

const FIRST := Vector3(-8949.0, 83.0, 0.0)
const SECOND := Vector3(-8940.0, 83.0, 0.0)
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
	print("FIXTURE INITIAL_READY")
	if not await wait_for_world_reset(client, WAIT_MS):
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
	if client.get_node("LoadingUI").visible:
		fail("Loading screen remained visible after reconnect")
		return
	print("FIXTURE RECONNECTED")
	client.free()
	quit(0)

func wait_for_world_reset(client: Node, timeout_ms: int) -> bool:
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
		if tile_root == null or tile_root.get_child_count() == 0 or player == null or player.position.distance_to(position) > 0.5:
			continue
		for tile in terrain.parsed_tiles:
			if tile.chunk_count <= 0 or not FileAccess.file_exists(tile.root_path):
				fail("World tile lacks parsed geometry or asset cache file: " + str(tile))
				return false
		return true
	fail("Timed out waiting for selected player and refreshed world at " + str(position) + ": " + str(client.account_state()))
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
