extends SceneTree
# Supplied owner state, actual picking and viewport input; no live queue/server claim.
var receipts: Array[Dictionary] = []

func _initialize() -> void:
	call_deferred("run_bonus")

func settle() -> void:
	for frame in range(6):
		await process_frame

func actions(ui: Node) -> Array[String]:
	var result: Array[String] = []
	for index in range(16):
		var action: String = ui.pop_action()
		if action.is_empty():
			break
		result.append(action)
	return result

func press(point: Vector2, down: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
	event.pressed = down
	root.push_input(event, true)
	await settle()

func activate(ui: Node, name: String, action: String, label: String) -> Dictionary:
	var button := ui.find_child(name, true, false) as BaseButton
	assert(button != null and button.is_visible_in_tree())
	var point := button.get_global_rect().get_center()
	assert(ui.pvp_ready_pointer_action(point) == action, label + " real top hit")
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await settle()
	assert(button.is_hovered(), label + " native hover")
	await press(point, true)
	assert(button.is_pressed(), label + " native pressed state")
	await press(point, false)
	var observed := actions(ui)
	assert(observed == [action], label + " physical release: " + str(observed))
	var result: Dictionary = ui.apply_pvp_bonus_preview_action(observed[0])
	assert(result.error == "", result.error)
	assert(result.specific_id == 1018, label + " Specific identity preserved")
	receipts.append({"label": label, "target": name, "action": action, "point": str(point), "request": result})
	await settle()
	return result

func select_mode(ui: Node, bonus: bool, label: String) -> void:
	await activate(ui, "HonorFrameTypeDropdown", "pvp:type:toggle", label + " dropdown")
	await activate(ui, "HonorFrameTypeMenuBonus" if bonus else "HonorFrameTypeMenuSpecific", "pvp:type:bonus" if bonus else "pvp:type:specific", label + " mode")

func probe(forever: bool, size: Vector2i) -> void:
	root.size = size
	await settle()
	var ui = ClassDB.instantiate("RegistryUi")
	root.add_child(ui)
	var error: String = ui.show_pvp_bonus_preview(forever)
	assert(error.is_empty(), error)
	await settle()
	var label := ("forever" if forever else "modern") + "/" + str(size)
	var dropdown := ui.find_child("HonorFrameTypeDropdown", true, false) as Control
	assert(dropdown.get_global_rect() == Rect2(531, 162, 180, 25))
	assert(ui.find_child("HonorFrameSpecificScrollBox", true, false) != null)
	await select_mode(ui, true, label)
	assert(ui.find_child("HonorFrameSpecificScrollBox", true, false) == null)
	var random := ui.find_child("HonorFrameBonusFrameRandomBGButton", true, false) as Control
	assert(random != null and random.get_global_rect() == Rect2(338, 200, 365, 54))
	var selected := ui.find_child("HonorFrameBonusFrameRandomBGButtonRowSelected", true, false) as Control
	assert(selected != null and selected.is_visible_in_tree())
	for omitted in ["HonorFrameBonusFrameRandomEpicBGButton", "HonorFrameBonusFrameBrawlButton", "HonorFrameBonusFrameRandomBGButtonReward"]:
		assert(ui.find_child(omitted, true, false) == null)
	await activate(ui, "HonorFrameBonusFrameRandomBGButton", "pvp:bonus:random", label + " selected Random")
	await select_mode(ui, false, label + " roundtrip")
	assert(ui.find_child("HonorFrameSpecificScrollBox", true, false) != null)
	assert(ui.find_child("HonorFrameBonusFrame", true, false) == null)
	await select_mode(ui, true, label + " restore Bonus")
	var request := await activate(ui, "HonorFrameQueueButton", "pvp:queue", label + " Join")
	assert(request.queue_id == 32 and request.pending)
	var join := ui.find_child("HonorFrameQueueButton", true, false) as BaseButton
	assert(join.is_disabled())
	var pending_dropdown := ui.find_child("HonorFrameTypeDropdown", true, false) as BaseButton
	assert(pending_dropdown.is_disabled())
	await press(join.get_global_rect().get_center(), true)
	await press(join.get_global_rect().get_center(), false)
	assert(actions(ui).is_empty(), label + " duplicate pending request suppressed")
	ui.free()
	await settle()

func run_bonus() -> void:
	root.content_scale_size = Vector2i.ZERO
	root.notify_mouse_entered()
	for forever in [false, true]:
		for size in [Vector2i(1280, 720), Vector2i(1920, 1080)]:
			await probe(forever, size)
	var path := OS.get_environment("PVP_RANDOM_BONUS_RECEIPT")
	assert(not path.is_empty())
	var output := FileAccess.open(path, FileAccess.WRITE)
	assert(output != null)
	output.store_string(JSON.stringify(receipts, "\t"))
	output.close()
	print("PASS Random32 Bonus native input cases=", receipts.size(), " both skins/sizes; no UDP claim")
	quit(0)
