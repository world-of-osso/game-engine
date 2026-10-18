extends SceneTree
# Supplied account context through production name selection and native HUD projection.
# Not authenticated transport, actual roster-head rendering, or sound acceptance.
const HUD_SCALE := 1080.0 / 768.0
const INK_OVERSCAN_PX := 4

func _initialize() -> void:
	call_deferred("run_difficulty")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func rect(control: Control) -> Dictionary:
	return {"position": [control.global_position.x, control.global_position.y], "size": [control.size.x, control.size.y]}

func controls(fixture: Node) -> Dictionary:
	var result := {}
	for name in ["ReadyCheckFrame", "ReadyCheckFrameTitle", "ReadyCheckFrameText", "ReadyCheckPortrait", "ReadyCheckFrameYesButton", "ReadyCheckFrameNoButton"]:
		var node := fixture.find_child(name, true, false) as Control
		assert(node != null)
		result[name] = rect(node)
	return result

func scan_ink_bounds(text_image: Image, blank: Image, region: Rect2i) -> Dictionary:
	var left := region.end.x
	var top := region.end.y
	var right := region.position.x
	var bottom := region.position.y
	var count := 0
	var bands: Array = []
	var band_start := -1
	for y in range(region.position.y, region.end.y):
		var row_has_ink := false
		for x in range(region.position.x, region.end.x):
			if text_image.get_pixel(x, y) != blank.get_pixel(x, y):
				left = mini(left, x)
				top = mini(top, y)
				right = maxi(right, x + 1)
				bottom = maxi(bottom, y + 1)
				count += 1
				row_has_ink = true
		if row_has_ink and band_start < 0:
			band_start = y
		if not row_has_ink and band_start >= 0:
			bands.append([band_start, y])
			band_start = -1
	if band_start >= 0:
		bands.append([band_start, region.end.y])
	assert(count > 0, "Text measurement must observe actual raster pixels")
	print("READY_INK_SCAN region=", region, " bounds=", [left, top, right, bottom], " pixels=", count, " row_bands=", bands)
	assert(left > region.position.x and top > region.position.y and right < region.end.x and bottom < region.end.y, "Ink must not touch the scan boundary; clipped metrics are invalid")
	return {"bounds": [left, top, right, bottom], "bottom_exclusive": bottom, "pixels": count, "row_bands": bands}

func capture_clearance(fixture: Node, message: Label, reference: Image, output: String, stem: String) -> Dictionary:
	var yes := fixture.find_child("ReadyCheckFrameYesButton", true, false) as Button
	var no := fixture.find_child("ReadyCheckFrameNoButton", true, false) as Button
	assert(yes != null and no != null and yes.visible and no.visible)
	var button_top := minf(yes.global_position.y, no.global_position.y)
	var line_heights: Array = []
	for line in range(message.get_line_count()):
		line_heights.append(message.get_line_height(line))
	var region := Rect2i(message.get_global_rect().grow(INK_OVERSCAN_PX)).intersection(Rect2i(Vector2i.ZERO, reference.get_size()))
	# Isolate complete text, including pixels otherwise occluded by the response buttons.
	yes.hide()
	no.hide()
	await settle()
	var shadowed := root.get_texture().get_image()
	var shadow_color := message.get_theme_color("font_shadow_color")
	message.add_theme_color_override("font_shadow_color", Color.TRANSPARENT)
	await settle()
	var glyphs := root.get_texture().get_image()
	message.hide()
	await settle()
	var blank := root.get_texture().get_image()
	message.show()
	message.add_theme_color_override("font_shadow_color", shadow_color)
	yes.show()
	no.show()
	await settle()
	assert(root.get_texture().get_image().get_data() == reference.get_data(), "Raster isolation must restore exact source frame")
	# Persist isolation frames before validation so a failed scan stays diagnosable.
	assert(shadowed.get_region(region).save_png(output.path_join(stem + "-unoccluded.png")) == OK)
	assert(glyphs.get_region(region).save_png(output.path_join(stem + "-glyphs.png")) == OK)
	assert(blank.get_region(region).save_png(output.path_join(stem + "-blank.png")) == OK)
	print("READY_INTRINSIC_CONTEXT ", stem, " body=", message.get_global_rect(), " lines=", line_heights, " button_top=", button_top)
	var glyph := scan_ink_bounds(glyphs, blank, region)
	var shadow := scan_ink_bounds(shadowed, glyphs, region)
	var combined := scan_ink_bounds(shadowed, blank, region)
	for ink in [glyph, shadow, combined]:
		ink["clearance_to_button_top_px"] = button_top - ink.bottom_exclusive
	return {"scan_region_px": [region.position.x, region.position.y, region.size.x, region.size.y], "button_top_px": button_top, "line_count": message.get_line_count(), "line_heights_ui": line_heights, "line_spacing_ui": message.get_theme_constant("line_spacing"), "hud_scale": HUD_SCALE, "glyph": glyph, "shadow": shadow, "combined": combined, "layout_gap": "READY_CHECK_MULTILINE_BODY_BUTTON_COLLISION" if combined.clearance_to_button_top_px <= 0 else ""}

