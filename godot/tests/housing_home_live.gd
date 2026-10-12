extends "res://tests/housing_property_live.gd"

# Real authored Home Interior map, not an inferred room/exterior component grant.
func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	var house_id: int = int(OS.get_environment("HOUSING_HOUSE_ID"))
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or house_id <= 0:
		fail("Home proof requires explicit private5870/fb_housing_* house identity")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingHomeVisit%d" % house_id) != null, "owned visit control"): return
	await click_control(housing_control("HousingHomeVisit%d" % house_id), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.account_state().screen == "InWorld" and int(client.account_state().world_map_id) == 2783, "real authored Home Interior admission", 420000): return
	await frames(120)
	if not await capture_housing("home-entry"): return
	property_receipts.append({"phase":"home-entry", "house_id":house_id, "state":client.account_state()})
	if not await connect_housing_account(): return
	if int(client.account_state().world_map_id) != 2783:
		fail("Persisted visited-house identity was lost: " + str(client.account_state()))
		return
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingHomeLeave") != null and housing_control("HousingHomeLeave").is_visible_in_tree(), "restored house layout"): return
	property_receipts.append({"phase":"home-relogged", "house_id":house_id, "state":client.account_state(), "status":housing_text("HouseInfoContentUnavailable")})
	if not await capture_housing("home-relogged"): return
	var file: FileAccess = FileAccess.open(shots.path_join("home-receipts.json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot record visited-house proof")
		return
	file.store_string(JSON.stringify(property_receipts, "\t"))
	file.close()
	await click_control(housing_control("HousingHomeLeave"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.account_state().screen == "InWorld" and int(client.account_state().world_map_id) == 2735, "authored plot return", 420000): return
	if not await capture_housing("home-left"): return
	print("PASS: private5870 authored Home Interior, no component grants, persisted visited identity and plot return ", housing_skin)
	client.free()
	quit(0)
