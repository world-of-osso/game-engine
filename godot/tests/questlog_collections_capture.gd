extends SceneTree

# Offline rendering proof; no account or world state is invented as live evidence.
func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var directory := OS.get_environment("UI_FIDELITY_CAPTURE_DIR")
	if directory.is_empty():
		push_error("UI_FIDELITY_CAPTURE_DIR required")
		quit(1)
		return
	for forever in [false, true]:
		for journal in ["questlog", "mounts", "pets", "toys"]:
			var node: Node
			var error: String
			if journal == "toys":
				node = ClassDB.instantiate("RegistryUi")
				root.add_child(node)
				error = node.call("show_toybox_pointer_fixture", forever)
			else:
				node = ClassDB.instantiate("UiAuditProbe")
				root.add_child(node)
				if journal == "questlog":
					error = node.call("mount_quest_overflow", forever, "log", false)
				else:
					error = node.call("mount_collections", forever, journal)
			if not error.is_empty():
				push_error(error)
				quit(1)
				return
			for frame in range(120):
				await process_frame
				await RenderingServer.frame_post_draw
			if journal != "questlog":
				var panel := node.find_child("CollectionsJournal", true, false) as Control
				if panel == null or not panel.get_global_rect().size.is_equal_approx(Vector2(703, 606)) or not root.get_visible_rect().encloses(panel.get_global_rect()):
					push_error("Collections shell must be fully visible at XML pixel size: " + str(panel.get_global_rect() if panel != null else null))
					quit(1)
					return
			var image := root.get_texture().get_image()
			var skin := "forever" if forever else "modern"
			var path := directory.path_join(skin + "-" + journal + ".png")
			if image == null or image.is_empty() or image.save_png(path) != OK:
				push_error("Capture failed: " + path)
				quit(1)
				return
			print("PASS captured ", path)
			node.queue_free()
			await process_frame
	quit(0)
