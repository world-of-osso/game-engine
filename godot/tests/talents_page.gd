extends SceneTree

# Actual cage pixels + native pointer hit, offline production Talents projection.
func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var directory := OS.get_environment("GODOT_CAPTURE_PATH")
	for skin in ["modern", "forever"]:
		OS.set_environment("GODOT_SPELLBOOK_TAB", "talents")
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
		var window_size := DisplayServer.window_get_size()
		var image := root.get_texture().get_image()
		if window_size != Vector2i(1920, 1080) or image.get_size() != window_size:
			push_error("Talents requires actual 1920x1080 window and framebuffer")
			quit(1)
			return
		var node := ui.find_child("TalentNode62121", true, false) as Control
		var spec := ui.find_child("TalentNode102439", true, false) as Control
		if node == null or spec == null:
			push_error("Mage Arcane Talents lacks cited class/spec nodes")
			quit(1)
			return
		if not rect_matches(node.get_global_rect(), Rect2(472, 210, 40, 40)) or not rect_matches(spec.get_global_rect(), Rect2(1192, 390, 40, 40)):
			push_error("Talent position does not match Blizzard /10 and pan offsets: ", node.get_global_rect(), " ", spec.get_global_rect())
			quit(1)
			return
		var prefix := directory.path_join(skin + "-talents")
		if image.save_png(prefix + ".png") != OK:
			push_error("Could not save Talents capture")
			quit(1)
			return
		var geometry: Dictionary = {"window": [1920, 1080], "node62121": str(node.get_global_rect()), "node102439": str(spec.get_global_rect())}
		var file := FileAccess.open(prefix + ".json", FileAccess.WRITE)
		file.store_string(JSON.stringify(geometry, "\t"))
		file.close()
		var host = ClassDB.instantiate("RegistryUi")
		host.layer = 8
		root.add_child(host)
		var pointer := node.get_global_rect().get_center()
		var motion := InputEventMouseMotion.new()
		motion.position = pointer
		motion.global_position = pointer
		root.push_input(motion)
		for frame in range(6):
			await process_frame
			error = ui.call("update_spellbook_preview_tooltip", host, pointer)
			if not error.is_empty():
				push_error(error)
				quit(1)
				return
			await RenderingServer.frame_post_draw
		var title := host.find_child("TooltipTitle", true, false) as Label
		if title == null or not title.is_visible_in_tree() or title.text != "Prismatic Barrier":
			push_error("Hovered grant lacks shared spell name")
			quit(1)
			return
		var words: PackedStringArray = []
		for label in host.find_children("TooltipLine*", "Label", true, false):
			if label.is_visible_in_tree():
				words.append(label.text)
		var content := " ".join(words)
		if not content.contains("Spell ID: 235450") or not content.contains("damage"):
			push_error("Hover lacks shared spell description/ID: ", content)
			quit(1)
			return
		if root.get_texture().get_image().save_png(prefix + "-hover.png") != OK:
			quit(1)
			return
		print("PASS ", skin, " Talents actual1080p, cited geometry, hover ", title.text, " ", content)
		host.queue_free()
		ui.queue_free()
		await process_frame
	quit(0)

func rect_matches(actual: Rect2, expected: Rect2) -> bool:
	return actual.position.distance_to(expected.position) < 0.01 and actual.size.distance_to(expected.size) < 0.01
