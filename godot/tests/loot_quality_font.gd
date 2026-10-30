extends SceneTree
# Supplied money/item rows through the existing production LootFrame preview.
func _initialize() -> void:
	call_deferred("run_quality_font")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func run_quality_font() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("LOOT_QUALITY_CAPTURE_PATH")
	var verify := OS.get_environment("LOOT_QUALITY_VERIFY") == "1"
	assert(not output.is_empty())
	var observations: Array[Dictionary] = []
	for forever in [true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.show_loot_hit_preview(forever).is_empty())
		await settle()
		var panel := ui.find_child("LootFrame", true, false) as Control
		var row := ui.find_child("LootFrameElement2", true, false) as Control
		var label := ui.find_child("LootFrameElement2QualityText", true, false) as Label
		assert(panel != null and panel.size == Vector2(220, 140) and panel.global_position == Vector2(850, 430))
		assert(label != null and label.is_visible_in_tree() and label.text == "Uncommon")
		assert(label.size.x == 96 and label.size.y >= 10)
		assert(label.global_position - row.global_position == Vector2(98, 2))
		assert(label.get_theme_font_size("font_size") == 8 and label.get_theme_color("font_color") == Color.WHITE)
		assert(ui.find_child("LootFrameElement1QualityText", true, false) == null)
		var shadow := label.get_theme_color("font_shadow_color")
		var offset := Vector2i(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y"))
		if verify:
			assert(shadow.a == 0 and offset == Vector2i.ZERO, "Plain SystemFont_Tiny2 has no inherited shadow")
		else:
			assert(shadow == Color.BLACK and offset == Vector2i(1, 1), "Genuine unwanted shared-helper shadow")
		var controls: Array[Dictionary] = []
		for name in ["LootFrameTitleText", "LootFrameElement1Text", "LootFrameElement2Text", "LootFrameElement2ItemCount"]:
			var other := ui.find_child(name, true, false) as Label
			assert(other != null and other.is_visible_in_tree())
			assert(other.get_theme_color("font_shadow_color") == Color.BLACK)
			assert(Vector2i(other.get_theme_constant("shadow_offset_x"), other.get_theme_constant("shadow_offset_y")) == Vector2i(1, 1))
			controls.append({"name": name, "text": other.text, "position": [other.global_position.x, other.global_position.y], "size": [other.size.x, other.size.y], "font_size": other.get_theme_font_size("font_size"), "color": str(other.get_theme_color("font_color"))})
		observations.append({"case": skin, "name": "LootFrameElement2QualityText", "position": [label.global_position.x, label.global_position.y], "size": [label.size.x, label.size.y], "panel_position": [panel.global_position.x, panel.global_position.y], "panel_size": [panel.size.x, panel.size.y], "shadow": str(shadow), "offset": str(offset), "controls": controls})
		var image := root.get_texture().get_image()
		assert(image.save_png(output.path_join(skin + "-context.png")) == OK)
		assert(image.get_region(Rect2i(panel.get_global_rect())).save_png(output.path_join(skin + "-panel-crop.png")) == OK)
		if verify:
			label.add_theme_color_override("font_shadow_color", Color.BLACK)
			label.add_theme_constant_override("shadow_offset_x", 1)
			label.add_theme_constant_override("shadow_offset_y", 1)
			await settle()
			assert(root.get_texture().get_image().save_png(output.path_join(skin + "-black-shadow-control.png")) == OK)
			label.add_theme_color_override("font_shadow_color", Color.WHITE)
			await settle()
			assert(root.get_texture().get_image().save_png(output.path_join(skin + "-white-shadow-control.png")) == OK)
			label.add_theme_color_override("font_shadow_color", Color.TRANSPARENT)
			label.add_theme_constant_override("shadow_offset_x", 0)
			label.add_theme_constant_override("shadow_offset_y", 0)
			await settle()
			assert(root.get_texture().get_image().get_data() == image.get_data(), "Restore exact plain-font source pixels")
		ui.queue_free()
		await process_frame
	var receipt := FileAccess.open(OS.get_environment("LOOT_QUALITY_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS loot-quality-font both-skin supplied rows; verified=", verify)
	quit(0)
