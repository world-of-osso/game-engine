extends SceneTree

const FIRST := Vector3(-8949.0, 83.0, 0.0)
const SECOND := Vector3(-8940.0, 83.0, 0.0)
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
	var first_terrain_id := first_terrain.get_instance_id()
	var first_tile_id := first_terrain.get_child(0).get_instance_id()
	var first_player_id := first_player.get_instance_id()
	transfer_requested = true
	print("FIXTURE INITIAL_READY")
	var deadline := Time.get_ticks_msec() + 60000
	var saw_error_while_loading := false
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if transfer_loading_error != "":
			fail(transfer_loading_error)
			return
		if not transfer_loading_seen:
			continue
		var state: Dictionary = client.account_state()
		var overlay = client.get_node_or_null("UIErrors")
		var label = overlay.find_child("UIErrorsFrameLine1", true, false) if overlay != null else null
		if label is Label and label.visible and label.text == ERROR_TEXT and state.screen == "Loading":
			saw_error_while_loading = true
		if state.screen != "InWorld" or not saw_error_while_loading:
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
		if overlay == null or not label is Label or label.text != ERROR_TEXT:
			fail("Native UIErrors overlay lost authored transfer error")
			return
		if client.get_node("LoadingUI").visible:
			fail("Loading screen remained visible after terrain readiness")
			return
		for _frame in range(60):
			await process_frame
			if client.account_state().screen != "InWorld" or client.account_state().unit_count != 1:
				fail("Transfer did not retain the live connected world")
				return
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
	print("FIXTURE TRANSFER_LOADING")

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
