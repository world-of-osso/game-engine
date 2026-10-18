extends SceneTree
# Message origin only: current font, height, color and supplied non-instance text are controls.
func _initialize() -> void:
	call_deferred("run_message")

func run_message() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("READY_MESSAGE_CAPTURE_PATH")
	var verify := OS.get_environment("READY_MESSAGE_VERIFY") == "1"
	assert(not output.is_empty())
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.show_ready_check_preview(forever).is_empty())
		for frame in range(8):
			await process_frame
			await RenderingServer.frame_post_draw
		var label := ui.find_child("ReadyCheckFrameText", true, false) as Label
		var panel := ui.find_child("ReadyCheckFrame", true, false) as Control
		assert(label != null and label.is_visible_in_tree())
		assert(label.text == "Raidleader has initiated a ready check.")
		assert(label.size.x == 240 and label.size.y > 0)
		var offset := label.get_global_rect().position - panel.get_global_rect().position
		print("READY_MESSAGE ", skin, " offset=", offset, " rect=", label.get_global_rect())
		if verify:
			assert(abs(offset.x - 61.5) <= 0.5 and offset.y == 37, "XML TOP(+20,-37), native rounding")
		for suffix in ["Yes", "No"]:
			var button := ui.find_child("ReadyCheckFrame" + suffix + "Button", true, false) as Control
			assert(button != null and button.size == Vector2(119, 24))
		var image := root.get_texture().get_image()
		assert(image.save_png(output.path_join(skin + "-message-context.png")) == OK)
		assert(image.get_region(Rect2i(panel.get_global_rect())).save_png(output.path_join(skin + "-panel-crop.png")) == OK)
		ui.queue_free()
		await process_frame
	print("PASS ready-message both-skin supplied message origin; verified=", verify)
	quit(0)
