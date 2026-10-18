extends SceneTree
# Title bounds only, on the production HUD scale with its unchanged roster head.
const HUD_SCALE := 1080.0 / 768.0
var fixture: Node

func _initialize() -> void:
	call_deferred("run_title_bounds")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func rect(control: Control) -> Dictionary:
	return {"position": [control.global_position.x, control.global_position.y], "logical_size": [control.size.x, control.size.y]}

func wait_for_head() -> void:
	for frame in range(1200):
		assert(fixture.tick() == "")
		await process_frame
		var state: Dictionary = fixture.ready_check_portrait_state()
		if state.get("model_shown", false) and state.get("mask_loaded", false) and not state.get("pending", true):
			await settle()
			return
	assert(false, "Unchanged roster head did not load")

func run_title_bounds() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("READY_TITLE_BOUNDS_CAPTURE_PATH")
	assert(not output.is_empty())
	var observations: Array[Dictionary] = []
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		fixture = ClassDB.instantiate("PartyPortraitFixture")
		root.add_child(fixture)
		assert(fixture.initialize(forever) == "")
		var names: Array[String] = ["Ann", "Bob"]
		assert(fixture.set_members(names, true, true) == "")
		assert(fixture.apply_ready_check(false) == "")
		await wait_for_head()
		var panel := fixture.find_child("ReadyCheckFrame", true, false) as Control
		var title := fixture.find_child("ReadyCheckFrameTitle", true, false) as Label
		var message := fixture.find_child("ReadyCheckFrameText", true, false) as Label
		var portrait := fixture.find_child("ReadyCheckPortrait", true, false) as Control
		assert(panel != null and title != null and message != null and portrait != null)
		assert(panel.size == Vector2(323, 100) and title.size == Vector2(241, 20))
		assert(((title.global_position - panel.global_position) / HUD_SCALE).is_equal_approx(Vector2(58, 6)))
		assert(title.text == "Ready Check")
		assert(message.text == "Ann has initiated a ready check." and message.size.x == 240 and message.size.y > 0)
		var message_anchor := (message.global_position - panel.global_position) / HUD_SCALE
		assert(absf(message_anchor.x - 61.5) <= 0.5 and message_anchor.y == 37)
		assert(portrait.size == Vector2(54, 54))
		assert(((portrait.global_position - panel.global_position) / HUD_SCALE).is_equal_approx(Vector2(0, -3)))
		for label in [title, message]:
			assert(label.get_theme_font_size("font_size") == 12)
			assert(label.get_theme_color("font_color").is_equal_approx(Color(1, 210.0 / 255.0, 0)))
			assert(label.get_theme_color("font_shadow_color") == Color.BLACK)
			assert(Vector2i(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y")) == Vector2i(1, 1))
		var buttons: Array[Dictionary] = []
		for suffix in ["Yes", "No"]:
			var button := fixture.find_child("ReadyCheckFrame" + suffix + "Button", true, false) as Button
			assert(button != null and not button.disabled and button.size == Vector2(119, 24))
			var caption := button.find_child("Text", true, false) as Label
			assert(caption != null and caption.text == ("Ready" if suffix == "Yes" else "Not Ready"))
			buttons.append({"name": suffix, "rect": rect(button), "caption": caption.text})
		var source_position := title.global_position
		var source_rect := rect(title)
		var image := root.get_texture().get_image()
		assert(image.get_size() == Vector2i(1920, 1080))
		assert(image.save_png(output.path_join(skin + "-context.png")) == OK)
		var crop_origin := panel.global_position - Vector2(16, 20) * HUD_SCALE
		var crop_size := Vector2(360, 140) * HUD_SCALE
		var crop := Rect2i(floori(crop_origin.x), floori(crop_origin.y), ceili(crop_size.x), ceili(crop_size.y))
		assert(image.get_region(crop).save_png(output.path_join(skin + "-panel-crop.png")) == OK)
		# Origin-only diagnostic: projection retains the current241px span.
		# The actual old323px span is covered by the retained61ef compiled captures.
		title.size = Vector2(241, 20)
		title.global_position = panel.global_position + Vector2(0, 4) * HUD_SCALE
		await settle()
		var previous_rect := rect(title)
		assert(title.size == Vector2(241, 20), "Origin-only control retains current width")
		var previous := root.get_texture().get_image()
		assert(previous.save_png(output.path_join(skin + "-previous-bounds-control.png")) == OK)
		assert(previous.get_region(crop).save_png(output.path_join(skin + "-previous-bounds-panel.png")) == OK)
		title.size = Vector2(241, 20)
		title.global_position = source_position
		await settle()
		assert(root.get_texture().get_image().get_data() == image.get_data(), "Restore source title bounds RGBA exactly")
		observations.append({"case": skin, "hud_scale": HUD_SCALE, "title": source_rect, "previous_title": previous_rect, "panel": rect(panel), "message": rect(message), "portrait": rect(portrait), "buttons": buttons, "source_logical_origin": [58, 6], "source_logical_width": 241, "conserved_height": 20})
		fixture.free()
		await settle()
	var receipt := FileAccess.open(OS.get_environment("READY_TITLE_BOUNDS_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS ready-title-bounds both-skin source origin/width and conservation")
	quit(0)
