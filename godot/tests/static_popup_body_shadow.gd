extends SceneTree
# Supplied PARTY_INVITE, production spec/ordinary StaticPopup; not server/lifecycle proof.
func _initialize() -> void:
	call_deferred("run_body_shadow")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func run_body_shadow() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("POPUP_SHADOW_CAPTURE_PATH")
	var verify := OS.get_environment("POPUP_SHADOW_VERIFY") == "1"
	assert(not output.is_empty())
	var observations: Array[Dictionary] = []
	for forever in [true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.show_party_invite_shadow_preview(forever).is_empty())
		await settle()
		var panel := ui.find_child("StaticPopup1", true, false) as Control
		var label := ui.find_child("StaticPopup1Text", true, false) as Label
		assert(panel != null and panel.size == Vector2(320, 88))
		assert(label != null and label.is_visible_in_tree())
		assert(label.text == "Adriel invites you to a group.")
		assert(label.get_theme_font_size("font_size") == 14)
		assert(label.get_theme_color("font_color") == Color.WHITE)
		assert(label.global_position - panel.global_position == Vector2(15, 16))
		assert(label.size.x == 290 and label.size.y >= 18)
		var shadow := label.get_theme_color("font_shadow_color")
		var offset := Vector2i(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y"))
		if verify:
			assert(shadow == Color.BLACK and offset == Vector2i(1, 1))
		else:
			assert(shadow.a == 0 and offset == Vector2i.ZERO, "Genuine missing inherited shadow")
		var buttons: Array[Dictionary] = []
		for suffix in ["Button1", "Button2"]:
			var button := ui.find_child("StaticPopup1" + suffix, true, false) as Control
			assert(button != null and button.size == Vector2(128, 26))
			var caption := button.find_child("Text", true, false) as Label
			assert(caption != null and caption.text == ("Accept" if suffix == "Button1" else "Decline"))
			buttons.append({"name": suffix, "position": [button.global_position.x, button.global_position.y], "size": [button.size.x, button.size.y], "text": caption.text})
		observations.append({"case": skin, "name": "StaticPopup1Text", "position": [label.global_position.x, label.global_position.y], "size": [label.size.x, label.size.y], "panel_position": [panel.global_position.x, panel.global_position.y], "panel_size": [panel.size.x, panel.size.y], "buttons": buttons, "shadow": str(shadow), "offset": str(offset)})
		var image := root.get_texture().get_image()
		assert(image.get_size() == Vector2i(1920, 1080))
		assert(image.save_png(output.path_join(skin + "-context.png")) == OK)
		assert(image.get_region(Rect2i(panel.get_global_rect())).save_png(output.path_join(skin + "-panel-crop.png")) == OK)
		if verify:
			label.add_theme_color_override("font_shadow_color", Color.TRANSPARENT)
			await settle()
			assert(root.get_texture().get_image().save_png(output.path_join(skin + "-shadow-off-control.png")) == OK)
			# Reveal raster contribution even where the existing dialog background is black.
			label.add_theme_color_override("font_shadow_color", Color.MAGENTA)
			await settle()
			assert(root.get_texture().get_image().save_png(output.path_join(skin + "-magenta-shadow-control.png")) == OK)
			label.visible = false
			await settle()
			assert(root.get_texture().get_image().save_png(output.path_join(skin + "-label-background-control.png")) == OK)
			label.visible = true
			label.add_theme_color_override("font_shadow_color", Color.BLACK)
			await settle()
			assert(root.get_texture().get_image().get_data() == image.get_data(), "Restore source-shadow pixels exactly")
		ui.queue_free()
		await process_frame
	var receipt := FileAccess.open(OS.get_environment("POPUP_SHADOW_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS static-popup-body-shadow both-skin supplied PARTY_INVITE; verified=", verify)
	quit(0)
