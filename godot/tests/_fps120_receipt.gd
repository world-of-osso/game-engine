extends SceneTree

## Steady-state in-world frame time. Enters the world as card 0, waits until every requested
## terrain, object and unit-visual job has drained, fixes the time of day and the camera orbit,
## then measures three segments of BENCH_SEGMENT_S seconds each (default 20):
##   idle   no input; camera at BENCH_CAMERA_YAW (radians, default 0), pitch -0.35, 15 yd
##   orbit  the camera yaw sweeps one full turn around the standing character
##   walk   real key input: W runs forward for BENCH_WALK_S seconds (default 2), S backpedals
##          the same distance back (run 7 yd/s, backpedal 4.5 yd/s), repeatedly
## Environment: GODOT_TEST_SERVER, BENCH_ACCOUNT (password fbtest), BENCH_CHARACTER;
## BENCH_SCREENSHOT=<png> saves the settled idle view before measuring.
## Place the character with `game-server-admin set-position|teleport` while it is offline.
## Each segment prints `BENCH_MARK <segment> start|end` (for an external profiler) and
## `BENCH_SEGMENT {json}`: wall-clock frame interval percentiles, per-frame means of the
## client's own process time and the viewport's render CPU time, and the means of Godot's
## TIME_PROCESS/TIME_PHYSICS_PROCESS monitors (each the longest step of the last second),
## the viewport's GPU render time, and the main thread's on-CPU time per frame
## (/proc/thread-self/schedstat, Linux only), so a frame splits into main-thread work and
## waiting. `render_cpu_ms`/`render_gpu_ms` are the viewport's render times of the segment's
## last frame; `threads` is the main and render threads' CPU and wait per frame on Linux and
## Windows (thread_cpu_split.gd; needs GameClient.thread_cpu_ms).
## Run with Godot's --gpu-profile and `render_areas` adds the mean CPU ms per frame of each
## renderer timestamp area (the time until the next timestamp), the 15 largest.
## Reports only; frame-time budgets are not asserted (they depend on host load).

const PASSWORD := "fbtest"
const Readiness = preload("res://tests/world_entry_readiness.gd")
const CharacterSelectReveal = preload("res://tests/character_select_reveal.gd")
const ThreadCpuSplit = preload("res://tests/thread_cpu_split.gd")
const RebuildGuard = preload("res://tests/rebuild_guard.gd")
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
var navigation_ms: Array[float] = []
var render_ms: Array[float] = []
var draw_calls: Array[float] = []
var shadow_draw_calls: Array[float] = []
var gpu_ms: Array[float] = []
var main_cpu_ms: Array[float] = []
var last_cpu_ns := 0
var area_ms := {}
var system_totals := {}
var node_totals := {}
var gates_total := 0.0
var thread_split := ThreadCpuSplit.new()
var rebuild_guard := RebuildGuard.new()

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
		navigation_ms.append(Performance.get_monitor(Performance.TIME_NAVIGATION_PROCESS) * 1000.0)
		main_cpu_ms.append((cpu_ns - last_cpu_ns) / 1e6)
		var stats: Dictionary = client.frame_stats()
		for name in stats.systems_ms:
			system_totals[name] = system_totals.get(name, 0.0) + stats.systems_ms[name]
		for name in stats.nodes_ms:
			node_totals[name] = node_totals.get(name, 0.0) + stats.nodes_ms[name]
		gates_total += stats.attachment_gates_ms
	last_usec = now
	last_cpu_ns = cpu_ns
	return false

func tag_render_thread() -> void:
	var result := OS.set_thread_name("FPS120_RENDER")
	print("RENDER_THREAD_TAG ", result, " godot_thread_id=", OS.get_thread_caller_id())

