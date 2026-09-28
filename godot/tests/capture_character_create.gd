extends SceneTree

# Renders character creation after real authentication; GODOT_CAPTURE_MODE=customize
# advances to Customize first.

func _initialize() -> void:
	call_deferred("_run")

func _fail(message: String, client: Node) -> void:
	push_error(message)
	client.queue_free()
	quit(1)

func _press(ui: Node, name: String) -> void:
	ui.find_child(name, true, false).emit_signal("pressed")
	await process_frame
	await process_frame

func _run() -> void:
	var output = OS.get_environment("GODOT_CAPTURE_PATH")
	var server = OS.get_environment("GODOT_TEST_SERVER")
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if output.is_empty() or server.is_empty():
		_fail("Set GODOT_CAPTURE_PATH and GODOT_TEST_SERVER", client)
		return
	var error = client.connect_account(server, "admin", "admin", false)
	if error != "":
		_fail(error, client)
		return
	var deadline = Time.get_ticks_msec() + 15000
	while client.account_state().screen != "CharacterSelect":
		if Time.get_ticks_msec() > deadline:
			_fail("Timed out waiting for character selection", client)
			return
		await process_frame
	await _press(client.get_node("CharacterSelectUI"), "CreateChar")
	var ui = client.get_node("CharacterCreateUI")
	if OS.get_environment("GODOT_CAPTURE_MODE") == "customize":
		await _press(ui, "CharCreateNext")
	for frame in range(10):
		await process_frame
		await RenderingServer.frame_post_draw
	var image = root.get_texture().get_image()
	if image == null or image.is_empty() or image.save_png(output) != OK:
		_fail("Character creation capture requires a rendering display", client)
		return
	print("PASS: rendered character creation captured")
	client.queue_free()
	quit(0)
