extends SceneTree

var completion = preload("res://tests/completion.gd").new(self, 2)

var fixture: Node

func _initialize() -> void:
	call_deferred("run_completion")

func settle() -> bool:
	for frame in range(8):
		await process_frame
		var error = fixture.call("tick")
		if error != "":
			push_error(error)
			completion.fail("hud_edit_controls_fidelity: settle failed")
			return false
		await RenderingServer.frame_post_draw

	return true

func click_frame(name: String) -> bool:
	var rect: Rect2 = fixture.call("frame_rect", name)
	if not completion.require(rect.size.x > 0 and rect.size.y > 0, "Missing " + name):
		return false
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = rect.get_center()
		event.global_position = event.position
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame
	if not completion.step(await settle(), "settle"):
		return false

	return true

func type_name(text: String) -> bool:
	if not completion.step(await click_frame("EditModeManagerFrameLayoutName"), "click_frame"):
		return false
	for letter in text:
		var key := InputEventKey.new()
		key.pressed = true
		key.unicode = letter.unicode_at(0)
		Input.parse_input_event(key)
		await process_frame
	if not completion.step(await settle(), "settle"):
		return false

	return true

func capture(skin: String, phase: String) -> bool:
	var image := root.get_texture().get_image()
	if not completion.require(image.get_size() == Vector2i(1920, 1080)):
		return false
	if not completion.require(image.save_png(OS.get_environment("GAMEPLAY_CAPTURES").path_join(skin + "-menus-hud-" + phase + ".png")) == OK):
		return false

	return true

func verify_controls() -> bool:
	for skin in ["modern", "forever"]:
		fixture = ClassDB.instantiate("HudEditControlsFixture")
		root.add_child(fixture)
		if not completion.require(fixture.call("initialize", skin == "forever", true) == ""):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		var manager: Rect2 = fixture.call("frame_rect", "EditModeManagerFrame")
		if not completion.require(fixture.call("frame_rect", "EditModeManagerFrameLayoutDropdown") == Rect2(manager.position + Vector2(25,52),Vector2(120,25)), "Retail LayoutDropdown anchor and size"):
			return false
		if not completion.step(await click_frame("EditModeManagerFrameLayoutDropdown"), "click_frame"):
			return false
		if not completion.step(capture(skin, "menu"), "capture"):
			return false
		if not completion.step(await click_frame("EditModeLayoutChoice1" if skin == "forever" else "EditModeLayoutChoice0"), "click_frame"):
			return false
		var preset := "Forever" if skin == "forever" else "Modern"
		if not completion.require(fixture.call("active_name") == preset, "Radio selects persisted layout"):
			return false
		if not completion.step(await click_frame("EditModeManagerFrameNew"), "click_frame"):
			return false
		if not completion.require(fixture.call("frame_rect", "EditModeLayoutDialog") == Rect2(800,465,320,150), "Retail naming dialog centered on UIParent"):
			return false
		if not completion.step(capture(skin, "name"), "capture"):
			return false
		if not completion.step(await click_frame("EditModeLayoutDialogAccept"), "click_frame"):
			return false
		if not completion.require(fixture.call("active_name") == preset, "Empty disabled Save does not persist"):
			return false
		if not completion.step(await type_name("Raid Night"), "type_name"):
			return false
		if not completion.step(await click_frame("EditModeLayoutDialogAccept"), "click_frame"):
			return false
		if not completion.require(fixture.call("active_name") == "Raid Night"):
			return false
		fixture.queue_free()
		await process_frame
		fixture = ClassDB.instantiate("HudEditControlsFixture")
		root.add_child(fixture)
		if not completion.require(fixture.call("initialize", skin == "forever", false) == ""):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.require(fixture.call("active_name") == "Raid Night", "File reload restores named layout"):
			return false
		if not completion.step(await click_frame("EditModeManagerFrameNew"), "click_frame"):
			return false
		if not completion.step(await type_name("Cancelled"), "type_name"):
			return false
		if not completion.step(await click_frame("EditModeLayoutDialogCancel"), "click_frame"):
			return false
		if not completion.require(fixture.call("active_name") == "Raid Night", "Cancel keeps saved selection"):
			return false
		fixture.queue_free()
		await process_frame
		if not completion.case_done():
			return false
	print("PASS: both-skin HUD physical layout/name/save/cancel/file reload")
	return true

func run_completion() -> void:
	completion.finish(await verify_controls())
