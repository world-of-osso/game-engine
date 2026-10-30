extends SceneTree
# Message shadow only; colour follows NORMAL_FONT_COLOR row1 (0xFFFFD200).
func _initialize() -> void:
	call_deferred("run_ready_shadow")

func run_ready_shadow() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("READY_SHADOW_CAPTURE_PATH")
	var verify := OS.get_environment("READY_SHADOW_VERIFY") == "1"
	assert(not output.is_empty())
	var observations: Array[Dictionary] = []
	for forever in [true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.show_ready_check_preview(forever).is_empty())
		for frame in range(8):
			await process_frame
			await RenderingServer.frame_post_draw
		var label := ui.find_child("ReadyCheckFrameText", true, false) as Label
		var panel := ui.find_child("ReadyCheckFrame", true, false) as Control
		assert(label != null and panel != null and label.is_visible_in_tree())
		assert(label.text == "Raidleader has initiated a ready check.")
		assert(label.size.x == 240 and label.size.y > 0)
		assert(label.get_theme_font_size("font_size") == 12)
		assert(label.get_theme_color("font_color").is_equal_approx(Color(1, 210.0 / 255.0, 0)))
		var anchor := label.global_position - panel.global_position
		assert(absf(anchor.x - 61.5) <= 0.5 and anchor.y == 37)
		var shadow := label.get_theme_color("font_shadow_color")
		var offset := Vector2i(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y"))
		if verify:
			assert(shadow == Color.BLACK and offset == Vector2i(1, 1))
		else:
			assert(shadow.a == 0 and offset == Vector2i.ZERO, "Genuine missing inherited shadow")
		for suffix in ["Yes", "No"]:
			var button := ui.find_child("ReadyCheckFrame" + suffix + "Button", true, false) as Control
			assert(button != null and button.size == Vector2(119, 24))
		observations.append({"skin": skin, "position": [label.global_position.x, label.global_position.y], "size": [label.size.x, label.size.y], "shadow": str(shadow), "offset": str(offset)})
		print("READY_SHADOW ", skin, " rect=", label.get_global_rect(), " shadow=", shadow, " offset=", offset)
		var image := root.get_texture().get_image()
		assert(image.get_size() == Vector2i(1920, 1080))
		assert(image.save_png(output.path_join(skin + "-ready-context.png")) == OK)
		assert(image.get_region(Rect2i(panel.get_global_rect())).save_png(output.path_join(skin + "-panel-crop.png")) == OK)
		if verify:
			label.add_theme_color_override("font_shadow_color", Color.TRANSPARENT)
			for frame in range(4):
				await process_frame
				await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(output.path_join(skin + "-shadow-off-control.png")) == OK)
		ui.queue_free()
		await process_frame
	var receipt := FileAccess.open(OS.get_environment("READY_SHADOW_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS ready-shadow both-skin supplied message; verified=", verify)
	quit(0)
