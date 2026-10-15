extends SceneTree

# P1 native acceptance: actual authored tail-only Blood Presence emitter, reached
# through particledebug. Hold simulation fixed when hiding the native particle pool.
const TAIL_MODEL := 165894
const WAIT_MS := 60000
var scene: Node

func _initialize() -> void:
	call_deferred("run_test")

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var output := OS.get_environment("GODOT_PARTICLE_SCREENSHOTS")
	if output.is_empty() or not DirAccess.dir_exists_absolute(output):
		fail("GODOT_PARTICLE_SCREENSHOTS must name an existing evidence directory")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var startup_deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < startup_deadline:
		scene = client.get_node_or_null("ParticleDebug")
		if scene != null:
			break
		await process_frame
	if scene == null:
		fail("particledebug screen missing")
		return
	for index in 3:
		var event := InputEventKey.new()
		event.keycode = KEY_TAB
		event.pressed = true
		Input.parse_input_event(event)
		await process_frame
		event = InputEventKey.new()
		event.keycode = KEY_TAB
		event.pressed = false
		Input.parse_input_event(event)
		await process_frame
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		var state: Dictionary = scene.debug_state()
		if state.get("model_fdid", 0) == TAIL_MODEL and state.get("drawn", 0) > 0:
			break
		await process_frame
	var state: Dictionary = scene.debug_state()
	if state.get("model_fdid", 0) != TAIL_MODEL or state.get("drawn", 0) <= 0:
		fail("Real tail-only emitter did not draw: " + str(state))
		return
	if not state.missing_textures.is_empty() or not state.overlay.contains("flags=0x40422"):
		fail("Real tail asset has missing texture or unexpected authored flags: " + str(state))
		return
	await create_timer(1.0).timeout
	# Freeze bone/material descendants too, so the surrounding red glow cannot
	# contaminate the particle-only difference.
	scene.process_mode = Node.PROCESS_MODE_DISABLED
	var pool := scene.find_child("Particles%d_0" % TAIL_MODEL, true, false) as MultiMeshInstance3D
	if pool == null or pool.multimesh.visible_instance_count <= 0:
		fail("No real native tail pool")
		return
	var shown := await capture(output + "/p1-bloodpresence-tails-on.png")
	var stable := await capture(output + "/p1-bloodpresence-tails-stable.png")
	if shown.get_data() != stable.get_data():
		fail("Frozen scene is not pixel-stable before hiding the tail pool")
		return
	pool.visible = false
	var hidden := await capture(output + "/p1-bloodpresence-tails-off.png")
	var changed := 0
	for y in shown.get_height():
		for x in shown.get_width():
			var a := shown.get_pixel(x, y)
			var b := hidden.get_pixel(x, y)
			if a != b:
				changed += 1
	print("P1_REAL_TAIL_PIXELS ", changed, " visible quads ", pool.multimesh.visible_instance_count)
	if changed <= 0:
		fail("Tail-only native pool changed no visible pixels")
		return
	print("PASS: clean-room P1 real tail-only pixels at 1920x1080")
	quit(0)

func capture(path: String) -> Image:
	await process_frame
	await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.get_size() != Vector2i(1920, 1080):
		fail("Wrong capture size: " + str(image.get_size()))
	if image.save_png(path) != OK:
		fail("Cannot save " + path)
	return image
