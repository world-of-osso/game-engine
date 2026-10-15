extends "res://tests/housing_cleanup_live.gd"

var tooltip_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = OS.get_environment("HOUSING_CLEANUP_ROLE")
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0 or not cleanup_role in ["owner", "visitor"]:
		fail("Cleanup tooltip proof requires explicit private5870 account/house/role")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if before.decor.size() != 1 or int(before.decor[0].decor_id) != 1693:
		fail("Cleanup tooltip requires retained paid bookcase")
		return
	if not await wait_housing(func(): return placed_node() != null, "Cleanup tooltip visual"): return
	if not await enter_cleanup(): return
	var hit: Dictionary = find_visible_cornerstone_triangle(placed_node(), int(before.decor[0].instance_id))
	if hit.is_empty():
		fail("Cleanup tooltip requires real visible pick triangle")
		return
	push_mouse_motion(hit.point)
	if not await wait_housing(func(): return client.tooltip_state().get("visible", false) and client.tooltip_state().get("title", "") == "Small Val'sharah Bookcase", "Cleanup source central tooltip", 15000): return
	var state: Dictionary = client.tooltip_state()
	var scale: float = housing_control("HouseEditorCleanupModeButton").get_global_rect().size.x / 64.0
	var point: Vector2 = hit.point / scale
	if absf(float(state.rect[0]) - point.x) > 1.5 or absf(float(state.rect[1]) - (point.y - 20.0)) > 1.5:
		fail("Cleanup tooltip source ANCHOR_CURSOR drifted: " + str(state))
		return
	if not state.get("lines", []).is_empty() or housing_control("HouseEditorCleanupTooltip") != null:
		fail("Cleanup invented pet/lock/removal metadata or retained bespoke tooltip")
		return
	if not await record_cleanup_tooltip("cleanup-source-tooltip"): return
	push_mouse_motion(housing_control("HouseEditorBasicDecorModeButton").get_global_rect().get_center())
	if not await wait_housing(func(): return not client.tooltip_state().get("visible", false) and highlighted_meshes() == 0, "Cleanup central leave clears"): return
	push_mouse_motion(hit.point)
	if not await wait_housing(func(): return client.tooltip_state().get("visible", false), "Cleanup central rehover"): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null and not client.tooltip_state().get("visible", false), "Cleanup mode exit clears central tooltip"): return
	if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
		fail("Cleanup tooltip inspection mutated paid authority")
		return
	if not await record_cleanup_tooltip("cleanup-source-tooltip-left"): return
	print("PASS: private5870 Cleanup central tooltip ", cleanup_role, " ", housing_skin)
	client.free()
	quit(0)

func record_cleanup_tooltip(phase: String) -> bool:
	await frames(5)
	if not await capture_housing(phase): return false
	tooltip_receipts.append({"phase": phase, "skin": housing_skin, "role": cleanup_role, "tooltip": client.tooltip_state(), "housing": client.housing_state()})
	var file := FileAccess.open(shots.path_join("cleanup-tooltip-receipts.json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot write Cleanup tooltip receipts")
		return false
	file.store_string(JSON.stringify(tooltip_receipts, "\t"))
	file.close()
	return true
