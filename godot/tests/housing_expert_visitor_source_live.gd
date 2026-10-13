extends "res://tests/housing_expert_source_live.gd"

func open_friend_plot_info(house: int) -> bool:
	if not await wait_housing(func():
		var pin: BaseButton = friend_plot_control(house) as BaseButton
		return pin!=null and not pin.disabled,"authoritative friend Finder projection",120000): return false
	if not await click_exposed_control(friend_plot_control(house)): return false
	return await wait_housing(func(): return housing_control("HouseFinderPlotVisitButton")!=null,"source friend visit control",120000)

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_VISIT_HOUSE"))
	cleanup_role = "visitor"
	if housing_server!="127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house<=0:
		fail("Expert visitor proof requires an explicit other-account private house")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if before.can_edit or before.decor.size()!=1 or int(before.decor[0].decor_id)!=1693:
		fail("Expert visitor proof needs genuine read-only placed paid bookcase")
		return
	if not await configure_expert_bindings(false): return
	if not await enter_expert(): return
	if not await select_saved_decor(): return
	if not await wait_housing(func(): return int(client.housing_state().get("selected",0))==int(before.decor[0].instance_id),"visitor may inspect actual placed instance"): return
	for name in ["TranslateSubmodeButton","RotateSubmodeButton","ScaleSubmodeButton"]:
		var button: BaseButton = housing_control(name) as BaseButton
		if button==null or not button.disabled:
			fail("Visitor precision control is not source-disabled: "+name)
			return
	for key: Key in [KEY_F9,KEY_F6,KEY_F11,KEY_F12,KEY_F5,KEY_F8,KEY_F7]:
		await source_key(key)
		await frames(10)
		if client.housing_state().has("draft") or client.housing_state().pending or not unchanged_visitor(before):
			fail("Assigned Expert visitor action staged or mutated owner decor")
			return
	if not await record_editor("expert-visitor-source-refused"): return
	if not await commit_expert_by_world_click(): return
	await frames(20)
	if not unchanged_visitor(before) or client.housing_state().pending:
		fail("Visitor world click persisted owner decor")
		return
	await press_escape()
	if not await configure_expert_bindings(true): return
	if not await connect_housing_account(): return
	if not await ready_dashboard(): return
	if not unchanged_visitor(before) or int(client.housing_state().house_id)!=editor_house:
		fail("Expert visitor relog lost read-only authority or conserved copies")
		return
	if not await record_editor("expert-visitor-source-relogged"): return
	print("PASS: private5870 assigned Expert visitor disabled controls/no draft/no commit/relog ",housing_skin)
	client.free()
	quit(0)

func unchanged_visitor(before: Dictionary) -> bool:
	var state: Dictionary = client.housing_state()
	return not state.can_edit and state.decor==before.decor and state.stored==before.stored and state.placed==before.placed
