extends "res://tests/housing_cleanup_live.gd"

var press_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Mode press proof requires explicit private5870/fb_housing_* owner")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	for name in ["HouseEditorBasicDecorModeButton", "HouseEditorCleanupModeButton", "HouseEditorExpertDecorModeButton", "TranslateSubmodeButton", "RotateSubmodeButton", "ScaleSubmodeButton"]:
		if not await click_exposed_control(housing_control(name)): return
		if not await prove_control_press(name): return
	if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
		fail("Mode press lifecycle changed confirmed paid state")
		return
	print("PASS: private5870 source mode pressed-hover lifecycle ", housing_skin)
	client.free()
	quit(0)

func prove_control_press(name: String) -> bool:
	var point: Vector2 = housing_control(name).get_global_rect().get_center()
	push_mouse_motion(point)
	if not await wait_housing(func(): return mode_hover_alpha(name) == 0.5, "source mode hover " + name): return false
	if not await record_mode_press(name, "hover"): return false
	push_mode_button(point, true)
	if not await wait_housing(func(): return (housing_control(name) as BaseButton).is_pressed() and mode_hover_alpha(name) == 0.0, "pressed suppresses source hover " + name): return false
	if not await record_mode_press(name, "pressed"): return false
	push_mode_button(point, false)
	if not await wait_housing(func(): return not (housing_control(name) as BaseButton).is_pressed() and mode_hover_alpha(name) == 0.5, "release inside restores source hover " + name): return false
	push_mode_button(point, true)
	if not await wait_housing(func(): return mode_hover_alpha(name) == 0.0, "second source press " + name): return false
	var outside := Vector2(1200.0, 100.0)
	push_mouse_motion(outside)
	push_mode_button(outside, false)
	if not await wait_housing(func(): return not (housing_control(name) as BaseButton).is_pressed() and mode_hover_alpha(name) == 0.0, "outside release leaves hover absent " + name): return false
	return await record_mode_press(name, "outside")

func mode_hover_alpha(name: String) -> float:
	var hover: CanvasItem = housing_control(name + "Hover") as CanvasItem
	return -1.0 if hover == null else hover.modulate.a

func push_mode_button(point: Vector2, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_LEFT
	event.position = point
	event.global_position = point
	event.pressed = pressed
	root.push_input(event, true)

func record_mode_press(name: String, phase: String) -> bool:
	press_receipts.append({"control": name, "phase": phase, "skin": housing_skin, "alpha": mode_hover_alpha(name), "pressed": (housing_control(name) as BaseButton).is_pressed(), "bounds": str(housing_control(name).get_global_rect()), "housing": client.housing_state()})
	var file := FileAccess.open(shots.path_join("mode-press-receipts.json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot write mode pressed-hover receipts")
		return false
	file.store_string(JSON.stringify(press_receipts, "\t"))
	file.close()
	if name in ["HouseEditorBasicDecorModeButton", "TranslateSubmodeButton"]:
		await frames(5)
		return await capture_housing("mode-press-" + name + "-" + phase)
	return true
