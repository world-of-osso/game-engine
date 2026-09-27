extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var output = OS.get_environment("GODOT_CAPTURE_PATH")
	if output.is_empty():
		push_error("Set GODOT_CAPTURE_PATH for the rendered login capture")
		quit(1)
		return
	var scene: PackedScene = load("res://scenes/client.tscn")
	var client = scene.instantiate()
	root.add_child(client)
	# Wait out the original 0.75s login fade-in.
	var faded = Time.get_ticks_msec() + 1000
	while Time.get_ticks_msec() < faded:
		await process_frame
	for frame in range(3):
		await process_frame
		await RenderingServer.frame_post_draw
	var image = root.get_texture().get_image()
	if image == null or image.is_empty():
		push_error("Login capture requires an actual rendering display, not the dummy driver")
		quit(1)
		return
	var error = image.save_png(output)
	if error != OK:
		push_error("Cannot save login capture: " + error_string(error))
		quit(1)
		return
	client.queue_free()
	print("PASS: rendered login captured at ", image.get_width(), "x", image.get_height())
	quit(0)
