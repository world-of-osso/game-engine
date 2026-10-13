extends "res://tests/housing_editor_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_VISIT_HOUSE"))
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Visitor proof requires explicit private5870/fb_housing_* target house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingDashboardHouseFinderButton") != null and client.housing_state().get("confirmed",false), "authoritative dashboard"): return
	await click_control(housing_control("HousingDashboardHouseFinderButton"), MOUSE_BUTTON_LEFT)
	if not await open_friend_plot_info(editor_house): return
	if not await record_editor("visitor-finder"): return
	await click_control(housing_control("HouseFinderPlotVisitButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.account_state().screen == "InWorld" and housing_named_control("MicroMenuUI", "HousingMicroButton") != null, "visitor admission",420000): return
	await frames(30)
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.housing_state().get("house_id",0) == editor_house and client.housing_state().get("can_edit",true) == false and client.housing_state().get("decor",[]).size() == 1, "read-only other-account layout"): return
	await click_control(housing_control("HousingDecorateButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_text("HouseEditorStorageTitle") == "Visitor — read-only" and placed_node() != null, "native read-only inspect scene"): return
	if not await editor_configure_bindings(false): return
	var before: Dictionary = client.housing_state()
	if int(placed_node().get_meta("model_file_data_id")) != 1096761:
		fail("Visitor did not render the owner's authentic bookcase model")
		return
	if not await select_saved_decor(): return
	await editor_key(KEY_F6)
	await editor_key(KEY_F7)
	await click(ground_point(8.0), MOUSE_BUTTON_LEFT)
	await frames(20)
	var after: Dictionary = client.housing_state()
	if after.can_edit or after.decor != before.decor or after.stored != before.stored:
		fail("Read-only visitor changed decor or storage: " + str(after))
		return
	if not await record_editor("visitor-readonly"): return
	if not await editor_configure_bindings(true): return
	if not await connect_housing_account(): return
	if int(client.account_state().world_map_id) != 2783:
		fail("Visitor relog lost the saved house map")
		return
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.housing_state().get("house_id",0) == editor_house and client.housing_state().get("can_edit",true) == false and client.housing_state().get("decor",[]).size() == 1, "persisted visited identity and read-only layout"): return
	if not await record_editor("visitor-relogged"): return
	print("PASS: private5870 other-account house visit, actual decor snapshot, read-only physical controls and persisted visitor identity ", housing_skin)
	client.free()
	quit(0)