func run_difficulty() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("READY_DIFFICULTY_CAPTURE_PATH")
	assert(not output.is_empty())
	var observations: Array[Dictionary] = []
	var phases := [["continent-raid", ""], ["actual3", "10 Player"], ["actual4", "25 Player"], ["actual5", "10 Player (Heroic)"], ["actual6", "25 Player (Heroic)"], ["continent-raid", ""], ["continent-dungeon", ""], ["selected4", "25 Player"], ["current14", ""], ["transfer", ""], ["actual3", "10 Player"]]
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		var fixture := ClassDB.instantiate("ReadyCheckDifficultyFixture") as Node
		root.add_child(fixture)
		assert(fixture.initialize(forever) == "")
		var conserved := {}
		var first_actual: PackedByteArray
		for index in range(phases.size()):
			var phase: Array = phases[index]
			assert(fixture.step(phase[0]) == "")
			await settle()
			var panel := fixture.find_child("ReadyCheckFrame", true, false) as Control
			var message := fixture.find_child("ReadyCheckFrameText", true, false) as Label
			var title := fixture.find_child("ReadyCheckFrameTitle", true, false) as Label
			assert(panel != null and message != null and title != null)
			var expected := "Ann has initiated a ready check."
			if not phase[1].is_empty():
				expected += "\nRaid Difficulty: " + phase[1]
			assert(message.text == expected, phase[0] + " source current-name rule")
			assert(message.size.x == 240 and message.size.y > 0 and panel.size == Vector2(323, 100))
			var anchor := (message.global_position - panel.global_position) / HUD_SCALE
			assert(absf(anchor.x - 61.5) <= 0.5 and anchor.y == 37)
			assert(title.text == "Ready Check" and title.size == Vector2(241, 20))
			for label in [message, title]:
				assert(label.get_theme_font_size("font_size") == 12)
				assert(label.get_theme_color("font_color").is_equal_approx(Color(1, 210.0 / 255.0, 0)))
				assert(label.get_theme_color("font_shadow_color") == Color.BLACK)
				assert(Vector2i(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y")) == Vector2i(1, 1))
			var current := controls(fixture)
			var fixed_controls := current.duplicate(true)
			fixed_controls.erase("ReadyCheckFrameText")
			if index == 0:
				conserved = fixed_controls
			assert(fixed_controls == conserved, "Frame/title/portrait/buttons remain unchanged")
			var image := root.get_texture().get_image()
			assert(image.get_size() == Vector2i(1920, 1080))
			var filename: String = skin + "-" + str(index) + "-" + phase[0] + ".png"
			assert(image.save_png(output.path_join(filename)) == OK)
			if index == 1:
				first_actual = image.get_data()
			if index == phases.size() - 1:
				assert(image.get_data() == first_actual, "Reopened actual3 restores exact RGBA")
			var clearance: Dictionary = await capture_clearance(fixture, message, image, output, filename.trim_suffix(".png"))
			assert(clearance.line_count == (1 if phase[1].is_empty() else 2))
			observations.append({"skin": skin, "index": index, "phase": phase[0], "text": message.text, "controls": current, "image": filename, "clearance": clearance, "message_pixel_rect": {"x": message.global_position.x, "y": message.global_position.y, "width": message.size.x * HUD_SCALE, "height": message.size.y * HUD_SCALE}})
		fixture.free()
		await settle()
	var receipt := FileAccess.open(OS.get_environment("READY_DIFFICULTY_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("OBSERVED ready-check-difficulty both-skin source intrinsic body; glyph/line/shadow clearance and named gaps recorded, not layout acceptance")
	quit(0)
