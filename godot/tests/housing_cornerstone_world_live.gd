extends "res://tests/housing_property_live.gd"

# New private account; no admin teleport/house/decor grants. Walk from authored entry.
var walk_samples: Array[Dictionary] = []
var right_turn_sign := 0.0

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	housing_server = OS.get_environment("GODOT_TEST_SERVER")
	housing_account = OS.get_environment("HOUSING_ACCOUNT")
	housing_skin = OS.get_environment("HOUSING_SKIN")
	shots = OS.get_environment("HOUSING_SHOTS")
	property_plot = 444 if housing_skin == "Modern" else 424
	property_entry = 527261 if housing_skin == "Modern" else 527282
	if housing_server != "127.0.0.1:5870" or not housing_account.begins_with("fb_housing_world_") or shots.is_empty():
		fail("Cornerstone WORLD proof needs isolated fresh world account")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await connect_housing_account(): return
	if not await enter_neighborhood(): return
	var close := housing_control("HousingDashboardFrameCloseButton")
	if close != null and close.is_visible_in_tree():
		await click_control(close, MOUSE_BUTTON_LEFT)
	# GameObjects527261 actual authored position, NOT NeighborhoodPlot's marker.
	var destination := Vector3(3641.85669, 104.272713, -258.151031) if housing_skin == "Modern" else Vector3(3569.20312, 102.340424, -265.888885)
	if not await walk_to_cornerstone(destination): return
	if not await capture_housing("walk-arrived-world"): return
	if not await purchase_property(): return
	if not await capture_housing("claimed-world"): return
	if not await inspect_owned_property("owned"): return
	if not await connect_housing_account(): return
	if not await inspect_owned_property("relogged"): return
	var state: Dictionary = client.housing_state()
	var output := FileAccess.open(shots.path_join("cornerstone-world-receipt.json"), FileAccess.WRITE)
	if output == null:
		fail("Cannot persist cornerstone world receipt")
		return
	output.store_string(JSON.stringify({"walk":walk_samples,"property":property_receipts,"final":state}, "\t"))
	output.flush()
	var error := output.get_error()
	output.close()
	if error != OK:
		fail("Cannot flush cornerstone world receipt")
		return
	print("PASS: private5870 walked authored cornerstone WORLD ", housing_skin)
	client.free()
	quit(0)

func walk_to_cornerstone(destination: Vector3) -> bool:
	var deadline := Time.get_ticks_msec() + 240000
	var last_position: Vector3 = client.account_state().local_player_position
	var stalled := 0
	while Time.get_ticks_msec() < deadline:
		var state: Dictionary = client.account_state()
		var position: Vector3 = state.local_player_position
		var to := destination - position
		walk_samples.append({"position":position,"server":state.local_server_position,"remaining":to.length()})
		if Vector2(to.x, to.z).length() < 3.5:
			push_key(KEY_W, false)
			await frames(30)
			return true
		if not await face_walk_direction(atan2(to.x, to.z)): return false
		push_key(KEY_W, true)
		await create_timer(0.6).timeout
		push_key(KEY_W, false)
		await frames(3)
		position = client.account_state().local_player_position
		stalled = stalled + 1 if position.distance_to(last_position) < 0.2 else 0
		last_position = position
		if walk_samples.size() % 20 == 0:
			print("HOUSING_WORLD_WALK ",walk_samples.back())
			if not await capture_housing("walk-world-%03d" % walk_samples.size()): return false
		if stalled >= 8:
			await capture_housing("walk-world-blocked")
			fail("Native walk blocked on authored world geometry at " + str(position))
			return false
	push_key(KEY_W, false)
	fail("Native walk exceeded4min; last=" + str(last_position))
	return false

func face_walk_direction(target: float) -> bool:
	for attempt in range(600):
		var facing: float = client.account_state().local_player_facing
		var error := angle_difference(facing, target)
		if absf(error) < 0.04:
			push_key(KEY_RIGHT, false)
			push_key(KEY_LEFT, false)
			return true
		if right_turn_sign == 0.0:
			push_key(KEY_RIGHT, true)
			await frames(5)
			push_key(KEY_RIGHT, false)
			right_turn_sign = signf(angle_difference(facing, float(client.account_state().local_player_facing)))
			continue
		var right := signf(error) == right_turn_sign
		push_key(KEY_RIGHT, right)
		push_key(KEY_LEFT, not right)
		await process_frame
		push_key(KEY_RIGHT, false)
		push_key(KEY_LEFT, false)
	fail("Native turning could not face cornerstone")
	return false

func capture_housing(phase: String) -> bool:
	await RenderingServer.frame_post_draw
	if root.get_texture().get_image().save_png(shots.path_join(housing_skin.to_lower()+"-"+phase+".png")) != OK:
		fail("Cannot save cornerstone WORLD image")
		return false
	return true
