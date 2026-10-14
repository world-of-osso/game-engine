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
			if page == "raid":
				var clipping_error := scrollbar_clipping_error(ui, "RaidInfoScrollBox")
				if clipping_error != "":
					push_error(clipping_error)
					quit(1)
					return
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
			if page == "arena-quick":
				ui.call("enable_gameplay_preview_scroll_input")
				var list := ui.find_child("HonorFrameSpecificScrollBox", true, false) as Control
				assert(list != null)
				for step in range(20):
					for pressed in [true, false]:
						var wheel := InputEventMouseButton.new()
						wheel.button_index = MOUSE_BUTTON_WHEEL_DOWN
						wheel.position = list.get_global_rect().get_center()
						wheel.global_position = wheel.position
						wheel.pressed = pressed
						Input.parse_input_event(wheel)
						await process_frame
				await RenderingServer.frame_post_draw
				var skirmish := ui.find_child("HonorFrameArena3v3Button", true, false) as Control
				assert(skirmish != null and list.get_global_rect().encloses(skirmish.get_global_rect()), "Wheel reveals final source skirmish row: list=" + str(list.get_global_rect()) + " row=" + str(skirmish.get_global_rect() if skirmish != null else Rect2()))
				var bar_error := scrollbar_clipping_error(ui, "HonorFrameSpecificScrollBox")
				assert(bar_error == "", bar_error)
				assert(root.get_texture().get_image().save_png(directory.path_join(skin + "-menus-arena-skirmish.png")) == OK)
			ui.queue_free()
			await process_frame
	print("PASS: both-skin gameplay-panel native captures")
	quit(0)

func scrollbar_clipping_error(ui: Node, list: String) -> String:
	var thumb = ui.find_child(list + "ScrollThumb", true, false)
	if not thumb is Control or not thumb.is_visible_in_tree():
		return list + " thumb is absent or hidden"
	var rect: Rect2 = thumb.get_global_rect()
	var ancestor = thumb.get_parent()
	while ancestor != null:
		if ancestor is Control and ancestor.clip_contents and not ancestor.get_global_rect().encloses(rect):
			return list + " thumb clipped by " + ancestor.name + ": " + str(rect)
		ancestor = ancestor.get_parent()
	return ""