func run_test() -> void:
	RenderingServer.call_on_render_thread(tag_render_thread)
	root.size = Vector2i(1920, 1080)
	var device = RenderingServer.get_rendering_device()
	if device == null or "llvmpipe" in device.get_device_name().to_lower() or root.size != Vector2i(1920, 1080) or DisplayServer.window_get_size() != Vector2i(1920, 1080):
		fail("Invalid GPU or resolution")
		return
	print("BENCH_DEVICE ", device.get_device_name(), " viewport=", root.size, " window=", DisplayServer.window_get_size())
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
	if client.account_state().get("ui_skin") != "Forever":
		fail("Capture requires the Forever UI skin")
		return
	print("BENCH_UI_SKIN Forever")
	player = client.get_node_or_null("WorldUnits/" + character) as Node3D
	if player == null:
		fail("No local player node WorldUnits/" + character)
		return
	client.set_camera_orbit(yaw, PITCH, DISTANCE)
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	print("BENCH_VSYNC ", DisplayServer.window_get_vsync_mode())
	if DisplayServer.window_get_vsync_mode() != DisplayServer.VSYNC_DISABLED:
		fail("VSync did not turn off")
		return
	# Let the camera and late streamed placements settle before measuring.
	await create_timer(5.0).timeout
	if not Readiness.is_ready(client.account_state()) or client.account_state().world_objects.failures != 0:
		fail("Work queued or failed after settling: " + str(client.account_state().world_objects))
		return
	var spawn := Vector2(player.position.x, player.position.z)
	print("BENCH_NODES ", JSON.stringify(node_census(root)))
	await create_timer(5.0).timeout
	write_placement_receipt()
	print_wmo_batch_counts()
	print("BENCH_SCENE ", JSON.stringify({"character": character, "spawn": [spawn.x, spawn.y],
		"objects": client.account_state().world_objects, "load": load_average()}))
	for run in range(3):
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		if image.get_size() != Vector2i(1920, 1080):
			fail("Screenshot is not 1920x1080: " + str(image.get_size()))
			return
		image.save_png(OS.get_environment("BENCH_SCREENSHOT") + "-" + str(run + 1) + ".png")
		await begin("idle_" + str(run + 1))
		for frame in range(301):
			await process_frame
		await finish("idle_" + str(run + 1), {})
		if not Readiness.is_ready(client.account_state()) or client.account_state().world_objects.failures != 0:
			fail("Work queued or failed during sample: " + str(client.account_state().world_objects))
			return
	print("FIXTURE FRAME_BENCHMARK_DONE")
	client.free()
	quit(0)

func measure(segment: String, seconds: float, step: Callable) -> void:
	await begin(segment)
	var started := Time.get_ticks_msec()
	while Time.get_ticks_msec() - started < seconds * 1000.0:
		step.call((Time.get_ticks_msec() - started) / 1000.0)
		await process_frame
	await finish(segment, {})

func begin(segment: String) -> void:
	for samples in [intervals, client_ms, process_ms, physics_ms, navigation_ms, render_ms, draw_calls, shadow_draw_calls, gpu_ms, main_cpu_ms]:
		samples.clear()
	area_ms.clear()
	system_totals.clear()
	node_totals.clear()
	gates_total = 0.0
	await thread_split.start(self)
	rebuild_guard.start(self)
	print("BENCH_MARK %s start %.6f" % [segment, Time.get_unix_time_from_system()])
	# Same clock as GODOT_FRAME_TRACE (OS ticks): frame_trace_analyze.py --from-usec/--to-usec.
	print("BENCH_TICKS %s start %d" % [segment, Time.get_ticks_usec()])
	append_proof({"event": "start", "segment": segment, "unix_time": Time.get_unix_time_from_system()})
	recording = true
	last_usec = 0

