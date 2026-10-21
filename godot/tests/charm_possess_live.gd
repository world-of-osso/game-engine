extends "res://tests/pet_bar_live.gd"

## Private native605/205364 proof. Reuses the real account/character, input and capture
## helpers. PET_SKIN=modern|forever; own priest near source Kobold Vermin spawns.
func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("PET_ACCOUNT")
	character = OS.get_environment("PET_CHARACTER")
	shots = OS.get_environment("PET_SHOTS")
	if server == "" or account == "" or character == "" or shots == "":
		fail("Private endpoint/account/character/screenshots required")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	error = client.preview_world_skin(OS.get_environment("PET_SKIN") == "forever")
	if error != "":
		fail("Skin: " + error)
		return
	await wait_frames(90)
	if not await select_kobold():
		return
	var controlled: int = client.target_state().target
	var body_start := player().global_position
	error = client.use_spell(605)
	if error != "":
		fail("Native605: " + error)
		return
	if not await wait_until(func(): return bar().get("shown", false) and int(bar().get("pet", 0)) == controlled, 20000, "native605 possession/bar"):
		return
	await wait_frames(60)
	var npc_start := unit_node(controlled).global_position
	var camera := client.get_node("WorldCamera") as Camera3D
	var camera_start := camera.global_position
	await capture("01-native605-possessed.png")
	push_key(KEY_W, true, false)
	await wait_real(1.2)
	push_key(KEY_W, false, false)
	await wait_frames(90)
	var npc_end := unit_node(controlled).global_position
	if npc_end.distance_to(npc_start) < 3.0 or player().global_position.distance_to(body_start) > 0.1:
		fail("Possess movement/body authority: npc=%s->%s body=%s->%s" % [npc_start, npc_end, body_start, player().global_position])
		return
	if camera.global_position.distance_to(camera_start) < 2.0:
		fail("Camera did not follow possessed NPC")
		return
	var server_position: Vector3 = client.account_state().local_server_position
	if server_position.distance_to(npc_end) > 1.0:
		fail("Possessed predicted/server position mismatch: %s vs %s" % [npc_end, server_position])
		return
	print("CHARM_POSSESS_MOVE skin=", client.account_state().ui_skin, " npc=", npc_start, "->", npc_end, " body=", player().global_position, " authoritative=", server_position)
	await capture("02-native605-npc-moved-body-parked.png")
	await press(KEY_ESCAPE)
	if not await wait_until(func(): return not bar().get("shown", false), 10000, "native605 channel cancel releases bar"):
		return
	push_key(KEY_W, true, false)
	await wait_real(0.4)
	push_key(KEY_W, false, false)
	await wait_frames(60)
	if player().global_position.distance_to(body_start) < 0.5:
		fail("Priest movement not restored after possess cancel")
		return
	await capture("03-native605-priest-control-restored.png")
	if not await select_kobold():
		return
	controlled = int(client.target_state().target)
	error = client.use_spell(205364)
	if error != "":
		fail("Native205364: " + error)
		return
	if not await wait_until(func(): return bar().get("shown", false) and int(bar().get("pet", 0)) == controlled, 20000, "native205364 charm/bar"):
		return
	body_start = player().global_position
	push_key(KEY_W, true, false)
	await wait_real(0.4)
	push_key(KEY_W, false, false)
	await wait_frames(60)
	if player().global_position.distance_to(body_start) < 0.5:
		fail("Ordinary charm incorrectly parked priest body")
		return
	await capture("04-native205364-charm-priest-active.png")
	print("CHARM_POSSESS_DONE skin=", client.account_state().ui_skin, " bar=", bar(), " viewport=", root.size)
	client.free()
	quit(0)

func select_kobold() -> bool:
	for attempt in range(24):
		await press(KEY_TAB)
		await wait_frames(6)
		if str(client.target_state().target_name) == "Kobold Vermin":
			return true
	fail("No source Kobold Vermin near private priest: %s" % client.target_state())
	return false
