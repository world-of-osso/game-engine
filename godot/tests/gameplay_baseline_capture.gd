extends SceneTree

func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	for skin in ["modern", "forever"]:
		for page in ["raid", "scoreboard", "arena-quick", "arena-rated"]:
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			var error = ui.call("show_gameplay_baseline_preview", skin == "forever", page)
			if error != "":
				push_error(error)
				quit(1)
				return
			for frame in range(240):
				await process_frame
				await RenderingServer.frame_post_draw
			var image := root.get_texture().get_image()
			assert(image.get_size() == Vector2i(1920,1080))
			assert(image.save_png(OS.get_environment("GAMEPLAY_CAPTURES").path_join(skin + "-menus-" + page + "-baseline.png")) == OK)
			if page == "arena-quick":
				ui.call("enable_gameplay_preview_scroll_input")
				var list := ui.find_child("HonorFrameSpecificScrollBox", true, false) as Control
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
				assert(root.get_texture().get_image().save_png(OS.get_environment("GAMEPLAY_CAPTURES").path_join(skin + "-menus-arena-skirmish-baseline.png")) == OK)
			ui.queue_free()
			await process_frame
	print("PASS: 642fcee41 raid/scoreboard/arena renderers captured in both skins")
	quit(0)
