extends "res://tests/achievement_search_full_results.gd"
# Offline native title rendering configuration; no realm or ink/pixel-equality claim.

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 6)
	var passed := true
	for forever in [true]:
		for kind in ["collapsed", "expanded", "full_search_control"]:
			if await check_title_case(forever, kind):
				if not completion.case_done():
					return
			else:
				passed = false
	for failure in failures:
		print("FAIL CASE ", failure)
	completion.finish(passed)

func check_title_case(forever: bool, kind: String) -> bool:
	var before := failures.size()
	var ui := ClassDB.instantiate("RegistryUi") as CanvasLayer
	root.add_child(ui)
	var full := kind == "full_search_control"
	var error: String = ui.call("show_achievement_full_results_preview", forever, full)
	if not error.is_empty():
		failures.append(error)
		ui.free()
		return false
	await frames(4)
	if kind == "expanded":
		error = ui.call("achievement_search_selection_preview_dispatch", "achievement:row:633")
		if not error.is_empty():
			failures.append(error)
		await frames(3)
	var title_name := "Achievement6Name" if full else "Achievement633Name"
	var title := ui.find_child(title_name, true, false) as Label
	if title == null:
		failures.append("missing title: " + title_name)
	else:
		var expected_size := 16 if full else 14
		if title.get_theme_font_size("font_size") != expected_size:
			failures.append(kind + " font=" + str(title.get_theme_font_size("font_size")) + " expected=" + str(expected_size))
		check_vector(title.size, Vector2(400, 16) if full else Vector2(320, 20), kind + " allocation")
		if not full:
			if title.get_theme_color("font_color") != Color.WHITE:
				failures.append("ordinary title white color changed")
			if title.get_theme_color("font_shadow_color") != Color.BLACK:
				failures.append("ordinary title source black shadow missing")
			var offset := Vector2i(title.get_theme_constant("shadow_offset_x"), title.get_theme_constant("shadow_offset_y"))
			if offset != Vector2i(1, 1):
				failures.append("source (+1,-1) shadow projected incorrectly: " + str(offset))
			var row := ui.find_child("Achievement633Row", true, false) as Control
			check_vector(title.global_position - row.global_position, Vector2(90, 6), "title origin")
			var row_height := row.size.y
			await click(title.get_global_rect().get_center())
			var raw: String = ui.call("achievement_search_keyboard_preview_dispatch")
			if JSON.parse_string(raw) != []:
				failures.append("cached title click emitted request: " + raw)
			await frames(3)
			row = ui.find_child("Achievement633Row", true, false) as Control
			if row == null or is_equal_approx(row_height, row.size.y):
				failures.append("title click did not preserve expand/collapse behavior")
	print("ACHIEVEMENT_CARD_TITLE_NATIVE ", JSON.stringify({"skin": "Forever" if forever else "Modern", "case": kind, "failures": failures.size() - before}))
	ui.free()
	await process_frame
	return failures.size() == before
