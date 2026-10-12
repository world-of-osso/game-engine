extends "res://tests/housing_property_live.gd"

# Source board576398, normal world-pointer interaction and Retail roster controls.
# External runner primes only its authenticated DB2 position after actual map entry.
func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	property_entry = 576398
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty():
		fail("Bulletin proof requires private5870/fb_housing_* configuration")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await enter_neighborhood(): return
	if not await inspect_board("roster"): return
	await click_control(housing_control("NeighborhoodRosterColumnPlotButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_text("NeighborhoodRosterEntry1Plot") == "#0" and housing_text("NeighborhoodRosterEntry2Plot") == "#1", "plot order"): return
	if not await capture_housing("plot-order"): return
	await click_control(housing_control("NeighborhoodRosterColumnNameButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_text("NeighborhoodRosterEntry1Name") == "Houseproof", "alphabetical order"): return
	await click_control(housing_control("HousingBulletinBoardFrameCloseButton"), MOUSE_BUTTON_LEFT)
	if not await connect_housing_account(): return
	if not await inspect_board("relogged"): return
	print("PASS: private5870 physical authored bulletin board, real residents, native column sorts and relog ", housing_skin)
	client.free()
	quit(0)

func inspect_board(phase: String) -> bool:
	var board: Dictionary = await find_cornerstone()
	if board.is_empty(): return false
	await click(board.point, MOUSE_BUTTON_RIGHT)
	if not await wait_housing(func(): return housing_text("HousingBulletinBoardNeighborhoodName") == "Founder's Point", "authoritative bulletin roster", 180000): return false
	if housing_text("NeighborhoodRosterEntry1Name") != "Houseproof" or housing_text("NeighborhoodRosterEntry2Name") != "Housevisitor":
		fail("Board must show actual paid owners alphabetically")
		return false
	var expected_online: int = 1 if housing_skin == "Modern" else 2
	for row in [1, 2]:
		var status: String = "Online" if row == expected_online else "Offline"
		if housing_text("NeighborhoodRosterEntry%dStatus" % row) != status:
			fail("Board status does not match the real connected character")
			return false
	var receipt: FileAccess = FileAccess.open(shots.path_join(phase + ".json"), FileAccess.WRITE)
	if receipt == null:
		fail("Cannot save board receipt")
		return false
	receipt.store_string(JSON.stringify({"skin": housing_skin, "account": client.account_state(), "board_entry": property_entry, "first": housing_text("NeighborhoodRosterEntry1Name"), "second": housing_text("NeighborhoodRosterEntry2Name"), "online_row": expected_online}, "\t"))
	return await capture_housing(phase)
