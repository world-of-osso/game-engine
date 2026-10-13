extends "res://tests/housing_cleanup_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Storage search needs explicit private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	if client.housing_state().decor.size() != 1 or int(client.housing_state().decor[0].decor_id) != 1693 or housing_count(2330, "stored") != 1:
		fail("Storage search requires actually paid bookcase and Tesla from previous receipts")
		return
	if not await enter_cleanup(): return
	if not await wait_housing(func(): return placed_node() != null, "paid storage-search actor"): return
	var hit: Dictionary = find_visible_cornerstone_triangle(placed_node(), int(client.housing_state().decor[0].instance_id))
	if hit.is_empty():
		fail("Storage search bookcase has no actual pick triangle")
		return
	await click(hit.point, MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return stored_bookcases() == 1 and client.housing_state().decor.is_empty() and not client.housing_state().get("pending", true), "normal Cleanup returns paid bookcase"): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStored1693") != null and housing_control("HouseEditorStored2330") != null and housing_control("HouseEditorStorageSearchBox") != null, "source owned storage search"): return
	var before: Dictionary = client.housing_state()
	for decor in [1693, 2330]:
		var icon: Control = housing_control("HouseEditorIcon%d" % decor)
		var image: TextureRect = icon.get_node_or_null("Parts/Part0") as TextureRect if icon != null else null
		print("STORAGE_THUMBNAIL id=", decor, " node=", icon, " image=", image, " texture=", image.texture if image != null else null)
		if image == null or image.texture == null:
			fail("Owned storage thumbnail is not loaded: %d" % decor)
			return
	if not await record_editor("storage-all"): return
	if not await type_storage_search("tEsLa"): return
	if not await wait_housing(func(): return housing_control("HouseEditorStored2330") != null and housing_control("HouseEditorStored1693") == null, "case-insensitive stored-name filter"): return
	if not await record_editor("storage-tesla"): return
	if not await enter_cleanup(): return
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return
	if not await wait_housing(func(): return storage_search_text() == "tEsLa" and housing_control("HouseEditorStored2330") != null and housing_control("HouseEditorStored1693") == null, "hidden storage remount retains search"): return
	if not await type_storage_search("Fireplace"): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageEmpty") != null and housing_control("HouseEditorStored1693") == null and housing_control("HouseEditorStored2330") == null and housing_control("HouseEditorStored80") == null, "search never includes unowned or placed-only catalog entries"): return
	if not await record_editor("storage-empty"): return
	if not await type_storage_search(""): return
	if not await wait_housing(func(): return housing_control("HouseEditorStored1693") != null and housing_control("HouseEditorStored2330") != null, "clear restores actual stored entries"): return
	if client.housing_state().stored != before.stored or client.housing_state().decor != before.decor or client.housing_state().placed != before.placed:
		fail("Storage browsing changed confirmed quantities/layout")
		return
	if not await record_editor("storage-cleared"): return
	await frames(15)
	if not await place_owned_bookcase(): return
	if total_bookcases(client.housing_state()) != 1 or housing_count(2330, "stored") != 1:
		fail("Restoring paid bookcase after search duplicated or lost storage")
		return
	print("PASS: private5870 owned-storage search ", housing_skin)
	client.free()
	quit(0)

func storage_search_text() -> String:
	var search: LineEdit = housing_control("HouseEditorStorageSearchBox") as LineEdit
	return search.text if search != null else ""

func type_storage_search(text: String) -> bool:
	var search: LineEdit = housing_control("HouseEditorStorageSearchBox") as LineEdit
	if search == null:
		fail("Source storage search is not a native input")
		return false
	if not await click_exposed_control(search): return false
	search.select_all()
	if text.is_empty():
		await tap(KEY_BACKSPACE)
	else:
		await housing_type_text(text)
	return await wait_housing(func(): return storage_search_text() == text, "physical storage search input")
