extends SceneTree
# Supplied draft and production pointer dispatch only; no close lifecycle/persistence claim.
var output: String
var verify: bool
var actions: Array[String] = []

func _initialize() -> void:
	call_deferred("run_buttons")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func drain(ui: Node) -> void:
	for action in ui.house_access_preview_actions():
		actions.append(action)

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

func capture(panel: Control, owner: Control, label: String, context: bool = false) -> Image:
	var image := root.get_texture().get_image()
	if context:
		assert(image.save_png(output.path_join(label + "-context.png")) == OK)
	var crop := image.get_region(Rect2i(owner.get_global_rect()))
	assert(crop.save_png(output.path_join(label + "-crop.png")) == OK)
	return crop

func run_buttons() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	output = OS.get_environment("HOUSE_ACTION_CAPTURE_PATH")
	verify = OS.get_environment("HOUSE_ACTION_VERIFY") == "1"
	assert(not output.is_empty())
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		for spec in [
			["HousingSettingsSave", false, false],
			["HousingSettingsIgnoreList", false, true],
			["HousingSettingsAbandon", false, true],
			["HousingSettingsSave", true, true]]:
			var label := skin + "-" + spec[0] + ("-pending" if spec[1] else "-current")
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			assert(ui.show_house_access_preview(forever, spec[1]).is_empty())
			await settle()
			var panel := ui.find_child("HousingHouseSettingsFrame", true, false) as Control
			var owner := ui.find_child(spec[0], true, false) as Control
			assert(panel != null and owner != null and owner.size == Vector2(152, 28))
			if verify:
				for part_spec in [[0, 0, 12], [1, 12, 128], [2, 140, 12]]:
					var part := owner.get_node("Parts/Part" + str(part_spec[0])) as TextureRect
					assert(part != null and part.texture != null)
					assert(part.position == Vector2(part_spec[1], 0))
					assert(part.size == Vector2(part_spec[2], 28))
			var outside := Vector2(150, 500)
			var point := owner.get_global_rect().get_center()
			await move(ui, outside)
			var idle := capture(panel, owner, label + "-idle", true)
			await move(ui, point)
			var hover := capture(panel, owner, label + "-hover")
			await press(ui, point, true)
			var pushed := capture(panel, owner, label + "-pressed")
			if spec[2]:
				assert(idle.get_data() == hover.get_data() and hover.get_data() == pushed.get_data(), "Disabled art stays static")
			else:
				assert(idle.get_data() != hover.get_data() and hover.get_data() != pushed.get_data(), "Enabled state art changes")
			await move(ui, outside)
			await press(ui, outside, false)
			assert(actions.is_empty(), "Outside release cancels")
			await move(ui, point)
			await press(ui, point, true)
			await press(ui, point, false)
			if spec[2]:
				assert(actions.is_empty(), "Disabled/pending refuses dispatch")
			else:
				assert(actions == ["housing:settings:save"], "Unchanged Save dispatch; no reducer/server call")
			actions.clear()
			print("HOUSE_ACTION ", label, " physical art/cancel/dispatch PASS")
			ui.queue_free()
			await process_frame
	print("PASS house-action both-skin states/cancel/dispatch; verified=", verify)
	quit(0)
