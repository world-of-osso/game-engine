extends SceneTree

# Offline Forever PlayerSpellsFrame batch. Manifest and all evidence live in data/.
func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var directory := OS.get_environment("GODOT_CAPTURE_PATH")
	var manifest = JSON.parse_string(FileAccess.get_file_as_string(directory.path_join("manifest.json")))
	if not manifest is Array:
		push_error("Classbook capture requires a manifest array")
		quit(1)
		return
	var results: Array = []
	for entry in manifest:
		OS.set_environment("GODOT_PREVIEW_CLASS", str(entry.class_id))
		OS.set_environment("GODOT_PREVIEW_SPEC", str(entry.spec_id))
		OS.set_environment("GODOT_SPELLBOOK_TAB", entry.page)
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error: String = ui.call("show_forever_spellbook_preview")
		var result: Dictionary = entry.duplicate()
		result["blocker"] = error
		if error.is_empty():
			for frame in range(120):
				await process_frame
				await RenderingServer.frame_post_draw
			var image := root.get_texture().get_image()
			if DisplayServer.window_get_size() != Vector2i(1920, 1080) or image == null or image.get_size() != Vector2i(1920, 1080):
				result["blocker"] = "Requires real 1920x1080 window and framebuffer"
			elif image.save_png(directory.path_join(entry.filename)) != OK:
				result["blocker"] = "PNG save failed"
			else:
				var geometry: Dictionary = {"window": [1920, 1080]}
				for name in ["SpellBookRoot", "PlayerSpellsTab1", "PlayerSpellsTab2", "PlayerSpellsTab3", "SpellBookCategoryTab1"]:
					var control := ui.find_child(name, true, false) as Control
					if control != null and control.is_visible_in_tree():
						var rect := control.get_global_rect()
						geometry[name] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
				result["geometry"] = geometry
		results.append(result)
		print("CLASSBOOK: ", entry.filename, " blocker=", result.blocker)
		ui.queue_free()
		await process_frame
	var file := FileAccess.open(directory.path_join("captures.json"), FileAccess.WRITE)
	if file == null:
		push_error("Cannot write captures.json")
		quit(1)
		return
	file.store_string(JSON.stringify(results, "\t"))
	file.close()
	quit(0)
