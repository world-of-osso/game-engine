extends SceneTree

## Steady-state in-world frame time. Enters the world as card 0, waits until every requested
## terrain, object and unit-visual job has drained, fixes the time of day and the camera orbit,
## then measures three segments of BENCH_SEGMENT_S seconds each (default 20):
##   idle   no input; camera at BENCH_CAMERA_YAW (radians, default 0), pitch -0.35, 15 yd
##   orbit  the camera yaw sweeps one full turn around the standing character
##   walk   real key input: W runs forward for BENCH_WALK_S seconds (default 2), S backpedals
##          the same distance back (run 7 yd/s, backpedal 4.5 yd/s), repeatedly
## Environment: GODOT_TEST_SERVER, BENCH_ACCOUNT (password fbtest), BENCH_CHARACTER;
## BENCH_SCREENSHOT=<png> saves the idle view after its measured segment.
## BENCH_SCENERY_RECEIPT=<path> records real Stormwind visibility for comparison.
## Place the character with `game-server-admin set-position|teleport` while it is offline.
## BENCH_SYSTEMS_S=<seconds> adds an `idle_systems` segment of that length that reads
## `GameClient.frame_stats()` every frame and reports each client step's median and p95,
## the same breakdown the Windows 120 FPS receipt reports as `top_systems_ms`.
## Each segment prints `BENCH_MARK <segment> start|end` (for an external profiler) and
## `BENCH_SEGMENT {json}`: wall-clock frame interval percentiles, per-frame means of the
## client's own process time and the viewport's render CPU time, and the means of Godot's
## TIME_PROCESS/TIME_PHYSICS_PROCESS monitors (each the longest step of the last second),
## the viewport's GPU render time, and the main thread's on-CPU time per frame
## (/proc/thread-self/schedstat), so a frame splits into main-thread work and waiting.
## Run with Godot's --gpu-profile and `render_areas` adds the mean CPU ms per frame of each
## renderer timestamp area (the time until the next timestamp), the 15 largest.
## Reports only; frame-time budgets are not asserted (they depend on host load).

const PASSWORD := "fbtest"
const Readiness = preload("res://tests/world_entry_readiness.gd")
const CharacterSelectReveal = preload("res://tests/character_select_reveal.gd")
const SceneryReceipt = preload("res://tests/scenery_visibility_receipt.gd")
const PITCH := -0.35
const DISTANCE := 15.0
## Noon in the half-minutes the world light samples.
const WORLD_MINUTES := 1440.0
const RUN_SPEED := 7.0
const BACKPEDAL_SPEED := 4.5

var client: Node
var player: Node3D
var last_usec := 0
var recording := false
var intervals: Array[float] = []
var client_ms: Array[float] = []
var process_ms: Array[float] = []
var physics_ms: Array[float] = []
var render_ms: Array[float] = []
var draw_calls: Array[float] = []
var shadow_draw_calls: Array[float] = []
var gpu_ms: Array[float] = []
var main_cpu_ms: Array[float] = []
var last_cpu_ns := 0
var area_ms := {}
var systems_ms := {}
var recording_systems := false

func _initialize() -> void:
	call_deferred("run_test")

func _process(_delta: float) -> bool:
	# Wall clock: Godot caps the process delta, so the delta hides a stall.
	var now := Time.get_ticks_usec()
	var cpu_ns := thread_cpu_ns()
	if recording and last_usec > 0:
		intervals.append((now - last_usec) / 1000.0)
		client_ms.append(client.process_ms())
		process_ms.append(Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0)
		physics_ms.append(Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS) * 1000.0)
		render_ms.append(RenderingServer.viewport_get_measured_render_time_cpu(root.get_viewport_rid())
			+ RenderingServer.get_frame_setup_time_cpu())
		draw_calls.append(Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME))
		shadow_draw_calls.append(root.get_render_info(Viewport.RENDER_INFO_TYPE_SHADOW,
			Viewport.RENDER_INFO_DRAW_CALLS_IN_FRAME))
		gpu_ms.append(RenderingServer.viewport_get_measured_render_time_gpu(root.get_viewport_rid()))
		main_cpu_ms.append((cpu_ns - last_cpu_ns) / 1e6)
		add_render_areas()
		if recording_systems:
			var steps: Dictionary = client.frame_stats().systems_ms
			for name in steps:
				if not systems_ms.has(name):
					systems_ms[name] = [] as Array[float]
				systems_ms[name].append(steps[name])
	last_usec = now
	last_cpu_ns = cpu_ns
	return false

