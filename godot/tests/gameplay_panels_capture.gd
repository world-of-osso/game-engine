extends SceneTree

func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	var directory := OS.get_environment("GAMEPLAY_CAPTURES")
	if directory.is_empty():
		push_error("GAMEPLAY_CAPTURES is required")
		quit(1)
		return
	for skin in ["modern", "forever"]:
		var pages := ["recipient", "duration", "personal"] if OS.get_environment("GAMEPLAY_GROUP") != "menus" else ["meter", "tracking", "transmog", "difficulty"]
		if not OS.get_environment("GAMEPLAY_PAGE").is_empty():
			pages = [OS.get_environment("GAMEPLAY_PAGE")]
		for page in pages:
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			var error = ui.call("show_customer_orders_preview", skin == "forever", page == "duration", page == "personal") if OS.get_environment("GAMEPLAY_GROUP") != "menus" else ui.call("show_gameplay_panels_preview", skin == "forever", page)
			if error != "":
				push_error(error)
				quit(1)
				return
			for frame in range(240):
				if ui.has_node("CustomerOrdersPreviewPortrait"):
					var portrait = ui.get_node("CustomerOrdersPreviewPortrait")
					var portrait_error = portrait.call("tick")
					if portrait_error != "":
						push_error(portrait_error)
						quit(1)
						return
				await process_frame
				await RenderingServer.frame_post_draw
			if ui.has_node("CustomerOrdersPreviewPortrait"):
				var portrait_state = ui.get_node("CustomerOrdersPreviewPortrait").call("portrait_state")
				if not portrait_state.get("model_shown", false):
					push_error("Customer-orders portrait model did not load: " + str(portrait_state))
					quit(1)
					return
			var image := root.get_texture().get_image()
			if image.get_size() != Vector2i(1920, 1080):
				push_error("Capture must be 1920x1080")
				quit(1)
				return
			if image.save_png(directory.path_join(skin + "-" + ("orders" if OS.get_environment("GAMEPLAY_GROUP") != "menus" else "menus") + "-" + page + ".png")) != OK:
				push_error("Capture write failed")
				quit(1)
				return
			ui.queue_free()
			await process_frame
	print("PASS: both-skin gameplay-panel native captures")
	quit(0)
