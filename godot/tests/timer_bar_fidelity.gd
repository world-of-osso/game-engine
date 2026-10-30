extends SceneTree
# Offline projection of concrete timer fixtures; no server schedule/authority claim.
func _initialize() -> void:
	call_deferred("capture_timer_bars")

func capture_timer_bars() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("TIMER_BAR_CAPTURE_PATH")
	if output.is_empty():
		push_error("Set TIMER_BAR_CAPTURE_PATH")
		quit(1)
		return
	for forever in [true]:
		var probe = ClassDB.instantiate("UiAuditProbe")
		root.add_child(probe)
		var error: String = probe.mount_encounter_timer_bars(forever)
		if not error.is_empty():
			push_error(error)
			quit(1)
			return
		for frame in range(120):
			await process_frame
			await RenderingServer.frame_post_draw
		var ui: Node = probe.find_child("EncounterTimerAuditUI", true, false)
		if not verify_timer_geometry(ui):
			root.get_texture().get_image().save_png(output.path_join("encounter-timer-geometry-failure.png"))
			quit(1)
			return
		var image := root.get_texture().get_image()
		if image.get_size() != Vector2i(1920, 1080):
			push_error("Native capture must be 1920x1080")
			quit(1)
			return
		var path := output.path_join("encounter-timer-bars-%s.png" % ("forever" if forever else "modern"))
		if image.save_png(path) != OK:
			push_error("Cannot save " + path)
			quit(1)
			return
		print("PASS ENCOUNTER_TIMER_BARS skin=", "forever" if forever else "modern", " full/half/empty source background/fill/spark", " capture=", path)
		probe.queue_free()
		await process_frame
	quit(0)

func verify_timer_geometry(ui: Node) -> bool:
	var view := ui.find_child("EncounterTimeline", true, false) as Control
	if view == null or view.size.distance_to(Vector2(320, 387)) > 0.01:
		push_error("TimerView Lua must resize the XML placeholder to 320x387")
		return false
	for index in range(1, 4):
		var prefix := "EncounterTimer%d" % index
		var fraction := (index - 1) * 0.5
		var row := ui.find_child(prefix, true, false) as Control
		var bar := ui.find_child(prefix + "Bar", true, false) as Control
		var background := ui.find_child(prefix + "Background", true, false) as Control
		var fill := ui.find_child(prefix + "Fill", true, false) as Control
		var spark := ui.find_child(prefix + "Spark", true, false) as Control
		if row == null or bar == null or background == null or fill == null or spark == null:
			push_error("Missing source timer regions: " + prefix)
			return false
		if row.size.distance_to(Vector2(300, 34)) > 0.01 or bar.size.distance_to(Vector2(256, 26)) > 0.01:
			push_error("Timer row/bar differs from source template")
			return false
		if row.global_position.distance_to(view.global_position + Vector2(10, 343 - (index - 1) * 36)) > 0.01:
			push_error("Timer row must use source horizontal centring and bottom-to-top offset")
			return false
		if bar.global_position.distance_to(row.global_position + Vector2(40, 4)) > 0.01:
			push_error("Timer bar must anchor to icon RIGHT +4")
			return false
		if background.size.distance_to(Vector2(264, 36)) > 0.01 or background.global_position.distance_to(bar.global_position + Vector2(-2, -3)) > 0.01:
			push_error("Timer background must retain XML 36px height/edge overhang")
			return false
		print("TIMER_GEOMETRY ", prefix, " fraction=", fraction, " fill=", fill.size, " hidden=", not fill.visible, " spark=", spark.size, " spark_visible=", spark.visible)
		# Hidden native regions have no projected rect; visibility is the zero-time contract.
		if fraction == 0:
			if fill.visible or spark.visible:
				push_error("Expired timer must hide its fill and spark")
				return false
			continue
		if absf(fill.size.x - 256 * fraction) > 0.01 or spark.size.distance_to(Vector2(10, 46)) > 0.01:
			push_error("Remaining fraction/spark atlas size differs from source")
			return false
		if spark.global_position.distance_to(bar.global_position + Vector2(256 * fraction - 5, -10)) > 0.01 or spark.visible != (fraction > 0):
			push_error("Spark must follow fill edge and hide at zero")
			return false
		if fraction > 0 and fill.find_children("*", "TextureRect", true, false).is_empty():
			push_error("Timer fill did not load real source art")
			return false
	return true
