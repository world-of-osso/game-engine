extends SceneTree

# One offline native batch. Manifest requires explicit class/spec/skin per view.
# Pair its real1920x1080 PNG/rect evidence with check_talent_layout.py.
func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var directory := OS.get_environment("GODOT_CAPTURE_PATH")
	var manifest = JSON.parse_string(FileAccess.get_file_as_string(directory.path_join("manifest.json")))
	if not manifest is Array:
		push_error("Talent layout capture requires a manifest array")
		quit(1)
		return
	var results: Array = []
	for entry in manifest:
		OS.set_environment("GODOT_PREVIEW_CLASS", str(int(entry.class_id)))
		OS.set_environment("GODOT_PREVIEW_SPEC", str(int(entry.spec_id)))
		OS.set_environment("GODOT_SPELLBOOK_TAB", "talents")
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var method: String
		match entry.skin:
			"modern": method = "show_spellbook_preview"
			"forever": method = "show_forever_spellbook_preview"
			_: push_error("Unknown capture skin"); quit(1); return
		var error: String = ui.call(method)
		var result: Dictionary = entry.duplicate()
		result["blocker"] = error
		if error.is_empty():
			for frame in range(120):
				await process_frame
				await RenderingServer.frame_post_draw
			var image := root.get_texture().get_image()
			if DisplayServer.window_get_size() != Vector2i(1920,1080) or image.get_size() != Vector2i(1920,1080):
				result.blocker = "Requires actual1920x1080"
			elif image.save_png(directory.path_join(entry.filename)) != OK:
				result.blocker = "PNG save failed"
			else:
				result["geometry"] = talent_rects(ui)
		results.append(result)
		print("TALENT_CAPTURE ",entry.skin," ",entry.spec_id," ",result.blocker)
		ui.queue_free()
		await process_frame
	var file := FileAccess.open(directory.path_join("captures.json"),FileAccess.WRITE)
	if file == null:
		push_error("Cannot write captures.json")
		quit(1)
		return
	file.store_string(JSON.stringify(results,"\t"))
	file.close()
	quit(0)

func talent_rects(ui: Node) -> Dictionary:
	var geometry: Dictionary = {}
	for control in ui.find_children("Talent*", "Control", true, false):
		if control.is_visible_in_tree():
			geometry[control.name] = rect_values(control.get_global_rect())
	for name in ["ClassTalentsFrame", "HeroSpecButton"]:
		var control := ui.find_child(name,true,false) as Control
		if control != null:
			geometry[name] = rect_values(control.get_global_rect())
	return geometry

func rect_values(rect: Rect2) -> Array:
	return [rect.position.x,rect.position.y,rect.size.x,rect.size.y]
