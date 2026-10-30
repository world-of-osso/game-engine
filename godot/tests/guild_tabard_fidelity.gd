extends SceneTree

var completion = preload("res://tests/completion.gd").new(self, 2)

func _initialize() -> void:
	call_deferred("run_completion")

func capture() -> bool:
	if not ClassDB.class_exists("GuildTabardFixture"):
		push_error("Missing production-path GuildTabardFixture")
		completion.fail("guild_tabard_fidelity: capture failed")
		return false
	var directory := OS.get_environment("TABARD_CAPTURES")
	if not completion.require(not directory.is_empty()):
		return false
	for skin in ["forever"]:
		var fixture: Node = ClassDB.instantiate("GuildTabardFixture")
		root.add_child(fixture)
		if not completion.require(fixture.call("initialize", skin == "forever") == ""):
			return false
		var ui := fixture.get_node("GuildTabardUI")
		if not completion.step(await ready_models(fixture), "ready_models"):
			return false
		var model_rect := control(ui, "TabardModel").get_global_rect()
		# At UI scale1, native layout.rs uses Taffy's pixel-rounded layout.
		# Blueprint/source17.5 is asserted separately by the UI-model regression.
		if not model_rect.is_equal_approx(Rect2(34, 185, 303, 317)):
			if not completion.step(capture_png(directory, skin + "-tabard-designer-diagnostic.png"), "capture_png"):
				return false
			push_error("TabardModel pixel-rounded source rect expected34,185,303,317; actual " + str(model_rect))
			completion.fail("guild_tabard_fidelity: capture failed")
			return false
		var portrait: Dictionary = fixture.call("portrait_state")
		if not completion.require(portrait.get("mask_loaded", false)):
			return false
		if not completion.require(not portrait.get("pending", true)):
			return false
		if not completion.step(capture_png(directory, skin + "-tabard-designer-default.png"), "capture_png"):
			return false
		if not completion.step(await check_controls(fixture, ui, directory, skin), "check_controls"):
			return false
		for tier in range(3):
			if not completion.require(fixture.call("set_tier", tier) == ""):
				return false
			if not completion.step(await ready_models(fixture), "ready_models"):
				return false
			if not completion.step(capture_png(directory, skin + "-tabard-equipped-tier" + str(tier) + ".png"), "capture_png"):
				return false
		if not completion.case_done():
			return false
		fixture.queue_free()
		await process_frame
	print("PASS: both-skin native vendor controls, masked NPC, real three-tier tabard models and designed Communities portrait; offline authority boundary")
	return true

func check_controls(fixture: Node, ui: Node, directory: String, skin: String) -> bool:
	for field in range(1, 6):
		for step in range(field):
			if not completion.step(await click(ui, "TabardFrameCustomization" + str(field) + "RightButton"), "click"):
				return false
			if not completion.require(fixture.call("dispatch") == ""):
				return false
	if not completion.require(fixture.call("receipt")["draft"] == "1,2,3,4,5"):
		return false
	if not completion.require(fixture.call("receipt")["registered"] == "none", "Draft cannot mutate authority"):
		return false
	if not completion.step(await ready_models(fixture), "ready_models"):
		return false
	if not completion.step(await move_to(control(ui, "TabardFrameCustomization1RightButton").get_global_rect().get_center()), "move_to"):
		return false
	if not completion.step(await settle(), "settle"):
		return false
	if not completion.step(capture_png(directory, skin + "-tabard-designer-hover.png"), "capture_png"):
		return false
	var yaw: float = fixture.call("receipt")["yaw"]
	if not completion.step(await press_at(control(ui, "TabardCharacterModelRotateRightButton").get_global_rect().get_center(), true), "press_at"):
		return false
	if not completion.require(fixture.call("tick", 0.25) == ""):
		return false
	if not completion.require(is_equal_approx(fixture.call("receipt")["yaw"], yaw + PI * 0.25), "Held rotation 0.5turn/s"):
		return false
	if not completion.step(await press_at(control(ui, "TabardCharacterModelRotateRightButton").get_global_rect().get_center(), false), "press_at"):
		return false
	if not completion.require(fixture.call("tick", 0.25) == ""):
		return false
	if not completion.require(is_equal_approx(fixture.call("receipt")["yaw"], yaw + PI * 0.25), "Released rotation stops"):
		return false
	if not completion.step(await click(ui, "TabardFrameAcceptButton"), "click"):
		return false
	var save_request: String = fixture.call("dispatch")
	if not save_request.contains("Save"):
		var accept := control(ui, "TabardFrameAcceptButton") as Button
		push_error("Accept produced '" + save_request + "'; receipt=" + str(fixture.call("receipt")) + "; disabled=" + str(accept.disabled) + "; rect=" + str(accept.get_global_rect()))
		completion.fail("guild_tabard_fidelity: check_controls failed")
		return false
	if not completion.require(fixture.call("receipt")["waiting"]):
		return false
	if not completion.require(fixture.call("receipt")["registered"] == "none"):
		return false
	if not completion.step(await click(ui, "TabardFrameAcceptButton"), "click"):
		return false
	if not completion.require(fixture.call("dispatch") == "", "Pending save cannot duplicate request"):
		return false
	if not completion.require(fixture.call("reply", false) == ""):
		return false
	if not completion.require(not fixture.call("receipt")["waiting"]):
		return false
	if not completion.require(fixture.call("receipt")["registered"] == "none"):
		return false
	if not completion.step(await click(ui, "TabardFrameAcceptButton"), "click"):
		return false
	if not completion.require(fixture.call("dispatch").contains("Save")):
		return false
	if not completion.require(fixture.call("reply", true) == ""):
		return false
	if not completion.require(fixture.call("receipt")["registered"] == "1,2,3,4,5"):
		return false
	if not completion.require(fixture.call("show_communities") == ""):
		return false
	if not completion.step(await settle(), "settle"):
		return false
	var communities := fixture.get_node("CommunitiesUI")
	if not completion.step(check_designed_portrait(communities), "check_designed_portrait"):
		return false
	if not completion.step(capture_png(directory, skin + "-communities-designed.png"), "capture_png"):
		return false
	if not completion.step(await click(ui, "TabardFrameCancelButton"), "click"):
		return false
	if not completion.require(fixture.call("dispatch") == "Close"):
		return false
	if not completion.require(fixture.call("reply", true) == ""):
		return false
	if not completion.require(fixture.call("receipt")["registered"] == "none", "Late reply cannot reopen a closed vendor"):
		return false
	if not completion.require(fixture.call("initialize", skin == "forever") == ""):
		return false
	# Reopen for worn-model tier captures using the saved source IDs.
	if not completion.require(fixture.call("restore_saved_design") == ""):
		return false

	return true

