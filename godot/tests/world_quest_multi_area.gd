extends "res://tests/world_quest_navigation.gd"
# Real82521 availability: sourceXYZ lower2256 + main2255 under one task identity.
# Controlled setup teleports only the player; no credit, NPC or polygon mutations.
const MULTI_TASK := 82521

func has_multi_task() -> bool:
	for entry in client.quest_state().log:
		if entry.quest_id == MULTI_TASK:
			return true
	return false

func wait_map_pin(map_id: int) -> bool:
	var deadline := Time.get_ticks_msec() + 45000
	while Time.get_ticks_msec() < deadline:
		var state: Dictionary = client.world_map_state()
		if state.get("open", false) and state.get("map_id", 0) == map_id and pin_index(MULTI_TASK) >= 0:
			var matches := 0
			for pin in state.pins:
				if pin.quest_id == MULTI_TASK and pin.type == "WorldQuest":
					matches += 1
			if matches != 1:
				stop("Duplicate task pin: " + str(state))
				return false
			print("WORLD_QUEST_MULTI_MAP ", JSON.stringify(state))
			return true
		await process_frame
	stop("Expected map/pin absent map=%s state=%s" % [map_id, client.world_map_state()])
	return false

func enter_authored_lower_area() -> bool:
	# Inside blob514681 but outside earlier overlapping tasks: 90% of its
	# authored vertex(-2663,-1796,-1606) +10% of its source centroid.
	var response := admin({"Teleport": {"character_name": character, "map_id": 2601, "x": -2623.54, "y": -1812.49, "z": -1604.57}})
	if response.is_empty() or response.has("Error"):
		stop("SourceXYZ player teleport failed")
		return false
	var deadline := Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		var watched = client.quest_state().watched
		var position = client.account_state().local_player_position
		var arrived := position != null and Vector2(position.x + 2623.54, position.z - 1812.49).length() < 8.0
		if has_multi_task() and watched.size() == 1 and watched[0] == MULTI_TASK and arrived and client.account_state().screen == "InWorld":
			await settle()
			print("WORLD_QUEST_MULTI_ARRIVAL ", JSON.stringify(client.quest_state()))
			return true
		await process_frame
	stop("Authored polygon arrival was not accepted")
	return false

func select_lower_pin() -> bool:
	await key(KEY_M)
	if not await wait_map_pin(2256):
		return false
	if not await capture_phase("lower-floor-source-height"):
		return false
	if not await click("WorldMapPin%s" % pin_index(MULTI_TASK)):
		return false
	if client.objective_tracker_state().super_tracked != MULTI_TASK:
		stop("Physical lower-floor pin did not select the task")
		return false
	return await capture_phase("lower-floor-selected")

func navigate_to_main_area() -> bool:
	var state: Dictionary = client.world_map_state()
	var parent_index: int = state.breadcrumb_ids.find(2274)
	if parent_index < 0 or not await click("WorldMapNav%s" % parent_index):
		stop("Authored Khaz Algar breadcrumb missing")
		return false
	if not await wait_map_pin(2274):
		return false
	if not await click_main_zone():
		return false
	if not await wait_map_pin(2255):
		return false
	if client.objective_tracker_state().super_tracked != MULTI_TASK:
		stop("Cross-map navigation lost selected quest identity")
		return false
	return await capture_phase("main-area-same-task")

func click_main_zone() -> bool:
	# Observe the actual authored zone highlight, avoiding every received pin.
	# A blind offset from82521 can select another of the82 continent task pins.
	var canvas := control("WorldMapCanvas")
	var state: Dictionary = client.world_map_state()
	var origin: Vector2 = state.pins[pin_index(MULTI_TASK)].uv
	for offset in [Vector2(-0.05, 0.02), Vector2(0.05, 0.02), Vector2(-0.1, -0.03), Vector2(0.1, -0.03), Vector2(0.0, 0.08)]:
		var uv: Vector2 = origin + offset
		var clear := true
		for pin in state.pins:
			if ((pin.uv - uv) * canvas.get_global_rect().size).length() < 32.0:
				clear = false
		if not clear:
			continue
		var point: Vector2 = canvas.get_global_rect().position + uv * canvas.get_global_rect().size
		var motion := InputEventMouseMotion.new()
		motion.position = point
		motion.global_position = point
		root.push_input(motion, true)
		await settle()
		if client.world_map_state().highlight != "Azj-Kahet":
			continue
		print("WORLD_QUEST_MULTI_ZONE_TARGET uv=", uv, " highlight=Azj-Kahet pinsClear=true")
		for pressed in [true, false]:
			await pointer_button(point, pressed)
		await settle()
		return true
	stop("No observed empty Azj-Kahet canvas target among bounded candidates")
	return false

func expire_received_areas() -> bool:
	var end := int(OS.get_environment("WQ_EPOCH")) + 86400
	var seconds := end - int(Time.get_unix_time_from_system()) + 2
	var response := admin({"ClockAdvance": {"seconds": seconds}})
	if response.is_empty() or response.has("Error"):
		stop("Private real-clock advance failed")
		return false
	var deadline := Time.get_ticks_msec() + 45000
	while Time.get_ticks_msec() < deadline:
		if pin_index(MULTI_TASK) < 0 and not has_multi_task() and client.objective_tracker_state().super_tracked != MULTI_TASK:
			print("WORLD_QUEST_MULTI_EXPIRED ", JSON.stringify(client.quest_state()))
			return await capture_phase("expired-all-areas")
		await process_frame
	stop("Real expiry retained an area, task log or local selection")
	return false

func run() -> void:
	var server := OS.get_environment("WQ_SERVER")
	var account := OS.get_environment("WQ_ACCOUNT")
	skin = OS.get_environment("WQ_SKIN")
	shots = OS.get_environment("WQ_SHOTS")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_wqnav") or shots.is_empty():
		stop("Explicit owned realm/account/captures required")
		return
	root.size = Vector2i(1920,1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	var error: String = client.connect_account(server, account, "fbtest", false)
	if not error.is_empty():
		stop(error)
		return
	if not await enter_world_when_ready():
		return
	character = client.account_state().selected_character_name
	if client.account_state().ui_skin.to_lower() != skin:
		stop("Skin differs from bound input")
		return
	deadline = Time.get_ticks_msec() + 60000
	while not MULTI_TASK in client.quest_state().available_world_quests and Time.get_ticks_msec() < deadline:
		await process_frame
	if not MULTI_TASK in client.quest_state().available_world_quests:
		stop("Actual82521 availability missing")
		return
	print("WORLD_QUEST_MULTI_AVAILABLE ", JSON.stringify(client.quest_state()))
	if not await enter_authored_lower_area() or not await select_lower_pin() or not await navigate_to_main_area() or not await expire_received_areas():
		return
	print("WORLD_QUEST_MULTI_PASS skin=%s sourceXYZ lower2256/main2255 physicalselection realexpiry" % skin)
	client.queue_free()
	await process_frame
	quit(0)
