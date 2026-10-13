extends "res://tests/housing_storage_transitions_live.gd"

var affordance_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	cleanup_role = "owner"
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0:
		fail("Drawer affordance proof needs explicit private5870/fb_housing_* owned house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await ready_dashboard(): return
	if client.housing_state().get("house_id",0) != editor_house or not client.housing_state().get("can_edit",false):
		fail("Drawer affordance proof needs saved paid owned house")
		return
	if not await click_exposed_control(housing_control("HousingDecorateButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null or housing_control("HouseEditorStorageExpandButton") != null, "affordance entry"): return
	if housing_control("HouseEditorStorageExpandButton") != null:
		if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null and absf(housing_control("HouseEditorStorageFrame").global_position.x)<0.01, "settled source drawer"): return
	var before: Dictionary = client.housing_state()
	var handle: Control = housing_control("HouseEditorStorageResizeButton")
	if handle == null:
		fail("Missing source resize control")
		return
	await hover_point(handle.get_global_rect().get_center())
	await frames(10)
	if Input.get_current_cursor_shape() != Input.CURSOR_FDIAGSIZE:
		fail("Native source resize cursor not active: " + str(Input.get_current_cursor_shape()))
		return
	await hover_point(Vector2(1100.0,500.0))
	await frames(10)
	if Input.get_current_cursor_shape() == Input.CURSOR_FDIAGSIZE:
		fail("Resize cursor persisted outside its source handle")
		return
	if not await prove_handle_glow("HouseEditorStorageCollapseButton","storage-collapse-hover"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageCollapseButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageExpandButton") != null and absf(housing_control("HouseEditorStorageExpandButton").global_position.x)<0.01, "settled source open handle"): return
	if not await prove_handle_glow("HouseEditorStorageExpandButton","storage-expand-hover"): return
	if not await click_exposed_control(housing_control("HouseEditorStorageExpandButton")): return
	if not await wait_housing(func(): return housing_control("HouseEditorStorageFrame") != null and absf(housing_control("HouseEditorStorageFrame").global_position.x)<0.01, "restored source drawer"): return
	if not drawer_conserved(before): return
	print("PASS: private5870 owned-storage affordances ",housing_skin)
	client.free()
	quit(0)

func prove_handle_glow(name: String, phase: String) -> bool:
	await hover_point(Vector2(1100.0,500.0))
	await frames(15)
	await RenderingServer.frame_post_draw
	var before: Image = root.get_texture().get_image()
	var control: Control = housing_control(name)
	var rect: Rect2 = control.get_global_rect()
	await hover_point(rect.get_center())
	await frames(15)
	await RenderingServer.frame_post_draw
	var after: Image = root.get_texture().get_image()
	var brighter: int = 0
	for y in range(int(rect.position.y),int(rect.end.y)):
		for x in range(maxi(0,int(rect.position.x)),mini(after.get_width(),int(rect.end.x))):
			var old: Color = before.get_pixel(x,y)
			var new: Color = after.get_pixel(x,y)
			if new.r+new.g+new.b > old.r+old.g+old.b+0.03: brighter += 1
	if brighter < 6:
		fail("Source ADD hover overlay did not brighten actual handle: " + name)
		return false
	if not await capture_housing(phase): return false
	affordance_receipts.append({"phase":phase,"handle":name,"rect":str(rect),"brighter_pixels":brighter,"cursor":Input.get_current_cursor_shape(),"housing":client.housing_state()})
	var file: FileAccess = FileAccess.open(shots.path_join("affordance-receipts.json"),FileAccess.WRITE)
	if file == null:
		fail("Cannot persist affordance receipts")
		return false
	file.store_string(JSON.stringify(affordance_receipts,"\t"))
	file.close()
	return true
