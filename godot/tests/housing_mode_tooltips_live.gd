extends "res://tests/housing_expert_source_live.gd"

const TOOLTIP_CONTROLS: Array = [
	["HouseEditorBasicDecorModeButton", "Decorate Mode", "F1"],
	["HouseEditorExpertDecorModeButton", "Advanced Mode", "F2"],
	["HouseEditorCleanupModeButton", "Cleanup Mode", "F3"],
	["TranslateSubmodeButton", "Move", "F8"],
	["RotateSubmodeButton", "Rotate", "F9"],
	["ScaleSubmodeButton", "Scale", "F10"],
]
var mode_tooltip_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Mode tooltip proof requires private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if not await configure_expert_bindings(true): return
	if not await hover_editor_mode("HouseEditorBasicDecorModeButton", "Decorate Mode"): return
	if not await enter_expert(): return
	if not await prove_editor_tooltips(false): return
	if not await configure_expert_bindings(false): return
	if not await enter_expert(): return
	if not await prove_editor_tooltips(true): return
	if not await configure_expert_bindings(true): return
	if not await enter_expert(): return
	if not await prove_editor_tooltips(false): return
	push_mouse_motion(Vector2(1200.0, 100.0))
	if not await wait_housing(func(): return not client.tooltip_state().get("visible", false), "mode tooltip leave clears"): return
	if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
		fail("Mode tooltip/binding browsing changed confirmed authority")
		return
	print("PASS: private5870 source mode tooltips ", housing_skin)
	client.free()
	quit(0)

func prove_editor_tooltips(assigned: bool) -> bool:
	for entry in TOOLTIP_CONTROLS:
		var expected: String = entry[1] + (" (" + entry[2] + ")" if assigned else "")
		if not await hover_editor_mode(entry[0], expected): return false
		if entry[0] in ["HouseEditorExpertDecorModeButton", "TranslateSubmodeButton"]:
			if not await record_mode_tooltip(entry[0] + ("-assigned" if assigned else "-unbound")): return false
	return true

func hover_editor_mode(name: String, expected: String) -> bool:
	var control: Control = housing_control(name)
	if control == null:
		fail("Mode tooltip control absent: " + name)
		return false
	push_mouse_motion(control.get_global_rect().get_center())
	if not await wait_housing(func(): return client.tooltip_state().get("visible", false) and client.tooltip_state().get("title", "") == expected, "source mode tooltip " + name, 15000): return false
	var state: Dictionary = client.tooltip_state()
	var scale: float = housing_control("HouseEditorExpertDecorModeButton").get_global_rect().size.x / 64.0
	var rect: Rect2 = control.get_global_rect()
	if absf(float(state.rect[0]) + float(state.rect[2]) / 2.0 - rect.get_center().x / scale) > 1.5 or absf(float(state.rect[1]) + float(state.rect[3]) - rect.position.y / scale) > 1.5:
		fail("Mode tooltip not source ANCHOR_TOP: " + str(state))
		return false
	return true

func record_mode_tooltip(phase: String) -> bool:
	await frames(5)
	if not await capture_housing("mode-tooltip-" + phase): return false
	mode_tooltip_receipts.append({"phase": phase, "skin": housing_skin, "tooltip": client.tooltip_state(), "housing": client.housing_state()})
	var file := FileAccess.open(shots.path_join("mode-tooltip-receipts.json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot write source mode tooltip receipts")
		return false
	file.store_string(JSON.stringify(mode_tooltip_receipts, "\t"))
	file.close()
	return true
