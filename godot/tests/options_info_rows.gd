extends SceneTree

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	for forever in [false, true]:
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error: String = ui.call("show_buttonfit_service_preview", "options:socialaddons", forever)
		if not error.is_empty():
			push_error(error)
			quit(1)
			return
		for frame in range(120):
			await process_frame
			await RenderingServer.frame_post_draw
		var api := ui.find_child("InfoDetailsocial_api", true, false) as Label
		var compatibility := ui.find_child("InfoDetailsocial_compat", true, false) as Label
		var row := ui.find_child("InfoRowsocial_api", true, false) as Control
		if api == null or compatibility == null or row == null or api.get_line_count() < 3:
			push_error("Wrapped Addon API fixture missing")
			quit(1)
			return
		var content := Rect2(api.get_global_rect().position, Vector2(api.size.x, api.get_minimum_size().y))
		if not row.get_global_rect().grow(1).encloses(content) or content.end.y > compatibility.get_global_rect().position.y:
			push_error("Addon API text escapes its row or overlaps Compatibility: content=%s row=%s next=%s" % [content, row.get_global_rect(), compatibility.get_global_rect()])
			quit(1)
			return
		var api_label := ui.find_child("RowLabelsocial_api", true, false) as Control
		if api_label == null or absf(api_label.get_global_rect().position.y - api.get_global_rect().position.y) > 1.0:
			push_error("Multiline Addon API label must align with the first detail line")
			quit(1)
			return
		var output := OS.get_environment("GODOT_OPTIONS_INFO_SHOTS")
		if not output.is_empty():
			var path := output.path_join("forever-options-socialaddons.png" if forever else "modern-options-socialaddons.png")
			if root.get_texture().get_image().save_png(path) != OK:
				push_error("Cannot save options proof")
				quit(1)
				return
		print("PASS options info rows forever=", forever, " content=", content, " row=", row.get_global_rect())
		ui.queue_free()
		await process_frame
	quit(0)
