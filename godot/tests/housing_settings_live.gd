extends "res://tests/housing_editor_live.gd"

var access_mode: String
var access_role: String

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_HOUSE_ID"))
	access_mode = OS.get_environment("HOUSING_ACCESS_MODE")
	access_role = OS.get_environment("HOUSING_ACCESS_ROLE")
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house <= 0 or not access_mode in ["anyone", "none", "friends"] or not access_role in ["owner", "visitor"]:
		fail("Settings proof requires private5870/fb_housing_* explicit house/role/mode")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if access_role == "owner":
		if not await prove_owner_settings(): return
	else:
		if not await prove_visitor_access(): return
	print("PASS: private5870 HouseSettings ", access_role, " ", access_mode, " ", housing_skin)
	client.free()
	quit(0)

func open_settings() -> bool:
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func():
		var button: BaseButton = housing_control("HousingSettings%d" % editor_house) as BaseButton
		return button != null and not button.disabled and client.housing_state().get("confirmed", false) and not client.housing_state().get("pending", true), "confirmed owned settings"): return false
	await frames(5)
	await click_control(housing_control("HousingSettings%d" % editor_house), MOUSE_BUTTON_LEFT)
	return await wait_housing(func(): return housing_control("HousingSettingsHouseMode") != null, "HouseSettings source panel")

func choose_mode(mode: String) -> bool:
	await click_control(housing_control("HousingSettingsHouseMode"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingSettingsMode" + mode) != null, "access mode dropdown"): return false
	await click_control(housing_control("HousingSettingsMode" + mode), MOUSE_BUTTON_LEFT)
	return await wait_housing(func(): return housing_control("HousingSettingsAccessMenu") == null, "closed mode dropdown")

func confirmed_access() -> Variant:
	for house in client.housing_state().get("houses", []):
		if int(house.house_id) == editor_house: return house.get("access")
	return null

func expected_access() -> Dictionary:
	var anyone: bool = access_mode == "anyone"
	return {"anyone": anyone, "neighbors": anyone, "guild": anyone, "friends": anyone or access_mode == "friends", "party": anyone}

func prove_owner_settings() -> bool:
	if not await open_settings(): return false
	var initial: Variant = confirmed_access()
	var before: Dictionary = client.housing_state()
	if not await choose_mode("none"): return false
	await frames(10)
	if confirmed_access() != initial:
		fail("Staged HouseSettings changed server permission before Save")
		return false
	if not await record_editor("staged-unsaved"): return false
	await click_control(housing_control("HousingSettingsClose"), MOUSE_BUTTON_LEFT)
	if not await open_settings(): return false
	if confirmed_access() != initial:
		fail("Closing HouseSettings saved its draft")
		return false
	if access_mode == "friends":
		if not await choose_mode("none"): return false
		if not await choose_mode("limited"): return false
		await click_control(housing_control("HousingSettingsFlagfriends"), MOUSE_BUTTON_LEFT)
		await frames(5)
	else:
		if not await choose_mode(access_mode): return false
	if confirmed_access() != initial:
		fail("Selection/toggle changed server state before Save")
		return false
	if not await record_editor("selected-unsaved"): return false
	await click_control(housing_control("HousingSettingsSave"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.get_node_or_null("HousingDashboardUI") == null and confirmed_access() == expected_access(), "authoritative saved flags"): return false
	var after: Dictionary = client.housing_state()
	if after.stored != before.stored or after.placed != before.placed or after.get("decor", []) != before.get("decor", []):
		fail("Saving access changed decor quantities/layout")
		return false
	if not await connect_housing_account(): return false
	if not await open_settings(): return false
	if confirmed_access() != expected_access():
		fail("Saved access did not survive real reauthentication")
		return false
	var expected_label: String = {"anyone": "Anyone", "none": "No One", "friends": "Limited"}[access_mode]
	var mode_button: Control = housing_control("HousingSettingsHouseMode")
	var mode_label: Label = mode_button.find_child("HousingSettingsHouseModeText", true, false) as Label if mode_button != null else null
	if mode_label == null or mode_label.text != expected_label:
		fail("Reopened mode differs from saved Retail settings")
		return false
	return await record_editor("saved-relogged")

func prove_visitor_access() -> bool:
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HousingDashboardHouseFinderButton") != null and client.housing_state().get("confirmed", false) and not client.housing_state().get("pending", true), "visitor housing dashboard"): return false
	if client.housing_state().get("house_id", 0) > 0:
		await click_control(housing_control("HousingHomeLeave"), MOUSE_BUTTON_LEFT)
		if not await wait_housing(func(): return client.account_state().screen == "InWorld" and int(client.account_state().world_map_id) == 2735, "leave previous house"): return false
		await frames(20)
		await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
		if not await wait_housing(func(): return housing_control("HousingDashboardHouseFinderButton") != null and not client.housing_state().get("pending", true), "fresh visitor dashboard"): return false
	await frames(5)
	await click_control(housing_control("HousingDashboardHouseFinderButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return housing_control("HouseFinderFrame") != null and not client.housing_state().get("pending", true), "ready friend Finder"): return false
	print("HOUSE_ACCESS_FINDER ", JSON.stringify(client.housing_state()))
	if not await open_friend_plot_info(editor_house): return false
	var before_map: int = int(client.account_state().world_map_id)
	await click_control(housing_control("HouseFinderPlotVisitButton"), MOUSE_BUTTON_LEFT)
	if access_mode == "none":
		if not await wait_housing(func(): return client.housing_state().get("error", "").contains("not allowed") and not client.housing_state().get("pending", true), "denied visitor reply"): return false
		if int(client.account_state().world_map_id) != before_map or client.housing_state().get("house_id", 0) == editor_house:
			fail("Denied visitor entered the owner's house")
			return false
		if not await record_editor("visitor-denied"): return false
		if not await connect_housing_account(): return false
		await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
		if not await wait_housing(func(): return client.housing_state().get("confirmed", false), "denied visitor relog"): return false
		if int(client.account_state().world_map_id) != before_map or client.housing_state().get("house_id", 0) == editor_house:
			fail("Denied admission became a persisted house grant")
			return false
		return await record_editor("visitor-denied-relogged")
	if not await wait_housing(func(): return client.account_state().screen == "InWorld" and int(client.account_state().world_map_id) == 2783 and housing_named_control("MicroMenuUI", "HousingMicroButton") != null, "allowed visitor arrival", 420000): return false
	await frames(20)
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.housing_state().get("house_id", 0) == editor_house and not client.housing_state().get("can_edit", true), "read-only allowed layout"): return false
	if not await record_editor("visitor-allowed"): return false
	if not await connect_housing_account(): return false
	await click_control(housing_named_control("MicroMenuUI", "HousingMicroButton"), MOUSE_BUTTON_LEFT)
	if not await wait_housing(func(): return client.housing_state().get("house_id", 0) == editor_house and not client.housing_state().get("can_edit", true), "allowed visitor saved admission"): return false
	if not await record_editor("visitor-allowed-relogged"): return false
	await click_control(housing_control("HousingHomeLeave"), MOUSE_BUTTON_LEFT)
	return await wait_housing(func(): return client.account_state().screen == "InWorld" and int(client.account_state().world_map_id) == 2735, "visitor leaves before owner changes permissions")
