extends SceneTree

const EXPECT_VISIBLE_ENV := "FPS_OVERLAY_EXPECT_VISIBLE"
const FPS_CAP := 30

func _initialize() -> void:
	Engine.max_fps = FPS_CAP
	call_deferred("run_test")

func run_test() -> void:
	var expected := OS.get_environment(EXPECT_VISIBLE_ENV)
	if expected != "true" and expected != "false":
		fail("%s must be true or false" % EXPECT_VISIBLE_ENV)
		return
	root.size = Vector2i(1280, 720)
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var overlay := client.get_node_or_null("FpsOverlay") as CanvasLayer
	if overlay == null:
		fail("Client scene lacks FpsOverlay CanvasLayer")
		return
	if not client.has_method("fps_overlay_enabled"):
		fail("Native client lacks fps_overlay_enabled option getter")
		return
	var enabled: bool = client.fps_overlay_enabled()
	if enabled != (expected == "true") or overlay.visible != enabled:
		fail("FPS overlay visibility does not match saved client option")
		return
	if not enabled:
		client.free()
		print("PASS: saved FPS overlay option hides the native HUD")
		quit(0)
		return

	var label := overlay.get_node_or_null("FpsLabel") as Label
	var graph := overlay.get_node_or_null("FrameTimeGraph") as Control
	if label == null or graph == null or not graph.visible:
		fail("Visible FPS overlay lacks its label or frame-time graph")
		return
	if graph.size != Vector2(192, 64) or graph.position.y < label.position.y + label.size.y:
		fail("Frame-time graph must be 192x64 below FPS text")
		return
	var deadline := Time.get_ticks_msec() + 3000
	while Time.get_ticks_msec() < deadline and (Engine.get_frames_per_second() <= 0.0 or not label.text.trim_prefix("FPS: ").is_valid_float()):
		await process_frame
	if not label.text.begins_with("FPS: "):
		fail("FPS label has no real measurement: " + label.text)
		return
	var displayed := label.text.trim_prefix("FPS: ")
	if not displayed.is_valid_float() or float(displayed) <= 0.0:
		fail("FPS label is not numeric: " + label.text)
		return
	var measured := Engine.get_frames_per_second()
	if absf(float(displayed) - measured) > 15.0 or float(displayed) > FPS_CAP + 15.0:
		fail("FPS label does not match the capped measured FPS: %s vs %.2f" % [displayed, measured])
		return
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var bounds := graph.get_global_rect()
	var colored := false
	for y in range(int(bounds.position.y), int(bounds.end.y)):
		for x in range(int(bounds.position.x), int(bounds.end.x)):
			var pixel := image.get_pixel(x, y)
			if pixel.r > pixel.b + 0.12 or pixel.g > pixel.b + 0.12:
				colored = true
				break
		if colored:
			break
	if not colored:
		fail("Frame-time graph did not render any colored history")
		return
	client.free()
	print("PASS: actual capped FPS, saved visibility and rendered frame-time history")
	quit(0)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
