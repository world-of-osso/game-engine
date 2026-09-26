extends SceneTree

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5000":
		fail("GODOT_TEST_SERVER must explicitly select 127.0.0.1:5000")
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
		await select_second_character_and_enter(client)
		return
	fail("Timed out waiting for authenticated character selection")

func select_second_character_and_enter(client: Node) -> void:
	var ui = client.get_node_or_null("CharacterSelectUI")
	var card = ui.find_child("CharCard_1", true, false) if ui != null else null
	var card_name = ui.find_child("CharCard_1Name", true, false) if ui != null else null
	var selected_name = ui.find_child("CharSelectCharacterName", true, false) if ui != null else null
	var highlight = ui.find_child("CharCard_1Selected", true, false) if ui != null else null
	var enter = ui.find_child("EnterWorld", true, false) if ui != null else null
	if not card is Control or not card.visible or not card_name is Label or not selected_name is Label or not highlight is TextureRect or not enter is Button or not enter.visible:
		fail("Authored second card or Enter World button missing")
		return
	if card_name.text.is_empty() or selected_name.text == card_name.text or highlight.visible:
		fail("Fixture must start with a different character selected")
		return
	await click_control(card)
	if selected_name.text != card_name.text or not highlight.visible:
		fail("Viewport click did not select second character: " + selected_name.text + " expected " + card_name.text)
		return
	var expected_name: String = card_name.text
	await click_control(enter)
	await assert_loading_response(client, expected_name)

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

func assert_loading_response(client: Node, expected_name: String) -> void:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "CharacterSelect":
			continue
		if state.screen != "Loading":
			fail("Enter World did not receive a successful loading transition: " + str(state))
			return
		if not state.has("selected_character_id") or not state.has("selected_character_name"):
			fail("Loading response does not expose established selected character: " + str(state))
			return
		if int(state.selected_character_id) <= 0 or state.selected_character_name != expected_name:
			fail("Response selected a different character: " + str(state) + " expected " + expected_name)
			return
		await assert_loading_ui(client)
		return
	fail("Timed out waiting for Enter World response after viewport button click")

func assert_loading_ui(client: Node) -> void:
	await process_frame
	var ui = client.get_node_or_null("LoadingUI")
	var character_ui = client.get_node_or_null("CharacterSelectUI")
	var login_ui = client.get_node_or_null("LoginUI")
	if ui == null or not ui.visible or character_ui != null and character_ui.visible or login_ui == null or login_ui.visible:
		fail("Loading must replace visible character selection and login")
		return
	var artwork = ui.find_child("LoadingArtwork", true, false)
	var shell = ui.find_child("LoadingBarBackground", true, false)
	var progress = ui.find_child("LoadingProgressText", true, false)
	if not artwork is TextureRect or artwork.texture == null or not shell is Control or not shell.visible or not progress is Label or progress.text != "0%":
		fail("Authored Loading artwork, shell or initial progress missing")
		return
	for index in range(3):
		var part = shell.get_node_or_null("ThreePart%d" % index)
		if not part is TextureRect or part.texture == null:
			fail("Authored loading shell part %d missing" % index)
			return
	for _frame in range(5):
		await process_frame
		if client.account_state().screen != "Loading" or not ui.visible or progress.text != "0%":
			fail("Loading advanced without established world readiness")
			return
	print("PASS: second selected character enters authored LoadingUI after real server response without fabricated readiness")
	client.free()
	quit(0)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