func run_test() -> void:
	root.size = parse_resolution(OS.get_environment("BENCH_RESOLUTION"))
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("BENCH_ACCOUNT")
	var character := OS.get_environment("BENCH_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, BENCH_ACCOUNT and BENCH_CHARACTER are required")
		return
	var segment_s := float(OS.get_environment("BENCH_SEGMENT_S")) if OS.get_environment("BENCH_SEGMENT_S") != "" else 20.0
	var yaw := float(OS.get_environment("BENCH_CAMERA_YAW")) if OS.get_environment("BENCH_CAMERA_YAW") != "" else 0.0
	var walk_s := float(OS.get_environment("BENCH_WALK_S")) if OS.get_environment("BENCH_WALK_S") != "" else 2.0
	RenderingServer.viewport_set_measure_render_time(root.get_viewport_rid(), true)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world(character):
		return
	player = client.get_node_or_null("WorldUnits/" + character) as Node3D
	if player == null:
		fail("No local player node WorldUnits/" + character)
		return
	client.set_world_minutes(WORLD_MINUTES)
	client.set_camera_orbit(yaw, PITCH, DISTANCE)
	# Let the camera and late streamed placements settle before measuring.
	await create_timer(5.0).timeout
	if not Readiness.is_ready(client.account_state()):
		fail("Work queued again after settling: " + str(client.account_state().world_objects))
		return
	var receipt_path := OS.get_environment("BENCH_SCENERY_RECEIPT")
	if receipt_path != "":
		var state: Dictionary = client.account_state()
		if state.get("ui_skin") != "Forever":
			fail("Scenery receipt requires the Forever skin")
			return
		if state.world_objects.failures != 0:
			fail("Scenery receipt has failed objects: " + str(state.world_objects))
			return
		var count: int = SceneryReceipt.write(client, receipt_path)
		if count < 1000:
			fail("Scenery receipt has insufficient placements: " + str(count))
			return
	var spawn := Vector2(player.position.x, player.position.z)
	print("BENCH_SCENE ", JSON.stringify({"character": character, "spawn": [spawn.x, spawn.y],
		"objects": client.account_state().world_objects, "load": load_average()}))
	if OS.get_environment("BENCH_NODES") == "1":
		print("BENCH_NODES ", JSON.stringify(node_census(root)))
	await measure("idle", segment_s, func(_t: float): pass)
	# After a measured segment, not on the first settled frame: the loading fade and the
	# world light both finish within those seconds, and their overlay would otherwise be
	# the captured frame.
	if OS.get_environment("BENCH_SCREENSHOT") != "":
		await RenderingServer.frame_post_draw
		root.get_texture().get_image().save_png(OS.get_environment("BENCH_SCREENSHOT"))
	var systems_s := OS.get_environment("BENCH_SYSTEMS_S")
	if systems_s != "":
		await measure("idle_systems", float(systems_s), func(_t: float): pass, true)
	await measure("orbit", segment_s, func(t: float):
		client.set_camera_orbit(yaw + TAU * t / segment_s, PITCH, DISTANCE))
	client.set_camera_orbit(yaw, PITCH, DISTANCE)
	begin("walk")
	var started := Time.get_ticks_msec()
	var legs := 0
	var travelled := 0.0
	while Time.get_ticks_msec() - started < segment_s * 1000.0:
		var forward := legs % 2 == 0
		var key := KEY_W if forward else KEY_S
		var from := Vector2(player.position.x, player.position.z)
		push_key(key, true)
		await create_timer(walk_s if forward else walk_s * RUN_SPEED / BACKPEDAL_SPEED).timeout
		push_key(key, false)
		travelled += from.distance_to(Vector2(player.position.x, player.position.z))
		legs += 1
	finish("walk", {"legs": legs, "travelled_yd": travelled})
	if travelled < legs * walk_s * RUN_SPEED * 0.5:
		fail("Walk covered %.1f yd in %d legs; the character is blocked" % [travelled, legs])
		return
	print("FIXTURE FRAME_BENCHMARK_DONE")
	client.free()
	quit(0)

func measure(segment: String, seconds: float, step: Callable, with_systems := false) -> void:
	begin(segment, with_systems)
	var started := Time.get_ticks_msec()
	while Time.get_ticks_msec() - started < seconds * 1000.0:
		step.call((Time.get_ticks_msec() - started) / 1000.0)
		await process_frame
	finish(segment, {})

func begin(segment: String, with_systems := false) -> void:
	for samples in [intervals, client_ms, process_ms, physics_ms, render_ms, draw_calls, shadow_draw_calls, gpu_ms, main_cpu_ms]:
		samples.clear()
	area_ms.clear()
	systems_ms.clear()
	recording_systems = with_systems
	print("BENCH_MARK %s start" % segment)
	recording = true
	last_usec = 0

