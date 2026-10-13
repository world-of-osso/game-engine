extends "res://tests/housing_basic_source_live.gd"

func open_friend_plot_info(house: int) -> bool:
	var related: Array = []
	for plot in client.housing_state().get("plots",[]):
		if int(plot.house_id)==house:
			related.append(plot)
	print("VISITOR_FINDER_DIAGNOSTIC target=",house," account=",housing_account," related=",related," homes=",client.housing_state().get("houses",[])," map=",client.account_state().world_map_id)
	var started: int = Time.get_ticks_msec()
	var reported: bool = false
	while Time.get_ticks_msec()-started<120000:
		var pin: BaseButton = friend_plot_control(house) as BaseButton
		if pin!=null and not pin.disabled:
			if not await click_exposed_control(pin): return false
			return await wait_housing(func(): return housing_control("HouseFinderPlotVisitButton")!=null,"source friend plot visit control",120000)
		if not reported and Time.get_ticks_msec()-started>5000:
			reported = true
			print("VISITOR_PIN_WAIT pending=",client.housing_state().get("pending",true)," pin=",pin," finder=",housing_control("HouseFinderFrame")," info=",housing_control("HouseFinderPlotVisitButton"))
		await process_frame
	fail("Housing timeout: authoritative friend pin after120s; no relationship synthesized")
	return false

func run_test() -> void:
	root.size = Vector2i(1280,720)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	editor_house = int(OS.get_environment("HOUSING_VISIT_HOUSE"))
	cleanup_role = "visitor"
	if housing_server!="127.0.0.1:5870" or not housing_account.begins_with("fb_housing_") or shots.is_empty() or editor_house<=0:
		fail("Visitor proof needs private5870/fb_housing_* explicit other-account home")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await open_cleanup_house(): return
	var before: Dictionary = client.housing_state()
	if before.can_edit or before.decor.size()!=1 or int(before.decor[0].decor_id)!=1693:
		fail("Visitor source proof needs authoritative read-only real paid bookcase")
		return
	var tile: BaseButton = housing_control("HouseEditorStored2330") as BaseButton
	if tile==null or not tile.disabled:
		fail("Other-account storage drag must be source-disabled")
		return
	var start: Vector2 = tile.get_global_rect().get_center()
	push_mouse_motion(start)
	await tile_mouse_button(start,true)
	await held_motion(start+Vector2(35,0),Vector2(35,0))
	if int(client.housing_state().get("placing",0))!=0 or root.gui_is_dragging():
		fail("Disabled visitor tile began an owned placement drag")
		return
	await tile_mouse_button(ground_point(7.0),false)
	if not await select_saved_decor(): return
	if not await wait_housing(func(): return int(client.housing_state().get("selected",0))==int(before.decor[0].instance_id),"visitor may inspect real picked instance"): return
	if not await source_instructions(): return
	if not await record_editor("basic-visitor-source-selected"): return
	await tile_mouse_button(ground_point(7.0),true)
	await tile_mouse_button(ground_point(7.0),false)
	await frames(20)
	if client.housing_state().decor!=before.decor or client.housing_state().stored!=before.stored or client.housing_state().get("pending",true):
		fail("Visitor release mutated owner placement or account storage")
		return
	await press_escape()
	if not await wait_housing(func(): return int(client.housing_state().get("selected",0))==0 and housing_control("HouseEditorStored2330")!=null,"visitor Escape restores read-only source tile"): return
	if not await record_editor("basic-visitor-release-refused"): return
	if not await connect_housing_account(): return
	if not await ready_dashboard(): return
	var restored: Dictionary = client.housing_state()
	if restored.get("house_id",0)!=editor_house or restored.get("can_edit",true) or restored.decor!=before.decor or restored.stored!=before.stored:
		fail("Visitor relog lost source authority/conservation")
		return
	if not await record_editor("basic-visitor-relogged"): return
	print("PASS: private5870 source Basic visitor disabled drag/overlay refusal/relog ",housing_skin)
	client.free()
	quit(0)
