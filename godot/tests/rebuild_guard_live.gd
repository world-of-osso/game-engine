extends "res://tests/frame_benchmark.gd"

## Steady-frame rebuild invariant: enters the world as frame_benchmark.gd does, keeps the
## server's live clock (no `set_world_minutes`, as the Windows receipt harness), holds the
## camera at BENCH_CAMERA_YAW, waits GUARD_SETTLE_S seconds (default 10) for the loading
## fade and late placements, then counts rebuilds over GUARD_SEGMENT_S seconds (default 10)
## of idle frames. Fails when a rebuild_guard.gd category averages above its allowlist
## budget, naming the category and its first call site. Stormwind's NPCs walk in and out
## of view: a window in which a unit visual entered the scene is not steady, so it is
## reported and measured again, up to GUARD_WINDOWS (default 3) windows. Run once per
## receipt pose (`game-server-admin set-position` while the character is offline).

func run_test() -> void:
	root.size = parse_resolution(OS.get_environment("BENCH_RESOLUTION"))
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("BENCH_ACCOUNT")
	var character := OS.get_environment("BENCH_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, BENCH_ACCOUNT and BENCH_CHARACTER are required")
		return
	var yaw := float(OS.get_environment("BENCH_CAMERA_YAW")) if OS.get_environment("BENCH_CAMERA_YAW") != "" else 0.0
	var settle_s := float(OS.get_environment("GUARD_SETTLE_S")) if OS.get_environment("GUARD_SETTLE_S") != "" else 10.0
	var segment_s := float(OS.get_environment("GUARD_SEGMENT_S")) if OS.get_environment("GUARD_SEGMENT_S") != "" else 10.0
	var windows := int(OS.get_environment("GUARD_WINDOWS")) if OS.get_environment("GUARD_WINDOWS") != "" else 3
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world(character):
		return
	client.set_camera_orbit(yaw, PITCH, DISTANCE)
	await create_timer(settle_s).timeout
	if not Readiness.is_ready(client.account_state()):
		fail("Work queued again after settling: " + str(client.account_state().world_objects))
		return
	var report := {}
	for window in windows:
		rebuild_guard.start(self)
		await create_timer(segment_s).timeout
		report = rebuild_guard.finish()
		var state: Dictionary = client.account_state()
		report["window"] = window
		report["position"] = state.local_player_position
		report["load"] = load_average()
		print("REBUILD_GUARD ", JSON.stringify(report))
		if not Readiness.is_ready(state):
			fail("World work queued during the guard segment: " + str(state.world_objects))
			return
		if report.world_events == 0:
			break
	if report.world_events != 0:
		fail("No steady window: unit visuals entered the scene in all %d windows" % windows)
		return
	var failed := RebuildGuard.violations(report)
	if not failed.is_empty():
		fail("Steady frames rebuild: " + "; ".join(failed))
		return
	print("FIXTURE REBUILD_GUARD_PASS")
	client.free()
	quit(0)
