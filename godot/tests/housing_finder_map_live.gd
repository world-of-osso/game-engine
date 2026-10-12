extends "res://tests/housing_collection_live.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty():
		fail("Finder proof requires private5870/fb_housing_* configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingDashboardHouseFinderButton") != null and not client.housing_state().get("pending", true), "confirmed housing dashboard"): return
	await click_control(housing_control("HousingDashboardHouseFinderButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseFinderMapTile0") != null and not client.housing_state().pending and client.housing_state().get("plots", []).size() == 55, "source map and authoritative plots"): return
	await frames(60)
	var expected_sale: int = 0
	var expected_friends: int = 0
	for plot in client.housing_state().plots:
		if int(plot.owner_type) == 0: expected_sale += 1
		if int(plot.owner_type) == 2: expected_friends += 1
	if expected_friends < 1 or pin_count("HouseFinderPlotForSalePin") != expected_sale or pin_count("HouseFinderFriendsPlotPin") != expected_friends:
		fail("Retail map must show exactly actual sale/friend relationships")
		return
	if not await capture_housing("finder-map"): return
	var target: Dictionary = {}
	for plot in client.housing_state().plots:
		if int(plot.plot_id) == 419: target = plot
	if target.is_empty() or int(target.owner_type) != 0:
		fail("Source419 must be genuinely available for unpaid visit proof")
		return
	var money: int = int(client.merchant_state().money)
	var owned: int = client.housing_state().houses.size()
	await click_control(housing_control("HouseFinderPlotForSalePin419"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func():
		var visit: BaseButton = housing_control("HouseFinderPlotVisitButton") as BaseButton
		return visit != null and not visit.disabled and housing_text("HouseFinderPlotLabel") == "Plot 2", "selected authored plot info"): return
	if not await capture_housing("finder-selected"): return
	await click_control(housing_control("HouseFinderPlotVisitButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return int(client.account_state().world_map_id) == 2735 and client.account_state().gameplay_input_allowed, "source plot transfer", 420000): return
	await frames(30)
	var expected: Vector3 = Vector3(target.teleport.x, target.teleport.z, -target.teleport.y)
	var position: Vector3 = client.account_state().local_server_position
	if position.distance_to(expected) > 1.0 or int(client.merchant_state().money) != money:
		fail("Finder visit must use exact source teleport without payment")
		return
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return not client.housing_state().pending, "post-visit authoritative ownership"): return
	if client.housing_state().houses.size() != owned:
		fail("Finder visit granted property")
		return
	var receipt: FileAccess = FileAccess.open(shots.path_join("finder-receipt.json"), FileAccess.WRITE)
	if receipt == null:
		fail("Cannot write Finder receipt")
		return
	receipt.store_string(JSON.stringify({"skin": housing_skin, "sale_pins": expected_sale, "friend_pins": expected_friends, "plot_id": 419, "money": money, "owned_before": owned, "state": client.housing_state(), "account": client.account_state()}, "\t"))
	if not await capture_housing("finder-arrived"): return
	print("PASS: private5870 sourced Finder map/sale/friend pins, native plot selection and unpaid source arrival ", housing_skin)
	client.free()
	quit(0)

func pin_count(prefix: String) -> int:
	var count: int = 0
	for node in client.get_node("HousingDashboardUI").find_children(prefix + "*", "", true, false):
		if node is BaseButton: count += 1
	return count
