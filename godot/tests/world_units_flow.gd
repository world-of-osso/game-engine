extends SceneTree

# Required environment inputs also apply to every inherited world flow.
# The account must have at least two characters; these flows select card 1.
var server: String
var account: String
var password: String

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	server = OS.get_environment("GODOT_TEST_SERVER")
	account = OS.get_environment("GODOT_TEST_ACCOUNT")
	password = OS.get_environment("GODOT_TEST_PASSWORD")
	if server.is_empty() or account.is_empty() or password.is_empty():
		fail("GODOT_TEST_SERVER, GODOT_TEST_ACCOUNT and GODOT_TEST_PASSWORD are required")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, password, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	# Character select mounts its UI only after asset startup finishes.
	var deadline := Time.get_ticks_msec() + 90000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if not state.reply_received:
			continue
		if state.screen != "CharacterSelect" or state.character_count < 2:
			fail("Fixture needs two authenticated characters: " + str(state))
			return
		if state.assets_starting or client.get_node_or_null("CharacterSelectUI") == null:
			continue
		await select_second_character(client)
		return
	fail("Timed out waiting for character selection")

func select_second_character(client: Node) -> void:
	var ui = client.get_node_or_null("CharacterSelectUI")
	var card = ui.find_child("CharCard_1", true, false) if ui != null else null
	var card_name = ui.find_child("CharCard_1Name", true, false) if ui != null else null
	var enter = ui.find_child("EnterWorld", true, false) if ui != null else null
	if not card is Control or not card_name is Label or not enter is Button:
		fail("Second character card or Enter World button missing")
		return
	var expected_name: String = card_name.text
	if expected_name.is_empty():
		fail("Second character name is empty")
		return
	await click_control(card)
	if client.account_state().selected_character_name == expected_name:
		fail("Selection must be established by server response, not before Enter World")
		return
	await click_control(enter)
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen not in ["Loading", "InWorld"] or state.selected_character_name != expected_name:
			continue
		var world = client.get_node_or_null("WorldUnits")
		var player = world.get_node_or_null(expected_name) if world != null else null
		if player == null:
			continue
		if not player is Node3D or state.unit_count < 1 or world.get_child_count() != state.unit_count:
			fail("Replicated units did not create matching Node3D children")
			return
		var position: Vector3 = player.position
		var yaw: float = player.rotation.y
		if not is_finite(position.x) or not is_finite(position.y) or not is_finite(position.z) or not is_finite(yaw):
			fail("Replicated local player has non-finite transform")
			return
		var instance_id := player.get_instance_id()
		for _frame in range(10):
			await process_frame
			var updated = world.get_node_or_null(expected_name)
			if updated == null or updated.get_instance_id() != instance_id:
				fail("Unit update replaced the selected character node")
				return
		var reconnect_error = client.connect_account(server, account, password, false)
		if reconnect_error != "":
			fail("Reconnect failed: " + reconnect_error)
			return
		if client.get_node_or_null("WorldUnits") != null or client.account_state().unit_count != 0:
			fail("Starting a new account connection retained previous world nodes")
			return
		print("PASS: real transport creates named stable unit Node3D and reconnect clears world nodes")
		client.free()
		quit(0)
		return
	fail("Timed out waiting for the selected character unit node")

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
