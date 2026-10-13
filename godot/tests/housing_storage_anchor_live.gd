extends "res://tests/housing_basic_selection_storage_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house<=0:
		fail("Storage anchor needs private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	if not await type_storage_search("tEs"): return
	var before: Dictionary = client.housing_state()
	for height: int in [720,1080]:
		root.size = Vector2i(1280,height)
		await frames(10)
		print("ANCHOR_WITNESS root=",root.size," panel=",housing_control("HouseEditorStorageFrame").get_global_rect()," search=",housing_control("HouseEditorStorageSearchBox").get_global_rect()," glyph=",housing_control("HouseEditorStorageSearchGlyph").get_global_rect())
		if not await wait_housing(func(): return root.size.y==height and panel_matches_source_anchor(), "source storage anchor at viewport"+str(height)): return
		if not await record_editor("storage-anchor-"+str(height)): return
		if not await select_saved_decor(): return
		if not await wait_housing(func(): return drawer_absent(), "anchor selection hides both parts"): return
		await click(Vector2(640,240),MOUSE_BUTTON_RIGHT)
		if not await wait_housing(func(): return int(client.housing_state().get("selected",0))==0 and storage_search_text()=="tEs" and panel_matches_source_anchor(), "cancel restores exact source anchor/query"): return
		if not drawer_conserved(before): return
		if not await record_editor("storage-anchor-"+str(height)+"-restored"): return
	print("PASS: private5870 storage source vertical anchor ",housing_skin)
	client.free()
	quit(0)

func panel_matches_source_anchor() -> bool:
	var panel: Control = housing_control("HouseEditorStorageFrame")
	var glyph: Control = housing_control("HouseEditorStorageSearchGlyph")
	if panel==null or glyph==null: return false
	var rect: Rect2 = panel.get_global_rect()
	var scale: float = glyph.get_global_rect().size.y/10.0
	var viewport_height: float = float(root.size.y)/scale
	var panel_height: float = rect.size.y/scale
	var expected_top: float = clampf((viewport_height-panel_height)/2.0-150.0,0.0,maxf(0.0,viewport_height-panel_height))*scale
	if absf(rect.position.y-expected_top)>0.2: return false
	print("STORAGE_ANCHOR viewport=",root.size," scale=",scale," height=",panel_height," top=",rect.position.y," expected=",expected_top)
	return true
