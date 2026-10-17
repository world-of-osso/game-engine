extends SceneTree
# Supplied listener state, production projection/physical input; not a response lifecycle.
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
	for action in ui.ready_check_preview_actions():
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

func capture(owner: Control, label: String) -> PackedByteArray:
	var image := root.get_texture().get_image()
	assert(image.save_png(output.path_join(label + "-context.png")) == OK)
	var crop := image.get_region(Rect2i(owner.get_global_rect()))
	assert(crop.save_png(output.path_join(label + "-crop.png")) == OK)
	return crop.get_data()

func run_buttons() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	output = OS.get_environment("READY_BUTTON_CAPTURE_PATH")
	verify = OS.get_environment("READY_BUTTON_VERIFY") == "1"
	assert(not output.is_empty())
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		for key in ["Yes", "No"]:
			var label: String = skin + "-" + key
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			assert(ui.show_ready_check_preview(forever).is_empty())
			await settle()
			var owner := ui.find_child("ReadyCheckFrame" + key + "Button", true, false) as Control
			var panel := ui.find_child("ReadyCheckFrame", true, false) as Control
			assert(owner != null and owner.size == Vector2(119, 24))
			var offset := owner.get_global_rect().position - panel.get_global_rect().position
			print("READY_BUTTON_GEOMETRY ", label, " offset=", offset)
			if verify:
				var source_x: float = 53.5 if key == "Yes" else 183.5
				assert(abs(offset.x - source_x) <= 0.5 and offset.y == 60, "Source anchors with native rounding")
			var point := owner.get_global_rect().get_center()
			var outside := Vector2(100, 100)
			await move(ui, outside)
			var idle := capture(owner, label + "-idle")
			await move(ui, point)
			var hover := capture(owner, label + "-hover")
			await press(ui, point, true)
			var pushed := capture(owner, label + "-pressed")
			assert(idle != hover and idle != pushed, "Physical hover and pressed art")
			if verify:
				var parts := owner.get_node("Parts")
				assert((parts.get_node("Part0") as Control).size.x == 12)
				assert((parts.get_node("Part1") as Control).size.x == 95)
				assert((parts.get_node("Part2") as Control).size.x == 12)
			await move(ui, outside)
			await press(ui, outside, false)
			assert(actions.is_empty(), "Outside release cancels")
			await move(ui, point)
			await press(ui, point, true)
			await press(ui, point, false)
			var expected: Array[String] = []
			expected.append("ready_check_yes" if key == "Yes" else "ready_check_no")
			assert(actions == expected, "Exact response action, no party reducer call")
			actions.clear()
			print("READY_BUTTON ", label, " source states/cancel/action PASS")
			ui.queue_free()
			await process_frame
	print("PASS ready-button both-skin source states; verified=", verify)
	quit(0)
