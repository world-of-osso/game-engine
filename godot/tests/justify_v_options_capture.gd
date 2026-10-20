extends "res://tests/achievement_search_selection_focus.gd"
# Existing authored Options InfoDetail TOP is now honored; offline panel, not realm UI.

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 2)
	var passed := true
	for forever in [false, true]:
		var before := failures.size()
		var ui := ClassDB.instantiate("RegistryUi") as CanvasLayer
		root.add_child(ui)
		var error: String = ui.call("show_buttonfit_settings_preview", "", true, forever)
		if not error.is_empty():
			failures.append(error)
		await frames(5)
		for key in ["social_addons", "social_api", "social_compat"]:
			var label := ui.find_child("InfoDetail" + key, true, false) as Label
			if label == null or label.vertical_alignment != VERTICAL_ALIGNMENT_TOP:
				failures.append("Options InfoDetail " + key + " must honor authored TOP")
		var output := OS.get_environment("JUSTIFY_V_OPTIONS_CAPTURES")
		if not output.is_empty():
			await RenderingServer.frame_post_draw
			var image := root.get_texture().get_image()
			var path := output.path_join("forever-options-top.png" if forever else "modern-options-top.png")
			if image == null or image.save_png(path) != OK:
				failures.append("Options capture failed: " + path)
		print("OPTIONS_JUSTIFY_V_NATIVE ", JSON.stringify({"skin": "Forever" if forever else "Modern", "failures": failures.size() - before}))
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
