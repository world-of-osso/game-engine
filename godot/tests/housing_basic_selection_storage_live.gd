extends "res://tests/housing_storage_focus_callbacks_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Basic selection needs private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	if not await wait_housing(func(): return placed_node() != null and client.housing_state().decor.size()==1 and stored_bookcases()==0 and housing_count(2330,"stored")==1, "actual paid selection fixture"): return
	if not await type_storage_search("tEs"): return
	var before: Dictionary = client.housing_state()
	var size_before: Vector2 = housing_control("HouseEditorStorageFrame").size
	if not await select_saved_decor(): return
	if not await wait_housing(func(): return drawer_absent(), "selected Basic target hides drawer and open handle"): return
	if not await record_editor("basic-selected-storage-hidden"): return
	if not await click_exposed_control(housing_control("HouseEditorCancel")): return
	if not await wait_housing(func(): return storage_search_text()=="tEs" and housing_control("HouseEditorStored2330") != null, "cancel restores retained query and paid Tesla"): return
	if housing_control("HouseEditorStorageFrame").size.distance_to(size_before)>0.01:
		fail("Selecting changed saved storage dimensions")
		return
	if not await record_editor("basic-cancel-storage-restored"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageCollapseButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageExpandButton") != null and absf(housing_control("HouseEditorStorageExpandButton").global_position.x)<0.01, "saved collapsed handle"): return
	if not await select_saved_decor(): return
	if not await wait_housing(func(): return drawer_absent(), "selected target hides collapsed handle"): return
	if not await click_exposed_control(housing_control("HouseEditorCancel")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageExpandButton") != null and housing_control("HouseEditorStorageFrame")==null, "cancel restores collapsed preference"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return
	if not await wait_housing(func(): return storage_search_text()=="tEs" and housing_control("HouseEditorStored2330") != null, "reexpanded query/Tesla retained"): return
	if not drawer_conserved(before): return
	print("PASS: private5870 Basic selection storage visibility ",housing_skin)
	client.free()
	quit(0)

func drawer_absent() -> bool:
	return housing_control("HouseEditorStorageFrame")==null and housing_control("HouseEditorStorageExpandButton")==null and housing_control("HouseEditorStorageSearchBox")==null
