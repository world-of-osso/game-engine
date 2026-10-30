extends SceneTree
# Offline production RegistryUi controls, not received comparison/click proof.

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
	for forever in [true]:
		for comparison in [false, true]:
			for query in ["", "bear"]:
				if not await check_header(forever, comparison, query, expected_font, failures):
					quit(1)
					return
	for failure in failures:
		push_error(failure)
	print("ACHIEVEMENT_SEARCH_GEOMETRY cases=8 failures=", failures.size())
	quit(0 if failures.is_empty() else 1)

func check_header(forever: bool, comparison: bool, query: String, expected_font: PackedByteArray, failures: Array[String]) -> bool:
	var ui := ClassDB.instantiate("RegistryUi") as CanvasLayer
	if ui == null:
		push_error("Native RegistryUi unavailable")
		return false
	root.add_child(ui)
	var error: String = ui.call("show_achievement_header_preview", forever, comparison, query)
	if not error.is_empty():
		push_error(error)
		ui.free()
		return false
	for frame in range(4):
		await process_frame
	var window := ui.find_child("AchievementFrame", true, false) as Control
	var search := ui.find_child("AchievementSearchBox", true, false) as LineEdit
	var filter := ui.find_child("AchievementFilterDropdown", true, false) as Control
	if window == null or search == null or filter == null:
		push_error("Native header controls missing")
		ui.free()
		return false
	var origin := window.get_global_rect().position
	var expected_x := 631.0 if comparison else 511.0
	var style := search.get_theme_stylebox("normal")
	var margins := Vector4(style.get_content_margin(SIDE_LEFT), style.get_content_margin(SIDE_RIGHT), style.get_content_margin(SIDE_TOP), style.get_content_margin(SIDE_BOTTOM))
	var native_font := search.get_theme_font("font") as FontFile
	var font_color := search.get_theme_color("font_color")
	var caret_color := search.get_theme_color("caret_color")
	var actual := {
		"forever": forever, "comparison": comparison, "query": query,
		"search": str(search.get_global_rect()), "margins_lrtb": str(margins),
		"font_size": search.get_theme_font_size("font_size"),
		"font_height": search.get_theme_font("font").get_height(search.get_theme_font_size("font_size")),
		"font_color_rgba": [font_color.r, font_color.g, font_color.b, font_color.a],
		"caret_color_rgba": [caret_color.r, caret_color.g, caret_color.b, caret_color.a],
		"minimum": str(search.get_combined_minimum_size())}
	print("ACHIEVEMENT_SEARCH_NATIVE_BOUNDS ", JSON.stringify(actual))
	if search.size != Vector2(107, 30) or margins != Vector4(16, 20, 0, 0):
		failures.append("Source107x30/L16R20T0B0 mismatch: " + JSON.stringify(actual))
	if search.text != query or search.get_theme_font_size("font_size") != 10:
		failures.append("Query/source Roman height10 mismatch: " + JSON.stringify(actual))
	# White caret is the accepted existing projection coupling, not Retail parity.
	if font_color != Color.WHITE or caret_color != Color.WHITE:
		failures.append("Source white text/accepted coupled caret mismatch: " + JSON.stringify(actual))
	if native_font == null or native_font.data != expected_font:
		failures.append("Source FRIZQT__.TTF bytes changed: " + JSON.stringify(actual))
	if search.get_global_rect().position - origin != Vector2(expected_x, 25):
		failures.append("Search position changed: " + JSON.stringify(actual))
	if filter.get_global_rect().position - origin != Vector2(expected_x + 114, 31) or filter.size != Vector2(112, 22):
		failures.append("Filter geometry changed: " + JSON.stringify(actual))
	check_instructions(ui, search, query, expected_font, failures)
	ui.free()
	await process_frame
	return true

func check_instructions(ui: CanvasLayer, search: LineEdit, query: String, expected_font: PackedByteArray, failures: Array[String]) -> void:
	var hint := ui.find_child("AchievementSearchInstructions", true, false) as Label
	if not query.is_empty():
		if hint != null and hint.is_visible_in_tree():
			failures.append("Nonempty query still shows Search instructions")
		return
	if hint == null or not hint.is_visible_in_tree():
		failures.append("Empty query lacks source Search instructions")
		return
	var color := hint.get_theme_color("font_color")
	var shadow := hint.get_theme_color("font_shadow_color")
	var offset := Vector2(hint.get_theme_constant("shadow_offset_x"), hint.get_theme_constant("shadow_offset_y"))
	var font := hint.get_theme_font("font") as FontFile
	print("ACHIEVEMENT_SEARCH_INSTRUCTIONS ", JSON.stringify({"text": hint.text, "rect": str(hint.get_global_rect()), "font_size": hint.get_theme_font_size("font_size"), "color": [color.r, color.g, color.b, color.a], "shadow": [shadow.r, shadow.g, shadow.b, shadow.a], "offset": str(offset), "mouse_filter": hint.mouse_filter}))
	if hint.text != "Search" or hint.get_global_rect().position - search.get_global_rect().position != Vector2(16, 0) or hint.size != Vector2(71, 30):
		failures.append("Source Search instruction text/bounds mismatch")
	if hint.get_theme_font_size("font_size") != 10 or font == null or font.data != expected_font or color != Color(0.35, 0.35, 0.35, 1):
		failures.append("Source Search instruction Roman10/gray mismatch")
	if shadow != Color.BLACK or offset != Vector2(1, 1):
		failures.append("Source instruction black(+1,-1) shadow mismatch in native y-down coordinates")
	if hint.horizontal_alignment != HORIZONTAL_ALIGNMENT_LEFT or hint.vertical_alignment != VERTICAL_ALIGNMENT_CENTER or hint.mouse_filter != Control.MOUSE_FILTER_IGNORE:
		failures.append("Source instruction alignment/noninteractive behavior mismatch")
