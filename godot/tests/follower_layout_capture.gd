extends SceneTree

# Same native Group Finder projection as the live window; offline catalog snapshot.
func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	var directory := OS.get_environment("GODOT_FOLLOWER_LAYOUT_CAPTURES")
	if directory.is_empty():
		push_error("GODOT_FOLLOWER_LAYOUT_CAPTURES required")
		quit(1)
		return
	if root.size != Vector2i(1920, 1080):
		push_error("Capture requires compositor output 1920x1080")
		quit(1)
		return
	for skin in ["modern", "forever"]:
		for menu in [false, true]:
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			var error = ui.call("show_follower_layout_preview", skin == "forever", menu)
			if error != "":
				push_error(error)
				quit(1)
				return
			for frame in range(120):
				await process_frame
				await RenderingServer.frame_post_draw
			var image := root.get_texture().get_image()
			var suffix := "-dropdown" if menu else ""
			var result := image.save_png(directory.path_join(skin + suffix + ".png"))
			if result != OK:
				push_error("Cannot save capture: " + str(result))
				quit(1)
				return
			ui.queue_free()
			await process_frame
	print("PASS both-skin Group Finder capture at 1920x1080")
	quit(0)
