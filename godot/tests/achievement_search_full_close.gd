extends "res://tests/achievement_search_full_results.gd"
# Offline received UI payload; close/conservation only, no realm/progress claims.

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 6)
	var passed := true
	for forever in [true]:
		for kind in ["geometry", "dismiss", "reopen"]:
			if await check_close_case(forever, kind):
				if not completion.case_done():
					return
			else:
				passed = false
	for failure in failures:
		print("FAIL CASE ", failure)
	completion.finish(passed)

func check_close_case(forever: bool, kind: String) -> bool:
	var before := failures.size()
	var ui := ClassDB.instantiate("RegistryUi") as CanvasLayer
	root.add_child(ui)
	var error: String = ui.call("show_achievement_full_results_preview", forever, true)
	if not error.is_empty():
		failures.append(error)
		ui.free()
		return false
	await frames(4)
	var button := ui.find_child("AchievementFullSearchClose", true, false) as Control
	if button == null:
		failures.append(kind + " missing source full-search close button")
	elif kind == "geometry":
		check_vector(button.size, Vector2(24, 24), "close size")
		var panel := ui.find_child("AchievementFullSearchResults", true, false) as Control
		check_vector(button.global_position - panel.global_position, Vector2(581, 0), "close anchor")
	else:
		var state_before = JSON.parse_string(ui.call("achievement_full_search_preview_state"))
		await click(button.get_global_rect().get_center())
		var raw: String = ui.call("achievement_search_keyboard_preview_dispatch")
		if JSON.parse_string(raw) != []:
			failures.append("presentation close emitted catalog request: " + raw)
		await frames(3)
		var state_after = JSON.parse_string(ui.call("achievement_full_search_preview_state"))
		if state_after != state_before:
			failures.append("close changed conserved state: " + JSON.stringify({"before": state_before, "after": state_after}))
		if ui.find_child("AchievementFullSearchResults", true, false) != null:
			failures.append("close left full-search pane mounted")
		var frame := ui.find_child("AchievementFrame", true, false) as Control
		if frame == null or not frame.is_visible_in_tree():
			failures.append("results close hid whole AchievementFrame")
		check_text(ui, "Achievement633Name", "Stormwind Stockade")
		var edit := ui.find_child("AchievementSearchBox", true, false) as LineEdit
		if edit == null or edit.text != "fixture" or edit.has_focus():
			failures.append("results close changed query/native focus")
		if kind == "reopen":
			error = ui.call("achievement_search_selection_preview_dispatch", "achievement:search_all")
			if not error.is_empty():
				failures.append(error)
			await frames(3)
			if ui.find_child("AchievementFullSearchResult633", true, false) == null:
				failures.append("cached sixth row missing on reopen")
			if JSON.parse_string(ui.call("achievement_full_search_preview_state")) != state_before:
				failures.append("reopen changed conserved cache/query/cursor/category")
	print("ACHIEVEMENT_SEARCH_CLOSE_NATIVE ", JSON.stringify({"skin": "Forever" if forever else "Modern", "case": kind, "failures": failures.size() - before}))
	ui.free()
	await process_frame
	return failures.size() == before
