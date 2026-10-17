extends SceneTree
# Supplied single/bundle state through the production screen. Shadow-only proof.
func _initialize() -> void:
	call_deferred("run_label_shadow")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func run_label_shadow() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("STACK_SHADOW_CAPTURE_PATH")
	var verify := OS.get_environment("STACK_SHADOW_VERIFY") == "1"
	assert(not output.is_empty())
	var observations: Array[Dictionary] = []
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		for bundles in [false, true]:
			var case_name: String = skin + ("-bundle" if bundles else "-single")
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			assert(ui.show_stack_split_preview(forever, bundles, true).is_empty())
			await settle()
			var panel := ui.find_child("StackSplitFrame", true, false) as Control
			assert(panel != null and panel.size == Vector2(172, 120 if bundles else 96))
			var names: Array[String] = ["StackSplitText"]
			if bundles:
				names.append("StackItemCountText")
			else:
				assert(ui.find_child("StackItemCountText", true, false) == null)
			for name in names:
				var label := ui.find_child(name, true, false) as Label
				assert(label != null and label.is_visible_in_tree())
				# Native font minimum may exceed the authored14px; receipts compare exact before/after bounds.
				assert(label.size.x == 100 and label.size.y >= 14)
				assert(label.get_theme_font_size("font_size") == 12)
				var total := name == "StackItemCountText"
				var expected_text := "20 Total" if total else ("4 Stack(s)" if bundles else "4")
				assert(label.text == expected_text)
				# Existing colors are conservation controls, not source-color qualification.
				assert(label.get_theme_color("font_color").is_equal_approx(Color(1, 0.82, 0) if total else Color.WHITE))
				var local_position := label.global_position - panel.global_position
				assert(local_position == Vector2(41 if bundles else 22, 48 if total else 23))
				var shadow := label.get_theme_color("font_shadow_color")
				var offset := Vector2i(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y"))
				if verify:
					assert(shadow == Color.BLACK and offset == Vector2i(1, 1))
				else:
					assert(shadow.a == 0 and offset == Vector2i.ZERO, "Genuine missing inherited shadow")
				observations.append({"case": case_name, "name": name, "position": [label.global_position.x, label.global_position.y], "size": [label.size.x, label.size.y], "shadow": str(shadow), "offset": str(offset)})
				print("STACK_LABEL_SHADOW ", case_name, "/", name, " rect=", label.get_global_rect(), " shadow=", shadow, " offset=", offset)
			var image := root.get_texture().get_image()
			assert(image.get_size() == Vector2i(1920, 1080))
			assert(image.save_png(output.path_join(case_name + "-context.png")) == OK)
			assert(image.get_region(Rect2i(panel.get_global_rect())).save_png(output.path_join(case_name + "-panel-crop.png")) == OK)
			if verify:
				for name in names:
					var label := ui.find_child(name, true, false) as Label
					label.add_theme_color_override("font_shadow_color", Color.TRANSPARENT)
				await settle()
				assert(root.get_texture().get_image().save_png(output.path_join(case_name + "-shadow-off-control.png")) == OK)
			ui.queue_free()
			await process_frame
	var receipt := FileAccess.open(OS.get_environment("STACK_SHADOW_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS stack-label-shadow both-skin single/bundle supplied state; verified=", verify)
	quit(0)
