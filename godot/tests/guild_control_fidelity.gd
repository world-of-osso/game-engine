extends SceneTree

func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	var directory := OS.get_environment("GUILD_CAPTURES")
	assert(not directory.is_empty())
	for skin in ["modern", "forever"]:
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error = ui.call("show_guild_ranks_preview" if skin == "modern" else "show_forever_guild_ranks_preview")
		assert(error == "", str(error))
		for frame in range(30):
			await process_frame
			await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		assert(image.get_size() == Vector2i(1920, 1080))
		assert(image.save_png(directory.path_join(skin + "-guild-control.png")) == OK)
		ui.queue_free()
		await process_frame
	print("PASS: both-skin guild control captures 1920x1080")
	quit(0)