func finish(segment: String, extra: Dictionary) -> void:
	recording = false
	var rebuilds := rebuild_guard.finish()
	print("BENCH_TICKS %s end %d" % [segment, Time.get_ticks_usec()])
	print("BENCH_MARK %s end %.6f" % [segment, Time.get_unix_time_from_system()])
	var threads := await thread_split.finish(self)
	var sorted := intervals.duplicate()
	sorted.sort()
	var report := {
		"segment": segment, "frames": sorted.size(),
		"p50_ms": percentile(sorted, 0.5), "p95_ms": percentile(sorted, 0.95),
		"p99_ms": percentile(sorted, 0.99), "max_ms": sorted.back() if not sorted.is_empty() else 0.0,
		"mean_ms": mean(intervals), "client_process_ms": mean(client_ms),
		"godot_process_max_1s_ms": mean(process_ms), "physics_max_1s_ms": mean(physics_ms),
		"navigation_max_1s_ms": mean(navigation_ms),
		"draw_calls": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
		"shadow_draw_calls": mean(shadow_draw_calls), "load": load_average(),
		"main_thread_cpu_ms": mean(main_cpu_ms),
	}
	var system_means := {}
	var system_sum := 0.0
	for name in system_totals:
		var mean_ms: float = system_totals[name] / sorted.size()
		system_means[name] = mean_ms
		system_sum += mean_ms
	var node_means := {}
	for name in node_totals:
		node_means[name] = node_totals[name] / sorted.size()
	report["systems_ms"] = system_means
	report["nodes_ms"] = node_means
	report["system_sum_ms"] = system_sum
	report["attachment_gates_ms"] = gates_total / sorted.size()
	if not area_ms.is_empty():
		var names := area_ms.keys()
		names.sort_custom(func(a, b): return area_ms[a] > area_ms[b])
		var areas := {}
		for name in names.slice(0, 15):
			areas[name] = area_ms[name] / sorted.size()
		report["render_areas"] = areas
	# Render-server reads synchronize with the render thread; one read of the last
	# frame per segment keeps them out of the measured frames.
	var viewport := root.get_viewport_rid()
	report["render_cpu_ms"] = RenderingServer.viewport_get_measured_render_time_cpu(viewport) \
		+ RenderingServer.get_frame_setup_time_cpu()
	report["render_gpu_ms"] = RenderingServer.viewport_get_measured_render_time_gpu(viewport)
	report["threads"] = threads
	# Rebuild counts per frame (rebuild_guard.gd); an idle segment must list no violations.
	report.merge(rebuilds)
	if not rebuilds.has("rebuild_guard_error"):
		report["rebuild_violations"] = RebuildGuard.violations(rebuilds)
	report.merge(frame_shape())
	report.merge(extra)
	print("BENCH_SEGMENT ", JSON.stringify(report))
	append_proof({"event": "end", "segment": segment, "unix_time": Time.get_unix_time_from_system(), "report": report})

func append_proof(record: Dictionary) -> void:
	var path := OS.get_environment("BENCH_SCREENSHOT") + "-measurements.jsonl"
	var file := FileAccess.open(path, FileAccess.READ_WRITE) if FileAccess.file_exists(path) else FileAccess.open(path, FileAccess.WRITE_READ)
	if file == null:
		fail("Cannot write measurement record: " + path)
		return
	file.seek_end()
	file.store_line(JSON.stringify(record))
	file.flush()

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
		var entry: Array = census.get(name, [0, 0, 0, 0, 0, 0])
		entry[0] += 1
		entry[1] += int(node.is_processing())
		entry[2] += int(node.is_physics_processing())
		entry[3] += int(node is Node3D and node.is_visible_in_tree())
		entry[4] += int(node.is_processing_internal())
		entry[5] += int(node.is_physics_processing_internal())
		census[name] = entry
		stack.append_array(node.get_children(true))
	return census

