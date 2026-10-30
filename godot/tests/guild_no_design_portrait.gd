extends SceneTree

func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	var directory := OS.get_environment("COMMUNITIES_CAPTURES")
	assert(not directory.is_empty())
	for skin in ["forever"]:
		var ui: Node = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.call("show_guild_communities_preview", skin == "forever") == "")
		for frame in range(8):
			await process_frame
			await RenderingServer.frame_post_draw
		for spec in [
			["CommunitiesTabardBackground", Rect2(548, 319, 60, 60), Color(0.2245, 0.2088, 0.1794, 1)],
			["CommunitiesTabardEmblem", Rect2(546, 318, 64, 64), Color.WHITE],
			["CommunitiesTabardBorder", Rect2(548, 320, 60, 59), Color(0.2, 0.2, 0.2, 1)],
		]:
			var layer := ui.find_child(spec[0], true, false) as Control
			assert(layer != null, "Missing native tabard layer " + spec[0])
			assert(layer.get_global_rect().is_equal_approx(spec[1]), str(layer.get_global_rect()))
			var texture := layer.get_node("Parts/Part0") as TextureRect
			assert(texture.texture != null, "Missing native tabard texture " + spec[0])
			assert(texture.self_modulate.is_equal_approx(spec[2]), str(texture.self_modulate))
		var image := root.get_texture().get_image()
		assert(image.get_size() == Vector2i(1920, 1080))
		assert(image.save_png(directory.path_join(skin + "-communities-no-design.png")) == OK)
		ui.queue_free()
		await process_frame
	print("PASS: source no-design portrait textures/colors/rectangles in both skins;1920x1080 PNGs")
	quit(0)