func finish(segment: String, extra: Dictionary) -> void:
	recording = false
	print("BENCH_MARK %s end" % segment)
	var sorted := intervals.duplicate()
	sorted.sort()
	var report := {
		"segment": segment, "frames": sorted.size(),
		"p50_ms": percentile(sorted, 0.5), "p95_ms": percentile(sorted, 0.95),
		"p99_ms": percentile(sorted, 0.99), "max_ms": sorted.back() if not sorted.is_empty() else 0.0,
		"mean_ms": mean(intervals), "client_process_ms": mean(client_ms),
		"godot_process_max_1s_ms": mean(process_ms), "physics_max_1s_ms": mean(physics_ms),
		"render_cpu_ms": mean(render_ms), "draw_calls": mean(draw_calls),
		"shadow_draw_calls": mean(shadow_draw_calls), "load": load_average(),
		"render_gpu_ms": mean(gpu_ms), "main_thread_cpu_ms": mean(main_cpu_ms),
	}
	if not area_ms.is_empty():
		var names := area_ms.keys()
		names.sort_custom(func(a, b): return area_ms[a] > area_ms[b])
		var areas := {}
		for name in names.slice(0, 15):
			areas[name] = area_ms[name] / sorted.size()
		report["render_areas"] = areas
	if recording_systems:
		report["systems_ms_median"] = per_system(systems_ms, 0.5)
		report["systems_ms_p95"] = per_system(systems_ms, 0.95)
	report.merge(extra)
	print("BENCH_SEGMENT ", JSON.stringify(report))

## Median (or nearest-rank percentile) of each client step, largest first.
func per_system(samples: Dictionary, fraction: float) -> Dictionary:
	var values := {}
	for name in samples:
		var sorted: Array[float] = samples[name].duplicate()
		sorted.sort()
		values[name] = percentile(sorted, fraction)
	var names := values.keys()
	names.sort_custom(func(a, b): return values[a] > values[b])
	var ordered := {}
	for name in names:
		ordered[name] = values[name]
	return ordered

## `WIDTHxHEIGHT`; the receipt poses use 1920x1080.
func parse_resolution(text: String) -> Vector2i:
	var parts := text.split("x")
	if parts.size() != 2:
		return Vector2i(1280, 720)
	return Vector2i(int(parts[0]), int(parts[1]))

## CPU ms between consecutive renderer timestamps of the last captured frame, per area.
func add_render_areas() -> void:
	var device := RenderingServer.get_rendering_device()
	var count := device.get_captured_timestamps_count() if device != null else 0
	for i in range(count - 1):
		var name := device.get_captured_timestamp_name(i)
		if name.begins_with("<") or name.begins_with(">"):
			continue
		var ms := (device.get_captured_timestamp_cpu_time(i + 1) - device.get_captured_timestamp_cpu_time(i)) / 1000.0
		area_ms[name] = area_ms.get(name, 0.0) + ms

## Per class: nodes, nodes with process/physics process enabled, visible 3D nodes.
func node_census(top: Node) -> Dictionary:
	var census := {}
	var stack: Array[Node] = [top]
	while not stack.is_empty():
		var node: Node = stack.pop_back()
		var name := node.get_class()
		var entry: Array = census.get(name, [0, 0, 0, 0])
		entry[0] += 1
		entry[1] += int(node.is_processing())
		entry[2] += int(node.is_physics_processing())
		entry[3] += int(node is Node3D and node.is_visible_in_tree())
		census[name] = entry
		stack.append_array(node.get_children(true))
	return census

## Nearest-rank percentile of ascending `sorted`.
func percentile(sorted: Array, fraction: float) -> float:
	if sorted.is_empty():
		return 0.0
	return sorted[maxi(0, ceili(sorted.size() * fraction) - 1)]

func mean(values: Array[float]) -> float:
	var total := 0.0
	for value in values:
		total += value
	return total / values.size() if not values.is_empty() else 0.0

## The main thread's scheduled run time (/proc/thread-self/schedstat), in nanoseconds.
func thread_cpu_ns() -> int:
	var file := FileAccess.open("/proc/thread-self/schedstat", FileAccess.READ)
	return int(file.get_line().split(" ")[0]) if file != null else 0

func load_average() -> String:
	var file := FileAccess.open("/proc/loadavg", FileAccess.READ)
	return file.get_line() if file != null else "unknown"

func enter_world(character: String) -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 \
				and client.get_node_or_null("CharacterSelectUI") != null:
			break
	var ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	if not await CharacterSelectReveal.wait_for_reveal(self, client, "CharCard_0", character):
		fail("Character select did not reveal")
		return false
	await click(ui.find_child("CharCard_0", true, false))
	if not await CharacterSelectReveal.wait_for_reveal(self, client, "CharCard_0", character):
		fail("Character select did not reveal after selection")
		return false
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != character:
		fail("Card 0 is %s, not %s" % [selected.text, character])
		return false
	await click(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + 900000
	var report_at := 0
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and Readiness.is_ready(state) and state.local_player_position != null:
			return true
		if Time.get_ticks_msec() >= report_at:
			report_at = Time.get_ticks_msec() + 10000
			print("BENCH_WAIT %s fps=%.1f objects=%s" % [state.screen, Engine.get_frames_per_second(), state.get("world_objects")])
	fail("World did not settle: " + str(client.account_state()))
	return false

func push_key(keycode: Key, pressed: bool) -> void:
	var key := InputEventKey.new()
	key.physical_keycode = keycode
	key.keycode = keycode
	key.pressed = pressed
	root.push_input(key, true)

func click(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await process_frame

func fail(message: String) -> void:
	push_error(message)
	print("FIXTURE FAIL ", message)
	if client != null and is_instance_valid(client):
		client.free()
	quit(1)
