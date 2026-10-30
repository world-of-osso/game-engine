extends SceneTree

# Place the private character with the server admin before this receipt census.
const PASSWORD := "fbtest"
const Readiness = preload("res://tests/world_entry_readiness.gd")
const CharacterSelectReveal = preload("res://tests/character_select_reveal.gd")
const PITCH := -0.35
const DISTANCE := 15.0
var client: Node
var player: Node3D

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
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
	var yaw := float(OS.get_environment("BENCH_CAMERA_YAW")) if OS.get_environment("BENCH_CAMERA_YAW") != "" else 0.0
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
	client.set_camera_orbit(yaw, PITCH, DISTANCE)
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	print("BENCH_VSYNC ", DisplayServer.window_get_vsync_mode())
	if DisplayServer.window_get_vsync_mode() != DisplayServer.VSYNC_DISABLED:
		fail("VSync did not turn off")
		return
	# Let the camera and late streamed placements settle before measuring.
	await create_timer(5.0).timeout
	if not Readiness.is_ready(client.account_state()):
		fail("Work queued or failed after settling: " + str(client.account_state().world_objects))
		return
	var spawn := Vector2(player.position.x, player.position.z)
	print("BENCH_SCENE ", JSON.stringify({"character": character, "spawn": [spawn.x, spawn.y],
		"objects": client.account_state().world_objects}))
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(OS.get_environment("BENCH_SCREENSHOT") + ".png")
	print("ZONE_REPORT ", JSON.stringify(client.account_state().world_objects))
	print("FIXTURE FRAME_BENCHMARK_DONE")
	client.free()
	quit(0)

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
