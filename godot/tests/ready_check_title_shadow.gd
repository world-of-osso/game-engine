extends SceneTree
# Post-gate supplied-state title-shadow proof, not full ReadyCheck parity.
func _initialize() -> void:
	call_deferred("run_title_shadow")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func rect(control: Control) -> Dictionary:
	return {"position": [control.global_position.x, control.global_position.y], "size": [control.size.x, control.size.y]}

func run_title_shadow() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("READY_TITLE_CAPTURE_PATH")
	assert(not output.is_empty())
	var observations: Array[Dictionary] = []
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.show_ready_check_preview(forever).is_empty())
		await settle()
		var panel := ui.find_child("ReadyCheckFrame", true, false) as Control
		var title := ui.find_child("ReadyCheckFrameTitle", true, false) as Label
		var message := ui.find_child("ReadyCheckFrameText", true, false) as Label
		assert(panel != null and title != null and message != null)
		assert(panel.size == Vector2(323, 100))
		assert(title.is_visible_in_tree() and title.text == "Ready Check")
		assert(title.size == Vector2(323, 20) and title.global_position - panel.global_position == Vector2(0, 4))
		assert(title.get_theme_font_size("font_size") == 12)
		assert(title.get_theme_color("font_color").is_equal_approx(Color(1, 210.0 / 255.0, 0)))
		assert(title.get_theme_color("font_shadow_color") == Color.BLACK)
		assert(Vector2i(title.get_theme_constant("shadow_offset_x"), title.get_theme_constant("shadow_offset_y")) == Vector2i(1, 1))
		assert(message.size == Vector2(240, 28) and message.text == "Raidleader has initiated a ready check.")
		assert(message.get_theme_font_size("font_size") == 12)
		assert(message.get_theme_color("font_color").is_equal_approx(Color(1, 210.0 / 255.0, 0)))
		assert(message.get_theme_color("font_shadow_color") == Color.BLACK)
		assert(Vector2i(message.get_theme_constant("shadow_offset_x"), message.get_theme_constant("shadow_offset_y")) == Vector2i(1, 1))
		var message_anchor := message.global_position - panel.global_position
		assert(absf(message_anchor.x - 61.5) <= 0.5 and message_anchor.y == 37)
		var buttons: Array[Dictionary] = []
		for suffix in ["Yes", "No"]:
			var button := ui.find_child("ReadyCheckFrame" + suffix + "Button", true, false) as Button
			assert(button != null and not button.disabled and button.size == Vector2(119, 24))
			var caption := button.find_child("Text", true, false) as Label
			assert(caption != null and caption.text == ("Ready" if suffix == "Yes" else "Not Ready"))
			buttons.append({"name": suffix, "rect": rect(button), "caption": caption.text})
		observations.append({"case": skin, "title": rect(title), "panel": rect(panel), "message": rect(message), "buttons": buttons, "shadow": str(title.get_theme_color("font_shadow_color")), "offset": [1, 1], "title_color": str(title.get_theme_color("font_color"))})
		var image := root.get_texture().get_image()
		assert(image.get_size() == Vector2i(1920, 1080))
		assert(image.save_png(output.path_join(skin + "-context.png")) == OK)
		assert(image.get_region(Rect2i(panel.get_global_rect())).save_png(output.path_join(skin + "-panel-crop.png")) == OK)
		# Same-binary old no-shadow field counterfactual; not an old compiled build.
		title.add_theme_color_override("font_shadow_color", Color.TRANSPARENT)
		await settle()
		var off := root.get_texture().get_image()
		assert(off.save_png(output.path_join(skin + "-shadow-off-control.png")) == OK)
		assert(off.get_region(Rect2i(panel.get_global_rect())).save_png(output.path_join(skin + "-shadow-off-panel.png")) == OK)
		title.add_theme_color_override("font_shadow_color", Color.BLACK)
		await settle()
		assert(root.get_texture().get_image().get_data() == image.get_data(), "Restore source title shadow RGBA exactly")
		ui.queue_free()
		await process_frame
	var receipt := FileAccess.open(OS.get_environment("READY_TITLE_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS ready-title-shadow both-skin supplied title")
	quit(0)
