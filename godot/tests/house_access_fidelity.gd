extends SceneTree
# Offline supplied draft; real pointer dispatch + production staging reducer, no persistence claim.
var output: String
var verify: bool
var actions: Array[String] = []

func _initialize() -> void:
	call_deferred("run_access")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func drain(ui: Node) -> void:
	var action: String = ui.pop_action()
	while not action.is_empty():
		actions.append(action)
		action = ui.pop_action()

func move(ui: Node, point: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	Input.parse_input_event(event)
	await process_frame
	drain(ui)
	await settle()

func press(ui: Node, point: Vector2, down: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = down
	Input.parse_input_event(event)
	await process_frame
	drain(ui)
	await settle()

func control(ui: Node, name: String) -> Control:
	var found := ui.find_child(name, true, false) as Control
	assert(found != null, name)
	return found

func click_apply(ui: Node, owner: Control, expected: String) -> void:
	var point := owner.get_global_rect().get_center()
	await move(ui, point)
	await press(ui, point, true)
	await press(ui, point, false)
	assert(actions == [expected], "Actual physical action: " + str(actions))
	actions.clear()
	assert(ui.apply_house_access_preview_action(expected).is_empty())
	await settle()

func capture(ui: Node, label: String) -> void:
	var image := root.get_texture().get_image()
	assert(image.save_png(output.path_join(label + "-context.png")) == OK)
	# Fixed source column rectangle, same before/after even when owner/menu geometry changes.
	var panel := control(ui, "HousingHouseSettingsFrame")
	var rect := Rect2i(panel.global_position + Vector2(213, 232), Vector2i(190, 128))
	assert(image.get_region(rect).save_png(output.path_join(label + "-crop.png")) == OK)

func run_access() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	output = OS.get_environment("HOUSE_ACCESS_CAPTURE_PATH")
	verify = OS.get_environment("HOUSE_ACCESS_VERIFY") == "1"
	assert(not output.is_empty())
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.show_house_access_preview(forever, false).is_empty())
		await settle()
		await move(ui, Vector2(150, 500))
		capture(ui, skin + "-idle")
		var owner := control(ui, "HousingSettingsHouseMode")
		if verify:
			assert(owner.size == Vector2(122, 25), "Retail WowStyle2 default extent")
			assert(control(ui, "HousingSettingsHouseModeBackground").size == Vector2(136, 39))
			assert(control(ui, "HousingSettingsHouseModeArrow").size == Vector2(12, 5))
		var point := owner.get_global_rect().get_center()
		await move(ui, point)
		capture(ui, skin + "-hover")
		await press(ui, point, true)
		capture(ui, skin + "-pressed")
		await move(ui, Vector2(150, 500))
		await press(ui, Vector2(150, 500), false)
		assert(actions.is_empty(), "Outside release cancels")
		for key in ["anyone", "none", "limited"]:
			await click_apply(ui, control(ui, "HousingSettingsHouseMode"), "housing:settings:menu")
			await move(ui, Vector2(150, 500))
			capture(ui, skin + "-open-" + key)
			if verify:
				assert(control(ui, "HousingSettingsAccessMenu").size == Vector2(122, 83))
			var row := control(ui, "HousingSettingsMode" + key)
			await move(ui, row.get_global_rect().get_center())
			capture(ui, skin + "-row-hover-" + key)
			await click_apply(ui, row, "housing:settings:" + key)
			assert(ui.find_child("HousingSettingsAccessMenu", true, false) == null)
			await move(ui, Vector2(150, 500))
			capture(ui, skin + "-selected-" + key)
			if verify:
				var caption := control(ui, "HousingSettingsHouseModeText") as Label
				assert(caption.text == {"anyone": "Anyone", "none": "No One", "limited": "Limited"}[key])
		ui.queue_free()
		await process_frame
		ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.show_house_access_preview(forever, true).is_empty())
		await settle()
		owner = control(ui, "HousingSettingsHouseMode")
		point = owner.get_global_rect().get_center()
		await move(ui, point)
		await press(ui, point, true)
		await press(ui, point, false)
		assert(actions.is_empty(), "Pending dropdown dispatches nothing")
		capture(ui, skin + "-disabled")
		ui.queue_free()
		await process_frame
	print("PASS house-access both-skin physical states/three draft modes/pending; verify=", verify)
	quit(0)
