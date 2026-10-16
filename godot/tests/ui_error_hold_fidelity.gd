extends SceneTree
# Deterministic supplied elapsed time, not wall-clock or complete fadePower acceptance.
func _initialize() -> void:
	call_deferred("run_hold")

func settle() -> void:
	for frame in range(6):
		await process_frame
		await RenderingServer.frame_post_draw

func run_hold() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("ERROR_HOLD_CAPTURE_PATH")
	var verify := OS.get_environment("ERROR_HOLD_VERIFY") == "1"
	assert(not output.is_empty())
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		for sample in [["new", 0.0, false], ["held", 2.0, false], ["fading", 2.25, false], ["expired", 2.5, false], ["repeat", 2.25, true]]:
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			assert(ui.show_error_hold_preview(forever, sample[1], sample[2]).is_empty())
			await settle()
			for index in [1, 2]:
				var label := ui.find_child("UIErrorsFrameLine" + str(index), true, false) as Label
				assert(label != null)
				var color := label.get_theme_color("font_color")
				print("ERROR_HOLD ", skin, " ", sample[0], " line=", index, " visible=", label.is_visible_in_tree(), " color=", color)
				if verify:
					if sample[0] == "expired":
						assert(not label.is_visible_in_tree(), "Hold2+fade.5 expires")
					else:
						assert(label.is_visible_in_tree())
						assert(label.text == ("Blackrock Worg slain: 1/6" if index == 1 else "Out of range."))
						assert(Vector3(color.r, color.g, color.b).is_equal_approx(Vector3(1, 1, 0) if index == 1 else Vector3(1, 0.1, 0.1)))
						if sample[0] == "fading":
							assert(color.a > 0 and color.a < 1, "Source hold ended; curve not qualified")
						else:
							assert(color.a == 1, "Opaque hold/repeated message reset")
			var image := root.get_texture().get_image()
			assert(image.save_png(output.path_join(skin + "-" + sample[0] + "-context.png")) == OK)
			assert(image.get_region(Rect2i(704, 122, 512, 60)).save_png(output.path_join(skin + "-" + sample[0] + "-crop.png")) == OK)
			ui.queue_free()
			await process_frame
	print("PASS error-hold both-skin supplied elapsed time; verified=", verify)
	quit(0)
