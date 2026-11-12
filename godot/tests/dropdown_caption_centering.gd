extends SceneTree

var completion

func _initialize() -> void:
	var views := [
		"auction",
		"primitives",
		"group_finder",
		"options_interface",
		"options_hud",
		"options_accessibility",
		"questlog",
	]
	completion = preload("res://tests/completion.gd").new(self, views.size())
	call_deferred("capture_dropdowns", views)

func capture_dropdowns(views: Array) -> void:
	for frame in range(2):
		await process_frame
		await RenderingServer.frame_post_draw
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	for frame in range(3):
		await process_frame
		await RenderingServer.frame_post_draw
	var viewport_size := Vector2i(root.get_visible_rect().size)
	var image_size := root.get_texture().get_image().get_size()
	print("DROPDOWN_VIEWPORT root=", root.size, " display=", DisplayServer.window_get_size(), " viewport=", viewport_size, " image=", image_size)
	if not completion.require(root.size == Vector2i(1920, 1080) and viewport_size == Vector2i(1920, 1080) and image_size == Vector2i(1920, 1080), "Capture viewport must be 1920x1080"):
		return
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("DROPDOWN_CENTER_CAPTURE_PATH")
	if not completion.require(not output.is_empty(), "Set DROPDOWN_CENTER_CAPTURE_PATH"):
		return
	for view in views:
		var ui = ClassDB.instantiate("UiAuditProbe") if view == "questlog" else ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error := show_view(ui, view)
		if not completion.require(error.is_empty(), error):
			return
		for frame in range(90):
			await process_frame
			await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		if not completion.require(image.get_size() == Vector2i(1920, 1080)):
			return
		var path := output.path_join("dropdown-forever-%s.png" % view)
		if not completion.require(image.save_png(path) == OK, "Cannot save " + path):
			return
		var dropdowns := dropdown_captions(view)
		for item in dropdowns:
			var owner := ui.find_child(item.name, true, false) as Control
			if not completion.require(owner != null, "Dropdown owner exists: " + item.name):
				return
			var crop_rect := Rect2i(owner.get_global_rect().grow(14)).intersection(Rect2i(Vector2i.ZERO, image.get_size()))
			var crop_path := output.path_join("dropdown-forever-%s-%s-crop.png" % [view, item.name])
			if not completion.require(image.get_region(crop_rect).save_png(crop_path) == OK, "Cannot save " + crop_path):
				return
			if not completion.step(await verify_caption(ui, item), "verify " + item.name):
				return
		if view == "auction" and not completion.step(verify_auction_font(ui), "auction_font"):
			return
		if view == "primitives" and not completion.step(await verify_long_caption(ui, output), "long_caption"):
			return
		print("DROPDOWN_CAPTURE ", view, " ", path)
		if not completion.case_done():
			return
		ui.queue_free()
		await process_frame
	completion.finish(true)

func show_view(ui: Node, view: String) -> String:
	match view:
		"auction":
			OS.set_environment("GODOT_AUCTION_VIEW", "sell")
			return ui.show_forever_auction_preview()
		"primitives":
			return ui.show_forever_menu_primitives_preview()
		"group_finder":
			return ui.call("show_follower_layout_preview", true, false)
		"options_interface":
			return ui.call("show_buttonfit_service_preview", "options:interface", true)
		"options_hud":
			return ui.call("show_buttonfit_service_preview", "options:hud", true)
		"options_accessibility":
			return ui.call("show_buttonfit_service_preview", "options:accessibility", true)
		"questlog":
			return ui.call("mount_quest_overflow", true, "log", false)
	return "Unknown dropdown capture view: " + view

