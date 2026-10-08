extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var ui = ClassDB.instantiate("RegistryUi")
	root.add_child(ui)
	var method = "show_forever_trainer_preview" if OS.get_environment("GODOT_TRAINER_SKIN") == "forever" else "show_trainer_preview"
	var error = ui.call(method)
	if error != "":
		push_error(error)
		quit(1)
		return
	for frame in range(240):
		await process_frame
		var portrait = ui.find_child("TrainerPreviewPortrait", true, false)
		if portrait != null:
			portrait.call("tick")
		await RenderingServer.frame_post_draw
	var coin := ui.find_child("ClassTrainerService2963CostCoin2", true, false) as Control
	var selected := ui.find_child("ClassTrainerService2963Selected", true, false) as Control
	var rect := Rect2i(coin.get_global_rect())
	var before = root.get_texture().get_image()
	before.save_png(OS.get_environment("GODOT_CAPTURE_PATH"))
	selected.hide()
	RenderingServer.force_draw()
	var after = root.get_texture().get_image()
	var changed := 0
	# Compare the opaque centre of the copper sprite, not transparent corners where selection belongs.
	for y in range(rect.position.y + 4, rect.end.y - 4):
		for x in range(rect.position.x + 4, rect.end.x - 2):
			if not before.get_pixel(x, y).is_equal_approx(after.get_pixel(x, y)):
				changed += 1
	print("COPPER_RECT=", rect, " selection changes opaque coin pixels=", changed)
	ui.queue_free()
	if changed > 0:
		push_error("Selection border draws over opaque copper coin pixels")
		quit(1)
	else:
		print("PASS: selection preserves copper coin pixels")
		quit(0)
