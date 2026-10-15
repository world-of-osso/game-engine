extends "res://tests/housing_expert_source_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Active mode proof requires explicit private5870/fb_housing_* owner")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if before.decor.size() != 1 or int(before.decor[0].decor_id) != 1693 or housing_count(2330, "stored") != 1:
		fail("Active mode proof requires retained paid bookcase and stored Tesla")
		return
	if not await configure_expert_bindings(false): return
	if not await prove_basic_active_modes(): return
	if not await prove_expert_active_modes(): return
	if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
		fail("Active mode activation changed confirmed paid state")
		return
	if not await configure_expert_bindings(true): return
	if not await record_editor("active-expert-cancelled"): return
	print("PASS: private5870 source active mode preserves selection/draft ", housing_skin)
	client.free()
	quit(0)

func prove_basic_active_modes() -> bool:
	if not await select_saved_decor(): return false
	var selected: int = int(client.housing_state().selected)
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return false
	await frames(8)
	if not prove_active_selection(selected, "Basic pointer"): return false
	await source_key(KEY_F1)
	await frames(8)
	if not prove_active_selection(selected, "Basic assigned key"): return false
	if not await record_editor("active-basic-selection-retained"): return false
	await press_escape()
	if not await wait_housing(func(): return housing_control("HouseEditorStored2330") != null, "paid Tesla storage after selection cancel"): return false
	if not await click_exposed_control(housing_control("HouseEditorStored2330")): return false
	push_mouse_motion(ground_point(8.0))
	if not await wait_housing(func(): return int(client.housing_state().get("placing", 0)) == 2330 and client.get_node_or_null("HousingDecorPreview") != null, "actual owned Tesla preview"): return false
	var preview_id: int = client.get_node("HousingDecorPreview").get_instance_id()
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return false
	await frames(8)
	if not prove_active_preview(preview_id, "Basic pointer"): return false
	await source_key(KEY_F1)
	await frames(8)
	if not prove_active_preview(preview_id, "Basic assigned key"): return false
	if not await record_editor("active-basic-owned-preview-retained"): return false
	await press_escape()
	return await wait_housing(func(): return int(client.housing_state().get("placing", 0)) == 0 and client.get_node_or_null("HousingDecorPreview") == null and housing_control("HouseEditorStored2330") != null, "active Basic preview Escape preserves storage")

func prove_expert_active_modes() -> bool:
	if not await enter_expert(): return false
	if not await select_saved_decor(): return false
	var selected: int = int(client.housing_state().selected)
	var saved_render: Transform3D = placed_node().global_transform
	if not await hold_assigned(KEY_F7): return false
	var draft: Dictionary = client.housing_state().draft.duplicate(true)
	var staged_render: Transform3D = placed_node().global_transform
	if staged_render.origin.distance_to(saved_render.origin) < 0.01:
		fail("Active Expert proof did not stage actual visible translation")
		return false
	if not await click_exposed_control(housing_control("HouseEditorPlacedDecorListButton")): return false
	if not await wait_housing(func(): return housing_control("HouseEditorPlacedDecorList") != null, "source Expert placed list open"): return false
	if not await click_exposed_control(housing_control("HouseEditorExpertDecorModeButton")): return false
	await frames(8)
	if not prove_active_draft(selected, draft, staged_render, "Expert pointer"): return false
	await source_key(KEY_F2)
	await frames(8)
	if not prove_active_draft(selected, draft, staged_render, "Expert assigned key"): return false
	if not await record_editor("active-expert-draft-retained"): return false
	await press_escape()
	return await wait_housing(func(): return int(client.housing_state().get("selected", 0)) == 0 and placed_node().global_transform.origin.distance_to(saved_render.origin) < 0.001, "active mode Escape restores saved render")

func prove_active_selection(selected: int, phase: String) -> bool:
	if int(client.housing_state().get("selected", 0)) != selected or housing_control("HouseEditorStorageFrame") != null:
		fail("Already-active " + phase + " discarded actual decor selection: " + str(client.housing_state()))
		return false
	return true

func prove_active_preview(preview_id: int, phase: String) -> bool:
	var preview: Node = client.get_node_or_null("HousingDecorPreview")
	if int(client.housing_state().get("placing", 0)) != 2330 or preview == null or preview.get_instance_id() != preview_id or housing_control("HouseEditorStorageFrame") != null:
		fail("Already-active " + phase + " discarded actual owned preview: " + str(client.housing_state()))
		return false
	return true

func prove_active_draft(selected: int, draft: Dictionary, staged_render: Transform3D, phase: String) -> bool:
	if int(client.housing_state().get("selected", 0)) != selected or client.housing_state().draft != draft or placed_node().global_transform.origin.distance_to(staged_render.origin) > 0.001 or housing_control("HouseEditorPlacedDecorList") == null:
		fail("Already-active " + phase + " discarded staged decor draft/render/list: " + str(client.housing_state()))
		return false
	return true
