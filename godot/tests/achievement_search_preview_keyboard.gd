extends "res://tests/achievement_search_selection_focus.gd"
# Physical native keys/pointer -> production dispatcher; explicit offline metadata, no realm facts.

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 24)
	var passed := true
	for forever in [true]:
		for kind in ["default", "down", "wrap_many", "wrap_two", "first_enter", "hover_enter", "all_enter", "hover_all_enter", "batched_enter", "short_enter", "pending_enter", "empty_enter"]:
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
	await move_pointer(Vector2(1800, 900))
	var ui := ClassDB.instantiate("RegistryUi") as CanvasLayer
	root.add_child(ui)
	var phase := "many"
	if kind == "wrap_two":
		phase = "two"
	elif kind in ["short_enter", "pending_enter", "empty_enter"]:
		phase = kind.trim_suffix("_enter")
	var error: String = ui.call("show_achievement_search_keyboard_preview", forever, phase)
	if not error.is_empty():
		failures.append(error)
		ui.free()
		return false
	await frames(4)
	var edit := control(ui, "AchievementSearchBox") as LineEdit
	edit.grab_focus()
	edit.caret_column = edit.text.length()
	await frames(2)
	await dispatch(ui)
	var stage := ("Forever" if forever else "Modern") + ":" + kind
	if kind == "default":
		check_selected(ui, "AchievementSearchPreview1Selected", stage)
	elif kind == "down":
		await press_key(KEY_DOWN)
		await dispatch(ui)
		check_selected(ui, "AchievementSearchPreview2Selected", stage)
	elif kind == "wrap_many":
		await press_key(KEY_UP)
		await dispatch(ui)
		check_selected(ui, "AchievementShowAllSearchResultsSelected", stage + ":up-wrap")
		await press_key(KEY_DOWN)
		await dispatch(ui)
		check_selected(ui, "AchievementSearchPreview1Selected", stage + ":down-wrap")
		for index in range(6):
			await press_key(KEY_DOWN)
			await dispatch(ui)
		check_selected(ui, "AchievementSearchPreview1Selected", stage + ":full-cycle")
	elif kind == "wrap_two":
		await press_key(KEY_UP)
		await dispatch(ui)
		check_selected(ui, "AchievementSearchPreview2Selected", stage + ":up-wrap")
		await press_key(KEY_DOWN)
		await dispatch(ui)
		check_selected(ui, "AchievementSearchPreview1Selected", stage + ":down-wrap")
		if control(ui, "AchievementShowAllSearchResults") != null:
			failures.append(stage + " unexpected Show All for two received rows")
	elif kind in ["first_enter", "hover_enter", "batched_enter"]:
		var expected_id := 6
		if kind == "hover_enter":
			await move_pointer(control(ui, "AchievementSearchPreview3").get_global_rect().get_center())
			await dispatch(ui)
			check_selected(ui, "AchievementSearchPreview3Selected", stage + ":hover")
			expected_id = 8
		if kind == "batched_enter":
			push_key_pair(KEY_DOWN)
			push_key_pair(KEY_ENTER)
			await frames(2)
			expected_id = 7
		else:
			await press_key(KEY_ENTER)
		var requests := await dispatch(ui)
		check_open(ui, requests, expected_id, stage)
	elif kind in ["all_enter", "hover_all_enter"]:
		if kind == "hover_all_enter":
			await move_pointer(control(ui, "AchievementShowAllSearchResults").get_global_rect().get_center())
		else:
			await press_key(KEY_UP)
		await dispatch(ui)
		check_selected(ui, "AchievementShowAllSearchResultsSelected", stage + ":selected")
		await press_key(KEY_ENTER)
		await dispatch(ui)
		check_state(ui, "Level", "Achievement6Name", "Level 10", false, stage)
		if control(ui, "AchievementSearchPreviewContainer") != null:
			failures.append(stage + " Show All did not hide preview")
	else:
		var query := edit.text
		await press_key(KEY_ENTER)
		var requests := await dispatch(ui)
		if not requests.is_empty():
			failures.append(stage + " no-op Enter emitted catalog selection " + JSON.stringify(requests))
		check_state(ui, query, "Achievement633Name", "Stormwind Stockade", true, stage)
		if control(ui, "AchievementSearchPreviewContainer") != null:
			failures.append(stage + " no-op phase showed result popup")
		push_key_pair(KEY_A, "a".unicode_at(0))
		await frames(3)
		edit = control(ui, "AchievementSearchBox") as LineEdit
		if edit.text != query + "a" or not edit.has_focus():
			failures.append(stage + " Enter stopped continued native typing: " + edit.text)
	if kind in ["default", "down", "wrap_many", "wrap_two"]:
		check_state(ui, "Level", "Achievement633Name", "Stormwind Stockade", true, stage)
	ui.free()
	await process_frame
	return failures.size() == before

func control(ui: CanvasLayer, name: String) -> Control:
	return ui.find_child(name, true, false) as Control

func dispatch(ui: CanvasLayer) -> Array:
	var raw: String = ui.call("achievement_search_keyboard_preview_dispatch")
	var value = JSON.parse_string(raw)
	if not value is Array:
		failures.append("Native keyboard dispatch failed: " + raw)
		return []
	await frames(2)
	return value

func check_selected(ui: CanvasLayer, expected: String, stage: String) -> void:
	var selected: Array[String] = []
	for index in range(1, 6):
		var name := "AchievementSearchPreview%dSelected" % index
		var node := control(ui, name)
		if node != null and node.is_visible_in_tree():
			selected.append(name)
	var all := control(ui, "AchievementShowAllSearchResultsSelected")
	if all != null and all.is_visible_in_tree():
		selected.append("AchievementShowAllSearchResultsSelected")
	print("ACHIEVEMENT_KEYBOARD_SELECTED ", JSON.stringify({"stage": stage, "actual": selected, "expected": expected}))
	if selected != [expected]:
		failures.append(stage + " selected=" + JSON.stringify(selected) + " expected=" + expected)

func check_open(ui: CanvasLayer, requests: Array, id: int, stage: String) -> void:
	var requested := false
	for request in requests:
		if request is Dictionary and request.has("Achievement") and int(request.Achievement.achievement_id) == id:
			requested = true
	print("ACHIEVEMENT_KEYBOARD_REQUESTS ", JSON.stringify({"stage": stage, "requests": requests, "expected_id": id}))
	if not requested:
		failures.append(stage + " Enter did not request selected achievement%d" % id)
	check_state(ui, "", "AchievementEmpty", "Loading achievement...", false, stage)

func check_state(ui: CanvasLayer, query: String, body_name: String, body_text: String, focused: bool, stage: String) -> void:
	var edit := control(ui, "AchievementSearchBox") as LineEdit
	var body := control(ui, body_name) as Label
	var record := {"stage": stage, "query": edit.text, "focus": edit.has_focus(), "body": body.text if body != null else "MISSING"}
	print("ACHIEVEMENT_KEYBOARD_STATE ", JSON.stringify(record))
	if edit.text != query or edit.has_focus() != focused or body == null or body.text != body_text:
		failures.append(stage + " native query/focus/body mismatch " + JSON.stringify(record))

func move_pointer(point: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	root.push_input(event, true)
	await frames(2)

func press_key(code: Key) -> void:
	push_key_pair(code)
	await frames(2)

func push_key_pair(code: Key, unicode_value := 0) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.unicode = unicode_value if pressed else 0
		event.pressed = pressed
		root.push_input(event, true)
