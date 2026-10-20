extends "res://tests/achievement_search_full_results.gd"
# Font-size and approved ordinary-description TOP alignment; retain ink/shadow/horizontal policy.

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 6)
	var passed := true
	for forever in [false, true]:
		for kind in ["collapsed", "expanded", "title_control"]:
			if await check_description_case(forever, kind):
				if not completion.case_done():
					return
			else:
				passed = false
	for failure in failures:
		print("FAIL CASE ", failure)
	completion.finish(passed)

func check_description_case(forever: bool, kind: String) -> bool:
	var before := failures.size()
	var ui := ClassDB.instantiate("RegistryUi") as CanvasLayer
	root.add_child(ui)
	var error: String = ui.call("show_achievement_full_results_preview", forever, false)
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
	var title_control := kind == "title_control"
	var node_name := "Achievement633Name" if title_control else "Achievement633Description"
	var label := ui.find_child(node_name, true, false) as Label
	if label == null:
		failures.append("missing label " + node_name)
	else:
		var expected_size := 14 if title_control else 10
		if label.get_theme_font_size("font_size") != expected_size:
			failures.append(kind + " font=" + str(label.get_theme_font_size("font_size")) + " expected=" + str(expected_size))
		check_vector(label.size, Vector2(320, 20) if title_control else Vector2(330, 34), kind + " allocation")
		if not title_control:
			var expected_ink := Color.WHITE if forever else Color(0.1, 0.08, 0.05, 1)
			if not label.get_theme_color("font_color").is_equal_approx(expected_ink):
				failures.append("description ink changed")
			if label.get_theme_color("font_shadow_color").a != 0:
				failures.append("description shadow changed")
			var offset := Vector2i(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y"))
			if offset != Vector2i.ZERO:
				failures.append("description shadow offset changed: " + str(offset))
			if label.horizontal_alignment != HORIZONTAL_ALIGNMENT_CENTER or label.vertical_alignment != VERTICAL_ALIGNMENT_TOP:
				failures.append("description expected horizontal CENTER and Retail vertical TOP")
			if label.text != "Defeat Hogger.":
				failures.append("description text changed")
			var row := ui.find_child("Achievement633Row", true, false) as Control
			check_vector(label.global_position - row.global_position, Vector2(85, 30), "description origin")
			var height := row.size.y
			await click(label.get_global_rect().get_center())
			var raw: String = ui.call("achievement_search_keyboard_preview_dispatch")
			if JSON.parse_string(raw) != []:
				failures.append("cached description click emitted request: " + raw)
			await frames(3)
			row = ui.find_child("Achievement633Row", true, false) as Control
			if row == null or is_equal_approx(height, row.size.y):
				failures.append("description click lost expand/collapse action")
	print("ACHIEVEMENT_DESCRIPTION_NATIVE ", JSON.stringify({"skin": "Forever" if forever else "Modern", "case": kind, "failures": failures.size() - before}))
	ui.free()
	await process_frame
	return failures.size() == before
