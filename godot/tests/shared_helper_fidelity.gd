extends SceneTree
# Real presentation screens with supplied samples; no server/persistence claim.

func _initialize() -> void:
	call_deferred("capture_panels")

func capture_panels() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("SHARED_HELPER_CAPTURE_PATH")
	assert(not output.is_empty(), "Set SHARED_HELPER_CAPTURE_PATH")
	for forever in [false, true]:
		for panel in ["quest", "mail", "raid", "bank"]:
			var probe = ClassDB.instantiate("UiAuditProbe")
			root.add_child(probe)
			var error: String = probe.mount_shared_helper_panel(forever, panel)
			assert(error.is_empty(), error)
			for frame in range(90):
				await process_frame
				await RenderingServer.frame_post_draw
			var button_name: String = {"quest": "QuestFrameAcceptButton", "mail": "SendMailCancelButton", "raid": "RaidInfoExtendButton", "bank": "BankFrameDepositAllButton"}[panel]
			var button := probe.find_child(button_name, true, false) as Control
			assert(button != null and button.size.x > 0, "Missing representative button " + button_name)
			var skin := "forever" if forever else "modern"
			var image := root.get_texture().get_image()
			assert(image.get_size() == Vector2i(1920, 1080), "Full-HD required")
			var path := output.path_join("shared-helper-%s-%s.png" % [skin, panel])
			assert(image.save_png(path) == OK, path)
			print("PASS SHARED_HELPER skin=", skin, " panel=", panel, " button=", button.get_global_rect(), " capture=", path)
			probe.queue_free()
			await process_frame
	quit(0)
