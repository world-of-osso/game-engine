extends "res://tests/achievement_card_description_font.gd"
# Offline player/guild title controls; native execution only after lead landing.

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 4)
	var passed := true
	for forever in [false, true]:
		for guild in [false, true]:
			var before := failures.size()
			var ui := ClassDB.instantiate("RegistryUi") as CanvasLayer
			root.add_child(ui)
			var error: String = ui.call("show_achievement_full_results_preview", forever, false)
			if not error.is_empty():
				failures.append(error)
			if guild:
				error = ui.call("achievement_search_selection_preview_dispatch", "achievement:guild")
				if not error.is_empty():
					failures.append(error)
			await frames(5)
			var title := ui.find_child("AchievementFrameTitle", true, false) as Label
			var frame := ui.find_child("AchievementFrame", true, false) as Control
			var expected := "Guild Achievements" if guild else "Warband Achievement Points"
			if title == null or frame == null:
				failures.append("missing header title/frame")
			else:
				if title.text != expected or title.get_theme_font_size("font_size") != 10:
					failures.append("source title content/font mismatch")
				if title.get_theme_color("font_shadow_color") != Color.BLACK:
					failures.append("source title requires black shadow")
				if title.get_theme_constant("shadow_offset_x") != 1 or title.get_theme_constant("shadow_offset_y") != 1:
					failures.append("source WoW(+1,-1) shadow must project to native(+1,+1)")
				check_vector(title.size, Vector2(150, 20), "header title allocation")
				check_vector(title.global_position - frame.global_position, Vector2(334, -29), "header title origin")
				var width := title.get_theme_font("font").get_string_size(expected, HORIZONTAL_ALIGNMENT_LEFT, -1, 10).x
				if width > 150:
					failures.append("actual Retail title overflows: " + str(width))
			var output := OS.get_environment("ACHIEVEMENT_HEADER_TITLE_CAPTURES")
			if not output.is_empty():
				await RenderingServer.frame_post_draw
				var name := ("forever" if forever else "modern") + ("-guild" if guild else "-player") + "-achievement-header-title.png"
				var image := root.get_texture().get_image()
				if image == null or image.save_png(output.path_join(name)) != OK:
					failures.append("header title capture failed")
			print("ACHIEVEMENT_HEADER_TITLE_NATIVE ", JSON.stringify({"skin": "Forever" if forever else "Modern", "guild": guild, "failures": failures.size() - before}))
			ui.free()
			await process_frame
			if failures.size() == before:
				if not completion.case_done():
					return
			else:
				passed = false
	for failure in failures:
		print("FAIL CASE ", failure)
	completion.finish(passed)
