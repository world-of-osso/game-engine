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
	# The first engine sample includes shader/font startup; wait for the cap to settle.
	var deadline := Time.get_ticks_msec() + 6000
	while Time.get_ticks_msec() < deadline:
		var value := label.text.trim_prefix("FPS: ")
		if value.is_valid_float() and absf(float(value) - FPS_CAP) <= 2.0 and absf(Engine.get_frames_per_second() - FPS_CAP) <= 2.0:
			break
		await process_frame
	if not label.text.begins_with("FPS: "):
		fail("FPS label has no real measurement: " + label.text)
		return
	var displayed := label.text.trim_prefix("FPS: ")
	if not displayed.is_valid_float() or float(displayed) <= 0.0:
		fail("FPS label is not numeric: " + label.text)
		return
	var measured := Engine.get_frames_per_second()
	var first_fps := float(displayed)
	if absf(first_fps - measured) > 15.0 or first_fps > FPS_CAP + 15.0:
		fail("FPS label does not match the capped measured FPS: %s vs %.2f" % [displayed, measured])
		return
	Engine.max_fps = 10
	var slower_deadline := Time.get_ticks_msec() + 4000
	var slow_fps := 0.0
	while Time.get_ticks_msec() < slower_deadline:
		await process_frame
		var value := label.text.trim_prefix("FPS: ")
		if value.is_valid_float():
			slow_fps = float(value)
		if absf(slow_fps - 10.0) <= 2.0 and absf(slow_fps - Engine.get_frames_per_second()) <= 2.0 and first_fps - slow_fps >= 5.0:
			break
	if absf(slow_fps - 10.0) > 2.0 or first_fps - slow_fps < 5.0:
		fail("FPS text did not follow 30-to-10 cap change: %.2f -> %s, measured %.2f" % [first_fps, label.text, Engine.get_frames_per_second()])
		return
	var fraction := label.text.split(".")
	if fraction.size() != 2 or fraction[1].length() != 2:
		fail("FPS display must preserve two decimal places: " + label.text)
		return

	graph.hide()
	await RenderingServer.frame_post_draw
	var without_graph := root.get_texture().get_image()
	graph.show()
	await RenderingServer.frame_post_draw
	var with_graph := root.get_texture().get_image()
	var bounds := graph.get_global_rect()
	var changed_bars := 0
	for y in range(int(bounds.position.y), int(bounds.end.y)):
		for x in range(int(bounds.position.x), int(bounds.end.x)):
			var drawn := with_graph.get_pixel(x, y)
			var background := without_graph.get_pixel(x, y)
			var difference := absf(drawn.r - background.r) + absf(drawn.g - background.g) + absf(drawn.b - background.b)
			if difference > 0.6:
				changed_bars += 1
	if changed_bars < 10:
		fail("Frame-time history did not change rendered graph pixels: %d" % changed_bars)
		return
	client.free()
	print("PASS: actual capped FPS, saved visibility and rendered frame-time history")
	quit(0)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