func check_designed_portrait(ui: Node) -> bool:
	# Installed70391 palette columns are Red,Blue,Green, not positional RGB.
	for spec in [
		["CommunitiesTabardBackground", Color(177.0 / 255, 0, 46.0 / 255, 1)],
		["CommunitiesTabardBorder", Color(99.0 / 255, 148.0 / 255, 0, 1)],
		["CommunitiesTabardEmblem", Color(103.0 / 255, 69.0 / 255, 0, 1)],
	]:
		var texture := control(ui, spec[0]).get_node("Parts/Part0") as TextureRect
		if not completion.require(texture.texture != null, spec[0]):
			return false
		if not completion.require(texture.self_modulate.is_equal_approx(spec[1]), str(texture.self_modulate)):
			return false
	var emblem := control(ui, "CommunitiesTabardEmblem")
	if not completion.require(emblem.get_global_rect().size.is_equal_approx(Vector2(56, 64))):
		return false
	var cropped := (emblem.get_node("Parts/Part0") as TextureRect).texture as AtlasTexture
	if not completion.require(cropped != null and cropped.region.is_equal_approx(Rect2(64, 0, 64, 64)), "GuildUtil source large emblem UV"):
		return false

	return true

func ready_models(fixture: Node) -> bool:
	for frame in range(1200):
		var error: String = fixture.call("tick", 0.0)
		if not error.is_empty():
			push_error("Tabard native model preparation: " + error)
			completion.fail("guild_tabard_fidelity: ready_models failed")
			return false
		var portrait: Dictionary = fixture.call("portrait_state")
		if fixture.call("receipt")["model_ready"] and portrait.get("exists", false) and not portrait.get("pending", true):
			if not completion.step(await settle(), "settle"):
				return false
			return true
		await process_frame
	push_error("Real player/NPC tabard models did not both load")
	completion.fail("guild_tabard_fidelity: ready_models failed")

	return false
func control(ui: Node, name: String) -> Control:
	var result := ui.find_child(name, true, false) as Control
	if not completion.require(result != null, name):
		return null
	return result

func move_to(point: Vector2) -> bool:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion)
	await process_frame

	return true

func press_at(point: Vector2, pressed: bool) -> bool:
	if not completion.step(await move_to(point), "move_to"):
		return false
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_LEFT
	event.position = point
	event.global_position = point
	event.pressed = pressed
	root.push_input(event)
	await process_frame

	return true

func click(ui: Node, name: String) -> bool:
	var point := control(ui, name).get_global_rect().get_center()
	if not completion.step(await press_at(point, true), "press_at"):
		return false
	if not completion.step(await press_at(point, false), "press_at"):
		return false
	if not completion.step(await settle(), "settle"):
		return false

	return true

func settle() -> bool:
	for frame in range(6):
		await process_frame
		await RenderingServer.frame_post_draw

	return true

func capture_png(directory: String, name: String) -> bool:
	var image := root.get_texture().get_image()
	if not completion.require(image.get_size() == Vector2i(1920, 1080)):
		return false
	if not completion.require(image.save_png(directory.path_join(name)) == OK):
		return false
	return true

func run_completion() -> void:
	completion.finish(await capture())
