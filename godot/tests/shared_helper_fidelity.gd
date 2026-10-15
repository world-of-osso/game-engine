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
		for panel in ["quest", "mail", "raid", "bank", "settings"]:
			var probe = ClassDB.instantiate("UiAuditProbe")
			root.add_child(probe)
			var error: String = probe.mount_shared_helper_panel(forever, panel)
			assert(error.is_empty(), error)
			for frame in range(90):
				await process_frame
				await RenderingServer.frame_post_draw
			var button_name: String = {"quest": "QuestFrameAcceptButton", "mail": "SendMailCancelButton", "raid": "RaidInfoExtendButton", "bank": "BankFrameDepositAllButton", "settings": "CooldownSettingsUndo"}[panel]
			var button := probe.find_child(button_name, true, false) as Control
			assert(button != null and button.size.x > 0, "Missing representative button " + button_name)
			var source_size: Vector2 = {"quest": Vector2(77, 22), "mail": Vector2(80, 22), "raid": Vector2(245, 30), "bank": Vector2(256, 24), "settings": Vector2(164, 22)}[panel]
			assert(button.size.is_equal_approx(source_size), "XML/Lua geometry at UI scale1: " + button_name + " got " + str(button.size))
			if panel != "raid":
				verify_window_region_bounds(probe, panel)
			var skin := "forever" if forever else "modern"
			var image := root.get_texture().get_image()
			assert(image.get_size() == Vector2i(1920, 1080), "Full-HD required")
			var path := output.path_join("shared-helper-%s-%s.png" % [skin, panel])
			assert(image.save_png(path) == OK, path)
			print("PASS SHARED_HELPER skin=", skin, " panel=", panel, " button=", button.get_global_rect(), " capture=", path)
			probe.queue_free()
			await process_frame
	quit(0)

func verify_window_region_bounds(probe: Node, panel: String) -> void:
	var prefix: String = {"quest": "QuestFrame", "mail": "MailFrame", "bank": "BankFrame", "settings": "CooldownViewerSettings"}[panel]
	var shell := probe.find_child(prefix, true, false) as Control
	var background := probe.find_child(prefix + "Bg", true, false) as Control
	var streaks := probe.find_child(prefix + "TopTileStreaks", true, false) as Control
	assert(background != null and streaks != null, "Source window regions " + prefix)
	assert(background.size == shell.size - Vector2(4, 23), "Full continuous rock region " + prefix)
	assert(background.global_position.is_equal_approx(shell.global_position + Vector2(2, 21)), "Source rock origin " + prefix)
	assert(streaks.size == Vector2(shell.size.x - 8, 43), "Source streak extent " + prefix)
	assert(streaks.global_position.is_equal_approx(shell.global_position + Vector2(6, 21)), "Source streak origin " + prefix)
