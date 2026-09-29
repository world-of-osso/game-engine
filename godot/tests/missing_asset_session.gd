extends SceneTree

# A missing asset is content, not a session failure (docs/specs/godot-conversion.md,
# "Frame failure policy"): in world on the dev server, the cursor and then the target ring
# are pointed at texture FDIDs that are not on disk. Each is reported by exactly one engine
# error naming its file, the session stays connected in world, the player still moves
# with a server-confirmed position, and F1 still targets the player (echoed by the
# server) without the ring.

const ACCOUNT := "fb_camera"
const PASSWORD := "fbtest"
const MISSING_CURSOR := 999999991
const MISSING_RING := 999999992
const WORLD_WAIT_MS := 120000
const MOVE_WAIT_MS := 10000
const SHOTS := "/tmp/claude/missing-asset/"

class ErrorLog extends Logger:
	var errors: Array[String] = []
	var mutex := Mutex.new()

	func _log_error(function: String, file: String, line: int, code: String, rationale: String, editor_notify: bool, error_type: int, script_backtraces: Array[ScriptBacktrace]) -> void:
		mutex.lock()
		errors.append(code + " " + rationale)
		mutex.unlock()

	func matching(text: String) -> Array[String]:
		mutex.lock()
		var found: Array[String] = errors.filter(func(error): return error.contains(text))
		mutex.unlock()
		return found

var client: Node
var log := ErrorLog.new()

func _initialize() -> void:
	Engine.max_fps = 60
	OS.add_logger(log)
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	DirAccess.make_dir_recursive_absolute(SHOTS)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5000":
		fail("GODOT_TEST_SERVER must explicitly select 127.0.0.1:5000")
		return
	for fdid in [MISSING_CURSOR, MISSING_RING]:
		if FileAccess.file_exists("res://../data/textures/%d.blp" % fdid):
			fail("Probe FDID %d exists on disk" % fdid)
			return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, ACCOUNT, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	await frames(60)

	var override: String = client.override_texture_fdid("cursor", MISSING_CURSOR)
	if override != "":
		fail(override)
		return
	await frames(120)
	if not await assert_session_survives(str(MISSING_CURSOR), "missing cursor"):
		return
	print("PASS_STEP cursor: one error, session connected in world")

	override = client.override_texture_fdid("target_ring", MISSING_RING)
	if override != "":
		fail(override)
		return
	await tap(KEY_F1)
	await frames(120)
	var target: Dictionary = client.target_state()
	if target.target == null or target.server_target != target.target or target.circle_on != null:
		fail("F1 must target the player, echoed by the server, without a ring: " + str(target))
		return
	if not await assert_session_survives(str(MISSING_RING), "missing target ring"):
		return
	print("PASS_STEP target ring: one error, player targeted, session connected")
	await capture("world-after-missing-assets.png")
	print("SCREENSHOT ", SHOTS + "world-after-missing-assets.png")
	client.queue_free()
	await frames(2)
	print("PASS: missing cursor and target ring art leave the session running")
	quit(0)

func assert_session_survives(fdid: String, label: String) -> bool:
	var state: Dictionary = client.account_state()
	if not state.connected or state.screen != "InWorld":
		fail("Session ended after %s: %s" % [label, str(state)])
		return false
	var reports := log.matching(fdid)
	if reports.size() != 1:
		fail("Expected one error for %s, got %d: %s" % [label, reports.size(), str(reports)])
		return false
	var dropped := log.matching("No active account connection")
	if not dropped.is_empty():
		fail("Account connection dropped after %s: %s" % [label, str(dropped)])
		return false
	return await assert_player_moves(label)

# Walk forward: the server-confirmed position changes only over a live session.
func assert_player_moves(label: String) -> bool:
	var start = client.account_state().local_server_position
	push_key(KEY_W, true)
	var deadline := Time.get_ticks_msec() + MOVE_WAIT_MS
	var moved := false
	while Time.get_ticks_msec() < deadline and not moved:
		await process_frame
		var now = client.account_state().local_server_position
		moved = now != null and start != null and now.distance_to(start) > 1.0
	push_key(KEY_W, false)
	await frames(30)
	if not moved:
		fail("Server position did not follow input after %s: %s" % [label, str(client.account_state())])
	return moved

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1:
			break
	var ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await click_control(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null:
			print("FIXTURE IN_WORLD ", state.selected_character_name, " at ", state.local_server_position)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func click_control(control: Control) -> void:
	if control == null:
		fail("Missing control to click")
		return
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
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await frames(3)

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.save_png(SHOTS + file) != OK:
		fail("Could not save " + file)

func tap(code: Key) -> void:
	push_key(code, true)
	await process_frame
	push_key(code, false)
	await frames(2)

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
