extends SceneTree
# Offline purchased-but-empty bank sample, through the production presenter.

func _initialize() -> void:
	call_deferred("verify_empty_bank")

func verify_empty_bank() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("BANK_GRID_CAPTURE_PATH")
	assert(not output.is_empty(), "Set BANK_GRID_CAPTURE_PATH")
	var failures: Array[String] = []
	for forever in [false, true]:
		var probe = ClassDB.instantiate("UiAuditProbe")
		root.add_child(probe)
		var error: String = probe.mount_shared_helper_panel(forever, "bank")
		assert(error.is_empty(), error)
		for frame in range(90):
			await process_frame
			await RenderingServer.frame_post_draw
		var skin := "forever" if forever else "modern"
		var shell := probe.find_child("BankFrame", true, false) as Control
		var count := 0
		for index in range(98):
			var prefix := "BankFrameItem%d" % (index + 1)
			var slot := probe.find_child(prefix, true, false) as Control
			if slot == null:
				continue
			count += 1
			var column := floori(index / 7.0)
			var row := index % 7
			var expected := shell.global_position + Vector2(26 + column * 45 + floori(column / 2.0) * 11, 63 + row * 47)
			assert(slot.global_position.is_equal_approx(expected), "Retail grid anchor " + prefix)
			assert(slot.size == Vector2(37, 37), "Retail item button size " + prefix)
			assert(slot.is_visible_in_tree(), "Empty slots stay visible " + prefix)
			assert(probe.find_child(prefix + "Background", true, false) != null, "Empty slot art " + prefix)
			assert(probe.find_child(prefix + "Icon", true, false) == null, "Empty sample has no item icon " + prefix)
		var image := root.get_texture().get_image()
		assert(image.get_size() == Vector2i(1920, 1080))
		var path := output.path_join("bank-empty-grid-%s.png" % skin)
		assert(image.save_png(path) == OK)
		print("BANK_GRID skin=", skin, " count=", count, " capture=", path)
		if count != 98:
			failures.append("%s expected98 actual%d" % [skin, count])
		probe.queue_free()
		await process_frame
	for failure in failures:
		push_error(failure)
	quit(0 if failures.is_empty() else 1)
