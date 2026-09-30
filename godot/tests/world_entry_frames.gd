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
## frame until the object queue drains, and 120 frames after, must stay under
## WORLD_ENTRY_FRAME_MS.

const PASSWORD := "fbtest"

var client: Node
var last_usec := 0
## "select", "loading" (Enter World pressed until InWorld) or "world".
var phase := "select"
var loading_ms: Array[float] = []
var world_ms: Array[float] = []
## Frames over the threshold after the loading screen hid: [seconds since hide, ms].
var slow_world: Array = []
var world_started_usec := 0
var frame_limit := 100.0

func _initialize() -> void:
	call_deferred("run_test")

func _process(_delta: float) -> bool:
	# Wall clock: Godot caps the process delta, so the delta hides a stall.
	var now := Time.get_ticks_usec()
	if last_usec > 0:
		var ms := (now - last_usec) / 1000.0
		if phase == "loading":
			loading_ms.append(ms)
		elif phase == "world":
			world_ms.append(ms)
			if ms > frame_limit:
				# The client's last main-thread process time and the viewport's render CPU time.
				var rid := root.get_viewport_rid()
				slow_world.append([(now - world_started_usec) / 1e6, ms, client.process_ms(),
					RenderingServer.viewport_get_measured_render_time_cpu(rid) + RenderingServer.get_frame_setup_time_cpu()])
	last_usec = now
	if client != null and is_instance_valid(client) and phase == "loading" and client.account_state().screen == "InWorld":
		phase = "world"
		world_started_usec = now
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
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await select_character(character):
		return
	var pressed_usec := Time.get_ticks_usec()
	phase = "loading"
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
	for i in 120:
		await process_frame
	phase = "done"
	var objects: Dictionary = client.account_state().world_objects
	var sorted := world_ms.duplicate()
	sorted.sort()
	print("FIXTURE WORLD_ENTRY loading_s=%.1f loading_frames=%d loading_max_ms=%.1f world_frames=%d world_median_ms=%.1f world_p99_ms=%.1f world_max_ms=%.1f over_%d_ms=%d settled=%s settled_s=%.1f objects=%s" % [
		loading_s, loading_ms.size(), max_of(loading_ms), world_ms.size(),
		sorted[sorted.size() / 2], sorted[int(sorted.size() * 0.99)], max_of(world_ms),
		int(frame_limit), slow_world.size(), settled, settled_s, objects])
	print("FIXTURE SLOW_WORLD_FRAMES ", slow_world.map(func(f): return "%.1fs:%.0fms(client %.0f render %.0f)" % f))
	var failures: Array[String] = []
	if max_of(loading_ms) > loading_limit:
		failures.append("a loading frame took %.1f ms (limit %.0f)" % [max_of(loading_ms), loading_limit])
	if not slow_world.is_empty():
		failures.append("%d frames after the loading screen hid exceeded %.0f ms" % [slow_world.size(), frame_limit])
	if not settled:
		failures.append("world_objects.pending did not reach 0 within %.0f s: %s" % [settle_s, objects])
	if not failures.is_empty():
		fail("; ".join(failures))
		return
	print("FIXTURE WORLD_ENTRY_FRAMES_DONE")
	client.free()
	quit(0)

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
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1:
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
