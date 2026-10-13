extends "res://tests/housing_storage_anchor_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house<=0:
		fail("Placed list needs private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if before.decor.size()!=1 or int(before.decor[0].decor_id)!=1693 or housing_count(2330,"stored")!=1:
		fail("Placed list requires actual paid bookcase/Tesla fixture")
		return
	var instance: int = int(before.decor[0].instance_id)
	if not await click_exposed_control(housing_control("HouseEditorExpertDecorModeButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorPlacedDecorListButton") != null, "Expert source list toggle"): return
	if housing_text("HouseEditorDecorBudgetText")!="— / —":
		fail("Placement budget was inferred instead of remaining unknown")
		return
	if not await click_exposed_control(housing_control("HouseEditorPlacedDecorListButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorPlacedDecorList") != null and housing_control("HouseEditorPlacedDecorEntry%d"%instance) != null, "actual confirmed placed list entry"): return
	if housing_text("HouseEditorPlacedDecorEntry%dName"%instance)!=BOOKCASE_NAME:
		fail("Placed entry lost authenticated name")
		return
	if not await record_editor("placed-list-open"): return
	if not await click_exposed_control(housing_control("HouseEditorPlacedDecorEntry%d"%instance)): return
	if not await wait_housing(func(): return int(client.housing_state().get("selected",0))==instance and housing_control("HouseEditorPlacedDecorList") != null, "row selects real instance without closing list"): return
	if not await click_exposed_control(housing_control("HouseEditorPlacedDecorEntry%d"%instance)): return
	if not await wait_housing(func(): return int(client.housing_state().get("selected",0))==0, "same row deselects"): return
	var row: Control = housing_control("HouseEditorPlacedDecorEntry%d"%instance)
	await click(row.get_global_rect().get_center(),MOUSE_BUTTON_RIGHT)
	if not await wait_housing(func(): return housing_control("HouseEditorPlacedDecorRemove") != null, "released right button opens Remove Decor menu"): return
	if housing_text("HouseEditorPlacedDecorRemoveText")!="Remove Decor":
		fail("Context action is not source Remove Decor")
		return
	if not await record_editor("placed-list-context"): return
	if not await click_exposed_control(housing_control("HouseEditorPlacedDecorRemove")): return
	if not await wait_housing(func(): return client.housing_state().decor.is_empty() and stored_bookcases()==1 and not client.housing_state().get("pending",true) and housing_control("HouseEditorPlacedDecorEntry%d"%instance)==null, "normal authoritative removal updates list and paid storage"): return
	if not await record_editor("placed-list-removed"): return
	if not await click_exposed_control(housing_control("HouseEditorPlacedDecorListCloseButton")): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStored1693") != null and housing_control("HouseEditorPlacedDecorListButton")==null, "mode exit hides list and shows real stored bookcase"): return
	if not await place_owned_bookcase(): return
	if total_bookcases(client.housing_state())!=1 or housing_count(2330,"stored")!=1:
		fail("List removal/normal re-place lost or duplicated paid decor")
		return
	print("PASS: private5870 source Expert PlacedDecorList ",housing_skin)
	client.free()
	quit(0)
