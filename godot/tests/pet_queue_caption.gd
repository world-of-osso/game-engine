extends SceneTree
# Supplied proposal through production pet journal, including closed-journal popup.

func _initialize() -> void:
	call_deferred("verify_caption")

func verify_caption() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("PET_QUEUE_CAPTURE_PATH")
	assert(not output.is_empty(), "Set PET_QUEUE_CAPTURE_PATH")
	var verify := OS.get_environment("PET_QUEUE_VERIFY") == "1"
	for forever in [false, true]:
		var probe = ClassDB.instantiate("UiAuditProbe")
		root.add_child(probe)
		var error: String = probe.mount_collections(forever, "pet-queue")
		assert(error.is_empty(), error)
		for frame in range(90):
			await process_frame
			await RenderingServer.frame_post_draw
		var popup := probe.find_child("PetBattleQueueReadyFrame", true, false) as Control
		assert(popup != null and popup.size == Vector2(320, 200), "Existing queue geometry")
		assert(probe.find_child("CollectionsJournal", true, false) == null, "Closed journal")
		if verify:
			var caption := probe.find_child("PetBattleQueueLabel", true, false) as Label
			assert(caption != null and caption.is_visible_in_tree(), "Rendered visible caption")
			assert(caption.text == "Your Pet Battle match is ready.", "Ready caption text")
			assert(caption.horizontal_alignment == HORIZONTAL_ALIGNMENT_CENTER, "Preserved CENTER")
			assert(caption.size == Vector2(300, 25), "Preserved caption geometry")
			assert(caption.global_position.is_equal_approx(popup.global_position + Vector2(10, 130)), "Preserved caption anchor")
			assert(caption.get_theme_font_size("font_size") == 14, "Preserved caption font size")
		var skin := "forever" if forever else "modern"
		var image := root.get_texture().get_image()
		assert(image.get_size() == Vector2i(1920, 1080))
		assert(image.save_png(output.path_join("pet-queue-caption-%s.png" % skin)) == OK)
		print("PET_QUEUE skin=", skin, " verified=", verify)
		probe.queue_free()
		await process_frame
	quit(0)
