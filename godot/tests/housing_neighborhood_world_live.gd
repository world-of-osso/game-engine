extends "res://tests/housing_collection_live.gd"

# Authored House Finder admission; no external position priming or synthetic ground.
func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty():
		fail("Neighborhood WORLD proof requires private5870 account and evidence path")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingDashboardHouseFinderButton") != null, "House Finder entry"): return
	await click_control(housing_control("HousingDashboardHouseFinderButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func():
		var visit := housing_control("HouseFinderVisitButton") as BaseButton
		return visit != null and not visit.disabled, "authored neighborhood destination"): return
	await click_control(housing_control("HouseFinderVisitButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.account_state().screen == "InWorld" and int(client.account_state().world_map_id) == 2735, "authored Founder entry", 420000): return
	var close := housing_control("HousingDashboardFrameCloseButton")
	if close != null and close.is_visible_in_tree():
		await click_control(close, MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.account_state().terrain.parsed_tiles.size() > 0, "real neighborhood ADT ground", 180000): return
	await frames(120)
	var state: Dictionary = client.account_state()
	var position: Vector3 = state.local_player_position
	# Retail coordinates (3807.76001,-160.427002,194.110992), converted to Y-up.
	var expected := Vector3(160.427002, 194.110992, -3807.76001)
	print("NEIGHBORHOOD_ENTRY_POSITION actual=", position, " expected=", expected, " terrain=", state.terrain)
	var output := FileAccess.open(shots.path_join("neighborhood-world-receipt.json"), FileAccess.WRITE)
	if output == null:
		fail("Cannot persist neighborhood world receipt")
		return
	output.store_string(JSON.stringify({"skin":housing_skin,"account":state,"housing":client.housing_state()}, "\t"))
	output.flush()
	var error := output.get_error()
	output.close()
	if error != OK:
		fail("Cannot flush neighborhood world receipt")
		return
	for phase in ["entry", "entry-turned"]:
		if phase == "entry-turned":
			push_key(KEY_RIGHT, true)
			await frames(25)
			push_key(KEY_RIGHT, false)
			await frames(30)
		await RenderingServer.frame_post_draw
		if root.get_texture().get_image().save_png(shots.path_join(housing_skin.to_lower()+"-neighborhood-world-"+phase+".png")) != OK:
			fail("Cannot save neighborhood WORLD image")
			return
	print("PASS: private5870 authored neighborhood WORLD ", housing_skin)
	client.free()
	quit(0)
