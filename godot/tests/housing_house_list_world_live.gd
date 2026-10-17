extends "res://tests/housing_collection_live.gd"

var house_list_world_receipts: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty():
		fail("House list WORLD proof requires private owner configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var first_houses: Array = []
	for phase in ["owned", "relogged"]:
		if not await connect_housing_account(): return
		if not await wait_housing(func(): return int(client.account_state().world_map_id) == 2735 and client.account_state().terrain.parsed_tiles.size() > 0, "persisted neighborhood WORLD", 180000): return
		await frames(60)
		if not await capture_world_list(phase+"-world"): return
		if not await click_exposed_control(housing_named_control("MicroMenuUI", "HousingMicroButton")): return
		if not await wait_housing(func(): return client.housing_state().get("confirmed", false) and housing_control("HousingDashboardHouseFinderButton") != null, "confirmed native house list"): return
		var state: Dictionary = client.housing_state()
		var houses: Array = state.houses
		if houses.is_empty():
			fail("House list proof requires actual persisted claimed ownership")
			return
		for house in houses:
			var row := housing_control("HousingHomeVisit%d" % int(house.house_id)) as BaseButton
			if row == null or row.disabled or not row.is_visible_in_tree():
				fail("Confirmed house lacks its actual native list action: "+str(house))
				return
		if phase == "owned":
			first_houses = houses.duplicate(true)
		elif houses != first_houses:
			fail("House list identity changed across actual relog")
			return
		house_list_world_receipts.append({"phase":phase,"skin":housing_skin,"account":client.account_state(),"houses":houses,"plots":state.plots})
		if not await capture_world_list(phase+"-house-list"): return
		if not await click_exposed_control(housing_control("HousingDashboardFrameCloseButton")): return
	var file := FileAccess.open(shots.path_join("house-list-world-receipts.json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot write house-list WORLD receipt")
		return
	file.store_string(JSON.stringify(house_list_world_receipts, "\t"))
	file.flush()
	var error := file.get_error()
	file.close()
	if error != OK:
		fail("Cannot flush house-list WORLD receipt")
		return
	print("PASS: private5870 persisted house list WORLD ",housing_skin)
	client.free()
	quit(0)

func capture_world_list(phase: String) -> bool:
	await RenderingServer.frame_post_draw
	if root.get_texture().get_image().save_png(shots.path_join(housing_skin.to_lower()+"-"+phase+".png")) != OK:
		fail("Cannot save house-list WORLD PNG")
		return false
	return true
