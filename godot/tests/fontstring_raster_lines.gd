extends SceneTree
# Historical allocation contract, not Retail 70448 or raster-ink acceptance.
const PIXELS_PER_UI := 1080.0 / 768.0
const LINE_HEIGHT := 17.0 / PIXELS_PER_UI
const BASELINE := 14.0 / PIXELS_PER_UI

func _initialize() -> void:
	call_deferred("run_contract")

func close(actual: float, expected: float, context: String) -> bool:
	if absf(actual - expected) > 0.00001:
		push_error("FONTSTRING_RASTER_CONTRACT %s: %.9f != %.9f" % [context, actual, expected])
		return false
	return true

func run_contract() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	for forever in [false, true]:
		var fixture := ClassDB.instantiate("ReadyCheckDifficultyFixture") as Node
		root.add_child(fixture)
		if fixture.initialize(forever) != "":
			quit(1)
			return
		for phase in ["continent-raid", "actual3"]:
			if fixture.step(phase) != "":
				quit(1)
				return
			for frame in range(8):
				await process_frame
			var label := fixture.find_child("ReadyCheckFrameText", true, false) as Label
			var panel := fixture.find_child("ReadyCheckFrame", true, false) as Control
			var button := fixture.find_child("ReadyCheckFrameYesButton", true, false) as Control
			var lines := 1 if phase == "continent-raid" else 2
			var font := label.get_theme_font("font")
			var checks := [
				close(label.size.y, lines * LINE_HEIGHT, phase + " intrinsic height"),
				close(font.get_height(12), LINE_HEIGHT, phase + " font allocation"),
				close(font.get_ascent(12), BASELINE, phase + " baseline"),
				close((label.global_position.y - panel.global_position.y) / PIXELS_PER_UI, 37.0, "body top"),
				close((button.global_position.y - panel.global_position.y) / PIXELS_PER_UI, 60.0 + (lines - 1) * LINE_HEIGHT, "bottom-anchored policy button top"),
				close(panel.size.y, 100.0 + (lines - 1) * LINE_HEIGHT, "allocation-grown policy panel height"),
				close(label.get_theme_font_size("font_size"), 12.0, "nominal request"),
			]
			if lines == 2:
				var second := label.text.find("\n") + 1
				checks.append(close(label.get_character_bounds(second).position.y - label.get_character_bounds(0).position.y, LINE_HEIGHT, "second-line placement"))
			if false in checks:
				fixture.free()
				quit(1)
				return
			print("FONTSTRING_RASTER_CONTRACT skin=", forever, " phase=", phase, " height=", label.size.y, " baseline=", font.get_ascent(12))
		fixture.free()
	quit(0)