## Nearest-rank percentile of ascending `sorted`.
## Shape of the segment's frame intervals: a 1 ms histogram, the slow frames (>= 14 ms)
## with their spacing and client time against the fast frames, and the ten worst frames
## with their neighbours. Built from per-frame arrays, so no render-server read is added.
func frame_shape() -> Dictionary:
	var histogram := {}
	var slow: Array[int] = []
	var slow_client := 0.0
	var fast_client := 0.0
	for i in intervals.size():
		var bin := str(floori(intervals[i]))
		histogram[bin] = histogram.get(bin, 0) + 1
		if intervals[i] >= 14.0:
			slow.append(i)
			slow_client += client_ms[i]
		else:
			fast_client += client_ms[i]
	var gaps: Array[int] = []
	for k in range(1, slow.size()):
		gaps.append(slow[k] - slow[k - 1])
	var order := range(intervals.size())
	order.sort_custom(func(a, b): return intervals[a] > intervals[b])
	var worst := []
	for i in order.slice(0, 10):
		worst.append({
			"i": i, "ms": snappedf(intervals[i], 0.001), "client_ms": snappedf(client_ms[i], 0.001),
			"prev_ms": snappedf(intervals[i - 1], 0.001) if i > 0 else -1.0,
			"next_ms": snappedf(intervals[i + 1], 0.001) if i + 1 < intervals.size() else -1.0,
		})
	var fast := intervals.size() - slow.size()
	return {
		"interval_histogram_ms": histogram,
		"slow_frames": {
			"count": slow.size(), "index_gaps": gaps.slice(0, 40),
			"client_ms_slow": slow_client / slow.size() if not slow.is_empty() else 0.0,
			"client_ms_fast": fast_client / fast if fast > 0 else 0.0,
		},
		"worst_frames": worst,
	}

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

func write_placement_receipt() -> void:
	var rows: Array[String] = []
	var material_groups := {}
	var stack: Array[Node] = [client]
	while not stack.is_empty():
		var node: Node = stack.pop_back()
		for child in node.get_children():
			stack.append(child)
		if not node is Node3D:
			continue
		var name := str(node.name)
		if not (name.begins_with("Doodad") or name.begins_with("Wmo")):
			continue
		var item: Node3D = node
		var source := str(node.get_meta("m2_source_path", ""))
		if source.ends_with("372894.m2"):
			var batch := node.get_node_or_null("Batch0") as MeshInstance3D
			if batch != null and batch.material_override != null:
				var material_id := str(batch.material_override.get_instance_id())
				material_groups[material_id] = material_groups.get(material_id, 0) + 1
		var value := {"path": str(node.get_path()), "source": str(node.get_meta("m2_source_path", "")), "wmo": str(node.get_meta("wmo_model", "")), "transform": str(item.transform)}
		rows.append(JSON.stringify(value))
	rows.sort()
	var file := FileAccess.open(OS.get_environment("BENCH_SCREENSHOT") + "-placements.jsonl", FileAccess.WRITE)
	if file == null:
		fail("Could not write placement receipt")
		return
	for row in rows:
		file.store_line(row)
	file.flush()
	print("PLACEMENT_RECEIPT ", rows.size())
	print("WALL_MATERIAL_GROUPS ", JSON.stringify(material_groups.values()))

func print_wmo_batch_counts() -> void:
	var total := 0
	var displayed := 0
	var shadow_only := 0
	var batch_rows: Array[String] = []
	var stack: Array[Node] = [client]
	while not stack.is_empty():
		var node: Node = stack.pop_back()
		stack.append_array(node.get_children())
		if not node is MeshInstance3D or not str(node.name).begins_with("Group") or not "_Batch" in str(node.name):
			continue
		total += 1
		batch_rows.append(JSON.stringify({"path": str(node.get_path()), "visible": node.is_visible_in_tree(), "shadow": int(node.cast_shadow)}))
		if node.is_visible_in_tree():
			if node.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY:
				shadow_only += 1
			else:
				displayed += 1
	batch_rows.sort()
	var batch_file := FileAccess.open(OS.get_environment("BENCH_SCREENSHOT") + "-wmo-batches.jsonl", FileAccess.WRITE)
	if batch_file == null:
		fail("Could not write WMO batch receipt")
		return
	for row in batch_rows:
		batch_file.store_line(row)
	batch_file.flush()
	print("WMO_BATCH_COUNTS ", JSON.stringify({"total": total, "visible_camera_mode": displayed, "visible_shadow_only": shadow_only}))
