extends SceneTree
# Native process/UI action-boundary controls. Explicit payload fixtures, no realm/progress facts.

var completion
var failures: Array[String] = []

func _initialize() -> void:
	call_deferred("run_cases")

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 12)
	var passed := true
	for forever in [false, true]:
		for kind in ["full", "open", "unknown_preview", "no_more_results", "malformed_preview", "unrelated_filter"]:
			if await check_case(forever, kind):
				if not completion.case_done():
					return
			else:
				passed = false
	for failure in failures:
		print("FAIL CASE ", failure)
	completion.finish(passed)

func check_case(forever: bool, kind: String) -> bool:
	var before := failures.size()
	var ui := ClassDB.instantiate("RegistryUi") as CanvasLayer
	root.add_child(ui)
	var error: String = ui.call("show_achievement_search_selection_preview", forever, kind != "no_more_results")
	if not error.is_empty():
		failures.append(error)
		ui.free()
		return false
	await frames(4)
	var edit := ui.find_child("AchievementSearchBox", true, false) as LineEdit
	edit.grab_focus()
	await frames(2)
	if not edit.has_focus():
		failures.append(kind + " initial native search focus missing")
		ui.free()
		return false
	var action: String = {
		"full": "achievement:search_all", "open": "achievement:preview:6",
		"unknown_preview": "achievement:preview:4294967295", "no_more_results": "achievement:search_all",
		"malformed_preview": "achievement:preview:not-an-id", "unrelated_filter": "achievement:filter_menu"}[kind]
	if kind in ["full", "open", "unrelated_filter"]:
		var button_name: String = {"full": "AchievementShowAllSearchResults", "open": "AchievementSearchPreview1", "unrelated_filter": "AchievementFilterDropdown"}[kind]
		var button := ui.find_child(button_name, true, false) as Control
		await click(button.get_global_rect().get_center())
		var actual_action: String = ui.call("pop_action")
		if actual_action != action:
			failures.append(kind + " physical action mismatch: " + actual_action)
			ui.free()
			return false
	error = ui.call("achievement_search_selection_preview_dispatch", action)
	if not error.is_empty():
		failures.append(error)
	await frames(4)
	edit = ui.find_child("AchievementSearchBox", true, false) as LineEdit
	var committed := kind in ["full", "open"]
	var expected_focus := not committed
	var expected_query := "" if kind == "open" else "Level"
	var expected_body := "Achievement6Name" if committed else "Achievement633Name"
	var body := ui.find_child(expected_body, true, false) as Label
	var record := {"skin": "Forever" if forever else "Modern", "case": kind, "action": action,
		"actual_native_focus": edit.has_focus(), "expected_native_focus": expected_focus,
		"query": edit.text, "body": body.text if body != null else "MISSING"}
	print("ACHIEVEMENT_SELECTION_NATIVE_FOCUS ", JSON.stringify(record))
	if edit.has_focus() != expected_focus:
		failures.append(JSON.stringify(record))
	if edit.text != expected_query or body == null:
		failures.append(kind + " query/body selection changed")
	ui.free()
	await process_frame
	return failures.size() == before

func frames(count: int) -> void:
	for index in range(count):
		await process_frame

func click(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await frames(2)
