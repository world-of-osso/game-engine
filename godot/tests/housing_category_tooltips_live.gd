extends "res://tests/housing_storage_tooltips_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Category tooltip proof needs explicit private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await ready_dashboard(): return
	if client.housing_state().get("house_id",0) != editor_house or not client.housing_state().get("can_edit",false):
		fail("Category tooltip proof requires saved owned house")
		return
	if not await click_exposed_control(housing_control("HousingDecorateButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null or housing_control("HouseEditorStorageExpandButton") != null, "category tooltip drawer"): return
	if housing_control("HouseEditorStorageExpandButton") != null:
		if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageCategory18") != null and absf(housing_control("HouseEditorStorageFrame").global_position.x)<0.01, "settled actual owned categories"): return
	var before: Dictionary = client.housing_state()
	var scale: float = housing_control("HouseEditorStorageResizeButton").get_global_rect().size.x/20.0
	if not await hover_category("HouseEditorStorageCategory18","All",scale,"storage-category-tooltip-all"): return
	if not await hover_category("HouseEditorStorageCategory1","Furnishings",scale,"storage-category-tooltip-furnishings"): return
	if not await hover_category("HouseEditorStorageCategory2","Structural",scale,"storage-category-tooltip-structural"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageCategory2")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageSubcategoryAll") != null, "source expanded child-All"): return
	if not await hover_category("HouseEditorStorageSubcategoryAll","All",scale,"storage-category-tooltip-child-all"): return
	await hover_point(Vector2(1100.0,500.0))
	if not await wait_housing(func(): return not client.tooltip_state().get("visible",false), "category tooltip leaves"): return
	if not drawer_conserved(before): return
	print("PASS: private5870 housing category tooltips ",housing_skin)
	client.free()
	quit(0)

func hover_category(name: String, title: String, scale: float, phase: String) -> bool:
	var control: Control = housing_control(name)
	if control == null:
		fail("Actual owned category missing: "+name)
		return false
	await hover_point(control.get_global_rect().get_center())
	if not await wait_housing(func(): return client.tooltip_state().get("visible",false) and client.tooltip_state().get("title","")==title, "source category tooltip "+title): return false
	var state: Dictionary = client.tooltip_state()
	var rect: Rect2 = control.get_global_rect()
	if absf(float(state.rect[0])-(rect.end.x/scale-12.0))>1.5 or absf(float(state.rect[1])+float(state.rect[3])-(rect.position.y/scale+12.0))>1.5:
		fail("Category tooltip source offset mismatch: "+str(state)+" owner="+str(rect))
		return false
	return await record_storage_tooltip(phase)
