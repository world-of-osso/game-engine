extends SceneTree

## Player health regeneration against a private server (game-server branch `regen`).
## Environment:
##   GODOT_TEST_SERVER              server address (a private test server)
##   REGEN_ACCOUNT / REGEN_CHARACTER account (password fbtest) and its only character
##   REGEN_SHOTS                    screenshot directory
## Self-targets (F1) so the TargetFrame shows the player's replicated health, prints
## FIXTURE READY, then waits for health to drop (the operator deals `.damage` through the
## admin socket), samples the TargetFrame health text every second and exits 0 once it is
## back at the full value it started with.

const PASSWORD := "fbtest"

var client: Node
var shots := "/tmp/claude/regen-live/"
var character := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("REGEN_ACCOUNT")
	character = OS.get_environment("REGEN_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, REGEN_ACCOUNT and REGEN_CHARACTER are required")
		return
	if OS.get_environment("REGEN_SHOTS") != "":
		shots = OS.get_environment("REGEN_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	await press(KEY_F1)
	if not await wait_until(func(): return target().target != null and target().server_target == target().target and str(target().health_text) != "", 10000, "self-target echoed by the server"):
		return
	var full := str(target().health_text)
	print("FIXTURE READY target=%s health=%s" % [target().target, full])
	await capture("01-full.png")
	if not await wait_until(func(): return str(target().health_text) != full, 180000, "damage"):
		return
	var start := Time.get_ticks_msec()
	var last := str(target().health_text)
	print("FIXTURE SAMPLE t=0 health=%s" % last)
	await capture("02-damaged.png")
	while Time.get_ticks_msec() - start < 180000:
		await wait_real(0.25)
		var now := str(target().health_text)
		if now != last:
			last = now
			print("FIXTURE SAMPLE t=%d health=%s" % [Time.get_ticks_msec() - start, now])
		if now == full:
			await wait_real(4.0)
			print("FIXTURE FULL t=%d health=%s after=%s" % [Time.get_ticks_msec() - start, now, target().health_text])
			await capture("03-full-again.png")
			if str(target().health_text) != full:
				fail("Health left full: " + str(target().health_text))
				return
			client.free()
			quit(0)
			return
	fail("Health did not climb back to %s: %s" % [full, last])

func target() -> Dictionary:
	return client.target_state()

func wait_until(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("Timed out waiting for %s: target=%s" % [what, client.target_state()])
	return false

func enter_world() -> bool:
	# Local CASC startup holds the character select UI back (`assets_starting`).
	var deadline := Time.get_ticks_msec() + 180000
	var ui = null
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		ui = client.get_node_or_null("CharacterSelectUI")
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 and ui != null:
			break
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await click(ui.find_child("CharCard_0", true, false))
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != character:
		fail("Card 0 is %s, not %s" % [selected.text, character])
		return false
	await click(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and state.local_server_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD at ", state.local_player_position)
			await wait_frames(60)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func press(code: Key) -> void:
	push_key(code, true)
	await wait_frames(2)
	push_key(code, false)
	await wait_frames(2)

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func move_mouse(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(2)

func click(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	await move_mouse(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func capture(file: String) -> void:
	print("FIXTURE MARK %s frame=%d" % [file, Engine.get_frames_drawn()])
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(shots + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func wait_real(seconds: float) -> void:
	var deadline := Time.get_ticks_msec() + int(seconds * 1000)
	while Time.get_ticks_msec() < deadline:
		await process_frame

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
