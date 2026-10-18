extends "res://tests/ready_check_difficulty.gd"
# Product policy, not inferred Retail resizing: actual glyph/shadow ink must clear buttons.

func _initialize() -> void:
	completion = preload("res://tests/completion.gd").new(self, 6)
	call_deferred("run_completion")

func run_difficulty() -> bool:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	var output := OS.get_environment("READY_DIFFICULTY_CAPTURE_PATH")
	if not completion.require(not output.is_empty()):
		return false
	var observations: Array[Dictionary] = []
	var all_clear := true
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		var fixture := ClassDB.instantiate("ReadyCheckDifficultyFixture") as Node
		root.add_child(fixture)
		if not completion.require(fixture.initialize(forever) == ""):
			return false
		for case in [["ordinary", "continent-raid", "Ann"], ["toggle", "actual3", "Ann"], ["long-name", "actual3", "Alexandriana-MoonGuard"]]:
			if not completion.require(fixture.set_initiator_name(case[2]) == "" and fixture.step(case[1]) == ""):
				return false
			if not completion.step(await settle(), "settle"):
				return false
			var panel := fixture.find_child("ReadyCheckFrame", true, false) as Control
			var body := fixture.find_child("ReadyCheckFrameText", true, false) as Label
			var yes := fixture.find_child("ReadyCheckFrameYesButton", true, false) as Button
			var no := fixture.find_child("ReadyCheckFrameNoButton", true, false) as Button
			if not completion.require(panel != null and body != null and yes != null and no != null):
				return false
			var image := root.get_texture().get_image()
			var stem: String = skin + "-" + case[0]
			if not completion.require(image.save_png(output.path_join(stem + ".png")) == OK):
				return false
			var ink: Dictionary = await capture_clearance(fixture, body, image, output, stem)
			if not completion.require(not ink.is_empty(), "Strict ink scan incomplete"):
				return false
			var extra: float = maxf(body.get_line_count() - 1, 0) * ink.resolved_line_height_ui
			var geometry_ok := absf(panel.size.y - (100.0 + extra)) < 0.00001
			geometry_ok = geometry_ok and absf((body.global_position.y - panel.global_position.y) / HUD_SCALE - 37.0) < 0.00001
			geometry_ok = geometry_ok and absf((yes.global_position.y - panel.global_position.y) / HUD_SCALE - (60.0 + extra)) < 0.00001
			geometry_ok = geometry_ok and absf((no.global_position.y - panel.global_position.y) / HUD_SCALE - (60.0 + extra)) < 0.00001
			geometry_ok = geometry_ok and yes.text == "Ready" and no.text == "Not Ready"
			var clear: bool = ink.glyph.clearance_to_button_top_px >= 0 and ink.shadow.clearance_to_button_top_px >= 0 and ink.combined.clearance_to_button_top_px >= 0
			all_clear = all_clear and clear and geometry_ok
			observations.append({"skin": skin, "case": case[0], "text": body.text, "line_count": body.get_line_count(), "expected_extra_ui": extra, "controls": controls(fixture), "clearance": ink, "geometry_ok": geometry_ok, "ink_clear": clear})
			print("READY_NO_OVERLAP ", JSON.stringify(observations[-1]))
			if not completion.case_done():
				return false
		fixture.free()
		if not completion.step(await settle(), "settle"):
			return false
	var receipt := FileAccess.open(OS.get_environment("READY_DIFFICULTY_RECEIPT"), FileAccess.WRITE)
	if not completion.require(receipt != null):
		return false
	receipt.store_string(JSON.stringify(observations, "\t"))
	receipt.close()
	return completion.require(all_clear, "READY_CHECK_BODY_NO_OVERLAP_POLICY requires glyph+shadow clearance and allocation-driven growth in every case")
