extends SceneTree
# Frozen production AH plus shared dropdown families; no server/trading claim.

func _initialize() -> void:
	call_deferred("capture_dropdowns")

func capture_dropdowns() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("DROPDOWN_CENTER_CAPTURE_PATH")
	assert(not output.is_empty(), "Set DROPDOWN_CENTER_CAPTURE_PATH")
	var verify := OS.get_environment("DROPDOWN_CENTER_VERIFY") == "1"
	for forever in [false, true]:
		for view in ["auction", "primitives"]:
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			var error: String
			if view == "auction":
				OS.set_environment("GODOT_AUCTION_VIEW", "sell")
				error = ui.show_forever_auction_preview() if forever else ui.show_auction_preview()
			else:
				error = ui.show_forever_menu_primitives_preview() if forever else ui.show_menu_primitives_preview()
			assert(error.is_empty(), error)
			for frame in range(90):
				await process_frame
				await RenderingServer.frame_post_draw
			var image := root.get_texture().get_image()
			assert(image.get_size() == Vector2i(1920, 1080))
			var skin := "forever" if forever else "modern"
			var prefix := "dropdown-%s-%s" % [skin, view]
			assert(image.save_png(output.path_join(prefix + ".png")) == OK)
			var names := ["AuctionHouseFrameItemSellFrameDurationDropdown"] if view == "auction" else ["ClosedDropdown", "OpenDropdown", "FilterDropdown", "Style2Dropdown"]
			for name in names:
				var owner := ui.find_child(name, true, false) as Control
				var caption := ui.find_child(name + "Text", true, false) as Label
				assert(owner != null and caption != null, "Actual shared dropdown and caption")
				var delta := caption.get_global_rect().get_center().y - owner.get_global_rect().get_center().y
				print("DROPDOWN_CENTER ", skin, " ", name, " owner=", owner.get_global_rect(), " caption=", caption.get_global_rect(), " font_height=", caption.get_theme_font("font").get_height(caption.get_theme_font_size("font_size")), " center_delta=", delta)
				if verify and name != "FilterDropdown":
					assert(absf(delta) <= 0.51, "Actual native caption rect centered: " + name)
				if view == "auction":
					assert(caption.text == "1 Week" and caption.horizontal_alignment == HORIZONTAL_ALIGNMENT_RIGHT and caption.get_theme_font_size("font_size") == 12, "Preserve AH caller text/font/alignment")
				var region := Rect2i(owner.get_global_rect().grow(14)).intersection(Rect2i(Vector2i.ZERO, image.get_size()))
				assert(image.get_region(region).save_png(output.path_join(prefix + "-" + name + "-crop.png")) == OK)
			if verify and view == "primitives":
				await verify_long_caption(ui, output, skin)
			ui.queue_free()
			await process_frame
	quit(0)

func verify_long_caption(ui: Node, output: String, skin: String) -> void:
	var caption := ui.find_child("ClosedDropdownText", true, false) as Label
	# Change content only: keep the production renderer's wrapping/overflow policy.
	caption.text = "A deliberately long specialization caption with several words"
	for frame in range(10):
		await process_frame
		await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	assert(image.save_png(output.path_join("dropdown-%s-long-caption.png" % skin)) == OK)
	print("DROPDOWN_LONG ", skin, " lines=", caption.get_line_count(), " rect=", caption.get_global_rect())
	assert(caption.get_line_count() == 1, "Source wordwrap=false must remain one line after centering")
	var owner := ui.find_child("ClosedDropdown", true, false) as Control
	assert(absf(caption.get_global_rect().get_center().y - owner.get_global_rect().get_center().y) <= 0.51, "Long caption remains centered")
