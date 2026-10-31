extends SceneTree

# Real RegistryUi rendering, no server or synthetic art. Both skins share source geometry.
func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	var directory := OS.get_environment("GODOT_PRIMITIVES_CAPTURES")
	if directory.is_empty():
		push_error("GODOT_PRIMITIVES_CAPTURES is required")
		quit(1)
		return
	for skin in ["forever"]:
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error = ui.call("show_forever_menu_primitives_preview")
		if error != "":
			push_error(error)
			quit(1)
			return
		for frame in range(180):
			await process_frame
			await RenderingServer.frame_post_draw
		for name in ["ContextBackdrop", "OpenDropdownMenuBackground"]:
			var node = ui.find_child(name, true, false)
			if node != null:
				if not is_equal_approx(node.modulate.a, 0.925):
					push_error("MenuStyle1 backdrop must retain source alpha .925: " + name)
					quit(1)
					return
			else:
				push_error("Missing source backdrop: " + name)
				quit(1)
				return
		var image := root.get_texture().get_image()
		if image == null or image.is_empty():
			push_error("Rendering display required")
			quit(1)
			return
		# GetAtlasInfo slices keep the 9px source metal edge at x=790+9;
		# stretching the68px member moves it30px inward and covers the labels.
		var border := image.get_pixel(799, 548)
		var inside := image.get_pixel(805, 548)
		if maxf(border.r, maxf(border.g, border.b)) < 0.24 or maxf(inside.r, maxf(inside.g, inside.b)) > 0.2:
			push_error("Menu atlas edge must retain source pixels instead of stretching: " + skin)
			quit(1)
			return
		var result := image.save_png(directory.path_join(skin + "-all.png"))
		if result != OK:
			push_error("Cannot save primitive capture: " + str(result))
			quit(1)
			return
		ui.queue_free()
		await process_frame
	print("PASS: Forever shared menu/NavBar capture")
	quit(0)
