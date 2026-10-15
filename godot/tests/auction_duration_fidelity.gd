extends SceneTree
# Supplied production auction snapshots; no server or trade lifecycle claim.

func _initialize() -> void:
	call_deferred("verify_duration")

func verify_duration() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("AH_DURATION_CAPTURE_PATH")
	assert(not output.is_empty(), "Set AH_DURATION_CAPTURE_PATH")
	var verify := OS.get_environment("AH_DURATION_VERIFY") == "1"
	for forever in [false, true]:
		for view in ["sell", "duration"]:
			OS.set_environment("GODOT_AUCTION_VIEW", view)
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			var error: String = ui.show_forever_auction_preview() if forever else ui.show_auction_preview()
			assert(error.is_empty(), error)
			for frame in range(100):
				await process_frame
				await RenderingServer.frame_post_draw
			if verify:
				verify_controls(ui, view == "duration")
			var skin := "forever" if forever else "modern"
			var image := root.get_texture().get_image()
			assert(image.get_size() == Vector2i(1920, 1080))
			assert(image.save_png(output.path_join("auction-duration-%s-%s.png" % [skin, view])) == OK)
			print("AH_DURATION skin=", skin, " view=", view, " verified=", verify)
			ui.queue_free()
			await process_frame
	quit(0)

func verify_controls(ui: Node, opened: bool) -> void:
	var owner := ui.find_child("AuctionHouseFrameItemSellFrameDurationDropdown", true, false) as Control
	assert(owner != null and owner.size == Vector2(115, 25), "Source caller width115")
	var label := ui.find_child("AuctionHouseFrameItemSellFrameDurationDropdownText", true, false) as Label
	assert(label != null and label.text == "1 Week", "Selected closed caption")
	assert(label.horizontal_alignment == HORIZONTAL_ALIGNMENT_RIGHT, "Caller RIGHT")
	assert(label.get_theme_font_size("font_size") == 12, "Number12Font")
	var menu := ui.find_child("AuctionHouseFrameDurationMenu", true, false) as Control
	if not opened:
		assert(menu == null, "Closed snapshot has no popup")
		return
	assert(menu != null and menu.size == Vector2(115, 83), "Source radios and menu insets")
	assert(menu.global_position.is_equal_approx(owner.global_position + Vector2(0, 25)), "Owner bottom-left anchor")
	for index in range(3):
		var prefix := "AuctionHouseFrameDurationMenuOption%d" % (index + 1)
		var row := ui.find_child(prefix, true, false) as Control
		assert(row != null and row.size == Vector2(99, 20), "Source row size")
		assert(row.global_position.is_equal_approx(menu.global_position + Vector2(8, 8 + index * 20)), "Source row anchor")
		var box := ui.find_child(prefix + "Box", true, false) as TextureRect
		assert(box != null and box.texture != null, "Radio texture loaded")
		assert((ui.find_child(prefix + "Tick", true, false) != null) == (index == 1), "Only Medium checked")
		var text := ui.find_child(prefix + "Text", true, false) as Label
		assert(text != null and text.get_theme_font_size("font_size") == 12, "Radio Number12Font")
