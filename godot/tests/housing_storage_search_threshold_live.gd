extends "res://tests/housing_storage_search_clear_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Threshold proof needs explicit private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await ready_dashboard(): return
	if client.housing_state().get("house_id",0) != editor_house or not client.housing_state().get("can_edit",false) or housing_count(2330,"stored") != 1:
		fail("Threshold proof requires actual owned house/paid Tesla")
		return
	if not await click_exposed_control(housing_control("HousingDecorateButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null or housing_control("HouseEditorStorageExpandButton") != null, "threshold drawer entry"): return
	if housing_control("HouseEditorStorageExpandButton") != null:
		if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageCategory2") != null and absf(housing_control("HouseEditorStorageFrame").global_position.x)<0.01, "settled actual owned Structural category"): return
	var before: Dictionary = client.housing_state()
	if not await click_exposed_control(housing_control("HouseEditorStorageCategory2")): return
	if not await wait_housing(func(): return storage_path_text()=="Structural" and housing_control("HouseEditorStored2330") != null, "source Structural paid result"): return
	for short: String in ["z","zz","éé"]:
		if not await type_storage_search(short): return
		if not await wait_housing(func(): return storage_path_text()=="Structural" and housing_control("HouseEditorStored2330") != null and clear_visible(), "short UTF8 raw text does not filter or reset focus: "+short): return
		print("STORAGE_THRESHOLD short=",short," raw=",storage_search_text()," path=",storage_path_text())
	if not await prove_paid_thumbnail_visible(): return
	if not await record_storage_tooltip("storage-search-short-utf8"): return
	if not await type_storage_search("zzz"): return
	if not await wait_housing(func(): return storage_path_text()=="All" and housing_control("HouseEditorStorageEmpty") != null and housing_control("HouseEditorStored2330") == null, "three UTF8 characters apply search and focus All"): return
	if not await record_storage_tooltip("storage-search-three-empty"): return
	if not await type_storage_search("zz"): return
	if not await wait_housing(func(): return storage_path_text()=="All" and housing_control("HouseEditorStored2330") != null, "shortening disables filter without restoring old focus"): return
	if not await type_storage_search("tEs"): return
	if not await wait_housing(func(): return storage_path_text()=="All" and housing_control("HouseEditorStored2330") != null, "three-character case-insensitive matching query"): return
	if not await click_clear(): return
	if not await enter_cleanup(): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return storage_search_text().is_empty() and storage_path_text()=="All" and not clear_visible() and housing_control("HouseEditorStored2330") != null, "threshold clear backing state survives remount"): return
	if not drawer_conserved(before): return
	print("PASS: private5870 owned-storage search threshold ",housing_skin)
	client.free()
	quit(0)

func storage_path_text() -> String:
	var path: Label = housing_control("HouseEditorStorageCategoryPath") as Label
	return path.text if path != null else ""
