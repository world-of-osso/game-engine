extends SceneTree
# Offline production RegistryUi cache-state controls, not received/live authority.

var completion
var failures: Array[String] = []

func _initialize() -> void:
	call_deferred("run_cases")

func run_cases() -> void:
	Engine.max_fps = 60
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 20)
	var passed := true
	for forever in [false, true]:
		for comparison in [false, true]:
			for phase in ["pending", "empty", "rows", "search", "error"]:
				if await check_case(forever, comparison, phase):
					if not completion.case_done():
						return
				else:
					passed = false
	for failure in failures:
		print("FAIL CASE ", failure)
	completion.finish(passed)

func label_text(ui: CanvasLayer, name: String) -> String:
	var node := ui.find_child(name, true, false) as Label
	return node.text if node != null and node.is_visible_in_tree() else ""

func check_case(forever: bool, comparison: bool, phase: String) -> bool:
	var before := failures.size()
	var ui := ClassDB.instantiate("RegistryUi") as CanvasLayer
	if ui == null:
		failures.append("Native RegistryUi unavailable")
		return false
	root.add_child(ui)
	var error: String = ui.call("show_achievement_category_pending_preview", forever, comparison, phase)
	if not error.is_empty():
		failures.append(error)
		ui.free()
		return false
	for frame in range(4):
		await process_frame
	var empty := label_text(ui, "AchievementEmpty")
	var row_name := label_text(ui, "Achievement633Name")
	var expected_message: String = {
		"pending": "", "empty": "No achievements in this category", "rows": "",
		"search": "Searching...", "error": "Achievement unavailable"}[phase]
	var identity := "%s comparison=%s phase=%s" % ["Forever" if forever else "Modern", comparison, phase]
	print("CATEGORY_PENDING_NATIVE ", JSON.stringify({"case": identity, "message": empty, "row": row_name, "comparison_name": label_text(ui, "AchievementComparisonName")}))
	if empty != expected_message:
		failures.append(identity + " expected message=" + expected_message + ", actual=" + empty)
	if row_name != ("Stormwind Stockade" if phase == "rows" else ""):
		failures.append(identity + " loaded-row visibility mismatch: " + row_name)
	if comparison and label_text(ui, "AchievementComparisonName") != "Loading...":
		failures.append(identity + " inspect-wait indicator changed")
	if comparison and phase == "rows" and label_text(ui, "Achievement633ComparisonStatus") != "Loading...":
		failures.append(identity + " inspect row wait indicator changed")
	check_header(ui, comparison, phase == "search", identity)
	ui.free()
	await process_frame
	return failures.size() == before

func check_header(ui: CanvasLayer, comparison: bool, searching: bool, identity: String) -> void:
	var window := ui.find_child("AchievementFrame", true, false) as Control
	var search := ui.find_child("AchievementSearchBox", true, false) as LineEdit
	var filter := ui.find_child("AchievementFilterDropdown", true, false) as Control
	if window == null or search == null or filter == null:
		failures.append(identity + " retained header controls missing")
		return
	var x := 631.0 if comparison else 511.0
	var origin := window.get_global_rect().position
	if search.get_global_rect().position - origin != Vector2(x, 25) or search.size != Vector2(107, 30):
		failures.append(identity + " search geometry changed")
	if filter.get_global_rect().position - origin != Vector2(x + 114, 31) or filter.size != Vector2(112, 22) or label_text(ui, "AchievementFilterText") != "All":
		failures.append(identity + " filter geometry/value changed")
	if search.get_theme_font_size("font_size") != 10 or search.get_theme_color("font_color") != Color.WHITE or search.get_theme_color("caret_color") != Color.WHITE:
		failures.append(identity + " accepted typed font/caret style changed")
	var hint := ui.find_child("AchievementSearchInstructions", true, false) as Label
	if searching:
		if hint != null and hint.is_visible_in_tree():
			failures.append(identity + " nonempty search still shows hint")
		return
	if hint == null or not hint.is_visible_in_tree() or hint.text != "Search":
		failures.append(identity + " retained empty hint missing")
		return
	if hint.get_global_rect().position - search.get_global_rect().position != Vector2(16, 0) or hint.size != Vector2(71, 30) or hint.mouse_filter != Control.MOUSE_FILTER_IGNORE:
		failures.append(identity + " retained hint bounds/input changed")
	if hint.get_theme_font_size("font_size") != 10 or hint.get_theme_color("font_color") != Color(0.35, 0.35, 0.35, 1):
		failures.append(identity + " retained hint font/color changed")
