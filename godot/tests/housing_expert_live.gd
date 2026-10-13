extends "res://tests/housing_expert_source_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Expert proof requires private5870 owned-house configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_editor_home(): return
	if not await wait_housing(func(): return placed_node() != null and client.housing_state().decor.size() == 1, "paid bookcase visual"): return
	if int(client.housing_state().decor[0].decor_id) != 1693 or stored_bookcases() != 0:
		fail("Expert needs the genuine conserved placed bookcase")
		return
	if OS.get_environment("HOUSING_RELOG_ONLY") == "1":
		var file: FileAccess = FileAccess.open(OS.get_environment("HOUSING_EXPECTED"), FileAccess.READ)
		if file == null:
			fail("Missing prior committed expert receipt")
			return
		var saved: Dictionary = JSON.parse_string(file.get_as_text())
		if not await wait_housing(func(): return matches_precision(saved), "durable precision relog"): return
		if not await capture_housing("expert-relogged"): return
		print("PASS: private5870 expert full transform retained on fresh-process relog ", housing_skin)
		client.free()
		quit(0)
		return
	if not await configure_expert_bindings(false): return
	await click_control(housing_control("HouseEditorExpertDecorModeButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("TranslateSubmodeButton") != null and housing_control("HouseEditorStorageFrame") == null, "Retail expert submodes hide storage"): return
	if not await select_saved_decor(): return
	var original: Dictionary = precision_receipt()
	if not await stage_precision(): return
	if not await capture_housing("expert-staged"): return
	if not matches_precision(original, false):
		fail("Staging changed authoritative transform before commit")
		return
	await tap(KEY_ESCAPE)
	if not await wait_housing(func(): return matches_precision(original), "cancel restores confirmed render"): return
	if housing_control("TranslateSubmodeButton") == null:
		fail("Expert cancel must retain the editor; Escape leaked to global window close")
		return
	if not await capture_housing("expert-cancelled"): return
	if not await select_saved_decor(): return
	if not await stage_precision(): return
	var node: Node3D = placed_node()
	var expected: Dictionary = {"position": [node.position.x, node.position.y, node.position.z], "pitch": node.rotation.x, "yaw": node.rotation.y, "roll": node.rotation.z, "scale": node.scale.x}
	if not await commit_expert_by_world_click(): return
	if not await wait_housing(func(): return matches_precision(expected) and not client.housing_state().pending, "authoritative expert commit"): return
	if housing_control("TranslateSubmodeButton") == null:
		fail("Expert commit must retain its controls")
		return
	if stored_bookcases() != 0 or client.housing_state().decor.size() != 1:
		fail("Expert changed owned quantity")
		return
	var output: FileAccess = FileAccess.open(shots.path_join("expert-expected.json"), FileAccess.WRITE)
	if output == null:
		fail("Cannot write expert receipt")
		return
	output.store_string(JSON.stringify(expected, "\t"))
	output.close()
	if not await capture_housing("expert-committed"): return
	if not await configure_expert_bindings(true): return
	print("PASS: private5870 native staged XYZ axes/translation/scale, cancel, atomic commit and conserved paid decor ", housing_skin)
	client.free()
	quit(0)

func stage_precision() -> bool:
	return await stage_assigned_precision()

func precision_receipt() -> Dictionary:
	var decor: Dictionary = client.housing_state().decor[0]
	return {"position": [decor.position.x, decor.position.y, decor.position.z], "pitch": decor.pitch, "yaw": decor.yaw, "roll": decor.roll, "scale": decor.scale}

func matches_precision(expected: Dictionary, include_render: bool = true) -> bool:
	var state: Dictionary = client.housing_state()
	if state.get("decor", []).size() != 1: return false
	var decor: Dictionary = state.decor[0]
	var point: Vector3 = Vector3(expected.position[0], expected.position[1], expected.position[2])
	if decor.position.distance_to(point) > 0.01: return false
	for key in ["pitch", "yaw", "roll", "scale"]:
		if absf(float(decor[key]) - float(expected[key])) > 0.01: return false
	if not include_render: return true
	var node: Node3D = placed_node()
	return node != null and node.position.distance_to(point) < 0.01 and absf(node.rotation.x - float(expected.pitch)) < 0.01 and absf(node.rotation.y - float(expected.yaw)) < 0.01 and absf(node.rotation.z - float(expected.roll)) < 0.01 and absf(node.scale.x - float(expected.scale)) < 0.01
