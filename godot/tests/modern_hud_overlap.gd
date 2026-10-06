extends SceneTree

# Capture layers separately: drawn overlap is the intersection of pixel masks,
# not an intersection of invisible Control rectangles. Never changes anchors.
func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var output := OS.get_environment("GODOT_CAPTURE_PATH").get_base_dir()
	var fixture := ClassDB.instantiate("ModernHudOverlapFixture") as Node
	root.add_child(fixture)
	var error: String = fixture.initialize()
	if not error.is_empty():
		push_error(error)
		quit(1)
		return
	var prefix := "modern-hud"
	if OS.get_environment("GODOT_CAPTURE_SCREEN") == "hud_reported":
		error = fixture.set_canvas_scale(1.0)
		if not error.is_empty():
			push_error(error)
			quit(1)
			return
		prefix = "modern-hud-reported"
	for frame in range(4):
		await process_frame
		await RenderingServer.frame_post_draw
	var bounds: Dictionary = fixture.bounds()
	var report := {}
	for key in bounds:
		var rect: Rect2 = bounds[key]
		report[key] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
	var overlap: Rect2 = bounds.PlayerFrame.intersection(bounds.ChatFrame)
	report["bounds_intersection"] = [overlap.position.x, overlap.position.y, overlap.size.x, overlap.size.y]
	var file := FileAccess.open(output.path_join(prefix + "-bounds.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(report, "\t"))
	file.close()
	for mode in [["empty", false, false], ["player", true, false], ["chat", false, true], ["both", true, true]]:
		fixture.set_drawn(mode[1], mode[2])
		for frame in range(3):
			await process_frame
			await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		if image.save_png(output.path_join(prefix + "-" + mode[0] + ".png")) != OK:
			push_error("save HUD capture")
			quit(1)
			return
	fixture.free()
	await process_frame
	print("PASS modern_hud_overlap_layers_captured ", report)
	quit(0)
