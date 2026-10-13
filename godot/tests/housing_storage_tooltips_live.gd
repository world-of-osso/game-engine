extends "res://tests/housing_storage_affordance_live.gd"

var storage_tooltip_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Storage tooltip proof requires explicit private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await ready_dashboard(): return
	if client.housing_state().get("house_id",0) != editor_house or not client.housing_state().get("can_edit",false) or housing_count(2330,"stored") != 1:
		fail("Storage tooltip proof requires the actual paid Tesla and owned house")
		return
	if not await click_exposed_control(housing_control("HousingDecorateButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null or housing_control("HouseEditorStorageExpandButton") != null, "source tooltip drawer entry"): return
	if housing_control("HouseEditorStorageExpandButton") != null:
		if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStored2330") != null and absf(housing_control("HouseEditorStorageFrame").global_position.x)<0.01, "settled paid stored card"): return
	var before: Dictionary = client.housing_state()
	var scale: float = housing_control("HouseEditorStorageResizeButton").get_global_rect().size.x/20.0
	var card: Control = housing_control("HouseEditorStored2330")
	await hover_point(card.get_global_rect().get_center())
	if not await wait_housing(func(): return client.tooltip_state().get("visible",false) and client.tooltip_state().get("title","")=="Gnomish Tesla Coil", "actual stored card tooltip"): return
	var state: Dictionary = client.tooltip_state()
	var has_count: bool = false
	for line in state.get("lines",[]):
		if str(line).contains("Total Owned: 1 (Placed: 0, Storage: 1)"): has_count = true
	if not has_count or not source_right_anchor(state,card,scale):
		fail("Stored tooltip lacks confirmed counts/right anchor: "+str(state))
		return
	if not await record_storage_tooltip("storage-card-tooltip"): return
	await hover_point(Vector2(1100.0,500.0))
	if not await wait_housing(func(): return not client.tooltip_state().get("visible",false), "stored tooltip leave hides"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageCollapseButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageExpandButton") != null and absf(housing_control("HouseEditorStorageExpandButton").global_position.x)<0.01, "settled House Chest handle"): return
	var handle: Control = housing_control("HouseEditorStorageExpandButton")
	await hover_point(handle.get_global_rect().get_center())
	if not await wait_housing(func(): return client.tooltip_state().get("visible",false) and client.tooltip_state().get("title","")=="House Chest", "source House Chest tooltip"): return
	if not source_right_anchor(client.tooltip_state(),handle,scale):
		fail("House Chest tooltip not anchored right")
		return
	if not await record_storage_tooltip("storage-house-chest-tooltip"): return
	if not await click_exposed_control(handle): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null and absf(housing_control("HouseEditorStorageFrame").global_position.x)<0.01 and client.tooltip_state().get("title","")!="House Chest", "unmounted handle discards tooltip"): return
	await hover_point(Vector2(1100.0,500.0))
	if not await wait_housing(func(): return not client.tooltip_state().get("visible",false), "tooltip cleared after drawer expansion"): return
	if not drawer_conserved(before): return
	print("PASS: private5870 owned-storage tooltips ",housing_skin)
	client.free()
	quit(0)

func source_right_anchor(state: Dictionary, control: Control, scale: float) -> bool:
	var rect: Rect2 = control.get_global_rect()
	return absf(float(state.rect[0])-rect.end.x/scale)<1.5 and absf(float(state.rect[1])+float(state.rect[3])-rect.position.y/scale)<1.5

func record_storage_tooltip(phase: String) -> bool:
	await frames(15)
	if not await capture_housing(phase): return false
	storage_tooltip_receipts.append({"phase":phase,"skin":housing_skin,"tooltip":client.tooltip_state(),"housing":client.housing_state()})
	var file: FileAccess = FileAccess.open(shots.path_join("storage-tooltip-receipts.json"),FileAccess.WRITE)
	if file == null:
		fail("Cannot save source tooltip receipts")
		return false
	file.store_string(JSON.stringify(storage_tooltip_receipts,"\t"))
	file.close()
	return true
