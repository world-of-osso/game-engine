extends "res://tests/housing_expert_live.gd"

# No synthetic items, 3D handles or snap increments. Actual selected M2 and Retail
# rotate arrows/scale thumb receive normal mouse events through the native viewport.
func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Mouse Expert proof requires private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_editor_home(): return
	if not await wait_housing(func(): return placed_node() != null and client.housing_state().decor.size() == 1, "real paid decor"): return
	if int(client.housing_state().decor[0].decor_id) != 1693 or stored_bookcases() != 0:
		fail("Mouse proof needs exactly the existing paid placed bookcase")
		return
	if not await configure_expert_bindings(false): return
	await click_control(housing_control("HouseEditorExpertDecorModeButton"), MOUSE_BUTTON_LEFT)
	if not await select_saved_decor(): return
	var instructions: Control = housing_control("HouseEditorInstructions")
	if instructions == null or absf(instructions.get_global_rect().get_center().y - root.size.y / 2.0) > 2.0:
		fail("Expert instructions must use the Retail right-centre anchor, not overlap the minimap")
		return
	var original: Dictionary = precision_receipt()
	if not await mouse_stage_all_axes(): return
	if not matches_precision(original, false):
		fail("Mouse preview modified the authoritative layout")
		return
	if not await capture_housing("mouse-staged"): return
	await tap(KEY_ESCAPE)
	if not await wait_housing(func(): return matches_precision(original) and not client.housing_state().manipulating, "mouse cancel restores saved render"): return
	if housing_control("TranslateSubmodeButton") == null:
		fail("Mouse cancel closed its source controls")
		return
	if not await capture_housing("mouse-cancelled"): return
	if not await select_saved_decor(): return
	if not await mouse_stage_all_axes(): return
	var node: Node3D = placed_node()
	var expected: Dictionary = {"position": [node.position.x, node.position.y, node.position.z], "pitch": node.rotation.x, "yaw": node.rotation.y, "roll": node.rotation.z, "scale": node.scale.x}
	if not await commit_expert_by_world_click(): return
	if not await wait_housing(func(): return matches_precision(expected) and not client.housing_state().pending, "mouse transform atomic commit"): return
	if stored_bookcases() != 0 or client.housing_state().decor.size() != 1:
		fail("Mouse editing changed the quantity of the paid decor")
		return
	if not await capture_housing("mouse-committed"): return
	if not await connect_housing_account(): return
	if not await open_editor_home(): return
	if not await wait_housing(func(): return matches_precision(expected), "mouse committed transform survives real reauthentication"): return
	if not await capture_housing("mouse-relogged"): return
	var receipt: FileAccess = FileAccess.open(shots.path_join("mouse-expected.json"), FileAccess.WRITE)
	if receipt == null:
		fail("Cannot write mouse proof receipt")
		return
	receipt.store_string(JSON.stringify(expected, "\t"))
	receipt.close()
	if not await configure_expert_bindings(true): return
	print("PASS: private5870 unsnapped mouse XYZ translation, source rotation arrows/scale thumb, release capture, staged cancel/commit, conservation and real relog ", housing_skin)
	client.free()
	quit(0)

func mouse_stage_all_axes() -> bool:
	await click_control(housing_control("TranslateSubmodeButton"), MOUSE_BUTTON_LEFT)
	for axis in [Vector3.RIGHT, Vector3.UP, Vector3.BACK]:
		if not await drag_selected_axis(axis): return false
		await source_key(KEY_F11)
	if not await capture_housing("mouse-translate"): return false
	await click_control(housing_control("RotateSubmodeButton"), MOUSE_BUTTON_LEFT)
	for rotation in ["pitch", "yaw", "roll"]:
		if not await hold_mouse_rotation(rotation): return false
		await source_key(KEY_F11)
	if not await capture_housing("mouse-rotation"): return false
	await click_control(housing_control("ScaleSubmodeButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorScaleThumb") != null, "sourced scale thumb"): return false
	var thumb: Control = housing_control("HouseEditorScaleThumb")
	var point: Vector2 = thumb.global_position + thumb.size / 2.0
	var before: float = placed_node().scale.x
	mouse_precision_motion(point)
	mouse_precision_button(point, true)
	await frames(3)
	if not client.housing_state().manipulating:
		fail("Scale thumb did not begin a native mouse manipulation")
		return false
	mouse_precision_motion(point + Vector2(60.0, 0.0))
	await frames(5)
	mouse_precision_button(point + Vector2(250.0, 0.0), false)
	if not await wait_housing(func(): return not client.housing_state().manipulating and placed_node().scale.x > before + 0.1, "scale drag ends outside the widget"): return false
	var released: float = placed_node().scale.x
	await frames(30)
	if absf(placed_node().scale.x - released) > 0.001:
		fail("Mouse scale continued after release outside its widget")
		return false
	return true

func drag_selected_axis(axis: Vector3) -> bool:
	var node: Node3D = placed_node()
	var before: Vector3 = node.position
	var hit: Dictionary = find_visible_cornerstone_triangle(node, int(node.get_meta("housing_instance_id")))
	if hit.is_empty():
		fail("No actual selected-decor mesh for mouse translation")
		return false
	var delta: Vector2 = camera().unproject_position(before + axis) - camera().unproject_position(before)
	var point: Vector2 = hit.point
	mouse_precision_motion(point)
	mouse_precision_button(point, true)
	await frames(3)
	if not client.housing_state().manipulating:
		fail("Selected decor mouse press did not begin translation")
		return false
	mouse_precision_motion(point + delta * 0.3)
	await frames(5)
	mouse_precision_button(point + delta * 0.3, false)
	return await wait_housing(func(): return not client.housing_state().manipulating and placed_node().position.distance_to(before) > 0.1, "selected-axis mouse translation")

func hold_mouse_rotation(rotation: String) -> bool:
	if not await wait_housing(func(): return housing_control("HouseEditorRotateControlRight") != null, "sourced rotation arrow"): return false
	var arrow: Control = housing_control("HouseEditorRotateControlRight")
	var point: Vector2 = arrow.global_position + arrow.size / 2.0
	var before: float = float(client.housing_state().draft[rotation])
	mouse_precision_motion(point)
	mouse_precision_button(point, true)
	await create_timer(0.3).timeout
	mouse_precision_button(point + Vector2(250.0, 0.0), false)
	if not await wait_housing(func(): return not client.housing_state().manipulating and float(client.housing_state().draft[rotation]) > before + 0.02, "held source rotation arrow"): return false
	var released: float = float(client.housing_state().draft[rotation])
	await frames(30)
	if absf(float(client.housing_state().draft[rotation]) - released) > 0.001:
		fail("Mouse rotation continued after release outside its widget")
		return false
	return true

func mouse_precision_button(point: Vector2, pressed: bool) -> void:
	var event: InputEventMouseButton = InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = pressed
	Input.parse_input_event(event)

func mouse_precision_motion(point: Vector2) -> void:
	var event: InputEventMouseMotion = InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	Input.parse_input_event(event)
