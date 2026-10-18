extends SceneTree

var completion = preload("res://tests/completion.gd").new(self, 2)
# Title bounds only, on the production HUD scale with its unchanged roster head.
const HUD_SCALE := 1080.0 / 768.0
var fixture: Node

func _initialize() -> void:
	call_deferred("run_completion")

func settle() -> bool:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

	return true

func rect(control: Control) -> Dictionary:
	return {"position": [control.global_position.x, control.global_position.y], "logical_size": [control.size.x, control.size.y]}

func wait_for_head() -> bool:
	for frame in range(1200):
		if not completion.require(fixture.tick() == ""):
			return false
		await process_frame
		var state: Dictionary = fixture.ready_check_portrait_state()
		if state.get("model_shown", false) and state.get("mask_loaded", false) and not state.get("pending", true):
			if not completion.step(await settle(), "settle"):
				return false
			return true
	if not completion.require(false, "Unchanged roster head did not load"):
		return false

	return true

func run_title_bounds() -> bool:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("READY_TITLE_BOUNDS_CAPTURE_PATH")
	if not completion.require(not output.is_empty()):
		return false
	var observations: Array[Dictionary] = []
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		fixture = ClassDB.instantiate("PartyPortraitFixture")
		root.add_child(fixture)
		if not completion.require(fixture.initialize(forever) == ""):
			return false
		var names: Array[String] = ["Ann", "Bob"]
		if not completion.require(fixture.set_members(names, true, true) == ""):
			return false
		if not completion.require(fixture.apply_ready_check(false) == ""):
			return false
		if not completion.step(await wait_for_head(), "wait_for_head"):
			return false
		var panel := fixture.find_child("ReadyCheckFrame", true, false) as Control
		var title := fixture.find_child("ReadyCheckFrameTitle", true, false) as Label
		var message := fixture.find_child("ReadyCheckFrameText", true, false) as Label
		var portrait := fixture.find_child("ReadyCheckPortrait", true, false) as Control
		if not completion.require(panel != null and title != null and message != null and portrait != null):
			return false
		if not completion.require(panel.size == Vector2(323, 100) and title.size == Vector2(241, 20)):
			return false
		if not completion.require(((title.global_position - panel.global_position) / HUD_SCALE).is_equal_approx(Vector2(58, 6))):
			return false
		if not completion.require(title.text == "Ready Check"):
			return false
		if not completion.require(message.text == "Ann has initiated a ready check." and message.size.x == 240 and message.size.y > 0):
			return false
		var message_anchor := (message.global_position - panel.global_position) / HUD_SCALE
		if not completion.require(absf(message_anchor.x - 61.5) <= 0.5 and message_anchor.y == 37):
			return false
		if not completion.require(portrait.size == Vector2(54, 54)):
			return false
		if not completion.require(((portrait.global_position - panel.global_position) / HUD_SCALE).is_equal_approx(Vector2(0, -3))):
			return false
		for label in [title, message]:
			if not completion.require(label.get_theme_font_size("font_size") == 12):
				return false
			if not completion.require(label.get_theme_color("font_color").is_equal_approx(Color(1, 210.0 / 255.0, 0))):
				return false
			if not completion.require(label.get_theme_color("font_shadow_color") == Color.BLACK):
				return false
			if not completion.require(Vector2i(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y")) == Vector2i(1, 1)):
				return false
		var buttons: Array[Dictionary] = []
		for suffix in ["Yes", "No"]:
			var button := fixture.find_child("ReadyCheckFrame" + suffix + "Button", true, false) as Button
			if not completion.require(button != null and not button.disabled and button.size == Vector2(119, 24)):
				return false
			var caption := button.find_child("Text", true, false) as Label
			if not completion.require(caption != null and caption.text == ("Ready" if suffix == "Yes" else "Not Ready")):
				return false
			buttons.append({"name": suffix, "rect": rect(button), "caption": caption.text})
		var source_position := title.global_position
		var source_rect := rect(title)
		var image := root.get_texture().get_image()
		if not completion.require(image.get_size() == Vector2i(1920, 1080)):
			return false
		if not completion.require(image.save_png(output.path_join(skin + "-context.png")) == OK):
			return false
		var crop_origin := panel.global_position - Vector2(16, 20) * HUD_SCALE
		var crop_size := Vector2(360, 140) * HUD_SCALE
		var crop := Rect2i(floori(crop_origin.x), floori(crop_origin.y), ceili(crop_size.x), ceili(crop_size.y))
		if not completion.require(image.get_region(crop).save_png(output.path_join(skin + "-panel-crop.png")) == OK):
			return false
		# Origin-only diagnostic: projection retains the current241px span.
		# The actual old323px span is covered by the retained61ef compiled captures.
		title.size = Vector2(241, 20)
		title.global_position = panel.global_position + Vector2(0, 4) * HUD_SCALE
		if not completion.step(await settle(), "settle"):
			return false
		var previous_rect := rect(title)
		if not completion.require(title.size == Vector2(241, 20), "Origin-only control retains current width"):
			return false
		var previous := root.get_texture().get_image()
		if not completion.require(previous.save_png(output.path_join(skin + "-previous-bounds-control.png")) == OK):
			return false
		if not completion.require(previous.get_region(crop).save_png(output.path_join(skin + "-previous-bounds-panel.png")) == OK):
			return false
		title.size = Vector2(241, 20)
		title.global_position = source_position
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.require(root.get_texture().get_image().get_data() == image.get_data(), "Restore source title bounds RGBA exactly"):
			return false
		observations.append({"case": skin, "hud_scale": HUD_SCALE, "title": source_rect, "previous_title": previous_rect, "panel": rect(panel), "message": rect(message), "portrait": rect(portrait), "buttons": buttons, "source_logical_origin": [58, 6], "source_logical_width": 241, "conserved_height": 20})
		fixture.free()
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.case_done():
			return false
	var receipt := FileAccess.open(OS.get_environment("READY_TITLE_BOUNDS_RECEIPT"), FileAccess.WRITE)
	if not completion.require(receipt != null):
		return false
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS ready-title-bounds both-skin source origin/width and conservation")
	return true

func run_completion() -> void:
	completion.finish(await run_title_bounds())
