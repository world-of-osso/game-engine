extends SceneTree

var completion = preload("res://tests/completion.gd").new(self, 2)
# Supplied rows, production physical input; no loot reducer/server acceptance.
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
	for action in ui.loot_hit_preview_actions():
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

func capture(panel: Control, label: String) -> bool:
	var image := root.get_texture().get_image()
	if not completion.require(image.save_png(output.path_join(label + "-context.png")) == OK):
		return false
	if not completion.require(image.get_region(Rect2i(panel.get_global_rect())).save_png(output.path_join(label + "-crop.png")) == OK):
		return false

	return true

func probe(ui: Node, panel: Control, point: Vector2, expected: Array[String], label: String) -> bool:
	actions.clear()
	if not completion.step(await move(ui, point), "move"):
		return false
	if not completion.step(await press(ui, point, true), "press"):
		return false
	var down := actions.duplicate()
	if not completion.step(capture(panel, label + "-pressed"), "capture"):
		return false
	actions.clear()
	if not completion.step(await press(ui, point, false), "press"):
		return false
	if not completion.step(capture(panel, label + "-released"), "capture"):
		return false
	print("LOOT_HIT_CASE ", label, " down=", down, " release=", actions, " expected=", expected)
	if verify:
		if not completion.require(down.is_empty(), "ItemButton may not loot on press"):
			return false
		if not completion.require(actions == expected, "Source hit rectangle and exact server slot action"):
			return false
	actions.clear()

	return true

func cancel(ui: Node, panel: Control, point: Vector2, label: String) -> bool:
	actions.clear()
	if not completion.step(await move(ui, point), "move"):
		return false
	if not completion.step(await press(ui, point, true), "press"):
		return false
	var down := actions.duplicate()
	if not completion.step(capture(panel, label + "-pressed"), "capture"):
		return false
	actions.clear()
	if not completion.step(await move(ui, Vector2(100, 100)), "move"):
		return false
	if not completion.step(await press(ui, Vector2(100, 100), false), "press"):
		return false
	if not completion.step(capture(panel, label + "-released"), "capture"):
		return false
	print("LOOT_HIT_CANCEL ", label, " down=", down, " release=", actions)
	if verify:
		if not completion.require(down.is_empty() and actions.is_empty(), "Outside release cancels loot"):
			return false
	actions.clear()

	return true

func run_hit_area() -> bool:
	root.content_scale_size = Vector2i.ZERO
	root.size = Vector2i(1920, 1080)
	RenderingServer.set_default_clear_color(Color(0.12, 0.12, 0.12))
	output = OS.get_environment("LOOT_HIT_CAPTURE_PATH")
	verify = OS.get_environment("LOOT_HIT_VERIFY") == "1"
	if not completion.require(not output.is_empty()):
		return false
	for forever in [true]:
		var skin := "forever" if forever else "modern"
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var preview_error: String = ui.show_loot_hit_preview(forever)
		print("LOOT_HIT_PREVIEW ", skin, " error=", preview_error)
		if not completion.require(preview_error.is_empty(), preview_error):
			return false
		if not completion.step(await settle(), "settle"):
			return false
		var panel := ui.find_child("LootFrame", true, false) as Control
		if not completion.require(panel != null):
			return false
		for index in [1, 2]:
			var label: String = skin + "-row" + str(index)
			var row := ui.find_child("LootFrameElement" + str(index), true, false) as Control
			if not completion.require(row != null):
				return false
			var origin := row.get_global_rect().position
			var item := ui.find_child("LootFrameElement" + str(index) + "Item", true, false) as Control
			print("LOOT_HIT_GEOMETRY ", label, " row=", row.get_global_rect(), " item=", item.get_global_rect() if item != null else "missing")
			if verify:
				if not completion.require(item != null and item.size == Vector2(37, 37)):
					return false
				if not completion.require(item.get_global_rect().position - origin == Vector2(5, 4)):
					return false
			if not completion.step(await move(ui, Vector2(100, 100)), "move"):
				return false
			if not completion.step(capture(panel, label + "-idle"), "capture"):
				return false
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
				if not completion.step(await probe(ui, panel, origin + case[1], expected, label + "-" + case[0]), "probe"):
					return false
			if not completion.step(await cancel(ui, panel, origin + Vector2(20, 20), label + "-cancel"), "cancel"):
				return false
		if not completion.case_done():
			return false
		ui.queue_free()
		await process_frame
	print("PASS loot-hit both-skin money/item supplied-state probes; verified=", verify)
	return true

func run_completion() -> void:
	completion.finish(await run_hit_area())
