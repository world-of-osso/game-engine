extends SceneTree
# Offline target cast samples through the production unit-frame screen.

func _initialize() -> void:
	call_deferred("capture_target_casts")

func capture_target_casts() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("TARGET_CAST_CAPTURE_PATH")
	assert(not output.is_empty(), "Set TARGET_CAST_CAPTURE_PATH")
	var verify_art := OS.get_environment("TARGET_CAST_VERIFY_ART") == "1"
	for forever in [false, true]:
		for phase in ["cast", "channel", "locked", "interrupted"]:
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			var error: String = ui.show_target_cast_preview(forever, phase)
			assert(error.is_empty(), error)
			for frame in range(90):
				await process_frame
				await RenderingServer.frame_post_draw
			var shell := ui.find_child("TargetFrameSpellBar", true, false) as Control
			var fill := ui.find_child("TargetCastingBarFill", true, false) as Control
			assert(shell != null and fill != null, "Actual target cast controls")
			assert(shell.size == (Vector2(240, 24) if forever else Vector2(150, 10)))
			assert(fill.size == (Vector2(108, 16) if forever else Vector2(75, 10)))
			if verify_art and not forever:
				verify_source_art(ui, shell, fill)
			var skin := "forever" if forever else "modern"
			var image := root.get_texture().get_image()
			assert(image.get_size() == Vector2i(1920, 1080))
			var path := output.path_join("target-cast-%s-%s.png" % [skin, phase])
			assert(image.save_png(path) == OK)
			print("PASS TARGET_CAST skin=", skin, " phase=", phase, " fill=", fill.get_global_rect(), " capture=", path)
			ui.queue_free()
			await process_frame
	quit(0)

func verify_source_art(ui: Node, shell: Control, fill: Control) -> void:
	for suffix in ["BackgroundArt", "TextBorder", "Fill", "Spark"]:
		var region := ui.find_child("TargetCastingBar" + suffix, true, false)
		assert(region != null, "Missing source region " + suffix)
		var part := region.get_node_or_null("Parts/Part0") as TextureRect
		assert(part != null and part.texture != null, "Loaded source texture " + suffix)
	var textbox := ui.find_child("TargetCastingBarTextBorder", true, false) as Control
	assert(textbox.size == Vector2(150, 22) and textbox.global_position.is_equal_approx(shell.global_position), "Source textbox anchors")
	var spark := ui.find_child("TargetCastingBarSpark", true, false) as Control
	assert(spark.size == Vector2(6, 16), "Source pip size")
	var center := spark.global_position + spark.size / 2.0
	assert(center.is_equal_approx(fill.global_position + Vector2(fill.size.x, 5)), "Source pip center follows fill edge")
