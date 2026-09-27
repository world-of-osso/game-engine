extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _fail(message: String, client: Node) -> void:
	push_error(message)
	client.queue_free()
	quit(1)

func _capture(path: String) -> bool:
	for frame in range(3):
		await process_frame
		await RenderingServer.frame_post_draw
	var image = root.get_texture().get_image()
	if image == null or image.is_empty():
		return false
	return image.save_png(path) == OK

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
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.account_state().screen == "CharacterSelect":
			# Let the character preview settle before capture.
			for frame in range(30):
				await process_frame
			if not await _capture(output):
				_fail("Character select capture requires a rendering display", client)
				return
			print("PASS: rendered character select captured")
			client.queue_free()
			quit(0)
			return
	_fail("Timed out waiting for character selection", client)
