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
		fail("Storage filter proof requires explicit private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	if client.housing_state().decor.size() != 1 or int(client.housing_state().decor[0].decor_id) != 1693 or housing_count(2330, "stored") != 1:
		fail("Storage filters require the actual paid bookcase and Tesla")
		return
	if not await enter_cleanup(): return
	if not await wait_housing(func(): return placed_node() != null, "paid filter-proof bookcase"): return
	var hit: Dictionary = find_visible_cornerstone_triangle(placed_node(), int(client.housing_state().decor[0].instance_id))
	if hit.is_empty():
		fail("Actual stored bookcase has no visible pick triangle")
		return
	await click(hit.point, MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return stored_bookcases() == 1 and client.housing_state().decor.is_empty() and not client.housing_state().get("pending", true), "paid bookcase returned normally"): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStored1693") != null and housing_control("HouseEditorStored2330") != null, "both actual paid stacks"): return
	var before: Dictionary = client.housing_state()
	if not await click_exposed_control(housing_control("HouseEditorStorageFilters")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageDyeable") != null, "source storage filter menu"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageSort")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageSortAlphabetical") != null, "sort submenu"): return
	if not (housing_control("HouseEditorStorageSortDateAdded") as BaseButton).disabled:
		fail("Unsourced acquisition dates became selectable")
		return
	if not await click_exposed_control(housing_control("HouseEditorStorageSortAlphabetical")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageSortAlphabeticalTick") != null and housing_control("HouseEditorStored2330").global_position.x < housing_control("HouseEditorStored1693").global_position.x, "alphabetical actual tile order"): return
	await frames(20)
	if not await record_editor("storage-filter-alphabetical"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageDyeable")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageDyeableTick") != null and housing_control("HouseEditorStorageEmpty") != null and housing_control("HouseEditorStored1693") == null and housing_control("HouseEditorStored2330") == null, "paid nondyeable decor excluded"): return
	await frames(20)
	if not await record_editor("storage-filter-dyeable-empty"): return
	if not await enter_cleanup(): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageDyeableTick") != null and housing_control("HouseEditorStorageEmpty") != null, "hidden storage retains filter"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageResetFilters")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStored1693") != null and housing_control("HouseEditorStored2330") != null and housing_control("HouseEditorStorageDyeableTick") == null and housing_control("HouseEditorStorageSortAlphabeticalTick") != null, "reset restores paid stacks without changing sort"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageFilters")): return
	if not await type_storage_search("Tesla"): return
	if not await wait_housing(func(): return housing_control("HouseEditorStored2330") != null and housing_control("HouseEditorStored1693") == null, "search composes with selected sort"): return
	if not await type_storage_search(""): return
	if not await wait_housing(func(): return housing_control("HouseEditorStored2330") != null and housing_control("HouseEditorStored1693") != null, "clear keeps paid stacks"): return
	await frames(20)
	if not await record_editor("storage-filter-reset"): return
	if client.housing_state().stored != before.stored or client.housing_state().placed != before.placed or client.housing_state().decor != before.decor:
		fail("Storage filtering changed ownership or layout")
		return
	if not await place_owned_bookcase(): return
	if total_bookcases(client.housing_state()) != 1 or housing_count(2330, "stored") != 1:
		fail("Storage filter proof lost or duplicated paid copies")
		return
	print("PASS: private5870 owned-storage filters ", housing_skin)
	client.free()
	quit(0)
