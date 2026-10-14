extends SceneTree

var fixture: Node

func _initialize() -> void:
	call_deferred("verify_controls")

func settle() -> void:
	for frame in range(8):
		await process_frame
		var error = fixture.call("tick")
		if error != "":
			push_error(error)
			quit(1)
			return
		await RenderingServer.frame_post_draw

func click_frame(name: String) -> void:
	var rect: Rect2 = fixture.call("frame_rect", name)
	assert(rect.size.x > 0 and rect.size.y > 0, "Missing " + name)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = rect.get_center()
		event.global_position = event.position
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame
	await settle()

func type_name(text: String) -> void:
	await click_frame("EditModeManagerFrameLayoutName")
	for letter in text:
		var key := InputEventKey.new()
		key.pressed = true
		key.unicode = letter.unicode_at(0)
		Input.parse_input_event(key)
		await process_frame
	await settle()

func capture(skin: String, phase: String) -> void:
	var image := root.get_texture().get_image()
	assert(image.get_size() == Vector2i(1920, 1080))
	assert(image.save_png(OS.get_environment("GAMEPLAY_CAPTURES").path_join(skin + "-menus-hud-" + phase + ".png")) == OK)

func verify_controls() -> void:
	for skin in ["modern", "forever"]:
		fixture = ClassDB.instantiate("HudEditControlsFixture")
		root.add_child(fixture)
		assert(fixture.call("initialize", skin == "forever", true) == "")
		await settle()
		await click_frame("EditModeManagerFrameLayoutDropdown")
		capture(skin, "menu")
		await click_frame("EditModeLayoutChoice1" if skin == "forever" else "EditModeLayoutChoice0")
		var preset := "Forever" if skin == "forever" else "Modern"
		assert(fixture.call("active_name") == preset, "Radio selects persisted layout")
		await click_frame("EditModeManagerFrameNew")
		capture(skin, "name")
		await click_frame("EditModeLayoutDialogAccept")
		assert(fixture.call("active_name") == preset, "Empty disabled Save does not persist")
		await type_name("Raid Night")
		await click_frame("EditModeLayoutDialogAccept")
		assert(fixture.call("active_name") == "Raid Night")
		fixture.queue_free()
		await process_frame
		fixture = ClassDB.instantiate("HudEditControlsFixture")
		root.add_child(fixture)
		assert(fixture.call("initialize", skin == "forever", false) == "")
		await settle()
		assert(fixture.call("active_name") == "Raid Night", "File reload restores named layout")
		await click_frame("EditModeManagerFrameNew")
		await type_name("Cancelled")
		await click_frame("EditModeLayoutDialogCancel")
		assert(fixture.call("active_name") == "Raid Night", "Cancel keeps saved selection")
		fixture.queue_free()
		await process_frame
	print("PASS: both-skin HUD physical layout/name/save/cancel/file reload")
	quit(0)
