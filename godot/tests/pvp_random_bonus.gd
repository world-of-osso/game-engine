extends SceneTree
# Supplied owner state, actual picking and viewport input; no live queue/server claim.
var receipts: Array[Dictionary] = []
var completed_cases := 0
var completion = preload("res://tests/completion.gd").new(self, 4)

func _initialize() -> void:
	call_deferred("run_completion")

func settle() -> bool:
	for frame in range(6):
		await process_frame

	return true

func actions(ui: Node) -> Array[String]:
	var result: Array[String] = []
	for index in range(16):
		var action: String = ui.pop_action()
		if action.is_empty():
			break
		result.append(action)
	return result

func press(point: Vector2, down: bool) -> bool:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
	event.pressed = down
	root.push_input(event, true)
	if not completion.step(await settle(), "settle"):
		return false

	return true

func activate(ui: Node, name: String, action: String, label: String) -> Dictionary:
	var button := ui.find_child(name, true, false) as BaseButton
	if not completion.require(button != null and button.is_visible_in_tree()):
		return {}
	var point := button.get_global_rect().get_center()
	if not completion.require(ui.pvp_ready_pointer_action(point) == action, label + " real top hit"):
		return {}
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	if not completion.step(await settle(), "settle"):
		return {}
	print("RANDOM_HOVER ", label, " point=", point, " expected=", button.get_path(), " hovered=", button.is_hovered(), " gui=", root.gui_get_hovered_control())
	if not completion.require(button.is_hovered(), label + " native hover"):
		return {}
	if not completion.step(await press(point, true), "press"):
		return {}
	if not completion.require(button.is_pressed(), label + " native pressed state"):
		return {}
	if not completion.step(await press(point, false), "press"):
		return {}
	var observed := actions(ui)
	if not completion.require(observed == [action], label + " physical release: " + str(observed)):
		return {}
	var result: Dictionary = ui.apply_pvp_bonus_preview_action(observed[0])
	if not completion.require(result.error == "", result.error):
		return {}
	if not completion.require(result.specific_id == 1018, label + " Specific identity preserved"):
		return {}
	receipts.append({"label": label, "target": name, "action": action, "point": str(point), "request": result})
	if not completion.step(await settle(), "settle"):
		return {}
	return result

func select_mode(ui: Node, bonus: bool, label: String) -> bool:
	if not completion.require(not (await activate(ui, "HonorFrameTypeDropdown", "pvp:type:toggle", label + " dropdown")).is_empty(), "activate incomplete"):
		return false
	if not completion.require(not (await activate(ui, "HonorFrameTypeMenuBonus" if bonus else "HonorFrameTypeMenuSpecific", "pvp:type:bonus" if bonus else "pvp:type:specific", label + " mode")).is_empty(), "activate incomplete"):
		return false

	return true

func probe(forever: bool, size: Vector2i) -> bool:
	root.size = size
	if not completion.step(await settle(), "settle"):
		return false
	var ui = ClassDB.instantiate("RegistryUi")
	root.add_child(ui)
	var error: String = ui.show_pvp_bonus_preview(forever)
	if not completion.require(error.is_empty(), error):
		return false
	if not completion.step(await settle(), "settle"):
		return false
	var label := ("forever" if forever else "modern") + "/" + str(size)
	var dropdown := ui.find_child("HonorFrameTypeDropdown", true, false) as Control
	if not completion.require(dropdown.get_global_rect() == Rect2(531, 162, 180, 25)):
		return false
	if not completion.require(ui.find_child("HonorFrameSpecificScrollBox", true, false) != null):
		return false
	if not completion.step(await select_mode(ui, true, label), "select_mode"):
		return false
	if not completion.require(ui.find_child("HonorFrameSpecificScrollBox", true, false) == null):
		return false
	var random := ui.find_child("HonorFrameBonusFrameRandomBGButton", true, false) as Control
	if not completion.require(random != null and random.get_global_rect() == Rect2(338, 200, 365, 54)):
		return false
	var selected := ui.find_child("HonorFrameBonusFrameRandomBGButtonRowSelected", true, false) as Control
	if not completion.require(selected != null and selected.is_visible_in_tree()):
		return false
	for omitted in ["HonorFrameBonusFrameRandomEpicBGButton", "HonorFrameBonusFrameBrawlButton", "HonorFrameBonusFrameRandomBGButtonReward"]:
		if not completion.require(ui.find_child(omitted, true, false) == null):
			return false
	if not completion.require(not (await activate(ui, "HonorFrameBonusFrameRandomBGButton", "pvp:bonus:random", label + " selected Random")).is_empty(), "activate incomplete"):
		return false
	if not completion.step(await select_mode(ui, false, label + " roundtrip"), "select_mode"):
		return false
	if not completion.require(ui.find_child("HonorFrameSpecificScrollBox", true, false) != null):
		return false
	if not completion.require(ui.find_child("HonorFrameBonusFrame", true, false) == null):
		return false
	if not completion.step(await select_mode(ui, true, label + " restore Bonus"), "select_mode"):
		return false
	var request := await activate(ui, "HonorFrameQueueButton", "pvp:queue", label + " Join")
	if not completion.require(not request.is_empty(), "Join incomplete"):
		return false
	if not completion.require(request.queue_id == 32 and request.pending):
		return false
	var join := ui.find_child("HonorFrameQueueButton", true, false) as BaseButton
	if not completion.require(join.is_disabled()):
		return false
	var pending_dropdown := ui.find_child("HonorFrameTypeDropdown", true, false) as BaseButton
	if not completion.require(pending_dropdown.is_disabled()):
		return false
	if not completion.step(await press(join.get_global_rect().get_center(), true), "press"):
		return false
	if not completion.step(await press(join.get_global_rect().get_center(), false), "press"):
		return false
	if not completion.require(actions(ui).is_empty(), label + " duplicate pending request suppressed"):
		return false
	ui.free()
	if not completion.step(await settle(), "settle"):
		return false
	if not completion.case_done():
		return false
	completed_cases += 1

	return true

func run_bonus() -> bool:
	root.content_scale_size = Vector2i.ZERO
	root.notify_mouse_entered()
	for forever in [true]:
		for size in [Vector2i(1280, 720), Vector2i(1920, 1080)]:
			if not completion.step(await probe(forever, size), "probe"):
				return false
	if receipts.size() != 32 or completed_cases != 4:
		push_error("Random Bonus fixture incomplete: expected32 physical actions, got" + str(receipts.size()))
		return completion.fail("Random Bonus incomplete: expected32 actions and4 cases")
	var path := OS.get_environment("PVP_RANDOM_BONUS_RECEIPT")
	if not completion.require(not path.is_empty()):
		return false
	var output := FileAccess.open(path, FileAccess.WRITE)
	if not completion.require(output != null):
		return false
	output.store_string(JSON.stringify(receipts, "\t"))
	output.close()
	print("PASS Random32 Bonus native input cases=", receipts.size(), " both skins/sizes; no UDP claim")
	return true

func run_completion() -> void:
	completion.finish(await run_bonus())
