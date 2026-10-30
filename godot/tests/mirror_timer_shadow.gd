extends SceneTree
# Supplied paused timers, source text shadow only. No cadence/server or full-timer claim.
func _initialize() -> void:
	call_deferred("run_mirror_shadow")

func run_mirror_shadow() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("MIRROR_SHADOW_CAPTURE_PATH")
	var verify := OS.get_environment("MIRROR_SHADOW_VERIFY") == "1"
	assert(not output.is_empty())
	var observations: Array[Dictionary] = []
	for forever in [true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.show_mirror_timer_shadow_preview(forever).is_empty())
		for frame in range(8):
			await process_frame
			await RenderingServer.frame_post_draw
		var labels: Array[Label] = []
		for index in range(3):
			var prefix := "MirrorTimer%d" % (index + 1)
			var row := ui.find_child(prefix, true, false) as Control
			var bar := ui.find_child(prefix + "Bar", true, false) as Control
			var label := ui.find_child(prefix + "Text", true, false) as Label
			assert(row != null and bar != null and label != null)
			assert(label.is_visible_in_tree())
			assert(row.size == Vector2(206, 32))
			assert(row.global_position == Vector2(857, 100 + index * 32))
			assert(bar.size == Vector2(195, 13))
			assert(label.size == Vector2(195, 13), "Native Friz10 minimum remains13; authored box remains12")
			assert(label.global_position - bar.global_position == Vector2(0, 13))
			assert(label.text == ["Breath", "Fatigue", "Feign Death"][index])
			assert(label.get_theme_font_size("font_size") == 10)
			assert(label.get_theme_color("font_color") == Color.WHITE)
			var shadow := label.get_theme_color("font_shadow_color")
			var offset := Vector2i(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y"))
			if verify:
				assert(shadow == Color.BLACK and offset == Vector2i(1, 1), "WoW(+1,-1) projects right/down(+1,+1)")
			else:
				assert(shadow.a == 0 and offset == Vector2i.ZERO, "Genuine baseline shadow gap")
			observations.append({"skin": skin, "label": prefix + "Text", "rect": str(label.get_global_rect()), "position": [label.global_position.x, label.global_position.y], "size": [label.size.x, label.size.y], "shadow": str(shadow), "offset": str(offset)})
			print("MIRROR_SHADOW ", skin, " ", prefix, " rect=", label.get_global_rect(), " color=", shadow, " offset=", offset)
			labels.append(label)
		var image := root.get_texture().get_image()
		assert(image.get_size() == Vector2i(1920, 1080))
		assert(image.save_png(output.path_join(skin + "-mirror-context.png")) == OK)
		assert(image.get_region(Rect2i(850, 95, 220, 110)).save_png(output.path_join(skin + "-timers-crop.png")) == OK)
		if verify:
			# Diagnostic control: transparent shadow on the same native labels must restore baseline pixels.
			for label in labels:
				label.add_theme_color_override("font_shadow_color", Color.TRANSPARENT)
			for frame in range(4):
				await process_frame
				await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(output.path_join(skin + "-shadow-off-control.png")) == OK)
		ui.queue_free()
		await process_frame
	var receipt := FileAccess.open(OS.get_environment("MIRROR_SHADOW_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS mirror-shadow both-skin three-kind supplied labels; verified=", verify)
	quit(0)
