extends "res://tests/housing_storage_input_chrome_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Focus proof needs explicit private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await ready_dashboard(): return
	if client.housing_state().get("house_id",0) != editor_house or not client.housing_state().get("can_edit",false) or housing_count(2330,"stored") != 1:
		fail("Focus proof requires actual owned house/paid Tesla")
		return
	if not await click_exposed_control(housing_control("HousingDecorateButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null or housing_control("HouseEditorStorageExpandButton") != null, "focus drawer entry"): return
	if housing_control("HouseEditorStorageExpandButton") != null:
		if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageSearchBox") != null and absf(housing_control("HouseEditorStorageFrame").global_position.x)<0.01, "settled focus input"): return
	var before: Dictionary = client.housing_state()
	for code: Key in [KEY_ENTER,KEY_ESCAPE]:
		if not await type_storage_search("tEs"): return
		await tap(code)
		if not await wait_housing(func(): return search_focus_released("tEs") and housing_control("HouseEditorStored2330") != null, "source key releases focus without clearing/closing: "+str(code)): return
		if chat_input_visible():
			fail("Storage search focus key opened chat input")
			return
		if not await prove_paid_thumbnail_visible(): return
		print("STORAGE_FOCUS key=",code," raw=",storage_search_text()," drawer_mounted=true")
	if not await record_storage_tooltip("storage-search-focus-retained"): return
	if not await type_storage_search(""): return
	await tap(KEY_KP_ENTER)
	if not await wait_housing(func(): return search_focus_released("") and not clear_visible() and housing_control("HouseEditorStored2330") != null, "empty keypadEnter focus release/clear hidden"): return
	if chat_input_visible():
		fail("Empty storage keypadEnter opened chat input")
		return
	var glyph: Control = housing_control("HouseEditorStorageSearchGlyph")
	var image: TextureRect = glyph.get_node_or_null("Parts/Part0") as TextureRect
	if not magnifier_color_matches(image.self_modulate,0.6):
		fail("Empty focus release did not restore source magnifier gray")
		return
	if not await record_storage_tooltip("storage-search-focus-empty"): return
	if not await enter_cleanup(): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return search_focus_released("") and not clear_visible(), "source focus release survives remount"): return
	if not drawer_conserved(before): return
	print("PASS: private5870 owned-storage focus keys ",housing_skin)
	client.free()
	quit(0)

func chat_input_visible() -> bool:
	var ui: Node = client.get_node_or_null("ChatFrameUI")
	if ui == null: return false
	var input: Control = ui.frame_control("ChatFrame1EditBox")
	return input != null and input.is_visible_in_tree()

func search_focus_released(expected: String) -> bool:
	var input: LineEdit = housing_control("HouseEditorStorageSearchBox") as LineEdit
	return input != null and not input.has_focus() and input.text==expected
