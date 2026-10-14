extends SceneTree

func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	for skin in ["modern", "forever"]:
		for page in ["hud", "cooldown"]:
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			var error = ui.call("show_gameplay_baseline_preview", skin == "forever", page == "cooldown")
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
			ui.queue_free()
			await process_frame
	print("PASS: dc7c36424 HUD/cooldown renderers captured in both skins")
	quit(0)
