extends "res://tests/housing_expert_source_live.gd"

var lifetime_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Menu lifetime needs explicit private5870 paid owner configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if not before.can_edit or before.decor.size() != 1 or housing_count(2330, "stored") != 1:
		fail("Menu lifetime needs the genuine paid bookcase/Tesla")
		return
	if not await configure_expert_bindings(false): return
	if not await open_filter(): return
	push_mouse_motion(housing_control("HouseEditorStorageSort").get_global_rect().get_center())
	if not await wait_housing(func(): return housing_control("HouseEditorStorageSortMenu") != null, "source delayed hover opens Sort without click", 4000): return
	if not await record_lifetime("hover-open"): return
	push_mouse_motion(housing_control("HouseEditorStorageDyeable").get_global_rect().get_center())
	if not await wait_housing(func(): return housing_control("HouseEditorStorageSortMenu") == null and menu_open(), "other root row collapses Sort without closing root", 4000): return
	await click_control(housing_control("HouseEditorStorageDyeable"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return menu_open() and housing_control("HouseEditorStorageDyeableTick") != null, "checkbox Refresh retains root"): return
	if not await record_lifetime("checkbox-refresh"): return
	await click_control(housing_control("HouseEditorStorageFiltersResetButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return not menu_open(), "reset press is outside menu hierarchy"): return
	if not await open_filter(): return
	if not await hover_sort(): return
	await click_control(housing_control("HouseEditorStorageSortAlphabetical"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return not menu_open() and housing_control("HouseEditorStorageSortMenu") == null, "radio default response closes all menus"): return
	if not await record_lifetime("radio-dismiss"): return
	# Leaving alone retains the root; auto-dismiss comes from outside presses/Escape.
	if not await open_filter(): return
	push_mouse_motion(Vector2(1100.0, 400.0))
	await create_timer(0.5).timeout
	if not menu_open():
		fail("Pointer leave incorrectly dismissed retained source root")
		return
	for button in [MOUSE_BUTTON_LEFT, MOUSE_BUTTON_RIGHT]:
		await click(Vector2(1100.0, 400.0), button)
		if not await wait_housing(func(): return not menu_open(), "outside press dismisses root"): return
		if client.housing_state().get("selected", 0) != 0 or client.housing_state().get("placing", 0) != 0:
			fail("Empty-ground dismissal unexpectedly started world editing")
			return
		if not await record_lifetime("outside-" + str(button)): return
		if not await open_filter(): return
	if not await hover_sort(): return
	await press_escape()
	if not await wait_housing(func(): return not menu_open() and housing_control("HouseEditorStorageFrame") != null and client.get_node_or_null("GameMenuUI") == null, "Escape dismisses menu before editor/menu handling"): return
	if not await record_lifetime("escape-dismiss"): return
	if not await open_filter(): return
	await source_key(KEY_F2)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") == null, "mode switch hides popup owner"): return
	await source_key(KEY_F1)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null and not menu_open(), "returning owner cannot revive stale popup"): return
	if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
		fail("Menu lifetime changed confirmed paid layout/storage")
		return
	if not await record_lifetime("owner-return"): return
	if not await configure_expert_bindings(true): return
	print("PASS: private5870 source storage menu lifetime ", housing_skin)
	client.free()
	quit(0)

func menu_open() -> bool:
	return housing_control("HouseEditorStorageFilterMenu") != null

func open_filter() -> bool:
	await click_control(housing_control("HouseEditorStorageFilters"), MOUSE_BUTTON_LEFT)
	return await wait_housing(func(): return menu_open() and housing_control("HouseEditorStorageSortMenu") == null, "fresh root opens without stale Sort")

func hover_sort() -> bool:
	push_mouse_motion(housing_control("HouseEditorStorageSort").get_global_rect().get_center())
	return await wait_housing(func(): return housing_control("HouseEditorStorageSortMenu") != null, "hover opens source Sort", 4000)

func record_lifetime(phase: String) -> bool:
	var state: Dictionary = client.housing_state()
	lifetime_receipts.append({"phase": phase, "skin": housing_skin, "root_open": menu_open(), "sort_open": housing_control("HouseEditorStorageSortMenu") != null, "decor": state.decor, "stored": state.stored})
	var output := FileAccess.open(shots.path_join("menu-lifetime-receipts.json"), FileAccess.WRITE)
	if output == null:
		fail("Cannot save menu lifetime receipts")
		return false
	output.store_string(JSON.stringify(lifetime_receipts, "\t"))
	output.flush()
	var error := output.get_error()
	output.close()
	if error != OK:
		fail("Cannot persist menu lifetime receipts")
		return false
	await RenderingServer.frame_post_draw
	if root.get_texture().get_image().save_png(shots.path_join(housing_skin.to_lower() + "-menu-lifetime-" + phase + ".png")) != OK:
		fail("Cannot save menu lifetime screenshot")
		return false
	return true
