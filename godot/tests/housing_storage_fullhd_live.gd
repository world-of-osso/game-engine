extends "res://tests/housing_storage_filter_header_live.gd"

var fullhd_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Full-HD storage capture requires explicit private5870 paid owner configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if not before.can_edit or before.decor.size() != 1 or int(before.decor[0].decor_id) != 1693 or housing_count(2330, "stored") != 1:
		fail("Full-HD capture requires genuine paid bookcase/Tesla state")
		return
	if not await capture_fullhd("closed", before): return
	await click_control(housing_control("HouseEditorStorageFilters"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFilterMenu") != null, "full-HD physical Filter root"): return
	var scale: float = housing_control("HouseEditorBasicDecorModeButton").get_global_rect().size.x / 64.0
	var owner: Rect2 = housing_control("HouseEditorStorageFilters").get_global_rect()
	var menu: Rect2 = housing_control("HouseEditorStorageFilterMenu").get_global_rect()
	if owner.size.distance_to(Vector2(125.0, 20.0) * scale) > 0.1 or menu.size.distance_to(Vector2(125.0, 109.0) * scale) > 0.1 or menu.position.distance_to(Vector2(owner.position.x + 6.0 * scale, owner.end.y - 2.0 * scale)) > 0.1:
		fail("Full-HD Filter/root lost Retail source bounds")
		return
	var placeable := housing_control("HouseEditorStorageLocationUnavailable") as BaseButton
	if placeable == null or not placeable.disabled:
		fail("Unqualified Placeable unexpectedly enabled")
		return
	if not await capture_fullhd("root", before): return
	push_mouse_motion(housing_control("HouseEditorStorageSort").get_global_rect().get_center())
	if not await wait_housing(func(): return housing_control("HouseEditorStorageSortMenu") != null, "full-HD hover Sort", 4000): return
	var sort_owner: Rect2 = housing_control("HouseEditorStorageSort").get_global_rect()
	var sort: Rect2 = housing_control("HouseEditorStorageSortMenu").get_global_rect()
	if sort.position.distance_to(Vector2(sort_owner.end.x, sort_owner.position.y)) > 0.1 or absf(sort.size.y - 63.0 * scale) > 0.1:
		fail("Full-HD Sort lost source owner TOPRIGHT/two-radio bounds")
		return
	var date := housing_control("HouseEditorStorageSortDateAdded") as BaseButton
	if date == null or not date.disabled:
		fail("Unqualified Date Added unexpectedly enabled")
		return
	if not await capture_fullhd("sort", before): return
	await click_control(date, MOUSE_BUTTON_LEFT)
	if housing_control("HouseEditorStorageSortMenu") == null:
		fail("Disabled Date Added unexpectedly dispatched")
		return
	await click_control(housing_control("HouseEditorStorageSortAlphabetical"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFilterMenu") == null, "full-HD radio closes popup"): return
	await click_control(housing_control("HouseEditorStorageFilters"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageDyeable") != null, "full-HD reopened root"): return
	await click_control(housing_control("HouseEditorStorageDyeable"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageDyeableTick") != null and housing_control("HouseEditorStorageFiltersResetButton").is_visible_in_tree(), "full-HD checkbox Refresh/reset"): return
	if not await capture_fullhd("dyeable", before): return
	await click_control(housing_control("HouseEditorStorageFiltersResetButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFilterMenu") == null and housing_control("HouseEditorStored2330") != null, "full-HD reset restores paid Tesla"): return
	if not await capture_fullhd("reset", before): return
	print("PASS: private5870 full-HD storage captures ", housing_skin)
	client.free()
	quit(0)

func capture_fullhd(phase: String, before: Dictionary) -> bool:
	await frames(3)
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var state: Dictionary = client.housing_state()
	if image.get_size() != Vector2i(1920, 1080) or state.decor != before.decor or state.stored != before.stored:
		fail("Full-HD resolution or paid-state conservation failed: " + phase)
		return false
	var menu := housing_control("HouseEditorStorageFilterMenu")
	var sort := housing_control("HouseEditorStorageSortMenu")
	fullhd_receipts.append({"phase": phase, "skin": housing_skin, "width": image.get_width(), "height": image.get_height(), "owner": housing_control("HouseEditorStorageFilters").get_global_rect(), "root": menu.get_global_rect() if menu != null else Rect2(), "sort": sort.get_global_rect() if sort != null else Rect2(), "decor": state.decor, "stored": state.stored})
	var output := FileAccess.open(shots.path_join("fullhd-receipts.json"), FileAccess.WRITE)
	if output == null:
		fail("Cannot save full-HD receipt")
		return false
	output.store_string(JSON.stringify(fullhd_receipts, "\t"))
	output.flush()
	var error := output.get_error()
	output.close()
	if error != OK or image.save_png(shots.path_join(housing_skin.to_lower() + "-storage-fullhd-" + phase + ".png")) != OK:
		fail("Cannot persist full-HD receipt/image: " + phase)
		return false
	return true
