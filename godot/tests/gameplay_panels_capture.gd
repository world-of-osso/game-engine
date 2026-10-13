extends SceneTree

func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	var directory := OS.get_environment("GAMEPLAY_CAPTURES")
	if directory.is_empty():
		push_error("GAMEPLAY_CAPTURES is required")
		quit(1)
		return
	for skin in ["modern", "forever"]:
		for page in ["recipient", "duration"]:
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			var error = ui.call("show_customer_orders_preview", skin == "forever", page == "duration")
			if error != "":
				push_error(error)
				quit(1)
				return
			for frame in range(120):
				await process_frame
				await RenderingServer.frame_post_draw
			var image := root.get_texture().get_image()
			if image.get_size() != Vector2i(1920, 1080):
				push_error("Capture must be 1920x1080")
				quit(1)
				return
			if image.save_png(directory.path_join(skin + "-orders-" + page + ".png")) != OK:
				push_error("Capture write failed")
				quit(1)
				return
			ui.queue_free()
			await process_frame
	print("PASS: both-skin customer-order native captures")
	quit(0)