func dropdown_captions(view: String) -> Array:
	match view:
		"auction":
			return [{"name": "AuctionHouseFrameItemSellFrameDurationDropdown", "center": 13.0}]
		"primitives":
			return [
				{"name": "ClosedDropdown", "center": 13.0},
				{"name": "OpenDropdown", "center": 13.0},
				{"name": "FilterDropdown", "center": 10.0},
				{"name": "Style2Dropdown", "center": 12.5},
			]
		"group_finder":
			return [{"name": "LFDQueueFrameTypeDropdown", "center": 13.0}]
		"options_interface":
			return [
				{"name": "Choiceencounter_view", "center": 12.5},
				{"name": "Choicestatus_text_display", "center": 12.5},
			]
		"options_hud":
			return [
				{"name": "Choiceui_layout", "center": 12.5},
				{"name": "Choicelayout_system", "center": 12.5},
				{"name": "Choicelayout_font", "center": 12.5},
			]
		"options_accessibility":
			return [{"name": "Choiceinteract_key_icons", "center": 12.5}]
	return []

func verify_caption(ui: Node, item: Dictionary) -> bool:
	var owner := ui.find_child(item.name, true, false) as Control
	var caption := ui.find_child(item.name + "Text", true, false) as Label
	if not completion.require(owner != null and caption != null, "Dropdown and Text region exist: " + item.name):
		return false
	if not completion.require(owner.is_visible_in_tree() and root.get_visible_rect().encloses(owner.get_global_rect()), "Dropdown is visible in the Forever view: " + item.name):
		return false
	if not completion.require(caption.vertical_alignment == VERTICAL_ALIGNMENT_CENTER, "Text uses template MIDDLE justification: " + item.name):
		return false
	var production_text := caption.text
	if not completion.require(not production_text.is_empty(), "Dropdown has a selected caption: " + item.name):
		return false
	caption.text = "Hg"
	for frame in range(4):
		await process_frame
		await RenderingServer.frame_post_draw
	var capital := caption.get_character_bounds(0)
	var descender := caption.get_character_bounds(1)
	if not completion.require(capital.has_area() and descender.has_area(), "Probe glyph bounds are visible: " + item.name):
		return false
	var line_height := float(caption.get_line_height(0))
	var line_box_center := caption.global_position.y + (caption.size.y - line_height) / 2.0 + line_height / 2.0
	var ink_center := caption.global_position.y + (capital.position.y + descender.end.y) / 2.0
	var expected_center := owner.global_position.y + float(item.center)
	caption.text = production_text
	for frame in range(4):
		await process_frame
		await RenderingServer.frame_post_draw
	if not completion.require(caption.get_line_count() == 1, "Dropdown caption remains one line: " + item.name):
		return false
	print("DROPDOWN_LINE ", item.name, " text=", production_text, " font_height=", caption.get_theme_font("font").get_height(caption.get_theme_font_size("font_size")), " line_box_center=", line_box_center, " ink_center=", ink_center, " template_center=", expected_center)
	var line_centered := absf(line_box_center - expected_center) <= 0.01
	var glyphs_centered := absf(ink_center - expected_center) <= 2.0
	return completion.require(line_centered and glyphs_centered, "Rendered caption line and glyphs follow Retail Text bounds: " + item.name)

func verify_auction_font(ui: Node) -> bool:
	var caption := ui.find_child("AuctionHouseFrameItemSellFrameDurationDropdownText", true, false) as Label
	return completion.require(caption.text == "1 Week" and caption.horizontal_alignment == HORIZONTAL_ALIGNMENT_RIGHT and caption.get_theme_font_size("font_size") == 12, "Preserve AH caller caption, font size and RIGHT alignment")

func verify_long_caption(ui: Node, output: String) -> bool:
	var caption := ui.find_child("ClosedDropdownText", true, false) as Label
	var original_text := caption.text
	caption.text = "A deliberately long caption with several words"
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := output.path_join("dropdown-forever-long-caption.png")
	if not completion.require(image.save_png(path) == OK, "Cannot save long-caption capture"):
		return false
	var line_count := caption.get_line_count()
	caption.text = original_text
	for frame in range(4):
		await process_frame
		await RenderingServer.frame_post_draw
	print("DROPDOWN_LONG text=", original_text, " visible_line_count=", line_count)
	return completion.require(line_count == 1, "Source wordwrap=false keeps long captions on one line")
