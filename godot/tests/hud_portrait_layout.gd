extends SceneTree
# Supplied default HUD and launcher through production screens; no live portrait/authority claim.
func _initialize() -> void:
	call_deferred("run_hud")

func run_hud() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("HUD_PORTRAIT_CAPTURE_PATH")
	var verify := OS.get_environment("HUD_PORTRAIT_VERIFY") == "1"
	assert(not output.is_empty())
	var observations: Array[Dictionary] = []
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.show_launcher_candidate(forever).is_empty())
		for frame in range(8):
			await process_frame
			await RenderingServer.frame_post_draw
		var player := ui.find_child("PlayerFrame", true, false) as Control
		var panel := ui.find_child("LauncherPanel", true, false) as Control
		var portrait := ui.find_child("PlayerPortrait", true, false) as Control
		var name_label := ui.find_child("PlayerName", true, false) as Label
		assert(player != null and panel != null and portrait != null and name_label != null)
		var player_rect := player.get_global_rect()
		var panel_rect := panel.get_global_rect()
		assert(player.is_visible_in_tree() and panel.is_visible_in_tree())
		assert(panel_rect.size == Vector2(526, 474))
		assert(name_label.text == "Launchpolish")
		assert(portrait.size == Vector2(60, 60))
		assert((portrait.global_position - player.global_position).distance_to(Vector2(24, 19)) <= 0.5)
		if forever:
			assert(player_rect.size == Vector2(321, 79))
			assert(player_rect.position.distance_to(Vector2(469.5, 770.5)) <= 0.75)
			var bars := ui.find_child("PlayerFrameBars", true, false) as Control
			assert(bars != null and bars.size == Vector2(240, 60))
			assert((bars.global_position - player.global_position).distance_to(Vector2(81, 0)) <= 0.5)
			assert(portrait.get_global_rect().end.x <= bars.get_global_rect().position.x + 4)
		if verify:
			assert(not panel_rect.intersects(player_rect), "Launcher must clear the complete portrait-inclusive player frame")
			assert(not panel_rect.intersects(name_label.get_global_rect()), "Launcher must clear player name")
			var expected_y := 293.5 if forever else 303.0
			assert(absf(panel_rect.position.y - expected_y) <= 0.5)
		print("HUD_PORTRAIT ", skin, " player=", player_rect, " launcher=", panel_rect, " overlap=", panel_rect.intersects(player_rect))
		observations.append({"skin": skin, "player": str(player_rect), "panel": str(panel_rect), "name": str(name_label.get_global_rect()), "overlap": panel_rect.intersects(player_rect)})
		var image := root.get_texture().get_image()
		assert(image.get_size() == Vector2i(1920, 1080))
		assert(image.save_png(output.path_join(skin + "-hud-context.png")) == OK)
		assert(image.get_region(Rect2i(400, 740, 450, 150)).save_png(output.path_join(skin + "-clearance-crop.png")) == OK)
		ui.queue_free()
		await process_frame
	var receipt := FileAccess.open(OS.get_environment("HUD_PORTRAIT_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS hud-portrait both-skin supplied default HUD; verified=", verify)
	quit(0)
