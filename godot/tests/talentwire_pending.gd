extends SceneTree

func _initialize() -> void:
	call_deferred("capture_pending")

func capture_pending() -> void:
	OS.set_environment("GODOT_SPELLBOOK_TAB", "talents")
	OS.set_environment("GODOT_TALENT_PENDING", "1")
	root.size = Vector2i(1920, 1080)
	var directory := OS.get_environment("GODOT_CAPTURE_PATH")
	for skin in ["modern", "forever"]:
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var method := "show_spellbook_preview" if skin == "modern" else "show_forever_spellbook_preview"
		var error: String = ui.call(method)
		if not error.is_empty():
			push_error(error)
			quit(1)
			return
		for frame in range(120):
			await process_frame
			await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		var window_size := DisplayServer.window_get_size()
		if window_size != Vector2i(1920, 1080) or image == null or image.get_size() != window_size:
			push_error("Expected actual 1920x1080 window and image: ", window_size)
			quit(1)
			return
		var apply := ui.find_child("TalentApply", true, false) as Button
		var undo := ui.find_child("TalentUndo", true, false) as Button
		if apply == null or apply.disabled or undo == null or not undo.is_visible_in_tree():
			push_error("Pending changes must enable Apply and show Undo")
			quit(1)
			return
		var geometry: Dictionary = {"window": [window_size.x, window_size.y]}
		for name in ["SpellBookRoot", "TalentClassName", "TalentSpecName", "TalentApply", "TalentUndo"]:
			var control := ui.find_child(name, true, false) as Control
			if control == null or not control.is_visible_in_tree():
				push_error("Missing talent element: ", name)
				quit(1)
				return
			var rect := control.get_global_rect()
			geometry[name] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
			if control is Label:
				geometry[name + "Text"] = control.text
				if not control.text.contains("Points Available:") or control.text.contains("—"):
					push_error("Missing authoritative point count: ", control.text)
					quit(1)
					return
		var prefix := directory.path_join(skin + "-pending")
		if image.save_png(prefix + ".png") != OK:
			push_error("Save talent capture failed: ", prefix)
			quit(1)
			return
		var file := FileAccess.open(prefix + ".json", FileAccess.WRITE)
		if file == null:
			push_error("Save talent geometry failed: ", prefix)
			quit(1)
			return
		file.store_string(JSON.stringify(geometry, "\t"))
		file.close()
		print("TALENT_CAPTURE ", prefix, " ", geometry)
		ui.queue_free()
		await process_frame
	quit(0)
