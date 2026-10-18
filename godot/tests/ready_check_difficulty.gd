extends SceneTree
# Supplied account context through production name selection and native HUD projection.
# Not authenticated transport, actual roster-head rendering, or sound acceptance.
const HUD_SCALE := 1080.0 / 768.0

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
			assert(message.size == Vector2(240, 28) and panel.size == Vector2(323, 100))
			var anchor := (message.global_position - panel.global_position) / HUD_SCALE
			assert(absf(anchor.x - 61.5) <= 0.5 and anchor.y == 37)
			assert(title.text == "Ready Check" and title.size == Vector2(241, 20))
			for label in [message, title]:
				assert(label.get_theme_font_size("font_size") == 12)
				assert(label.get_theme_color("font_color").is_equal_approx(Color(1, 210.0 / 255.0, 0)))
				assert(label.get_theme_color("font_shadow_color") == Color.BLACK)
				assert(Vector2i(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y")) == Vector2i(1, 1))
			var current := controls(fixture)
			if index == 0:
				conserved = current
			assert(current == conserved, "All existing control bounds conserved")
			var image := root.get_texture().get_image()
			assert(image.get_size() == Vector2i(1920, 1080))
			var filename := skin + "-" + str(index) + "-" + phase[0] + ".png"
			assert(image.save_png(output.path_join(filename)) == OK)
			if index == 1:
				first_actual = image.get_data()
			if index == phases.size() - 1:
				assert(image.get_data() == first_actual, "Reopened actual3 restores exact RGBA")
			observations.append({"skin": skin, "index": index, "phase": phase[0], "text": message.text, "controls": current, "image": filename, "message_pixel_rect": {"x": message.global_position.x, "y": message.global_position.y, "width": message.size.x * HUD_SCALE, "height": message.size.y * HUD_SCALE}})
		fixture.free()
		await settle()
	var receipt := FileAccess.open(OS.get_environment("READY_DIFFICULTY_RECEIPT"), FileAccess.WRITE)
	assert(receipt != null)
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	print("PASS ready-check-difficulty both-skin exact current-name/toggle text, bounds and restoration")
	quit(0)
