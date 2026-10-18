extends SceneTree
# Offline production RegistryUi geometry, not received comparison/click proof.

func _initialize() -> void:
	call_deferred("run_geometry")

func run_geometry() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	var failures: Array[String] = []
	var expected_font := FileAccess.get_file_as_bytes("res://../data/fonts/FRIZQT__.TTF")
	if expected_font.is_empty():
		push_error("Source Roman font unavailable")
		quit(1)
		return
	for forever in [false, true]:
		for comparison in [false, true]:
			var ui := ClassDB.instantiate("RegistryUi") as CanvasLayer
			if ui == null:
				push_error("Native RegistryUi unavailable")
				quit(1)
				return
			root.add_child(ui)
			var error: String = ui.call("show_achievement_header_preview", forever, comparison)
			if not error.is_empty():
				push_error(error)
				quit(1)
				return
			for frame in range(4):
				await process_frame
			var window := ui.find_child("AchievementFrame", true, false) as Control
			var search := ui.find_child("AchievementSearchBox", true, false) as LineEdit
			var filter := ui.find_child("AchievementFilterDropdown", true, false) as Control
			if window == null or search == null or filter == null:
				push_error("Native header controls missing")
				quit(1)
				return
			var origin := window.get_global_rect().position
			var expected_x := 631.0 if comparison else 511.0
			var style := search.get_theme_stylebox("normal")
			var margins := Vector4(style.get_content_margin(SIDE_LEFT), style.get_content_margin(SIDE_RIGHT), style.get_content_margin(SIDE_TOP), style.get_content_margin(SIDE_BOTTOM))
			var native_font := search.get_theme_font("font") as FontFile
			var font_color := search.get_theme_color("font_color")
			var caret_color := search.get_theme_color("caret_color")
			var actual := {
				"forever": forever, "comparison": comparison,
				"search": str(search.get_global_rect()), "margins_lrtb": str(margins),
				"font_size": search.get_theme_font_size("font_size"),
				"font_height": search.get_theme_font("font").get_height(search.get_theme_font_size("font_size")),
				"font_color_rgba": [font_color.r, font_color.g, font_color.b, font_color.a],
				"caret_color_rgba": [caret_color.r, caret_color.g, caret_color.b, caret_color.a],
				"minimum": str(search.get_combined_minimum_size())}
			print("ACHIEVEMENT_SEARCH_NATIVE_BOUNDS ", JSON.stringify(actual))
			if search.size != Vector2(107, 30) or margins != Vector4(16, 20, 0, 0):
				failures.append("Source107x30/L16R20T0B0 mismatch: " + JSON.stringify(actual))
			if search.get_theme_font_size("font_size") != 10:
				failures.append("Source Roman height10 mismatch: " + JSON.stringify(actual))
			# White caret is the accepted existing projection coupling, not Retail parity.
			if font_color != Color.WHITE or caret_color != Color.WHITE:
				failures.append("Source white text/accepted coupled caret mismatch: " + JSON.stringify(actual))
			if native_font == null or native_font.data != expected_font:
				failures.append("Source FRIZQT__.TTF bytes changed: " + JSON.stringify(actual))
			if search.get_global_rect().position - origin != Vector2(expected_x, 25):
				failures.append("Search position changed: " + JSON.stringify(actual))
			if filter.get_global_rect().position - origin != Vector2(expected_x + 114, 31) or filter.size != Vector2(112, 22):
				failures.append("Filter geometry changed: " + JSON.stringify(actual))
			ui.free()
			await process_frame
	for failure in failures:
		push_error(failure)
	print("ACHIEVEMENT_SEARCH_GEOMETRY cases=4 failures=", failures.size())
	quit(0 if failures.is_empty() else 1)
