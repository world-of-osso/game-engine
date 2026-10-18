extends SceneTree
# Supplied owner snapshots only. Real projection/picking/input; no UDP acceptance claim.
var failures: Array[String] = []
var receipts: Array[Dictionary] = []
var overlaps: Array[Dictionary] = []

func _initialize() -> void:
	call_deferred("run_placement")

func settle() -> void:
	for frame in range(6):
		await process_frame

func check(condition: bool, description: String) -> void:
	if not condition:
		failures.append(description)

func click(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await settle()

func collect_actions(ui: Node) -> Array[String]:
	var actions: Array[String] = []
	for index in range(16):
		var action: String = ui.pop_action()
		if action.is_empty():
			break
		actions.append(action)
	return actions

func probe(forever: bool, size: Vector2i, phase: String) -> void:
	root.size = size
	await settle()
	var ui = ClassDB.instantiate("RegistryUi")
	root.add_child(ui)
	var error: String = ui.show_gameplay_panels_preview(forever, "pvp-ready-" + phase)
	assert(error.is_empty(), error)
	await settle()
	var dialog := ui.find_child("PVPReadyDialog", true, false) as Control
	var queue := ui.find_child("PVPUIFrame", true, false) as Control
	assert(dialog != null and queue != null)
	var label := ("forever" if forever else "modern") + "/" + str(size) + "/" + phase
	var expected := Vector2((size.x - 306.0) / 2.0, 135.0)
	check(dialog.get_global_rect() == Rect2(expected, Vector2(306, 210)), label + " root anchor")
	check(queue.is_visible_in_tree(), label + " queue remains visible")
	check(queue.get_global_rect() == Rect2(100, 84, 788, 428), label + " unchanged queue geometry")
	var sidebar := ui.find_child("PVPUIHonorInset", true, false) as Control
	assert(sidebar != null)
	var overlap := dialog.get_global_rect().intersection(sidebar.get_global_rect())
	assert(overlap.has_area())
	var body_point := Vector2(overlap.get_center().x, expected.y + 40.0)
	var body_top: String = ui.pvp_ready_pointer_frame(body_point)
	check(body_top == "PVPReadyDialog", label + " dialog body/sidebar top hit: " + body_top)
	overlaps.append({"case": label, "point": str(body_point), "overlap": str(overlap), "top_frame": body_top})
	for action in ["pvp:accept", "pvp:decline"]:
		# Close and Leave Queue both dispatch decline; select the exact source action button.
		var name := "PVPReadyDialogEnterBattleButton" if action == "pvp:accept" else "PVPReadyDialogLeaveQueueButton"
		var button := ui.find_child(name, true, false) as Control
		assert(button != null)
		var x := 31.0 if action == "pvp:accept" else 160.0
		var point := expected + Vector2(x + 57.5, 174.0)
		var top_action: String = ui.pvp_ready_pointer_action(point)
		var actual_button_top: String = ui.pvp_ready_pointer_action(button.get_global_rect().get_center())
		check(top_action == action, label + " source-position top hit " + action + ": " + top_action)
		check(actual_button_top == action, label + " actual-button top hit " + action + ": " + actual_button_top)
		await click(point)
		var actions := collect_actions(ui)
		check(actions == [action], label + " physical action " + action + ": " + str(actions))
		receipts.append({"case": label, "action": action, "dialog": str(dialog.get_global_rect()), "queue": str(queue.get_global_rect()), "point": str(point), "top_action": top_action, "actual_button_top": actual_button_top, "actual_control": str(button.get_path()), "physical_actions": actions})
	ui.free()
	await settle()

func run_placement() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.notify_mouse_entered()
	for forever in [false, true]:
		for size in [Vector2i(1280, 720), Vector2i(1920, 1080)]:
			for phase in ["waiting", "running"]:
				await probe(forever, size, phase)
	var path := OS.get_environment("PVP_READY_PLACEMENT_RECEIPT")
	assert(not path.is_empty())
	var output := FileAccess.open(path, FileAccess.WRITE)
	assert(output != null)
	output.store_string(JSON.stringify({"cases": receipts, "overlaps": overlaps, "failures": failures}, "\t"))
	output.close()
	print("PVP_READY_PLACEMENT cases=", receipts.size(), " failures=", failures)
	quit(0 if failures.is_empty() else 1)
