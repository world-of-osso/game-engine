extends "res://tests/frame_benchmark.gd"

## Steady-frame rebuild invariant: enters the world as frame_benchmark.gd does, keeps the
## server's live clock (no `set_world_minutes`, as the Windows receipt harness), holds the
## camera at BENCH_CAMERA_YAW, waits GUARD_SETTLE_S seconds (default 10) for the loading
## fade and late placements, then counts rebuilds over GUARD_SEGMENT_S seconds (default 10)
## of idle frames. GUARD_TALENTS=1 switches to Forever, opens N, and guards the talents panel.
## Fails when a rebuild_guard.gd category averages above its allowlist
## budget, naming the category and its first call site. Stormwind's NPCs walk in and out
## of view: a window in which a unit visual entered the scene is not steady, so it is
## reported and measured again, up to GUARD_WINDOWS (default 3) windows. Run once per
## receipt pose (`game-server-admin set-position` while the character is offline).

const ASSET_STARTUP_TIMEOUT_MS := 180000

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
	var talents := OS.get_environment("GUARD_TALENTS") == "1"
	if talents:
		if not await wait_for_startup_world(character):
			return
	else:
		if not await wait_for_asset_startup():
			return
		var error = client.connect_account(server, account, PASSWORD, false)
		if error != "":
			fail("Fixture connection: " + error)
			return
		if not await enter_world(character):
			return
	if talents and not await open_talents_panel():
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
		if report.has("rebuild_guard_error"):
			fail(report.rebuild_guard_error)
			return
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

func wait_for_startup_world(character: String) -> bool:
	var deadline := Time.get_ticks_msec() + 900000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and Readiness.is_ready(state) and state.local_player_position != null:
			return true
	fail("Startup did not enter a ready world: " + str(client.account_state()))
	return false

func wait_for_asset_startup() -> bool:
	var deadline := Time.get_ticks_msec() + ASSET_STARTUP_TIMEOUT_MS
	while Time.get_ticks_msec() < deadline and client.account_state().assets_starting:
		await process_frame
	if client.account_state().assets_starting:
		fail("CASC asset startup did not finish within %d ms" % ASSET_STARTUP_TIMEOUT_MS)
		return false
	return true

func open_talents_panel() -> bool:
	var skin_error: String = client.preview_world_skin(true)
	if not skin_error.is_empty() or client.account_state().ui_skin != "Forever":
		fail("Could not select Forever for the talents guard: " + skin_error)
		return false
	client.set_world_minutes(WORLD_MINUTES)
	push_key(KEY_N, true)
	await process_frame
	push_key(KEY_N, false)
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var panel := client.find_child("ClassTalentsFrame", true, false) as Control
		if panel != null and panel.is_visible_in_tree():
			var nodes := client.find_children("TalentNode*", "Control", true, false)
			if not nodes.is_empty():
				await RenderingServer.frame_post_draw
				push_key(KEY_N, false)
				print("REBUILD_GUARD_PANEL talents ui_skin=Forever nodes=", nodes.size())
				return true
	push_key(KEY_N, false)
	fail("N did not render the Forever talents panel")
	return false
