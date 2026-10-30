extends SceneTree
# Supplied draft and production pointer dispatch only; no close lifecycle/persistence claim.
var output: String
var verify: bool
var actions: Array[String] = []

func _initialize() -> void:
	call_deferred("run_close")

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

func capture(panel: Control, label: String) -> Image:
	var image := root.get_texture().get_image()
	assert(image.save_png(output.path_join(label + "-context.png")) == OK)
	# Union of old/new close bounds, fixed before/after.
	var crop := image.get_region(Rect2i(panel.global_position + Vector2(558, 0), Vector2(48, 40)))
	assert(crop.save_png(output.path_join(label + "-crop.png")) == OK)
	return crop

func run_close() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	output = OS.get_environment("HOUSE_CLOSE_CAPTURE_PATH")
	verify = OS.get_environment("HOUSE_CLOSE_VERIFY") == "1"
	assert(not output.is_empty())
	for forever in [true]:
		var skin := "forever" if forever else "modern"
		for pending in [false, true]:
			var label := skin + ("-pending" if pending else "-enabled")
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			assert(ui.show_house_access_preview(forever, pending).is_empty())
			await settle()
			var panel := ui.find_child("HousingHouseSettingsFrame", true, false) as Control
			var close := ui.find_child("HousingSettingsClose", true, false) as Control
			assert(panel != null and close != null)
			if verify:
				assert(close.size == Vector2(24, 24))
				assert(close.global_position == panel.global_position + Vector2(577, 0))
				var part := close.get_node("Parts/Part0") as TextureRect
				assert(part != null and part.texture != null)
			var outside := Vector2(150, 500)
			var point := close.get_global_rect().get_center()
			await move(ui, outside)
			var idle := capture(panel, label + "-idle")
			await move(ui, point)
			var hover := capture(panel, label + "-hover")
			if verify:
				var glow := close.get_node("Parts/Part1") as TextureRect
				assert(glow != null and glow.texture != null)
				var material := glow.material as CanvasItemMaterial
				assert(material != null and material.blend_mode == CanvasItemMaterial.BLEND_MODE_ADD)
			await press(ui, point, true)
			var pushed := capture(panel, label + "-pressed")
			assert(idle.get_data() != hover.get_data(), "Hover paints source art")
			assert(hover.get_data() != pushed.get_data(), "Press paints source art")
			await move(ui, outside)
			await press(ui, outside, false)
			assert(actions.is_empty(), "Outside release cancels")
			assert(capture(panel, label + "-cancel").get_data() == idle.get_data())
			await move(ui, point)
			await press(ui, point, true)
			await press(ui, point, false)
			assert(actions == ["housing:close"], "Unchanged source close action also while pending")
			actions.clear()
			print("HOUSE_CLOSE ", label, " rect=", close.get_global_rect(), " physical states/cancel/action PASS")
			ui.queue_free()
			await process_frame
	print("PASS house-settings close both-skin physical states/cancel/action; verified=", verify)
	quit(0)
