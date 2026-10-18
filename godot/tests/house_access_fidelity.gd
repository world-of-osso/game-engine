extends SceneTree

var completion = preload("res://tests/completion.gd").new(self, 2)
# Offline supplied draft; real pointer dispatch + production staging reducer, no persistence claim.
var output: String
var verify: bool
var actions: Array[String] = []

func _initialize() -> void:
	call_deferred("run_completion")

func settle() -> bool:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

	return true

func drain(ui: Node) -> bool:
	for action in ui.house_access_preview_actions():
		actions.append(action)

	return true

func move(ui: Node, point: Vector2) -> bool:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	Input.parse_input_event(event)
	await process_frame
	if not completion.step(drain(ui), "drain"):
		return false
	if not completion.step(await settle(), "settle"):
		return false

	return true

func press(ui: Node, point: Vector2, down: bool) -> bool:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = down
	Input.parse_input_event(event)
	await process_frame
	if not completion.step(drain(ui), "drain"):
		return false
	if not completion.step(await settle(), "settle"):
		return false

	return true

func control(ui: Node, name: String) -> Control:
	var found := ui.find_child(name, true, false) as Control
	if not completion.require(found != null, name):
		return null
	return found

func click_apply(ui: Node, owner: Control, expected: String) -> bool:
	var point := owner.get_global_rect().get_center()
	if not completion.step(await move(ui, point), "move"):
		return false
	if not completion.step(await press(ui, point, true), "press"):
		return false
	if not completion.step(await press(ui, point, false), "press"):
		return false
	if not completion.require(actions == [expected], "Actual physical action: " + str(actions)):
		return false
	actions.clear()
	if not completion.require(ui.apply_house_access_preview_action(expected).is_empty()):
		return false
	if not completion.step(await settle(), "settle"):
		return false

	return true

func capture(ui: Node, label: String) -> bool:
	var image := root.get_texture().get_image()
	if not completion.require(image.save_png(output.path_join(label + "-context.png")) == OK):
		return false
	# Fixed source column rectangle, same before/after even when owner/menu geometry changes.
	var panel := control(ui, "HousingHouseSettingsFrame")
	var rect := Rect2i(panel.global_position + Vector2(213, 232), Vector2i(190, 128))
	if not completion.require(image.get_region(rect).save_png(output.path_join(label + "-crop.png")) == OK):
		return false

	return true

func run_access() -> bool:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	output = OS.get_environment("HOUSE_ACCESS_CAPTURE_PATH")
	verify = OS.get_environment("HOUSE_ACCESS_VERIFY") == "1"
	if not completion.require(not output.is_empty()):
		return false
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		if not completion.require(ui.show_house_access_preview(forever, false).is_empty()):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		if not completion.step(await move(ui, Vector2(150, 500)), "move"):
			return false
		if not completion.step(capture(ui, skin + "-idle"), "capture"):
			return false
		var owner := control(ui, "HousingSettingsHouseMode")
		if verify:
			if not completion.require(owner.size == Vector2(122, 25), "Retail WowStyle2 default extent"):
				return false
			if not completion.require(control(ui, "HousingSettingsHouseModeBackground").size == Vector2(136, 39)):
				return false
			if not completion.require(control(ui, "HousingSettingsHouseModeArrow").size == Vector2(12, 5)):
				return false
		var point := owner.get_global_rect().get_center()
		if not completion.step(await move(ui, point), "move"):
			return false
		if not completion.step(capture(ui, skin + "-hover"), "capture"):
			return false
		if not completion.step(await press(ui, point, true), "press"):
			return false
		if not completion.step(capture(ui, skin + "-pressed"), "capture"):
			return false
		if not completion.step(await move(ui, Vector2(150, 500)), "move"):
			return false
		if not completion.step(await press(ui, Vector2(150, 500), false), "press"):
			return false
		if not completion.require(actions.is_empty(), "Outside release cancels"):
			return false
		for key in ["anyone", "none", "limited"]:
			if not completion.step(await click_apply(ui, control(ui, "HousingSettingsHouseMode"), "housing:settings:menu"), "click_apply"):
				return false
			if not completion.step(await move(ui, Vector2(150, 500)), "move"):
				return false
			if not completion.step(capture(ui, skin + "-open-" + key), "capture"):
				return false
			if verify:
				if not completion.require(control(ui, "HousingSettingsAccessMenu").size == Vector2(122, 83)):
					return false
			var row := control(ui, "HousingSettingsMode" + key)
			if not completion.step(await move(ui, row.get_global_rect().get_center()), "move"):
				return false
			if not completion.step(capture(ui, skin + "-row-hover-" + key), "capture"):
				return false
			if not completion.step(await click_apply(ui, row, "housing:settings:" + key), "click_apply"):
				return false
			if not completion.require(ui.find_child("HousingSettingsAccessMenu", true, false) == null):
				return false
			if not completion.step(await move(ui, Vector2(150, 500)), "move"):
				return false
			if not completion.step(capture(ui, skin + "-selected-" + key), "capture"):
				return false
			if verify:
				var caption := control(ui, "HousingSettingsHouseModeText") as Label
				if not completion.require(caption.text == {"anyone": "Anyone", "none": "No One", "limited": "Limited"}[key]):
					return false
		ui.queue_free()
		await process_frame
		ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		if not completion.require(ui.show_house_access_preview(forever, true).is_empty()):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		owner = control(ui, "HousingSettingsHouseMode")
		point = owner.get_global_rect().get_center()
		if not completion.step(await move(ui, point), "move"):
			return false
		if not completion.step(await press(ui, point, true), "press"):
			return false
		if not completion.step(await press(ui, point, false), "press"):
			return false
		if not completion.require(actions.is_empty(), "Pending dropdown dispatches nothing"):
			return false
		if not completion.step(capture(ui, skin + "-disabled"), "capture"):
			return false
		ui.queue_free()
		await process_frame
		if not completion.case_done():
			return false
	print("PASS house-access both-skin physical states/three draft modes/pending; verify=", verify)
	return true

func run_completion() -> void:
	completion.finish(await run_access())
