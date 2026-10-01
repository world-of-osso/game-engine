extends SceneTree

## Frame times from Enter World until the world has settled. Environment:
##   GODOT_TEST_SERVER          server address (a private test server)
##   WORLD_ENTRY_ACCOUNT / WORLD_ENTRY_CHARACTER   account (password fbtest) and character
##   WORLD_ENTRY_FRAME_MS       longest frame allowed once the loading screen hides
##                              (default 100)
##   WORLD_ENTRY_SETTLE_S       seconds after the loading screen hides within which
##                              world_objects.pending must reach 0 (default 300)
## The loading screen may cover world-entry loading, but no frame, loading or not, may
## block for seconds: the longest loading frame is reported, and must stay under
## WORLD_ENTRY_LOADING_FRAME_MS (default 1000). After the loading screen hides, every
## frame until the object queue drains, and a bounded settled observation after,
## must stay under WORLD_ENTRY_FRAME_MS. These limits are fixture policy, NOT product
## budgets. WORLD_ENTRY_MEASURE_S controls settled observation (60–300 s, default 60).
## PERF_PHASE / PERF_REPORT JSON records separate startup, loading, queue drain and
## settled process-frame intervals; these are not GPU or presentation timings.

const PASSWORD := "fbtest"

var client: Node
var last_usec := 0
## A frame crossing a phase boundary belongs to the outgoing phase.
var phase := "select"
var startup_ms: Array[float] = []
var loading_ms: Array[float] = []
var transition_ms: Array[float] = []
var settled_ms: Array[float] = []
var world_ms: Array[float] = []
var memory: Dictionary = {}
var script_started_usec := 0
## Frames over the threshold after the loading screen hid: [seconds since hide, ms].
var slow_world: Array = []
var world_started_usec := 0
var frame_limit := 100.0

func _initialize() -> void:
	script_started_usec = Time.get_ticks_usec()
	last_usec = script_started_usec
	record_phase("script_start")
	call_deferred("run_test")

func _process(_delta: float) -> bool:
	# Wall clock: Godot caps the process delta, so the delta hides a stall.
	var now := Time.get_ticks_usec()
	if last_usec > 0:
		var ms := (now - last_usec) / 1000.0
		if phase == "select":
			startup_ms.append(ms)
		elif phase == "loading":
			loading_ms.append(ms)
		elif phase == "world" or phase == "settled":
			world_ms.append(ms)
			if phase == "world":
				transition_ms.append(ms)
			else:
				settled_ms.append(ms)
			if ms > frame_limit:
				# The client's last main-thread process time and the viewport's render CPU time.
				var rid := root.get_viewport_rid()
				var client_ms: float = client.process_ms() if client.has_method("process_ms") else -1.0
				slow_world.append([(now - world_started_usec) / 1e6, ms, client_ms,
					RenderingServer.viewport_get_measured_render_time_cpu(rid) + RenderingServer.get_frame_setup_time_cpu()])
	last_usec = now
	if client != null and is_instance_valid(client) and phase == "loading" and client.account_state().screen == "InWorld":
		phase = "world"
		world_started_usec = now
		record_phase("loading_hidden")
	return false

