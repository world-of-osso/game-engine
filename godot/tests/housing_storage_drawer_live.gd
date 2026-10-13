extends "res://tests/housing_storage_search_live.gd"

var drawer_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Drawer proof requires explicit private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await ready_dashboard(): return
	if client.housing_state().get("house_id", 0) != editor_house or not client.housing_state().get("can_edit", false) or total_bookcases(client.housing_state()) != 1 or housing_count(2330, "stored") != 1:
		fail("Drawer proof needs saved owned house and conserved paid copies")
		return
	if not await click_exposed_control(housing_control("HousingDecorateButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null or housing_control("HouseEditorStorageExpandButton") != null, "saved source drawer entry"): return
	var before: Dictionary = client.housing_state()
	if OS.get_environment("HOUSING_DRAWER_PHASE") == "reload":
		if not await prove_drawer_reload(before): return
	else:
		if not await prove_drawer_controls(before): return
	print("PASS: private5870 owned-storage drawer ", housing_skin, " ", OS.get_environment("HOUSING_DRAWER_PHASE"))
	client.free()
	quit(0)

func prove_drawer_controls(before: Dictionary) -> bool:
	if housing_control("HouseEditorStorageExpandButton") != null:
		if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return false
		if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null, "explicit saved drawer expansion"): return false
	var frame: Control = housing_control("HouseEditorStorageFrame")
	var scale: float = housing_control("HouseEditorStorageResizeButton").get_global_rect().size.x / 20.0
	var initial: Vector2 = frame.get_global_rect().size / scale
	if not await resize_drawer(Vector2(700.0, 730.0), initial, scale): return false
	if not await wait_drawer_size(Vector2(725.0, 730.0), scale): return false
	if not await record_drawer("storage-drawer-resized"): return false
	if not await resize_drawer(Vector2(900.0, 900.0), Vector2(725.0, 730.0), scale): return false
	if not await wait_drawer_size(Vector2(827.0, 800.0), scale): return false
	if not await record_drawer("storage-drawer-max"): return false
	if not await resize_drawer(Vector2(200.0, 500.0), Vector2(827.0, 800.0), scale): return false
	if not await wait_drawer_size(Vector2(415.0, 651.0), scale): return false
	if not await record_drawer("storage-drawer-min"): return false
	if not await resize_drawer(Vector2(700.0, 730.0), Vector2(415.0, 651.0), scale): return false
	if not await wait_drawer_size(Vector2(725.0, 730.0), scale): return false
	if not await type_storage_search("Tesla"): return false
	if not await click_exposed_control(housing_control("HouseEditorStorageFilters")): return false
	if not await click_exposed_control(housing_control("HouseEditorStorageDyeable")): return false
	if not await wait_housing(func(): return housing_control("HouseEditorStorageEmpty") != null and housing_control("HouseEditorStorageDyeableTick") != null, "source drawer filter state"): return false
	if not await click_exposed_control(housing_control("HouseEditorStorageCollapseButton")): return false
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") == null and housing_control("HouseEditorStorageExpandButton") != null, "source collapsed drawer"): return false
	if not await record_drawer("storage-drawer-collapsed"): return false
	if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return false
	if not await wait_housing(func(): return storage_search_text() == "Tesla" and housing_control("HouseEditorStorageDyeableTick") != null and housing_control("HouseEditorStorageEmpty") != null, "expand retains search and filter"): return false
	if not await click_exposed_control(housing_control("HouseEditorStorageResetFilters")): return false
	if not await click_exposed_control(housing_control("HouseEditorStorageFilters")): return false
	if not await type_storage_search(""): return false
	if not await click_exposed_control(housing_control("HouseEditorStorageCategory2")): return false
	if not await wait_housing(func(): return housing_text("HouseEditorStorageCategoryPath") == "Structural", "category before collapse"): return false
	if not await click_exposed_control(housing_control("HouseEditorStorageCollapseButton")): return false
	if not await wait_housing(func(): return housing_control("HouseEditorStorageExpandButton") != null, "saved collapsed category drawer"): return false
	if not await enter_cleanup(): return false
	if housing_control("HouseEditorStorageExpandButton") != null:
		fail("Cleanup exposed collapsed storage handle")
		return false
	if not await click_exposed_control(housing_control("HouseEditorBasicDecorModeButton")): return false
	if not await wait_housing(func(): return housing_control("HouseEditorStorageExpandButton") != null, "Basic preserves collapsed preference"): return false
	if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return false
	if not await wait_housing(func(): return housing_text("HouseEditorStorageCategoryPath") == "Structural", "collapse retains source category"): return false
	if not await wait_drawer_size(Vector2(725.0, 730.0), scale): return false
	if not await record_drawer("storage-drawer-expanded"): return false
	if not await click_exposed_control(housing_control("HouseEditorStorageCollapseButton")): return false
	if not await wait_housing(func(): return housing_control("HouseEditorStorageExpandButton") != null, "persist collapsed preference before fresh process"): return false
	return drawer_conserved(before)

func prove_drawer_reload(before: Dictionary) -> bool:
	if housing_control("HouseEditorStorageFrame") != null or housing_control("HouseEditorStorageExpandButton") == null:
		fail("Fresh process did not restore collapsed preference")
		return false
	if not await record_drawer("storage-drawer-relogged-collapsed"): return false
	if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return false
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null, "fresh-process expanded drawer"): return false
	var scale: float = housing_control("HouseEditorStorageResizeButton").get_global_rect().size.x / 20.0
	if not await wait_drawer_size(Vector2(725.0,730.0), scale): return false
	if not await record_drawer("storage-drawer-relogged-expanded"): return false
	return drawer_conserved(before)

func resize_drawer(target: Vector2, initial: Vector2, scale: float) -> bool:
	var handle: Control = housing_control("HouseEditorStorageResizeButton")
	if handle == null:
		fail("Missing source bottom-right resize input")
		return false
	var start: Vector2 = handle.get_global_rect().get_center()
	await hover_point(start)
	await drawer_mouse_button(start, true)
	push_mouse_motion(start + (target - initial) * scale)
	await frames(20)
	# Release outside the original handle. Capture must survive reflow.
	await drawer_mouse_button(start + (target - initial) * scale, false)
	await frames(20)
	return true

func drawer_mouse_button(point: Vector2, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = pressed
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
	root.push_input(event, true)
	await frames(3)

func wait_drawer_size(expected: Vector2, scale: float) -> bool:
	return await wait_housing(func():
		var frame: Control = housing_control("HouseEditorStorageFrame")
		return frame != null and frame.get_global_rect().size.distance_to(expected * scale) < 0.2, "source drawer size " + str(expected))

func drawer_conserved(before: Dictionary) -> bool:
	if client.housing_state().stored != before.stored or client.housing_state().placed != before.placed or client.housing_state().decor != before.decor:
		fail("Drawer interaction changed confirmed decor or quantities")
		return false
	return true

func record_drawer(phase: String) -> bool:
	await frames(20)
	if not await record_editor(phase): return false
	var frame: Control = housing_control("HouseEditorStorageFrame")
	drawer_receipts.append({"phase":phase, "skin":housing_skin, "frame_rect":str(frame.get_global_rect()) if frame != null else "collapsed", "expand":housing_control("HouseEditorStorageExpandButton") != null, "search":storage_search_text(), "category":housing_text("HouseEditorStorageCategoryPath"), "housing":client.housing_state()})
	var file: FileAccess = FileAccess.open(shots.path_join("drawer-receipts.json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot persist drawer receipts")
		return false
	file.store_string(JSON.stringify(drawer_receipts,"\t"))
	file.close()
	return true
