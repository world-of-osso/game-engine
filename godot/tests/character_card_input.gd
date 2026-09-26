extends SceneTree

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server.is_empty():
		fail("GODOT_TEST_SERVER must select a local server")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, "admin", "admin", false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if not state.reply_received:
			continue
		if state.screen != "CharacterSelect" or state.character_count < 2:
			fail("Fixture needs successful auth and two characters: " + str(state))
			return
		var ui = client.get_node_or_null("CharacterSelectUI")
		var card = ui.find_child("CharCard_1", true, false) if ui != null else null
		var name_label = ui.find_child("CharSelectCharacterName", true, false) if ui != null else null
		var card_name = ui.find_child("CharCard_1Name", true, false) if ui != null else null
		var selected = ui.find_child("CharCard_1Selected", true, false) if ui != null else null
		if not card is Control or not card.visible or not name_label is Label or not card_name is Label or not selected is TextureRect:
			fail("Authored character card and selection controls missing")
			return
		if name_label.text == card_name.text or selected.visible:
			fail("Fixture must begin with character 0 selected")
			return
		var point: Vector2 = card.get_global_rect().get_center()
		var press := InputEventMouseButton.new()
		press.position = point
		press.button_index = MOUSE_BUTTON_LEFT
		press.pressed = true
		root.push_input(press, true)
		await process_frame
		if name_label.text != card_name.text or not selected.visible:
			fail("Viewport left press did not select second card: selected=" + name_label.text + " expected=" + card_name.text + " highlight=" + str(selected.visible))
			return
		var release := InputEventMouseButton.new()
		release.position = point
		release.button_index = MOUSE_BUTTON_LEFT
		release.pressed = false
		root.push_input(release, true)
		await process_frame
		await process_frame
		if name_label.text != card_name.text or not selected.visible:
			fail("Releasing left button lost second-card selection")
			return
		client.free()
		print("PASS: viewport left press selects authored second character card")
		quit(0)
		return
	fail("Timed out waiting for authenticated character selection")

func fail(message: String) -> void:
	push_error(message)
	quit(1)