func env_float(name: String, fallback: float) -> float:
	var value := OS.get_environment(name)
	return float(value) if value != "" else fallback

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("WORLD_ENTRY_ACCOUNT")
	var character := OS.get_environment("WORLD_ENTRY_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, WORLD_ENTRY_ACCOUNT and WORLD_ENTRY_CHARACTER are required")
		return
	frame_limit = env_float("WORLD_ENTRY_FRAME_MS", 100.0)
	RenderingServer.viewport_set_measure_render_time(root.get_viewport_rid(), true)
	var loading_limit := env_float("WORLD_ENTRY_LOADING_FRAME_MS", 1000.0)
	var settle_s := env_float("WORLD_ENTRY_SETTLE_S", 300.0)
	var measure_s := env_float("WORLD_ENTRY_MEASURE_S", 60.0)
	if not is_finite(settle_s) or settle_s <= 0 or settle_s > 300 \
			or not is_finite(measure_s) or measure_s < 60 or measure_s > 300 \
			or not is_finite(frame_limit) or frame_limit <= 0 \
			or not is_finite(loading_limit) or loading_limit <= 0:
		fail("Fixture limits must be finite/positive; settle <=300 s, measure 60–300 s")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	record_phase("client_mounted")
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await select_character(character):
		return
	record_phase("character_selected")
	var pressed_usec := Time.get_ticks_usec()
	phase = "loading"
	# Exclude the preceding selection frame from the Enter World interval.
	last_usec = pressed_usec
	record_phase("enter_world")
	await click(client.get_node("CharacterSelectUI").find_child("EnterWorld", true, false))
	var deadline := Time.get_ticks_msec() + 600000
	while phase == "loading" and Time.get_ticks_msec() < deadline:
		await process_frame
	if phase != "world":
		fail("Loading screen did not hide: " + str(client.account_state()))
		return
	var loading_s := (world_started_usec - pressed_usec) / 1e6
	print("FIXTURE LOADING_HIDDEN after %.1f s, %d frames, longest %.1f ms" % [loading_s, loading_ms.size(), max_of(loading_ms)])
	deadline = Time.get_ticks_msec() + int(settle_s * 1000)
	var settled := false
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.account_state().world_objects.pending == 0:
			settled = true
			break
	var settled_s := (Time.get_ticks_usec() - world_started_usec) / 1e6
	var measurement_started_usec := Time.get_ticks_usec()
	var settled_pending_changed := false
	if settled:
		record_phase("queue_drained")
		phase = "settled"
		last_usec = measurement_started_usec
		# End only after captured intervals cover the requested duration. process_frame
		# resumes before _process; elapsed wall time alone could omit the final interval.
		deadline = Time.get_ticks_msec() + int((measure_s + 5.0) * 1000)
		while last_usec - measurement_started_usec < int(measure_s * 1e6) and Time.get_ticks_msec() < deadline:
			await process_frame
			if client.account_state().world_objects.pending != 0:
				settled_pending_changed = true
	var settled_elapsed_s := (Time.get_ticks_usec() - measurement_started_usec) / 1e6 if settled else 0.0
	phase = "done"
	record_phase("settled_end" if settled else "queue_timeout")
	var objects: Dictionary = client.account_state().world_objects
	var sorted := world_ms.duplicate()
	sorted.sort()
	print("PERF_REPORT ", JSON.stringify({
		"startup_ms": startup_ms, "loading_ms": loading_ms,
		"transition_ms": transition_ms, "settled_ms": settled_ms,
		"loading_s": loading_s, "queue_drain_s": settled_s,
		"settled_elapsed_s": settled_elapsed_s, "settled_pending_changed": settled_pending_changed,
		"frame_limit_ms": frame_limit, "loading_limit_ms": loading_limit,
		"memory": memory, "objects": objects, "queue_drained": settled,
		"startup_scope": "script initialize through character selection; launch/import excluded",
	}))
	print("FIXTURE MEMORY ", resident_memory())
	print("FIXTURE WORLD_ENTRY loading_s=%.1f loading_frames=%d loading_max_ms=%.1f world_frames=%d world_median_ms=%.1f world_p99_ms=%.1f world_max_ms=%.1f over_%d_ms=%d settled=%s settled_s=%.1f objects=%s" % [
		loading_s, loading_ms.size(), max_of(loading_ms), world_ms.size(),
		sorted[sorted.size() / 2] if not sorted.is_empty() else 0.0,
		sorted[mini(int(sorted.size() * 0.99), sorted.size() - 1)] if not sorted.is_empty() else 0.0, max_of(world_ms),
		int(frame_limit), slow_world.size(), settled, settled_s, objects])
	print("FIXTURE SLOW_WORLD_FRAMES ", slow_world.map(func(f): return "%.1fs:%.0fms(client %.0f render %.0f)" % f))
	var failures: Array[String] = []
	if max_of(loading_ms) > loading_limit:
		failures.append("a loading frame took %.1f ms (limit %.0f)" % [max_of(loading_ms), loading_limit])
	if not slow_world.is_empty():
		failures.append("%d frames after the loading screen hid exceeded %.0f ms" % [slow_world.size(), frame_limit])
	if settled_pending_changed:
		failures.append("world_objects.pending changed during settled observation")
	if not settled:
		failures.append("world_objects.pending did not reach 0 within %.0f s: %s" % [settle_s, objects])
	if not failures.is_empty():
		fail("; ".join(failures))
		return
	print("FIXTURE WORLD_ENTRY_FRAMES_DONE")
	client.free()
	quit(0)

## VmRSS and VmHWM of this process (/proc/self/status).
func resident_memory() -> Dictionary:
	# Linux process RSS and lifetime HWM. Not VRAM, allocations, growth or leak proof.
	var file := FileAccess.open("/proc/self/status", FileAccess.READ)
	var found := {"VmRSS_kib": null, "VmHWM_kib": null}
	if file == null:
		found["error"] = "Cannot open /proc/self/status: %s" % FileAccess.get_open_error()
		push_error(found.error)
		return found
	while not file.eof_reached():
		var fields := file.get_line().replace("\t", " ").split(" ", false)
		if fields.size() >= 3 and fields[0] in ["VmRSS:", "VmHWM:"]:
			found[fields[0].trim_suffix(":") + "_kib"] = int(fields[1])
	if found.VmRSS_kib == null or found.VmHWM_kib == null:
		found["error"] = "VmRSS/VmHWM missing from /proc/self/status"
		push_error(found.error)
	return found

func record_phase(name: String) -> void:
	var snapshot := resident_memory()
	memory[name] = snapshot
	print("PERF_PHASE ", JSON.stringify({
		"phase": name, "since_script_s": (Time.get_ticks_usec() - script_started_usec) / 1e6,
		"memory": snapshot,
	}))

func max_of(values: Array[float]) -> float:
	var longest := 0.0
	for value in values:
		longest = maxf(longest, value)
	return longest

func select_character(character: String) -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		# The session reaches character select before CASC startup ends; its UI after.
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 \
				and client.get_node_or_null("CharacterSelectUI") != null:
			break
	var ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await click(ui.find_child("CharCard_0", true, false))
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != character:
		fail("Card 0 is %s, not %s" % [selected.text, character])
		return false
	return true

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
