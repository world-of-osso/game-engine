extends SceneTree
# Run only after the colour head lands, in the lead-agreed native-proof window.
const MESSAGE_COLOR := Color(1, 210.0 / 255.0, 0)

func _initialize() -> void:
	call_deferred("run_message_color")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func rect(control: Control) -> Dictionary:
	return {"position": [control.global_position.x, control.global_position.y], "size": [control.size.x, control.size.y]}

func run_message_color() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("READY_COLOR_CAPTURE_PATH")
	assert(not output.is_empty())
	var observations: Array[Dictionary] = []
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.show_ready_check_preview(forever).is_empty())
		await settle()
		var panel := ui.find_child("ReadyCheckFrame", true, false) as Control
		var message := ui.find_child("ReadyCheckFrameText", true, false) as Label
		var title := ui.find_child("ReadyCheckFrameTitle", true, false) as Label
		assert(panel != null and message != null and title != null)
		assert(panel.size == Vector2(323, 100) and message.size.x == 240 and message.size.y > 0)
		assert(message.is_visible_in_tree() and message.text == "Raidleader has initiated a ready check.")
		assert(message.get_theme_font_size("font_size") == 12)
		var source_color := message.get_theme_color("font_color")
		assert(source_color.is_equal_approx(MESSAGE_COLOR))
		assert(source_color.r8 == 255 and source_color.g8 == 210 and source_color.b8 == 0 and source_color.a8 == 255)
		assert(message.get_theme_color("font_shadow_color") == Color.BLACK)
		assert(Vector2i(message.get_theme_constant("shadow_offset_x"), message.get_theme_constant("shadow_offset_y")) == Vector2i(1, 1))
		var anchor := message.global_position - panel.global_position
		assert(absf(anchor.x - 61.5) <= 0.5 and anchor.y == 37)
		# Existing title palette/geometry are conservation controls, not source-title qualification.
		assert(title.text == "Ready Check" and title.get_theme_font_size("font_size") == 12)
		assert(title.get_theme_color("font_color").is_equal_approx(Color(1, 210.0 / 255.0, 0)))
		var buttons: Array[Dictionary] = []
		for suffix in ["Yes", "No"]:
			var button := ui.find_child("ReadyCheckFrame" + suffix + "Button", true, false) as Button
			assert(button != null and not button.disabled and button.size == Vector2(119, 24))
			var caption := button.find_child("Text", true, false) as Label
			assert(caption != null and caption.text == ("Ready" if suffix == "Yes" else "Not Ready"))
			buttons.append({"name": suffix, "rect": rect(button), "caption": caption.text})
		observations.append({"case": skin, "message": rect(message), "panel": rect(panel), "title": rect(title), "title_color": str(title.get_theme_color("font_color")), "buttons": buttons, "source_color": str(source_color), "source_rgba8": [source_color.r8, source_color.g8, source_color.b8, source_color.a8]})
		var image := root.get_texture().get_image()
		assert(image.get_size() == Vector2i(1920, 1080))
		assert(image.save_png(output.path_join(skin + "-context.png")) == OK)
		assert(image.get_region(Rect2i(panel.get_global_rect())).save_png(output.path_join(skin + "-panel-crop.png")) == OK)
		# Counterfactual previous TEXT_COLOR, not a claim of an old production-binary capture.
		message.add_theme_color_override("font_color", Color.WHITE)
		await settle()
		var white := root.get_texture().get_image()
		assert(white.save_png(output.path_join(skin + "-white-control.png")) == OK)
		assert(white.get_region(Rect2i(panel.get_global_rect())).save_png(output.path_join(skin + "-white-panel-crop.png")) == OK)
		message.add_theme_color_override("font_color", MESSAGE_COLOR)
		await settle()
		assert(root.get_texture().get_image().get_data() == image.get_data(), "Restore source NORMAL_FONT_COLOR pixels exactly")
		ui.queue_free()
		await process_frame
	var receipt := FileAccess.open(OS.get_environment("READY_COLOR_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS ready-message-color both-skin supplied message")
	quit(0)
