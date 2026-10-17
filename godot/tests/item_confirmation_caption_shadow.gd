extends SceneTree
# Supplied production item-presenter states; no purchase/animation/lifetime acceptance.
func _initialize() -> void:
	call_deferred("run_caption_shadow")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func run_caption_shadow() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("CAPTION_SHADOW_CAPTURE_PATH")
	var verify := OS.get_environment("CAPTION_SHADOW_VERIFY") == "1"
	assert(not output.is_empty())
	var observations: Array[Dictionary] = []
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		for state in ["normal", "delay", "pending"]:
			var case_name: String = skin + "-" + state
			var pending: bool = state == "pending"
			var accept_enabled: bool = state != "delay"
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			assert(ui.show_item_confirmation_shadow_preview(forever, pending, accept_enabled).is_empty())
			await settle()
			var panel := ui.find_child("ShadowConfirmDialog", true, false) as Control
			assert(panel != null and panel.size.x == 320 and panel.global_position == Vector2(800, 135))
			var names: Array[String] = []
			for suffix in ["Button", "CancelButton"]:
				var button := ui.find_child("ShadowConfirm" + suffix, true, false) as Button
				assert(button != null and button.size == Vector2(128, 21))
				var accept: bool = suffix == "Button"
				assert(button.disabled == (pending or (accept and not accept_enabled)))
				var name: String = "ShadowConfirm" + suffix + "DialogButtonText"
				names.append(name)
				var label := ui.find_child(name, true, false) as Label
				assert(label != null and label.is_visible_in_tree())
				assert(label.text == ("Confirm" if accept else "Cancel"))
				assert(label.size == Vector2(128, 21))
				assert(label.global_position - button.global_position == Vector2(0, -1))
				assert(label.get_theme_font_size("font_size") == 12)
				assert(label.get_theme_color("font_color").is_equal_approx(Color(1, 0.82, 0)))
				var shadow := label.get_theme_color("font_shadow_color")
				var offset := Vector2i(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y"))
				if verify:
					assert(shadow == Color.BLACK and offset == Vector2i(1, 1))
				else:
					assert(shadow.a == 0 and offset == Vector2i.ZERO, "Genuine missing caption shadow")
				observations.append({"case": case_name, "name": name, "position": [label.global_position.x, label.global_position.y], "size": [label.size.x, label.size.y], "text": label.text, "button_position": [button.global_position.x, button.global_position.y], "disabled": button.disabled, "panel_position": [panel.global_position.x, panel.global_position.y], "panel_size": [panel.size.x, panel.size.y], "shadow": str(shadow), "offset": str(offset)})
			var body := ui.find_child("ShadowConfirmTextBefore", true, false) as Label
			assert(body != null and body.get_theme_font_size("font_size") == 12)
			assert(body.get_theme_color("font_shadow_color") == Color.BLACK, "Existing body shadow is a control")
			var image := root.get_texture().get_image()
			assert(image.save_png(output.path_join(case_name + "-context.png")) == OK)
			assert(image.get_region(Rect2i(panel.get_global_rect())).save_png(output.path_join(case_name + "-panel-crop.png")) == OK)
			if verify:
				for name in names:
					var label := ui.find_child(name, true, false) as Label
					label.add_theme_color_override("font_shadow_color", Color.TRANSPARENT)
				await settle()
				assert(root.get_texture().get_image().save_png(output.path_join(case_name + "-shadow-off-control.png")) == OK)
				for name in names:
					var label := ui.find_child(name, true, false) as Label
					label.add_theme_color_override("font_shadow_color", Color.MAGENTA)
				await settle()
				assert(root.get_texture().get_image().save_png(output.path_join(case_name + "-magenta-shadow-control.png")) == OK)
				for name in names:
					var label := ui.find_child(name, true, false) as Label
					label.add_theme_color_override("font_shadow_color", Color.BLACK)
				await settle()
				assert(root.get_texture().get_image().get_data() == image.get_data(), "Restore every source pixel")
			ui.queue_free()
			await process_frame
	var receipt := FileAccess.open(OS.get_environment("CAPTION_SHADOW_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS item-confirmation-caption-shadow both-skin normal/delay/pending; verified=", verify)
	quit(0)
