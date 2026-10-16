extends SceneTree
# Supplied rows, production physical input; no loot reducer/server acceptance.
var output: String
var verify: bool
var actions: Array[String] = []

func _initialize() -> void:
	call_deferred("run_hit_area")

func settle() -> void:
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw

func drain(ui: Node) -> void:
	for action in ui.loot_hit_preview_actions():
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

func capture(panel: Control, label: String) -> void:
	var image := root.get_texture().get_image()
	assert(image.save_png(output.path_join(label + "-context.png")) == OK)
	assert(image.get_region(Rect2i(panel.get_global_rect())).save_png(output.path_join(label + "-crop.png")) == OK)

func probe(ui: Node, panel: Control, point: Vector2, expected: Array[String], label: String) -> void:
	actions.clear()
	await move(ui, point)
	await press(ui, point, true)
	var down := actions.duplicate()
	capture(panel, label + "-pressed")
	actions.clear()
	await press(ui, point, false)
	capture(panel, label + "-released")
	print("LOOT_HIT_CASE ", label, " down=", down, " release=", actions, " expected=", expected)
	if verify:
		assert(down.is_empty(), "ItemButton may not loot on press")
		assert(actions == expected, "Source hit rectangle and exact server slot action")
	actions.clear()

func cancel(ui: Node, panel: Control, point: Vector2, label: String) -> void:
	actions.clear()
	await move(ui, point)
	await press(ui, point, true)
	var down := actions.duplicate()
	capture(panel, label + "-pressed")
	actions.clear()
	await move(ui, Vector2(100, 100))
	await press(ui, Vector2(100, 100), false)
	capture(panel, label + "-released")
	print("LOOT_HIT_CANCEL ", label, " down=", down, " release=", actions)
	if verify:
		assert(down.is_empty() and actions.is_empty(), "Outside release cancels loot")
	actions.clear()

func run_hit_area() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	output = OS.get_environment("LOOT_HIT_CAPTURE_PATH")
	verify = OS.get_environment("LOOT_HIT_VERIFY") == "1"
	assert(not output.is_empty())
	for forever in [false, true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		assert(ui.show_loot_hit_preview(forever).is_empty())
		await settle()
		var panel := ui.find_child("LootFrame", true, false) as Control
		assert(panel != null)
		for index in [1, 2]:
			var label: String = skin + "-row" + str(index)
			var row := ui.find_child("LootFrameElement" + str(index), true, false) as Control
			assert(row != null)
			var origin := row.get_global_rect().position
			var item := ui.find_child("LootFrameElement" + str(index) + "Item", true, false) as Control
			print("LOOT_HIT_GEOMETRY ", label, " row=", row.get_global_rect(), " item=", item.get_global_rect() if item != null else "missing")
			if verify:
				assert(item != null and item.size == Vector2(37, 37))
				assert(item.get_global_rect().position - origin == Vector2(5, 4))
			await move(ui, Vector2(100, 100))
			capture(panel, label + "-idle")
			for case in [
				["top", Vector2(20, 2), false],
				["bottom", Vector2(20, 44), false],
				["left", Vector2(2, 20), false],
				["icon", Vector2(20, 20), true],
				["name", Vector2(100, 20), true],
				["extension", Vector2(198.5, 20), true],
				["past", Vector2(200, 20), false],
			]:
				var expected: Array[String] = []
				if case[2]:
					expected.append("loot_slot:5" if index == 1 else "loot_slot:9")
				await probe(ui, panel, origin + case[1], expected, label + "-" + case[0])
			await cancel(ui, panel, origin + Vector2(20, 20), label + "-cancel")
		ui.queue_free()
		await process_frame
	print("PASS loot-hit both-skin money/item supplied-state probes; verified=", verify)
	quit(0)
