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
		fail("Search clear needs explicit private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await ready_dashboard(): return
	if client.housing_state().get("house_id",0) != editor_house or not client.housing_state().get("can_edit",false) or housing_count(2330,"stored") != 1:
		fail("Search clear requires actual owned house/paid Tesla")
		return
	if not await click_exposed_control(housing_control("HousingDecorateButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null or housing_control("HouseEditorStorageExpandButton") != null, "clear drawer entry"): return
	if housing_control("HouseEditorStorageExpandButton") != null:
		if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageSearchBox") != null and absf(housing_control("HouseEditorStorageFrame").global_position.x)<0.01, "settled clear search input"): return
	var before: Dictionary = client.housing_state()
	if not await type_storage_search(""): return
	if not await wait_housing(func(): return clear_visible(), "focused empty search shows clear"): return
	if not await click_clear(): return
	if not await type_storage_search("__housing_no_match__"): return
	if not await wait_housing(func(): return clear_visible() and housing_control("HouseEditorStorageEmpty") != null, "unmatched query still clearable"): return
	var search: LineEdit = housing_control("HouseEditorStorageSearchBox") as LineEdit
	var clear: Control = housing_control("HouseEditorStorageSearchClearButton")
	var scale: float = search.get_global_rect().size.y/30.0
	var search_rect: Rect2 = search.get_global_rect()
	var clear_rect: Rect2 = clear.get_global_rect()
	if clear_rect.size.distance_to(Vector2(17.0,17.0)*scale)>0.2 or absf(clear_rect.end.x-(search_rect.end.x-3.0*scale))>0.2 or absf(clear_rect.get_center().y-search_rect.get_center().y)>0.2:
		fail("Native clear source rectangle mismatch: "+str(clear_rect)+" search="+str(search_rect))
		return
	var icon: Control = housing_control("HouseEditorStorageSearchClearIcon")
	var image: TextureRect = icon.get_node_or_null("Parts/Part0") as TextureRect if icon != null else null
	if image == null or image.texture == null:
		fail("Authentic clear-button atlas did not load")
		return
	if not await prove_clear_affordance(clear,icon,scale): return
	if not await record_storage_tooltip("storage-search-clear-query"): return
	if not await click_clear(): return
	if not await wait_housing(func(): return housing_control("HouseEditorStored2330") != null, "clear immediately restores paid stored result"): return
	if not await prove_paid_thumbnail_visible(): return
	if not await enter_cleanup(): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return storage_search_text().is_empty() and not clear_visible() and housing_control("HouseEditorStored2330") != null, "remount preserves cleared backing search"): return
	if not drawer_conserved(before): return
	if not await record_storage_tooltip("storage-search-cleared"): return
	print("PASS: private5870 owned-storage search clear ",housing_skin)
	client.free()
	quit(0)

func prove_clear_affordance(clear: Control, icon: Control, scale: float) -> bool:
	var art: Control = housing_control("HouseEditorStorageSearchClearArt")
	if art == null:
		fail("Clear icon normal-alpha container missing")
		return false
	await hover_point(Vector2(1100.0,500.0))
	if absf(art.modulate.a-0.5)>0.01:
		fail("Source clear normal alpha must be0.5")
		return false
	await hover_point(clear.get_global_rect().get_center())
	if absf(art.modulate.a-1.0)>0.01:
		fail("Source clear hover alpha must be1.0, actual="+str(art.modulate.a))
		return false
	if not await record_storage_tooltip("storage-search-clear-hover"): return false
	drawer_mouse_button(clear.get_global_rect().get_center(),true)
	await frames(4)
	var pressed_inset: Vector2 = (icon.global_position-clear.global_position)/scale
	if pressed_inset.distance_to(Vector2(4.0,4.0))>0.2 or absf(art.modulate.a-1.0)>0.01:
		fail("Source clear pressed inset must be4,4, actual="+str(pressed_inset))
		return false
	if not await record_storage_tooltip("storage-search-clear-pressed"): return false
	await hover_point(Vector2(1100.0,500.0))
	drawer_mouse_button(Vector2(1100.0,500.0),false)
	await frames(4)
	var released_inset: Vector2 = (icon.global_position-clear.global_position)/scale
	if released_inset.distance_to(Vector2(3.0,3.0))>0.2 or absf(art.modulate.a-0.5)>0.01 or storage_search_text()!="__housing_no_match__":
		fail("Source clear release outside must restore3,3/alpha0.5 without clicking")
		return false
	return true

func clear_visible() -> bool:
	var clear: Control = housing_control("HouseEditorStorageSearchClearButton")
	return clear != null and clear.is_visible_in_tree()

func click_clear() -> bool:
	if not await click_exposed_control(housing_control("HouseEditorStorageSearchClearButton")): return false
	return await wait_housing(func():
		var input: LineEdit = housing_control("HouseEditorStorageSearchBox") as LineEdit
		return input != null and input.text.is_empty() and not input.has_focus() and not clear_visible(), "clear empties backing input, releases focus and hides")
