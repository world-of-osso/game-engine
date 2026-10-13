extends "res://tests/housing_storage_focus_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Focus callbacks need private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await ready_dashboard(): return
	if client.housing_state().get("house_id",0) != editor_house or not client.housing_state().get("can_edit",false) or housing_count(2330,"stored") != 1:
		fail("Focus callbacks require real owned house/paid Tesla")
		return
	if not await click_exposed_control(housing_control("HousingDecorateButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null or housing_control("HouseEditorStorageExpandButton") != null, "callback drawer entry"): return
	if housing_control("HouseEditorStorageExpandButton") != null:
		if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageCategory2") != null and absf(housing_control("HouseEditorStorageFrame").global_position.x)<0.01, "callback Structural root"): return
	var before: Dictionary = client.housing_state()
	if not await click_exposed_control(housing_control("HouseEditorStorageCategory2")): return
	if not await type_storage_search("zz"): return
	if not await wait_housing(func(): return storage_path_text()=="Structural" and housing_control("HouseEditorStored2330") != null and housing_control("HouseEditorStorageSubcategoryAll") != null, "callback short query/child-All"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageSubcategoryAll")): return
	if not await wait_housing(func(): return storage_search_text()=="zz" and storage_path_text()=="Structural" and housing_control("HouseEditorStored2330") != null, "same child-All focus leaves raw query unchanged"): return
	if not await record_storage_tooltip("storage-focus-callback-noop"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageCategoryBack")): return
	if not await wait_housing(func(): return storage_search_text()=="zz" and storage_path_text()=="All" and housing_control("HouseEditorStored2330") != null, "Back retains raw short query"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageCategory1")): return
	if not await wait_housing(func(): return search_focus_released("") and not clear_visible() and storage_path_text()=="Furnishings", "changed specific focus clears backing query and focus"): return
	var glyph: Control = housing_control("HouseEditorStorageSearchGlyph")
	var image: TextureRect = glyph.get_node_or_null("Parts/Part0") as TextureRect
	if not magnifier_color_matches(image.self_modulate,0.6):
		fail("Changed specific focus did not restore gray magnifier")
		return
	if not await record_storage_tooltip("storage-focus-callback-cleared"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageCategoryBack")): return
	if not await wait_housing(func(): return search_focus_released("") and storage_path_text()=="All" and housing_control("HouseEditorStored2330") != null, "return All restores paid result"): return
	if not drawer_conserved(before): return
	print("PASS: private5870 owned-storage focus callbacks ",housing_skin)
	client.free()
	quit(0)
