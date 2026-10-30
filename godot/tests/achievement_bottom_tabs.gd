extends "res://tests/achievement_card_description_font.gd"
# Authored AchievementFrameTabButtonTemplate, not generic PanelTab art.

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 6)
	var passed := true
	for forever in [true]:
		for action in ["achievement:achievements", "achievement:guild", "achievement:statistics"]:
			var before := failures.size()
			var ui := ClassDB.instantiate("RegistryUi") as CanvasLayer
			root.add_child(ui)
			var error: String = ui.call("show_achievement_full_results_preview", forever, false)
			if not error.is_empty():
				failures.append(error)
			error = ui.call("achievement_search_selection_preview_dispatch", action)
			if not error.is_empty():
				failures.append(error)
			await frames(5)
			for name in ["AchievementTabAchievements", "AchievementTabGuild", "AchievementTabStatistics"]:
				var text := ui.find_child(name + "Text", true, false) as Label
				if text == null or text.horizontal_alignment != HORIZONTAL_ALIGNMENT_CENTER:
					failures.append(name + " caption must be centered")
				elif text.get_theme_font_size("font_size") != 10:
					failures.append(name + " requires GameFontNormalSmall")
			var output := OS.get_environment("ACHIEVEMENT_TAB_CAPTURES")
			if not output.is_empty() and action == "achievement:achievements":
				await RenderingServer.frame_post_draw
				var name := "forever-achievement-bottom-tabs.png" if forever else "modern-achievement-bottom-tabs.png"
				var image := root.get_texture().get_image()
				if image == null or image.save_png(output.path_join(name)) != OK:
					failures.append("bottom tab capture failed")
			print("ACHIEVEMENT_BOTTOM_TABS_NATIVE ", JSON.stringify({"skin": "Forever" if forever else "Modern", "action": action, "failures": failures.size() - before}))
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
