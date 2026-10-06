extends SceneTree

# Owned UDP 5286 / fb_swload2 fixture. Preserve the same assets and driver for
# before/after; report entry state separately from the first rendered frame.
var client: Node
var artifacts := OS.get_environment("SWLOAD_ARTIFACTS")
var phase := OS.get_environment("SWLOAD_PHASE")

func _initialize() -> void:
	call_deferred("run_capture")

func run_capture() -> void:
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().get("assets_starting", true) and Time.get_ticks_msec() < deadline:
		await process_frame
	var error: String = client.connect_account("127.0.0.1:5286", "fb_swload2", "fbtest", false)
	if not error.is_empty():
		fail(error)
		return
	deadline = Time.get_ticks_msec() + 90000
	while client.account_state().screen != "CharacterSelect" and Time.get_ticks_msec() < deadline:
		await process_frame
	var ui := client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		fail("Character select unavailable: " + str(client.account_state()))
		return
	for _frame in range(10):
		await process_frame
	await click_control(ui.find_child("CharCard_0", true, false))
	var started := Time.get_ticks_msec()
	await click_control(ui.find_child("EnterWorld", true, false))
	var next_report := started
	var entered := -1
	var pending_at_entry := -1
	var frames := 0
	while Time.get_ticks_msec() - started < 180000:
		await process_frame
		frames += 1
		var state: Dictionary = client.account_state()
		var elapsed := Time.get_ticks_msec() - started
		if state.screen == "InWorld" and entered < 0:
			entered = elapsed
			var objects: Dictionary = state.world_objects
			if state.area_id != 1519 or objects.nearby_total <= 0 or objects.nearby_done != objects.nearby_total or objects.nearby_collision_pending != 0 or objects.failures != 0:
				fail("Entered with incomplete Stormwind prerequisites: " + str(state))
				return
			pending_at_entry = objects.pending
			print("SWLOAD INWORLD ms=", elapsed, " frames=", frames, " objects=", objects)
			if DisplayServer.get_name() != "headless":
				await RenderingServer.frame_post_draw
				print("SWLOAD FIRST_DRAW ms=", Time.get_ticks_msec() - started)
				root.get_texture().get_image().save_webp(artifacts.path_join(phase + "-inworld.webp"))
		if Time.get_ticks_msec() >= next_report:
			next_report = Time.get_ticks_msec() + 1000
			print("SWLOAD SAMPLE ", JSON.stringify({"ms": elapsed, "frames": frames, "screen": state.screen, "area": state.area_id, "position": str(state.local_player_position), "terrain_pending": state.terrain.pending_count, "objects": state.world_objects, "process_s": Performance.get_monitor(Performance.TIME_PROCESS), "physics_s": Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS), "draw_calls": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME), "pipelines": pipeline_counts()}))
		if entered >= 0 and elapsed > entered + 5000:
			if state.screen != "InWorld" or pending_at_entry <= 0 or state.world_objects.pending >= pending_at_entry:
				fail("Distant work was lost or did not continue streaming: " + str(state))
				return
			print("SWLOAD TEST PASS entered_ms=", entered, " state=", state)
			quit(0)
			return
	fail("Stormwind entry timed out: " + str(client.account_state()))

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func click_control(control: Control) -> void:
	if control == null:
		fail("Missing entry control")
		return
	var point := control.get_global_rect().get_center()
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame

func pipeline_counts() -> Dictionary:
	var counts := {}
	for kind in ["CANVAS", "MESH", "SURFACE", "DRAW", "SPECIALIZATION"]:
		var name := "RENDERING_INFO_PIPELINE_COMPILATIONS_" + kind
		if ClassDB.class_has_integer_constant("RenderingServer", name):
			counts[kind] = RenderingServer.get_rendering_info(ClassDB.class_get_integer_constant("RenderingServer", name))
	return counts
