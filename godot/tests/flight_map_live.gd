extends "res://tests/bank_live.gd"

# Private flight-master flow: real world picking and native button input.
func run_test() -> void:
	root.size = Vector2i(1280, 720)
	role = "flightmap"
	shots = OS.get_environment("BANK_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail("connect: " + error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	if not await click_pickable(func(node): return str(node.name) == "Dungar Longdrink", "unit_server_id"):
		return
	if not await wait_until(func(): return shown("GossipOption0Text"), "flight-master ride option"):
		return
	await press("GossipOption0Text")
	if not await wait_until(func(): return client.flight_map_state().open, "native taxi map"):
		return
	var state: Dictionary = client.flight_map_state()
	print("FIXTURE FLIGHT_MAP_OPEN ", state)
	if state.map_id != 13 or not state.node_ids.has(2) or not state.node_ids.has(4) or state.route_count < 1:
		fail("Expected Eastern Kingdoms, current Stormwind and reachable Sentinel Hill: " + str(state))
		return
	await capture("1-open")
	var destination := control("FlightMapNode4")
	if destination == null:
		fail("Sentinel Hill pin missing")
		return
	await hover(destination.get_global_rect().get_center())
	if not await wait_until(func(): return client.flight_map_state().hovered == 4, "Sentinel Hill hover"):
		return
	state = client.flight_map_state()
	if state.tooltip != "Sentinel Hill, Westfall\n0g 0s 5c" or text("FlightMapTooltipText") != state.tooltip:
		fail("Rendered destination/cost tooltip missing: " + str(state) + " label=" + text("FlightMapTooltipText"))
		return
	print("FIXTURE FLIGHT_MAP_HOVER ", state)
	await capture("2-hover")
	var before: Vector3 = client.account_state().local_server_position
	await press("FlightMapNode4")
	if not await wait_until(func(): return not client.flight_map_state().open and client.flight_map_state().controlled, "closed map and server-driven flight"):
		return
	if not await wait_until(func(): return client.account_state().local_server_position.distance_to(before) > 20.0, "server flight position advance"):
		return
	print("FIXTURE FLIGHT_STARTED ", client.flight_map_state(), " account=", client.account_state())
	await capture("3-flight-started")
	print("FIXTURE FLIGHTMAP_DONE")
	client.free()
	quit(0)

func hover(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(5)
