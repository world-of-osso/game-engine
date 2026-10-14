extends SceneTree
# Offline source presentation only; no persistence, known-spell or server authority claim.

func _initialize() -> void:
	call_deferred("capture_settings")

func capture_settings() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("SETTINGS_ART_CAPTURE_PATH")
	assert(not output.is_empty(), "Set SETTINGS_ART_CAPTURE_PATH")
	for forever in [false, true]:
		for auras in [false, true]:
			var probe = ClassDB.instantiate("UiAuditProbe")
			root.add_child(probe)
			var error: String = probe.mount_cooldown_settings(forever, auras)
			assert(error.is_empty(), error)
			for frame in range(120):
				await process_frame
				await RenderingServer.frame_post_draw
			var ui: Node = probe.find_child("CooldownSettingsAuditUI", true, false)
			verify_source_regions(ui, auras)
			var image := root.get_texture().get_image()
			assert(image.get_size() == Vector2i(1920, 1080), "1920x1080 required")
			var skin := "forever" if forever else "modern"
			var mode := "auras" if auras else "spells"
			var path := output.path_join("cooldown-settings-%s-%s.png" % [skin, mode])
			assert(image.save_png(path) == OK, "Cannot save " + path)
			print("PASS COOLDOWN_SETTINGS_ART skin=", skin, " mode=", mode, " capture=", path)
			probe.queue_free()
			await process_frame
	quit(0)

func verify_source_regions(ui: Node, auras: bool) -> void:
	var shell := ui.find_child("CooldownViewerSettings", true, false) as Control
	var background := ui.find_child("CooldownSettingsInsetBackground", true, false) as Control
	var corner := ui.find_child("CooldownSettingsInsetTopLeft", true, false) as Control
	assert(shell.get_global_rect() == Rect2(16, 116, 399, 609), "ButtonFrame shell")
	assert(background != null and corner != null, "Settings background and shared inset border")
	assert(background.get_global_rect() == Rect2(20, 176, 389, 523), "Background setAllPoints to ButtonFrame Inset")
	var background_parts := background.find_children("*", "TextureRect", true, false)
	assert(not background_parts.is_empty() and background_parts[0].texture != null, "Decoded character-panel background")
	for index in range(3):
		var name: String = ["Spells", "Auras", "GroupBuffs"][index]
		var tab := ui.find_child("CooldownSettings%sTab" % name, true, false) as Control
		assert(tab.get_global_rect() == Rect2(415, 144 + index * 58, 43, 55), "LargeSideTabButtonTemplate")
	if not auras:
		var icon := ui.find_child("CooldownSettingsItem26Icon", true, false) as Control
		assert(icon.size == Vector2(38, 38), "SettingsItemTemplate icon")
		return
	var item := ui.find_child("CooldownSettingsItem68661", true, false) as Control
	var fill := ui.find_child("CooldownSettingsItem68661Bar", true, false) as Control
	var bar_background := ui.find_child("CooldownSettingsItem68661BarBackground", true, false) as Control
	assert(item.size == Vector2(317, 38), "SettingsBarItemTemplate")
	assert(fill.size == Vector2(274, 19), "Settings bar LEFT=icon RIGHT+5, RIGHT=item RIGHT")
	assert(fill.global_position.distance_to(item.global_position + Vector2(43, 9.5)) <= 0.51, "Settings source bar anchor")
	assert(bar_background.size == Vector2(282, 30), "Source settings bar background")
	var parts := fill.find_children("*", "TextureRect", true, false)
	assert(not parts.is_empty(), "Rendered settings fill")
	assert(parts[0].self_modulate.is_equal_approx(Color(1, 0.5, 0.25, 1)), "COOLDOWN_BAR_DEFAULT_COLOR")
