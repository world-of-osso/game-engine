extends "res://tests/housing_property_live.gd"

# New private account; no admin teleport/house/decor grants. Walk from authored entry.
var walk_samples: Array[Dictionary] = []
var floor_probes: Array[Dictionary] = []
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
	if not record_floor_probe("entry"): return
	# GameObjects527261 actual authored position, NOT NeighborhoodPlot's marker.
	var destination := Vector3(3641.85669, 104.272713, -258.151031) if housing_skin == "Modern" else Vector3(3569.20312, 102.340424, -265.888885)
	# The authored spawn is inside the entrance fountain's raised stone ring.
	# Approach its observed west wall, then use ordinary W+Space to clear it.
	if not await walk_to_cornerstone(Vector3(3785.0, 192.0, 160.427), 0.6): return
	if not record_floor_probe("wall-approach"): return
	push_key(KEY_W, true)
	push_key(KEY_SPACE, true)
	await frames(2)
	push_key(KEY_SPACE, false)
	await create_timer(1.5).timeout
	push_key(KEY_W, false)
	await frames(20)
	if not record_floor_probe("after-jump"): return
	if not await capture_housing("fountain-wall-jump-world"): return
	for waypoint in [Vector3(3750.0, 180.0, 160.427), destination]:
		if not await walk_to_cornerstone(waypoint): return
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

func enter_neighborhood() -> bool:
	if not await wait_housing(func():
		var button := housing_named_control("MicroMenuUI", "HousingMicroButton")
		return button != null and button.is_visible_in_tree(), "visible world Housing entry", 180000): return false
	if not await click_exposed_control(housing_named_control("MicroMenuUI", "HousingMicroButton")): return false
	print("WORLD_FINDER_OPEN housing=",client.housing_state()," dashboard=",client.get_node_or_null("HousingDashboardUI"))
	if not await wait_housing(func(): return housing_control("HousingDashboardHouseFinderButton") != null, "new-account House Finder entry"): return false
	if not await click_exposed_control(housing_control("HousingDashboardHouseFinderButton")): return false
	if not await wait_housing(func():
		var visit := housing_control("HouseFinderVisitButton") as BaseButton
		return visit != null and not visit.disabled, "authored neighborhood visit"): return false
	if not await click_exposed_control(housing_control("HouseFinderVisitButton")): return false
	if not await wait_housing(func(): return client.account_state().screen == "InWorld" and int(client.account_state().world_map_id) == 2735, "actual authored world admission", 420000): return false
	await frames(90)
	return await capture_housing("authored-entry-world")

func walk_to_cornerstone(destination: Vector3, stop_radius: float = 3.5) -> bool:
	var deadline := Time.get_ticks_msec() + 240000
	var last_position: Vector3 = client.account_state().local_player_position
	var stalled := 0
	while Time.get_ticks_msec() < deadline:
		var state: Dictionary = client.account_state()
		var position: Vector3 = state.local_player_position
		var to := destination - position
		walk_samples.append({"position":position,"server":state.local_server_position,"remaining":to.length()})
		if Vector2(to.x, to.z).length() < stop_radius:
			push_key(KEY_W, false)
			await frames(30)
			return true
		if not await face_walk_direction(atan2(to.x, to.z)): return false
		client.set_camera_orbit(float(client.account_state().local_player_facing) - PI, -0.4, 20.0)
		push_key(KEY_W, true)
		await create_timer(minf(0.6, Vector2(to.x, to.z).length() / 14.0)).timeout
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

func record_floor_probe(label: String) -> bool:
	var state: Dictionary = client.account_state()
	var feet: Vector3 = state.local_player_position
	var sample: Dictionary = {"phase":label,"feet":feet,"server":state.local_server_position,"terrain_height":client.terrain_height_at(feet.x, feet.z)}
	for layer in [2, 8]:
		var query := PhysicsRayQueryParameters3D.create(feet + Vector3.UP * 20.0, feet - Vector3.UP * 5.0)
		query.collision_mask = layer
		var hit: Dictionary = client.get_world_3d().direct_space_state.intersect_ray(query)
		var collider := hit.get("collider") as Node
		sample[str(layer)] = {"hit":hit.get("position"),"collider":str(collider.get_path()) if collider != null else ""}
	floor_probes.append(sample)
	print("WORLD_FLOOR_PROBE ",sample)
	var file := FileAccess.open(shots.path_join("floor-probes.json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot save world floor probe")
		return false
	file.store_string(JSON.stringify(floor_probes,"\t"))
	file.flush()
	var error := file.get_error()
	file.close()
	if error != OK:
		fail("Cannot flush world floor probe")
		return false
	return true

func fail(message: String) -> void:
	push_key(KEY_W, false)
	push_key(KEY_RIGHT, false)
	push_key(KEY_LEFT, false)
	if client != null:
		print("WORLD_FAILURE account=",client.account_state()," housing=",client.housing_state()," hovered=",root.gui_get_hovered_control())
		client.print_tree_pretty()
	push_error(message)
	quit(1)

func capture_housing(phase: String) -> bool:
	await RenderingServer.frame_post_draw
	if root.get_texture().get_image().save_png(shots.path_join(housing_skin.to_lower()+"-"+phase+".png")) != OK:
		fail("Cannot save cornerstone WORLD image")
		return false
	return true
