extends SceneTree
# Four native, off-display 1920x1080 acceptance captures. No network/session.
func _initialize() -> void:
	call_deferred("capture_cases")

func capture_cases() -> void:
	var output := OS.get_environment("GODOT_BUTTONFIT_CAPTURE_PATH")
	if output.is_empty():
		push_error("Set GODOT_BUTTONFIT_CAPTURE_PATH")
		quit(1)
		return
	for forever in [false, true]:
		for case in ["professions", "auction"]:
			if not await capture_case(output, case, forever):
				quit(1)
				return
	print("PASS: four native buttonfit captures")
	quit(0)

func capture_case(output: String, case: String, forever: bool) -> bool:
	var ui = ClassDB.instantiate("RegistryUi")
	root.add_child(ui)
	var error
	if case == "auction":
		error = ui.call("show_buttonfit_auction_preview", forever)
	else:
		error = ui.call("show_forever_professions_preview" if forever else "show_professions_preview")
	if error != "":
		push_error(error)
		return false
	for frame in range(240):
		await process_frame
		await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.get_size() != Vector2i(1920, 1080) or DisplayServer.window_get_size() != Vector2i(1920, 1080):
		push_error("Native compositor/window/framebuffer must all be 1920x1080")
		return false
	if not caption_fits(ui, case):
		return false
	var name := "%s-%s.png" % [case, "forever" if forever else "modern"]
	if image.save_png(output.path_join(name)) != OK:
		push_error("Cannot save " + name)
		return false
	print("PASS: ", name, " rendered1920x1080")
	ui.queue_free()
	await process_frame
	return true

func caption_fits(ui: Node, case: String) -> bool:
	if case == "professions":
		var button := ui.find_child("ProfessionsCreateAll", true, false) as Control
		var label := button.get_node("Parts/Text") as Label
		var measured := label.get_theme_font("font").get_string_size(label.text, HORIZONTAL_ALIGNMENT_LEFT, -1, label.get_theme_font_size("font_size")).x
		if button.size != Vector2(114, 22) or measured > button.size.x - 40:
			push_error("CreateAll text/padding does not fit114x22: " + str(measured))
			return false
		print("PASS: CreateAll text=", measured, " width=", button.size.x, " padding40")
	else:
		for name in ["AuctionPagePrev", "AuctionPageNext"]:
			var button := ui.find_child(name, true, false) as Button
			if button == null or button.size != Vector2(32, 32) or button.disabled or button.has_node("Parts/Text"):
				push_error("Expected enabled Retail32px page arrow: " + name)
				return false
		var label := ui.find_child("AuctionPageLabel", true, false) as Label
		var measured := label.get_theme_font("font").get_string_size(label.text, HORIZONTAL_ALIGNMENT_LEFT, -1, label.get_theme_font_size("font_size")).x
		if label.text != "Results 2/3" or measured > label.size.x:
			push_error("Page summary does not fit")
			return false
		print("PASS: enabled arrows32; summary=", measured, " available=", label.size.x)
	return true
