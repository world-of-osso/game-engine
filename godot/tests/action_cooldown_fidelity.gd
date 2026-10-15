extends SceneTree
# Frozen production action-bar views through the actual post-sync swipe function.

func _initialize() -> void:
	call_deferred("verify_action_swipes")

func verify_action_swipes() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("ACTION_SWIPE_CAPTURE_PATH")
	assert(not output.is_empty(), "Set ACTION_SWIPE_CAPTURE_PATH")
	var verify := OS.get_environment("ACTION_SWIPE_VERIFY") == "1"
	for forever in [false, true]:
		var probe = ClassDB.instantiate("UiAuditProbe")
		root.add_child(probe)
		var error: String = probe.mount_action_cooldown_bars(forever)
		assert(error.is_empty(), error)
		var baseline: Image
		var fractions := [0.0, 0.25, 0.5, 0.75, 1.0, 0.0]
		for step in range(fractions.size()):
			var fraction: float = fractions[step]
			error = probe.set_action_cooldown_fraction(fraction)
			assert(error.is_empty(), error)
			for frame in range(45):
				await process_frame
				await RenderingServer.frame_post_draw
			var image := root.get_texture().get_image()
			if step == 0:
				baseline = image
			if verify:
				verify_quadrants(probe, baseline, image, fraction)
			var skin := "forever" if forever else "modern"
			assert(image.get_size() == Vector2i(1920, 1080))
			assert(image.save_png(output.path_join("action-cooldown-%s-%d.png" % [skin, step])) == OK)
			print("ACTION_SWIPE skin=", skin, " fraction=", fraction, " verified=", verify)
		probe.queue_free()
		await process_frame
	quit(0)

func verify_quadrants(probe: Node, baseline: Image, image: Image, fraction: float) -> void:
	# Top-right, bottom-right, bottom-left, top-left: clockwise from twelve o'clock.
	var samples := [Vector2(0.75, 0.25), Vector2(0.75, 0.75), Vector2(0.25, 0.75), Vector2(0.25, 0.25)]
	for button in [1, 2]:
		var prefix := "ActionButton%d" % button
		var icon := probe.find_child(prefix + "Icon", true, false) as Control
		var host := probe.find_child(prefix + "Cooldown", true, false) as Control
		assert(icon != null and host != null, "Actual action icon and cooldown host")
		var scale: float = icon.size.x / 45.0
		assert(host.size.is_equal_approx(Vector2(39, 39) * scale), "Full source inset square")
		assert(host.global_position.is_equal_approx(icon.global_position + Vector2(3, 3) * scale), "Source icon inset")
		for index in range(samples.size()):
			var point := Vector2i(host.global_position + samples[index] * host.size)
			var original := baseline.get_pixelv(point)
			var actual := image.get_pixelv(point)
			var covered: bool = (float(index) + 0.5) / 4.0 < fraction
			var original_luma: float = (original.r + original.g + original.b) / 3.0
			assert(original_luma > 0.03, "Concrete source icon sample is observable")
			if covered:
				var expected := original * 0.36 # Source black at 0.64 opacity.
				assert(absf(actual.r - expected.r) < 0.015 and absf(actual.g - expected.g) < 0.015 and absf(actual.b - expected.b) < 0.015, "Source opacity quadrant %s/%d: original=%s actual=%s expected=%s" % [prefix, index, original, actual, expected])
			else:
				assert(absf(actual.r - original.r) < 0.01 and absf(actual.g - original.g) < 0.01 and absf(actual.b - original.b) < 0.01, "Uncovered/reset quadrant %s/%d" % [prefix, index])
