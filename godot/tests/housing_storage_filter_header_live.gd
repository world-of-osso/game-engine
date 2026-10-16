extends "res://tests/housing_storage_tiles_live.gd"

var filter_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Storage Filter header requires explicit private5870 paid owner configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if not before.can_edit or before.decor.size() != 1 or int(before.decor[0].decor_id) != 1693 or housing_count(2330, "stored") != 1:
		fail("Filter header requires genuine paid bookcase and stored Tesla")
		return
	if not await wait_housing(func(): return tile_atlas("HouseEditorStorageFiltersBackground") != null, "source painted WowStyle1 Filter background", 3000): return
	var control := housing_control("HouseEditorStorageFilters")
	var storage := housing_control("HouseEditorStorageFrame")
	var search := housing_control("HouseEditorStorageSearch")
	var scale: float = storage.get_global_rect().size.x / 600.0
	var bounds: Rect2 = control.get_global_rect()
	if bounds.size.distance_to(Vector2(125.0, 20.0) * scale) > 0.1 or absf(bounds.position.x - search.get_global_rect().end.x - 10.0 * scale) > 0.1 or absf(storage.get_global_rect().end.x - bounds.end.x - 25.0 * scale) > 0.1 or absf(bounds.get_center().y - search.get_global_rect().get_center().y) > 0.1:
		fail("Filter header lost source fixed125x20 SearchBox+10/right25 anchors")
		return
	var label := housing_control("HouseEditorStorageFiltersText") as Label
	if label == null or label.text != "Filter":
		fail("Source Filter label is missing")
		return
	var reset := housing_control("HouseEditorStorageFiltersResetButton")
	if reset == null or reset.is_visible_in_tree():
		fail("Source reset must exist hidden at default filters")
		return
	push_mouse_motion(Vector2(1100.0, 400.0))
	await frames(3)
	var closed_region: Rect2 = tile_atlas("HouseEditorStorageFiltersBackground").region
	if not await record_filter_header("closed"): return
	await click_control(control, MOUSE_BUTTON_LEFT)
	push_mouse_motion(Vector2(1100.0, 400.0))
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFilterMenu") != null and tile_atlas("HouseEditorStorageFiltersBackground").region != closed_region, "physical Filter opens menu and source open-state art"): return
	if not await record_filter_header("open"): return
	await click_control(housing_control("HouseEditorStorageDyeable"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageDyeableTick") != null and housing_control("HouseEditorStorageFiltersResetButton").is_visible_in_tree(), "normal Dyeable filter shows source reset"): return
	reset = housing_control("HouseEditorStorageFiltersResetButton")
	var reset_rect: Rect2 = reset.get_global_rect()
	bounds = housing_control("HouseEditorStorageFilters").get_global_rect()
	if reset_rect.size.distance_to(Vector2(23.0, 23.0) * scale) > 0.1 or reset_rect.get_center().distance_to(Vector2(bounds.end.x - 3.0 * scale, bounds.position.y)) > 0.1:
		fail("Source UIResetButton is not23x23 TOPRIGHT-3")
		return
	if not await record_filter_header("filtered"): return
	await click_control(reset, MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return not housing_control("HouseEditorStorageFiltersResetButton").is_visible_in_tree() and housing_control("HouseEditorStored2330") != null, "physical shared reset restores paid Tesla tile"): return
	if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
		fail("Filter/reset header changed confirmed paid layout or storage")
		return
	await click_control(housing_control("HouseEditorStorageFilters"), MOUSE_BUTTON_LEFT)
	push_mouse_motion(Vector2(1100.0, 400.0))
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFilterMenu") == null and tile_atlas("HouseEditorStorageFiltersBackground").region == closed_region, "physical Filter closes with original source art"): return
	if not await record_filter_header("reset-closed"): return
	print("PASS: private5870 source storage Filter Reset header ", housing_skin)
	client.free()
	quit(0)

func record_filter_header(phase: String) -> bool:
	var control := housing_control("HouseEditorStorageFilters")
	var background := housing_control("HouseEditorStorageFiltersBackground")
	var atlas := tile_atlas("HouseEditorStorageFiltersBackground")
	var reset := housing_control("HouseEditorStorageFiltersResetButton")
	filter_receipts.append({"phase": phase, "skin": housing_skin, "filter_rect": control.get_global_rect(), "background_rect": background.get_global_rect(), "atlas_region": atlas.region, "image_size": atlas.get_image().get_size(), "reset_visible": reset.is_visible_in_tree(), "reset_rect": reset.get_global_rect(), "housing": client.housing_state()})
	var output := FileAccess.open(shots.path_join("filter-header-receipts.json"), FileAccess.WRITE)
	if output == null:
		fail("Cannot save native Filter header receipts")
		return false
	output.store_string(JSON.stringify(filter_receipts, "\t"))
	await RenderingServer.frame_post_draw
	if root.get_texture().get_image().save_png(shots.path_join(housing_skin.to_lower() + "-filter-header-" + phase + ".png")) != OK:
		fail("Cannot save native Filter header pixels")
		return false
	return true
