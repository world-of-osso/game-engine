extends "res://tests/housing_mouse_expert_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Active submode proof requires explicit private5870 paid owner configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	if not await wait_housing(func(): return placed_node() != null and client.housing_state().decor.size() == 1, "paid owner decor ready"): return
	var before: Dictionary = client.housing_state()
	if not before.can_edit or int(before.decor[0].decor_id) != 1693:
		fail("Active submode proof needs the real editable paid bookcase")
		return
	var original: Dictionary = precision_receipt()
	if not await configure_expert_bindings(false): return
	if not await enter_expert(): return
	for spec in [["Translate", "TranslateSubmodeButton", KEY_F8], ["Rotate", "RotateSubmodeButton", KEY_F9], ["Scale", "ScaleSubmodeButton", KEY_F10]]:
		if not await prove_active_submode_drag(spec): return
		if not matches_precision(original) or client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
			fail("Active submode input changed paid state or failed to restore the saved render")
			return
	if not await configure_expert_bindings(true): return
	print("PASS: private5870 active Expert submode keys preserve live mouse manipulation ", housing_skin)
	client.free()
	quit(0)

func prove_active_submode_drag(spec: Array) -> bool:
	var mode: String = spec[0]
	if not await click_exposed_control(housing_control(spec[1])): return false
	if not await wait_housing(func(): return client.housing_state().precision_submode == mode, "actual " + mode + " submode"): return false
	if not await select_saved_decor(): return false
	var drag: Dictionary = await active_submode_drag_input(mode)
	if drag.is_empty(): return false
	var point: Vector2 = drag.point
	var delta: Vector2 = drag.delta
	mouse_precision_motion(point)
	mouse_precision_button(point, true)
	if not await wait_housing(func(): return client.housing_state().manipulating, "actual held " + mode + " manipulation"): return false
	if delta != Vector2.ZERO:
		mouse_precision_motion(point + delta)
		await frames(3)
	if not await record_editor("active-submode-" + mode.to_lower() + "-before-key"): return false
	var staged: Dictionary = client.housing_state().draft.duplicate(true)
	var pose: Transform3D = placed_node().transform
	await source_key(spec[2])
	if not await record_editor("active-submode-" + mode.to_lower() + "-after-key"): return false
	if not client.housing_state().manipulating:
		fail("Assigned active " + mode + " key ended the live mouse manipulation before release")
		return false
	if mode != "Rotate" and (client.housing_state().draft != staged or not placed_node().transform.is_equal_approx(pose)):
		fail("Assigned active " + mode + " key changed the existing drag anchor or staged pose")
		return false
	var continued_from: Transform3D = placed_node().transform
	if delta != Vector2.ZERO:
		mouse_precision_motion(point + delta * 2.0)
	if not await wait_housing(func(): return client.housing_state().manipulating and not placed_node().transform.is_equal_approx(continued_from), "continued " + mode + " motion after active key"): return false
	mouse_precision_button(point + Vector2(250.0, 0.0), false)
	if not await wait_housing(func(): return not client.housing_state().manipulating, "global " + mode + " mouse release"): return false
	var released: Transform3D = placed_node().transform
	await frames(5)
	if not placed_node().transform.is_equal_approx(released):
		fail(mode + " mouse manipulation continued after release")
		return false
	await source_key(KEY_ESCAPE)
	if not await wait_housing(func(): return int(client.housing_state().selected) == 0 and not client.housing_state().manipulating, "active submode Escape cancels local draft"): return false
	return await record_editor("active-submode-" + mode.to_lower() + "-cancelled")

func active_submode_drag_input(mode: String) -> Dictionary:
	if mode == "Translate":
		var node: Node3D = placed_node()
		var hit: Dictionary = find_visible_cornerstone_triangle(node, int(node.get_meta("housing_instance_id")))
		if hit.is_empty():
			fail("Active Translate proof needs genuine visible selected-decor geometry")
			return {}
		var axis: Vector2 = camera().unproject_position(node.position + Vector3.RIGHT) - camera().unproject_position(node.position)
		return {"point": hit.point, "delta": axis * 0.3}
	var name: String = "HouseEditorRotateControlRight" if mode == "Rotate" else "HouseEditorScaleThumb"
	if not await wait_housing(func(): return housing_control(name) != null, "source " + mode + " manipulation control"): return {}
	var control: Control = housing_control(name)
	var delta := Vector2.ZERO if mode == "Rotate" else Vector2(60.0, 0.0)
	return {"point": control.get_global_rect().get_center(), "delta": delta}
