extends SceneTree

func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	if not ClassDB.class_exists("GuildTabardFixture"):
		push_error("Missing production-path GuildTabardFixture")
		quit(1)
		return
	var directory := OS.get_environment("TABARD_CAPTURES")
	assert(not directory.is_empty())
	for skin in ["modern", "forever"]:
		var fixture: Node = ClassDB.instantiate("GuildTabardFixture")
		root.add_child(fixture)
		assert(fixture.call("initialize", skin == "forever") == "")
		var ui := fixture.get_node("GuildTabardUI")
		await ready_models(fixture)
		assert(control(ui, "TabardModel").get_global_rect().is_equal_approx(Rect2(33.5, 185, 303, 317)))
		var portrait: Dictionary = fixture.call("portrait_state")
		assert(portrait.get("mask_loaded", false))
		assert(not portrait.get("pending", true))
		capture_png(directory, skin + "-tabard-designer-default.png")
		await check_controls(fixture, ui, directory, skin)
		for tier in range(3):
			assert(fixture.call("set_tier", tier) == "")
			await ready_models(fixture)
			capture_png(directory, skin + "-tabard-equipped-tier" + str(tier) + ".png")
		fixture.queue_free()
		await process_frame
	print("PASS: both-skin native vendor controls, masked NPC, real three-tier tabard models and designed Communities portrait; offline authority boundary")
	quit(0)

func check_controls(fixture: Node, ui: Node, directory: String, skin: String) -> void:
	for field in range(1, 6):
		for step in range(field):
			await click(ui, "TabardFrameCustomization" + str(field) + "RightButton")
			assert(fixture.call("dispatch") == "")
	assert(fixture.call("receipt")["draft"] == "1,2,3,4,5")
	assert(fixture.call("receipt")["registered"] == "none", "Draft cannot mutate authority")
	await ready_models(fixture)
	await move_to(control(ui, "TabardFrameCustomization1RightButton").get_global_rect().get_center())
	await settle()
	capture_png(directory, skin + "-tabard-designer-hover.png")
	var yaw: float = fixture.call("receipt")["yaw"]
	await press_at(control(ui, "TabardCharacterModelRotateRightButton").get_global_rect().get_center(), true)
	assert(fixture.call("tick", 0.25) == "")
	assert(is_equal_approx(fixture.call("receipt")["yaw"], yaw + PI * 0.25), "Held rotation 0.5turn/s")
	await press_at(control(ui, "TabardCharacterModelRotateRightButton").get_global_rect().get_center(), false)
	assert(fixture.call("tick", 0.25) == "")
	assert(is_equal_approx(fixture.call("receipt")["yaw"], yaw + PI * 0.25), "Released rotation stops")
	await click(ui, "TabardFrameAcceptButton")
	assert(fixture.call("dispatch").contains("Save"))
	assert(fixture.call("receipt")["waiting"])
	assert(fixture.call("receipt")["registered"] == "none")
	await click(ui, "TabardFrameAcceptButton")
	assert(fixture.call("dispatch") == "", "Pending save cannot duplicate request")
	assert(fixture.call("reply", false) == "")
	assert(not fixture.call("receipt")["waiting"])
	assert(fixture.call("receipt")["registered"] == "none")
	await click(ui, "TabardFrameAcceptButton")
	assert(fixture.call("dispatch").contains("Save"))
	assert(fixture.call("reply", true) == "")
	assert(fixture.call("receipt")["registered"] == "1,2,3,4,5")
	assert(fixture.call("show_communities") == "")
	await settle()
	var communities := fixture.get_node("CommunitiesUI")
	check_designed_portrait(communities)
	capture_png(directory, skin + "-communities-designed.png")
	await click(ui, "TabardFrameCancelButton")
	assert(fixture.call("dispatch") == "Close")
	assert(fixture.call("reply", true) == "")
	assert(fixture.call("receipt")["registered"] == "none", "Late reply cannot reopen a closed vendor")
	assert(fixture.call("initialize", skin == "forever") == "")
	# Reopen for worn-model tier captures using the saved source IDs.
	assert(fixture.call("restore_saved_design") == "")

func check_designed_portrait(ui: Node) -> void:
	# Installed70391 palette columns are Red,Blue,Green, not positional RGB.
	for spec in [
		["CommunitiesTabardBackground", Color(177.0 / 255, 0, 46.0 / 255, 1)],
		["CommunitiesTabardBorder", Color(99.0 / 255, 148.0 / 255, 0, 1)],
		["CommunitiesTabardEmblem", Color(103.0 / 255, 69.0 / 255, 0, 1)],
	]:
		var texture := control(ui, spec[0]).get_node("Parts/Part0") as TextureRect
		assert(texture.texture != null, spec[0])
		assert(texture.self_modulate.is_equal_approx(spec[1]), str(texture.self_modulate))
	var emblem := control(ui, "CommunitiesTabardEmblem")
	assert(emblem.get_global_rect().size.is_equal_approx(Vector2(56, 64)))
	var cropped := (emblem.get_node("Parts/Part0") as TextureRect).texture as AtlasTexture
	assert(cropped != null and cropped.region.is_equal_approx(Rect2(64, 0, 64, 64)), "GuildUtil source large emblem UV")

func ready_models(fixture: Node) -> void:
	for frame in range(1200):
		assert(fixture.call("tick", 0.0) == "")
		if fixture.call("receipt")["model_ready"]:
			await settle()
			return
		await process_frame
	assert(false, "Real tabard player model did not load")

func control(ui: Node, name: String) -> Control:
	var result := ui.find_child(name, true, false) as Control
	assert(result != null, name)
	return result

func move_to(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion)
	await process_frame

func press_at(point: Vector2, pressed: bool) -> void:
	await move_to(point)
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_LEFT
	event.position = point
	event.global_position = point
	event.pressed = pressed
	root.push_input(event)
	await process_frame

func click(ui: Node, name: String) -> void:
	var point := control(ui, name).get_global_rect().get_center()
	await press_at(point, true)
	await press_at(point, false)
	await settle()

func settle() -> void:
	for frame in range(6):
		await process_frame
		await RenderingServer.frame_post_draw

func capture_png(directory: String, name: String) -> void:
	var image := root.get_texture().get_image()
	assert(image.get_size() == Vector2i(1920, 1080))
	assert(image.save_png(directory.path_join(name)) == OK)
