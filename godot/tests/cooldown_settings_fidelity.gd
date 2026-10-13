extends SceneTree

var fixture: Node

func _initialize() -> void:
	call_deferred("verify_settings")

func settle() -> void:
	for frame in range(8):
		await process_frame
		var error = fixture.call("tick")
		if error != "":
			push_error(error)
			quit(1)
			return
		await RenderingServer.frame_post_draw

func click_at(point: Vector2, button: MouseButton = MOUSE_BUTTON_LEFT) -> void:
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = button
		event.position = point
		event.global_position = point
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame
	await settle()

func click_frame(name: String, button: MouseButton = MOUSE_BUTTON_LEFT) -> void:
	var rect: Rect2 = fixture.call("frame_rect", name)
	if rect.size.x <= 0 or rect.size.y <= 0:
		push_error("Missing physical control " + name)
		quit(1)
		return
	await click_at(rect.get_center(), button)

func capture(skin: String, phase: String) -> void:
	var directory := OS.get_environment("GAMEPLAY_CAPTURES")
	var image := root.get_texture().get_image()
	assert(image.get_size() == Vector2i(1920, 1080), "1920x1080 required")
	assert(image.save_png(directory.path_join(skin + "-menus-cooldown-" + phase + ".png")) == OK)

func verify_settings() -> void:
	for skin in ["modern", "forever"]:
		fixture = ClassDB.instantiate("CooldownSettingsFixture")
		root.add_child(fixture)
		assert(fixture.call("initialize", skin == "forever", true) == "")
		await settle()
		assert(fixture.call("selected", 27, 71))
		capture(skin, "open")
		await click_frame("CooldownSettingsItem26")
		assert(fixture.call("picked") == 26, "Left release begins Retail order change")
		await click_frame("CooldownSettingsItem27")
		var order = fixture.call("order")
		assert(order[0] == 26 and order[1] == 27, "Drop reorders before target")
		await click_frame("CooldownSettingsItem27", MOUSE_BUTTON_RIGHT)
		await click_frame("CooldownSettingsVisibility")
		assert(not fixture.call("selected", 27, 71), "Source assignment removes selected spec entry")
		assert(fixture.call("selected", 27, 72), "Another specialization remains unchanged")
		capture(skin, "hidden")
		fixture.queue_free()
		await process_frame
		fixture = ClassDB.instantiate("CooldownSettingsFixture")
		root.add_child(fixture)
		assert(fixture.call("initialize", skin == "forever", false) == "")
		await settle()
		assert(not fixture.call("selected", 27, 71), "File restoration preserves hidden entry")
		order = fixture.call("order")
		assert(order[0] == 26 and order[1] == 27, "File restoration preserves custom order")
		await click_frame("CooldownSettingsItem27", MOUSE_BUTTON_RIGHT)
		await click_frame("CooldownSettingsVisibility")
		assert(fixture.call("selected", 27, 71))
		await click_frame("CooldownSettingsUndo")
		assert(not fixture.call("selected", 27, 71), "Revert restores the reopen restore point")
		fixture.queue_free()
		await process_frame
	print("PASS: both-skin native cooldown pointer/reorder/assignment/file/revert flow")
	quit(0)
