extends SceneTree

# Real-server character select actions: keyboard roster navigation, the delete
# confirmation gate (cancelled, never confirmed), and Create New Character navigation.

func _initialize() -> void:
	call_deferred("run")

func fail(message: String, client: Node) -> void:
	push_error(message)
	client.queue_free()
	quit(1)

func press(keycode: Key) -> void:
	var key := InputEventKey.new()
	key.keycode = keycode
	key.pressed = true
	root.push_input(key, true)
	await process_frame
	await process_frame

func run() -> void:
	root.size = Vector2i(1280, 720)
	var server = OS.get_environment("GODOT_TEST_SERVER")
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if server.is_empty():
		fail("GODOT_TEST_SERVER must select a local server", client)
		return
	var error = client.connect_account(server, "admin", "admin", false)
	if error != "":
		fail(error, client)
		return
	var deadline = Time.get_ticks_msec() + 15000
	while client.account_state().screen != "CharacterSelect":
		if Time.get_ticks_msec() > deadline:
			fail("Timed out waiting for character selection", client)
			return
		await process_frame
	var ui = client.get_node("CharacterSelectUI")
	if client.account_state().character_count < 2:
		fail("Fixture account needs at least two characters", client)
		return
	var first = ui.frame_text("CharSelectCharacterName")
	await press(KEY_DOWN)
	var second = ui.frame_text("CharSelectCharacterName")
	await press(KEY_UP)
	if second == first or ui.frame_text("CharSelectCharacterName") != first:
		fail("Up/Down must move the roster selection: %s -> %s -> %s" % [first, second, ui.frame_text("CharSelectCharacterName")], client)
		return

	ui.find_child("DeleteChar", true, false).emit_signal("pressed")
	await process_frame
	await process_frame
	var input = ui.find_child("DeleteCharacterConfirmInput", true, false)
	var confirm = ui.find_child("DeleteCharacterConfirmButton", true, false)
	if input == null or confirm == null or not input.has_focus():
		fail("Delete must open the focused confirmation dialog", client)
		return
	input.insert_text_at_caret("delete")
	input.text_changed.emit(input.text)
	await process_frame
	await process_frame
	if input.text != "DELETE" or not confirm.disabled:
		fail("Typed token must upper-case and stay locked during the delay: text=%s disabled=%s" % [input.text, confirm.disabled], client)
		return
	var unlock = Time.get_ticks_msec() + 3500
	while Time.get_ticks_msec() < unlock:
		await process_frame
	if confirm.disabled:
		fail("Confirmation must unlock after the original 3s delay with DELETE typed", client)
		return
	input.release_focus()
	await press(KEY_ESCAPE)
	if ui.find_child("DeleteCharacterDialog", true, false) != null:
		fail("Escape must cancel the pending deletion", client)
		return

	ui.find_child("CreateChar", true, false).emit_signal("pressed")
	await process_frame
	await process_frame
	var create = client.get_node_or_null("CharacterCreateUI")
	if client.account_state().screen != "CharacterCreate" or create == null or not create.visible:
		fail("Create New Character must open character creation", client)
		return
	create.find_child("CharCreateBack", true, false).emit_signal("pressed")
	await process_frame
	await process_frame
	if client.account_state().screen != "CharacterSelect" or not client.get_node("CharacterSelectUI").visible:
		fail("Creation Back must return to character selection", client)
		return
	print("PASS: roster keys, delete confirmation gate and creation navigation")
	client.queue_free()
	quit(0)
