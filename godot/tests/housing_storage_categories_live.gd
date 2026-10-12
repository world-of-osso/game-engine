extends "res://tests/housing_storage_search_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Storage categories need explicit private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	if client.housing_state().decor.size() != 1 or int(client.housing_state().decor[0].decor_id) != 1693 or housing_count(2330, "stored") != 1:
		fail("Category proof needs the actual paid bookcase and Tesla")
		return
	if not await enter_cleanup(): return
	if not await wait_housing(func(): return placed_node() != null, "actual category-proof bookcase"): return
	var hit: Dictionary = find_visible_cornerstone_triangle(placed_node(), int(client.housing_state().decor[0].instance_id))
	if hit.is_empty():
		fail("Category proof has no visible actual decor pick triangle")
		return
	print("STORAGE_CATEGORY_REMOVE_BEFORE picked=", UnitPicker.pick(camera(), hit.point), " expected=", client.housing_state().decor[0].instance_id, " state=", category_debug_state())
	await click(hit.point, MOUSE_BUTTON_LEFT)
	await frames(10)
	print("STORAGE_CATEGORY_REMOVE_AFTER state=", category_debug_state())
	if not await wait_housing(func(): return stored_bookcases() == 1 and client.housing_state().decor.is_empty() and not client.housing_state().get("pending", true), "paid decor moved into storage"): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageCategory18") != null and housing_control("HouseEditorStorageCategory1") != null and housing_control("HouseEditorStorageCategory2") != null, "only actual owned category roots"): return
	for unowned in [4, 5, 6, 9, 17]:
		if housing_control("HouseEditorStorageCategory%d" % unowned) != null:
			fail("Unowned/Featured category became a storage browse choice")
			return
	var before: Dictionary = client.housing_state()
	if not await record_editor("storage-category-all"): return
	if not await choose_storage_category(1, "Furnishings", 1693, 2330): return
	if housing_control("HouseEditorStorageSubcategory6") != null:
		fail("Source hides specific child buttons with only one owned child")
		return
	if not await record_editor("storage-category-furnishings"): return
	if not await type_storage_search("tEsLa"): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageCategoryBack") == null and housing_text("HouseEditorStorageCategoryPath") == "All" and housing_control("HouseEditorStored1693") == null and housing_control("HouseEditorStored2330") != null, "search clears category focus to All"): return
	if not await record_editor("storage-category-search-all"): return
	if not await choose_storage_category(1, "Furnishings", 1693, 2330): return
	if storage_search_text() != "":
		fail("Category selection did not clear the real input")
		return
	if not await click_exposed_control(housing_control("HouseEditorStorageCategoryBack")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageCategory2") != null and housing_control("HouseEditorStored1693") != null and housing_control("HouseEditorStored2330") != null, "Back restores owned roots and stored entries"): return
	if not await choose_storage_category(2, "Structural", 2330, 1693): return
	if not await enter_cleanup(): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return housing_text("HouseEditorStorageCategoryPath") == "Structural" and housing_control("HouseEditorStored2330") != null and housing_control("HouseEditorStored1693") == null, "hidden storage retains source category"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageSubcategoryAll")): return
	if not await record_editor("storage-category-structural"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageCategoryBack")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageCategory18") != null, "top-level All category"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageCategory18")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStored1693") != null and housing_control("HouseEditorStored2330") != null, "All keeps actual storage results"): return
	if client.housing_state().stored != before.stored or client.housing_state().placed != before.placed or client.housing_state().decor != before.decor:
		fail("Storage category browsing changed quantities or layout")
		return
	await frames(15)
	if not await place_owned_bookcase(): return
	if total_bookcases(client.housing_state()) != 1 or housing_count(2330, "stored") != 1:
		fail("Category proof duplicated or lost paid decor")
		return
	print("PASS: private5870 owned-storage categories ", housing_skin)
	client.free()
	quit(0)

func category_debug_state() -> Dictionary:
	var state: Dictionary = client.housing_state()
	return {"house":state.get("house_id"), "can_edit":state.get("can_edit"), "category":housing_text("HouseEditorStorageCategoryPath"), "search":storage_search_text(), "pending":state.get("pending"), "stored":state.get("stored"), "decor":state.get("decor"), "error":state.get("error")}

func choose_storage_category(id: int, title: String, shown: int, hidden: int) -> bool:
	print("STORAGE_CATEGORY_CLICK_BEFORE id=", id, " rect=", housing_control("HouseEditorStorageCategory%d" % id).get_global_rect(), " state=", category_debug_state())
	if not await click_exposed_control(housing_control("HouseEditorStorageCategory%d" % id)): return false
	await frames(20)
	print("STORAGE_CATEGORY_CLICK_AFTER id=", id, " path=", housing_text("HouseEditorStorageCategoryPath"), " back=", housing_control("HouseEditorStorageCategoryBack"), " shown=", housing_control("HouseEditorStored%d" % shown), " hidden=", housing_control("HouseEditorStored%d" % hidden), " state=", category_debug_state())
	if not await capture_housing("storage-category-click-%d-diagnostic" % id): return false
	return await wait_housing(func(): return housing_text("HouseEditorStorageCategoryPath") == title and housing_control("HouseEditorStorageCategoryBack") != null and housing_control("HouseEditorStored%d" % shown) != null and housing_control("HouseEditorStored%d" % hidden) == null, "source category " + title)
