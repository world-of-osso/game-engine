extends "res://tests/housing_storage_filter_header_live.gd"

var popup_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Source storage popup needs explicit private5870 paid owner configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if not before.can_edit or before.decor.size() != 1 or int(before.decor[0].decor_id) != 1693 or housing_count(2330, "stored") != 1:
		fail("Source storage popup requires genuine paid bookcase/Tesla state")
		return
	await click_control(housing_control("HouseEditorStorageFilters"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFilterMenu") != null, "physical source storage popup"): return
	var scale: float = housing_control("HouseEditorBasicDecorModeButton").get_global_rect().size.x / 64.0
	var owner: Rect2 = housing_control("HouseEditorStorageFilters").get_global_rect()
	var menu: Rect2 = housing_control("HouseEditorStorageFilterMenu").get_global_rect()
	if menu.position.distance_to(Vector2(owner.position.x + 6.0 * scale, owner.end.y - 2.0 * scale)) > 0.1 or menu.size.distance_to(Vector2(125.0, 109.0) * scale) > 0.1:
		fail("Storage popup lost source Filter+6/2 content-fit125x109 geometry: " + str({"owner": owner, "menu": menu, "scale": scale}))
		return
	if housing_text("HouseEditorStorageSortText") != "Sort By" or housing_control("HouseEditorStorageSortSubmenuArrow") == null or housing_control("HouseEditorStorageFilterDivider") == null:
		fail("Storage root lost source Sort By arrow/divider composition")
		return
	var placeable := housing_control("HouseEditorStorageLocationUnavailable") as BaseButton
	if placeable == null or not placeable.disabled or housing_text("HouseEditorStorageLocationUnavailableText") != "Placeable":
		fail("Unsupported source Placeable must remain disabled, not a fabricated filter")
		return
	if not await record_popup("root"): return
	await click_control(housing_control("HouseEditorStorageSort"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageSortMenu") != null, "actual Sort By submenu"): return
	var sort_owner: Rect2 = housing_control("HouseEditorStorageSort").get_global_rect()
	var sort: Rect2 = housing_control("HouseEditorStorageSortMenu").get_global_rect()
	if sort.position.distance_to(Vector2(sort_owner.end.x, sort_owner.position.y)) > 0.1 or absf(sort.size.y - 63.0 * scale) > 0.1:
		fail("Sort popup lost source owner TOPRIGHT and two20px radio bounds")
		return
	var date := housing_control("HouseEditorStorageSortDateAdded") as BaseButton
	if date == null or not date.disabled or housing_text("HouseEditorStorageSortDateAddedText") != "Date Added":
		fail("Unauthenticated Date Added must stay a disabled source radio")
		return
	await click_control(housing_control("HouseEditorStorageSortAlphabetical"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageSortAlphabeticalTick") != null, "real Alphabetical checked radio"): return
	if not await record_popup("sort"): return
	await click_control(housing_control("HouseEditorStorageDyeable"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageDyeableTick") != null and housing_control("HouseEditorStorageFiltersResetButton").is_visible_in_tree(), "source Dyeable checkbox and reset"): return
	if not await record_popup("dyeable"): return
	await click_control(housing_control("HouseEditorStorageFiltersResetButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorStored2330") != null, "reset restores actual stored Tesla"): return
	if housing_control("HouseEditorStorageDyeableTick") != null or housing_control("HouseEditorStorageSortAlphabeticalTick") == null:
		fail("Reset changed independent sort policy or retained Dyeable filter")
		return
	if client.housing_state().decor != before.decor or client.housing_state().stored != before.stored:
		fail("Popup actions changed confirmed paid decor/storage")
		return
	if not await record_popup("reset"): return
	print("PASS: private5870 source storage filter popup composition ", housing_skin)
	client.free()
	quit(0)

func record_popup(phase: String) -> bool:
	var state: Dictionary = client.housing_state()
	# Keep only relevant paid-state evidence, not unrelated plot-list copies.
	popup_receipts.append({"phase": phase, "skin": housing_skin, "menu_rect": housing_control("HouseEditorStorageFilterMenu").get_global_rect(), "sort_rect": housing_control("HouseEditorStorageSortMenu").get_global_rect() if housing_control("HouseEditorStorageSortMenu") != null else Rect2(), "decor": state.decor, "stored": state.stored, "sort_label": housing_text("HouseEditorStorageSortText"), "dyeable": housing_control("HouseEditorStorageDyeableTick") != null})
	var output := FileAccess.open(shots.path_join("filter-popup-receipts.json"), FileAccess.WRITE)
	if output == null:
		fail("Cannot save storage popup receipts")
		return false
	output.store_string(JSON.stringify(popup_receipts, "\t"))
	output.flush()
	var receipt_error := output.get_error()
	output.close()
	if receipt_error != OK:
		fail("Cannot persist storage popup receipt: " + str(receipt_error))
		return false
	push_mouse_motion(Vector2(1100.0, 400.0))
	await RenderingServer.frame_post_draw
	if root.get_texture().get_image().save_png(shots.path_join(housing_skin.to_lower() + "-filter-popup-" + phase + ".png")) != OK:
		fail("Cannot save storage popup pixels")
		return false
	return true
