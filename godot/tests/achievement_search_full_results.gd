extends "res://tests/achievement_search_selection_focus.gd"
# Offline native allocations/text/actions; supplied pages, not semantic search or realm progress.

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 8)
	var passed := true
	for forever in [false, true]:
		for kind in ["geometry", "metadata", "open", "category"]:
			if await check_full_case(forever, kind):
				if not completion.case_done():
					return
			else:
				passed = false
	for failure in failures:
		print("FAIL CASE ", failure)
	completion.finish(passed)

func check_full_case(forever: bool, kind: String) -> bool:
	var before := failures.size()
	var ui := ClassDB.instantiate("RegistryUi") as CanvasLayer
	root.add_child(ui)
	var error: String = ui.call("show_achievement_full_results_preview", forever, kind != "category")
	if not error.is_empty():
		failures.append(error)
		ui.free()
		return false
	await frames(4)
	match kind:
		"geometry":
			check_rect(ui, "AchievementFullSearchResults", Vector2(600, 382))
			var panel := ui.find_child("AchievementFullSearchResults", true, false) as Control
			var frame := ui.find_child("AchievementFrame", true, false) as Control
			if panel != null and frame != null:
				check_vector(panel.global_position - frame.global_position, Vector2(84, 111), "panel origin")
			check_rect(ui, "AchievementFullSearchResult6", Vector2(575, 49))
			check_rect(ui, "AchievementFullSearchResult6IconFrame", Vector2(38, 38))
			check_rect(ui, "AchievementFullSearchResult6Icon", Vector2(36, 35))
			var first := ui.find_child("AchievementFullSearchResult6", true, false) as Control
			var second := ui.find_child("AchievementFullSearchResult7", true, false) as Control
			if first != null and second != null:
				check_vector(second.global_position - first.global_position, Vector2(0, 49), "row stride")
		"metadata":
			check_text(ui, "Achievement6Name", "Level 10")
			check_text(ui, "Achievement7Name", "Level 20")
			check_text(ui, "AchievementFullSearchResult6Path", "Characters")
			check_text(ui, "AchievementFullSearchResult633Path", "Dungeons & Raids > Classic")
			check_text(ui, "AchievementFullSearchResult6ResultType", "Completed")
			check_text(ui, "AchievementFullSearchResult7ResultType", "Incomplete")
		"open":
			var row := ui.find_child("AchievementFullSearchResult633", true, false) as Control
			if row == null:
				failures.append("received full-search row633 missing")
			else:
				await click(row.get_global_rect().get_center())
				var raw: String = ui.call("achievement_search_keyboard_preview_dispatch")
				var requests = JSON.parse_string(raw)
				var requested := false
				if requests is Array:
					for request in requests:
						if request is Dictionary and request.has("Achievement") and int(request.Achievement.achievement_id) == 633:
							requested = true
				print("ACHIEVEMENT_FULL_SEARCH_REQUESTS ", raw)
				if not requested:
					failures.append("sixth received row did not request achievement633: " + raw)
				await frames(3)
				var edit := ui.find_child("AchievementSearchBox", true, false) as LineEdit
				if edit == null or not edit.text.is_empty():
					failures.append("existing received-ID open did not clear query")
				if ui.find_child("AchievementFullSearchResults", true, false) != null:
					failures.append("full-search presentation remained after received-ID open")
		"category":
			check_text(ui, "Achievement633Name", "Stormwind Stockade")
			if ui.find_child("AchievementFullSearchResults", true, false) != null:
				failures.append("full-search panel rendered in category mode")
			var row := ui.find_child("Achievement633Row", true, false) as Control
			if row == null or not is_equal_approx(row.size.x, 500.0):
				failures.append("ordinary category card allocation changed")
	print("ACHIEVEMENT_FULL_SEARCH_NATIVE ", JSON.stringify({"skin": "Forever" if forever else "Modern", "case": kind, "failures": failures.size() - before}))
	ui.free()
	await process_frame
	return failures.size() == before

func check_rect(ui: Node, node_name: String, expected: Vector2) -> void:
	var node := ui.find_child(node_name, true, false) as Control
	if node == null:
		failures.append("missing native allocation: " + node_name)
	else:
		check_vector(node.size, expected, node_name)

func check_vector(actual: Vector2, expected: Vector2, label: String) -> void:
	if not actual.is_equal_approx(expected):
		failures.append(label + " actual=" + str(actual) + " expected=" + str(expected))

func check_text(ui: Node, node_name: String, expected: String) -> void:
	var node := ui.find_child(node_name, true, false) as Label
	if node == null or node.text != expected:
		failures.append(node_name + " actual=" + (node.text if node != null else "MISSING") + " expected=" + expected)
