extends SceneTree
# No server. Prove production Button -> cursor queue coordinates/right-click once per skin.
func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_fixture")

func run_fixture() -> void:
	root.size = Vector2i(1920, 1080)
	for skin in ["modern", "forever"]:
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error: String = ui.show_toybox_pointer_fixture(skin == "forever")
		if not error.is_empty(): fail(error); return
		for frame in range(90): await process_frame
		var tile := ui.find_child("ToySpellButton1", true, false) as Control
		if tile == null: fail("missing toy tile"); return
		var point := tile.get_global_rect().get_center()
		var motion := InputEventMouseMotion.new()
		motion.position = point
		motion.global_position = point
		root.push_input(motion, true)
		await process_frame
		for button in [MOUSE_BUTTON_LEFT, MOUSE_BUTTON_RIGHT]:
			for down in [true, false]:
				var event := InputEventMouseButton.new()
				event.position = point
				event.global_position = point
				event.button_index = button
				event.pressed = down
				root.push_input(event, true)
				await process_frame
		var result: String = ui.assert_toybox_pointer_fixture()
		if not result.is_empty(): fail(skin + " " + result); return
		await RenderingServer.frame_post_draw
		var path := OS.get_environment("TOYBOX_SHOTS").path_join(skin + "-offline-pointer-half-swipe.png")
		root.get_texture().get_image().save_png(path)
		print("TOYBOX_POINTER_PASS ", skin, " ", path)
		ui.free()
		await process_frame
	print("PASS: Toy Box native pointer both skins")
	quit(0)

func fail(message: String) -> void:
	push_error("Toy Box pointer fixture: " + message)
	quit(1)
