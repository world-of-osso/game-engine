extends SceneTree

# Actual RegistryUi projection, offline: no server or player state changes.
# GODOT_LAUNCHER_SKIN=modern|forever; chosen filled glyphs only.
# GODOT_CAPTURE_PATH: 1920x1080 PNG. Invoked by scripts/capture-launcher-candidates.py.
func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var output = OS.get_environment("GODOT_CAPTURE_PATH")
	var forever = OS.get_environment("GODOT_LAUNCHER_SKIN") == "forever"
	var ui = ClassDB.instantiate("RegistryUi")
	root.add_child(ui)
	var error = ui.show_launcher_candidate(forever)
	if error != "":
		push_error(error)
		quit(1)
		return
	for frame in range(5):
		await process_frame
		await RenderingServer.frame_post_draw
	var image = root.get_texture().get_image()
	if image == null or image.get_size() != Vector2i(1920, 1080):
		push_error("Launcher candidates require an actual 1920x1080 rendering display")
		quit(1)
		return
	if image.save_png(output) != OK:
		push_error("Cannot save launcher candidate: " + output)
		quit(1)
		return
	# Activation travels through Godot's real pointer input, not Registry click helpers.
	var button = ui.control_for_action("micro:PlayerSpellsMicroButton")
	if button == null:
		push_error("Missing native Spellbook button")
		quit(1)
		return
	var position = button.get_global_rect().get_center()
	for pressed in [true, false]:
		var click = InputEventMouseButton.new()
		click.button_index = MOUSE_BUTTON_LEFT
		click.position = position
		click.global_position = position
		click.pressed = pressed
		Input.parse_input_event(click)
		await process_frame
	if ui.pop_action() != "micro:PlayerSpellsMicroButton":
		push_error("Native Spellbook click did not dispatch")
		quit(1)
		return
	print("PASS: launcher candidate rendered at 1920x1080; native Spellbook click dispatched")
	ui.queue_free()
	await process_frame
	quit(0)
