extends "res://tests/adventure_journeys_watch.gd"
# Real private realm snapshots, physical map input, polygon acceptance and controlled NPC deaths.
const TASK := 86948
var character := ""
var skin := ""

func _initialize() -> void:
	call_deferred("arm_watchdog")
	call_deferred("run")

func arm_watchdog() -> void:
	await create_timer(600.0).timeout
	stop("Native proof exceeded600s; inspect runtime errors")

func stop(reason: String) -> void:
	push_error("WORLD_QUEST_NAV_FAIL " + reason)
	if client != null:
		client.queue_free()
	quit(1)

func key(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.physical_keycode = code
		event.keycode = code
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await settle()

func admin(request: Dictionary) -> Dictionary:
	var output: Array[String] = []
	var result := OS.execute("/usr/bin/uv", ["run", "--with", "msgpack", "python", OS.get_environment("WQ_ADMIN"), Marshalls.utf8_to_base64(JSON.stringify(request))], output, true)
	if result != 0:
		stop("Private admin failed: " + str(output))
		return {}
	var lines := "\n".join(output).strip_edges().split("\n")
	var reply = JSON.parse_string(lines[-1])
	print("WORLD_QUEST_ADMIN ", JSON.stringify(request), " reply=", JSON.stringify(reply))
	return reply if reply is Dictionary else {}

func task_in_log() -> bool:
	for entry in client.quest_state().log:
		if entry.quest_id == TASK:
			return true
	return false

func wait_task(accepted: bool) -> bool:
	var deadline := Time.get_ticks_msec() + 45000
	while Time.get_ticks_msec() < deadline:
		if task_in_log() == accepted:
			return true
		await process_frame
	stop("Task log acceptance mismatch wanted=" + str(accepted))
	return false

func teleport(x: float, y: float, z: float) -> bool:
	var reply := admin({"Teleport": {"character_name": character, "map_id": 0, "x": x, "y": y, "z": z}})
	if reply.is_empty():
		return false
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		var position = client.account_state().local_player_position
		if position != null and Vector2(position.x-x, position.z+y).length() < 8.0:
			await create_timer(2.0).timeout
			return true
		await process_frame
	stop("Authoritative teleport not observed")
	return false

func pin_index(id: int) -> int:
	var state: Dictionary = client.world_map_state()
	var pins: Array = state.get("pins", [])
	for index in pins.size():
		if pins[index].quest_id == id and pins[index].type == "WorldQuest":
			return index
	return -1

func open_task_map() -> bool:
	await key(KEY_M)
	var deadline := Time.get_ticks_msec() + 45000
	while Time.get_ticks_msec() < deadline:
		if client.world_map_state().get("open", false) and pin_index(TASK) >= 0:
			return true
		await process_frame
	stop("Real task pin absent map=" + str(client.world_map_state()) + " quests=" + str(client.quest_state()))
	return false

func capture_phase(phase: String) -> bool:
	await RenderingServer.frame_post_draw
	var path := shots.path_join(skin + "-" + phase + ".png")
	var image := root.get_texture().get_image()
	if image.get_size() != Vector2i(1920,1080) or image.save_png(path) != OK:
		stop("FullHD PNG failed")
		return false
	print("WORLD_QUEST_CAPTURE ", path)
	return true

func check_pin_navigation() -> bool:
	if task_in_log():
		stop("Outside polygon already accepted task")
		return false
	if not await open_task_map():
		return false
	var index := pin_index(TASK)
	var map_before: int = client.world_map_state().map_id
	if not await move_to("WorldMapPin%s" % index):
		return false
	var tooltip := control("WorldQuestMapTooltipFrame")
	if tooltip == null or not tooltip.is_visible_in_tree():
		stop("Native world quest reward tooltip missing")
		return false
	var texts := []
	for node in tooltip.find_children("*", "Label", true, false):
		texts.append(node.text)
	print("WORLD_QUEST_TOOLTIP ", texts)
	if not "Rewards" in texts or not "11765 XP" in texts:
		stop("Real task reward preview not present: " + str(texts))
		return false
	if not await capture_phase("map-reward-tooltip") or not await click("WorldMapPin%s" % index):
		return false
	if client.world_map_state().map_id != map_before or client.objective_tracker_state().super_tracked != TASK or task_in_log():
		stop("Pin click navigated map/accepted quest instead of selecting received task")
		return false
	if not await capture_phase("map-selected"):
		return false
	await key(KEY_M)
	var deadline := Time.get_ticks_msec()+30000
	while control("MinimapSuperTrack%s" % TASK) == null and Time.get_ticks_msec()<deadline:
		await process_frame
	if client.objective_tracker_state().super_tracked != TASK or control("MinimapSuperTrack%s" % TASK) == null:
		stop("Pre-accept selection did not persist into minimap direction")
		return false
	return await capture_phase("minimap-selected-direction")

func task_progress() -> int:
	for entry in client.quest_state().log:
		if entry.quest_id == TASK and not entry.objectives.is_empty():
			return int(str(entry.objectives[0]).split("/")[0])
	return -1

func kill_authored_elemental() -> bool:
	# NPC authority does not depend on optional rendered meshes/UnitPick children.
	var report := admin({"NpcInfo":{"map_id":0,"instance_id":0,"entry":46327}})
	var pattern := RegEx.new()
	pattern.compile("entity ([0-9]+)\\) level [0-9]+ health ([0-9.]+)/")
	for line in report.get("NpcInfoList",[[]])[0]:
		var found := pattern.search(str(line))
		if found != null and float(found.get_string(2))>0.0:
			var id := int(found.get_string(1))
			print("WORLD_QUEST_ACTOR ", line)
			var reply := admin({"Damage":{"character_name":character,"amount":1000000.,"target":id}})
			return not reply.is_empty() and not reply.has("Error")
	stop("Authoritative living entry46327 absent")
	return false

func check_arrival_and_reward() -> bool:
	# Lead-approved controlled fixture omits only source spawn20383796 (Deathwing).
	# Real-world overlap is an open source gap, not a product exclusion or immunity.
	var hazard := admin({"NpcInfo":{"map_id":0,"instance_id":0,"entry":44744}})
	if not hazard.has("NpcInfoList"):
		stop("Authoritative hazard absence query failed: " + str(hazard))
		return false
	if not hazard.NpcInfoList[0].is_empty():
		stop("Controlled reward fixture must omit Deathwing spawn20383796; ordinary-world hazard remains open")
		return false
	print("WORLD_QUEST_FIXTURE omittedSpawn=20383796 sourceNPC46327Unmodified=true noCreditInjection=true")
	# Real polygon centroid, then actual source NPCs. Administrator damage is not natural combat proof.
	if not await teleport(-4655.,-4310.,294.) or not await wait_task(true):
		return false
	var deadline := Time.get_ticks_msec()+30000
	while control("MinimapWorldQuest%s" % TASK) == null and Time.get_ticks_msec()<deadline:
		await process_frame
	if control("MinimapWorldQuest%s" % TASK) == null or control("WorldQuest%sProgressLabel" % TASK) != null or control("MinimapQuestObjective%s" % TASK) != null:
		stop("Missing unnumbered task marker or invented counted bar/duplicate POI")
		return false
	if not await capture_phase("minimap-arrived"):
		return false
	# Keep authored actors in place; remote administrator Damage is controlled proof,
	# not natural combat. Approaching their southern spawn also wakes Deathwing's
	# authentic addon83508 -> Molten Flame94644 instakill hazard (400yd).
	for count in 7:
		if not await kill_authored_elemental():
			return false
		deadline = Time.get_ticks_msec()+30000
		while task_progress()<count+1 and Time.get_ticks_msec()<deadline:
			await process_frame
		if task_progress()!=count+1:
			stop("Actual creature death credit mismatch: " + str(client.quest_state()))
			return false
	if not await capture_phase("tracker-seven-of-eight"):
		return false
	var before: int = client.quest_state().get("xp",-1)
	if not await kill_authored_elemental() or not await wait_task(false):
		return false
	deadline = Time.get_ticks_msec()+30000
	while not "Experience gained: 11765." in client.quest_state().system_lines and Time.get_ticks_msec()<deadline:
		await process_frame
	var after: int = client.quest_state().get("xp",-1)
	if after-before<11765 or not "Experience gained: 11765." in client.quest_state().system_lines:
		stop("Actual quest reward XP acknowledgement/settlement missing")
		return false
	await settle()
	print("WORLD_QUEST_REWARD quest=%s XP=%s->%s delta=%s realQuestXP11765 logRemoved=true" % [TASK,before,after,after-before])
	if not await capture_phase("completed-reward"):
		return false
	if not await teleport(-4000.,-4300.,198.) or not await teleport(-4655.,-4310.,294.):
		return false
	if task_in_log():
		stop("Completed task replayed in same active window")
		return false
	return true

func check_window_ui() -> bool:
	if not await teleport(-4655.,-4310.,294.) or not await wait_task(true):
		return false
	var end := int(OS.get_environment("WQ_EPOCH"))+86400
	admin({"ClockAdvance":{"seconds":end-int(Time.get_unix_time_from_system())-600}})
	var deadline := Time.get_ticks_msec()+30000
	var warning: Label
	while Time.get_ticks_msec()<deadline:
		warning = control("QuestBlock%sLine1Text" % TASK) as Label
		if warning != null and warning.text.begins_with("Time Left:"):
			break
		await process_frame
	if warning == null or not warning.text.begins_with("Time Left:") or control("WorldQuest%sProgressLabel" % TASK) != null:
		stop("Real low-window warning/count-only tracker mismatch")
		return false
	print("WORLD_QUEST_LOW_TIME ",warning.text," realmOffset=",client.quest_state().world_quest_server_offset)
	if not await capture_phase("tracker-low-window"):
		return false
	await key(KEY_M)
	var index := pin_index(TASK)
	if index<0 or not await move_to("WorldMapPin%s" % index):
		return false
	if not await capture_phase("map-low-window-reward"):
		return false
	admin({"ClockAdvance":{"seconds":610}})
	deadline = Time.get_ticks_msec()+30000
	while (pin_index(TASK)>=0 or task_in_log()) and Time.get_ticks_msec()<deadline:
		await process_frame
	if pin_index(TASK)>=0 or task_in_log() or client.objective_tracker_state().super_tracked==TASK:
		stop("Authoritative window end retained task/pin/selection")
		return false
	return await capture_phase("expired-window-map")

func check_expiry() -> bool:
	# Advance the private realm clock to its real policy window end, with an actual gap.
	var end := int(OS.get_environment("WQ_EPOCH"))+86400
	var remaining := end-int(Time.get_unix_time_from_system())
	admin({"ClockAdvance": {"seconds": remaining+2}})
	await key(KEY_M)
	var deadline := Time.get_ticks_msec()+30000
	while pin_index(86941)>=0 and Time.get_ticks_msec()<deadline:
		await process_frame
	if pin_index(86941)>=0 or pin_index(TASK)>=0:
		stop("Expired availability pin retained across real window end")
		return false
	return await capture_phase("expired-window-map")

func run() -> void:
	var server := OS.get_environment("WQ_SERVER")
	var account := OS.get_environment("WQ_ACCOUNT")
	skin = OS.get_environment("WQ_SKIN")
	shots = OS.get_environment("WQ_SHOTS")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_wqnav") or shots.is_empty():
		stop("Explicit owned private realm/accounts and screenshots required")
		return
	root.size = Vector2i(1920,1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec()+180000
	while client.account_state().assets_starting and Time.get_ticks_msec()<deadline:
		await process_frame
	var error: String = client.connect_account(server,account,"fbtest",false)
	if not error.is_empty():
		stop(error)
		return
	if not await enter_world_when_ready():
		return
	character = client.account_state().selected_character_name
	if client.account_state().local_player_health <= 0.0:
		# Explicit player-only controlled setup, never NPC health or quest credit.
		admin({"Revive":{"character_name":character}})
		deadline = Time.get_ticks_msec()+30000
		while client.account_state().local_player_health <= 0.0 and Time.get_ticks_msec()<deadline:
			await process_frame
		if client.account_state().local_player_health <= 0.0:
			stop("Controlled player resurrection failed")
			return
		print("WORLD_QUEST_SETUP playerRevive=true NPCHealthUnchanged=true")
	if client.account_state().ui_skin.to_lower() != skin:
		stop("Actual skin differs from receipt")
		return
	if OS.get_environment("WQ_WINDOW_ONLY")=="1":
		if OS.get_environment("WQ_RESUME_WINDOW")!="1" and not await check_pin_navigation():
			return
		if not await check_window_ui():
			return
		print("WORLD_QUEST_UI_WINDOW_PASS skin=%s actual86948/lowtime/expiry; reward settlement NOT certified" % skin)
		client.queue_free()
		await process_frame
		quit(0)
		return
	if not await check_pin_navigation() or not await check_arrival_and_reward() or not await check_expiry():
		return
	print("WORLD_QUEST_NAV_PASS skin=%s real86948 selection/minimap/area/actualDeaths/XP/replay/expiry" % skin)
	client.queue_free()
	await process_frame
	quit(0)
