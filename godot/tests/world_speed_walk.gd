extends SceneTree

## Dev-server regression: the client predicts at the speed the server grants, and releasing
## movement reports one stop the server applies. Place the character with
## `game-server-admin teleport <name> <map> <x> <y> <z>` while offline; deep water tests swim
## speed (the server caps a client-reported position at swim speed x its movement bank).
## Holds W for SPEED_RUN_SECONDS (default 2.5) of game time, releases, and compares the
## client's and the server's positions; then holds S (the server replicates speed x 0.6) and
## releases: only a stop input makes the server replicate the unmodified speed again.
## Requires GODOT_TEST_SERVER=127.0.0.1:5000, SPEED_ACCOUNT, SPEED_PASSWORD, SPEED_CHARACTER
## (card 0), and --fixed-fps 60 or SPEED_REAL_TIME=1. Off-screen --fixed-fps runs at ~18 fps,
## 3x slower than wall time, so the server's bank never caps it; real frame deltas can, but
## then a server simulating slower than wall time caps a correct client too.
## SPEED_YAW (radians, default PI) is the run heading; SPEED_SWIM=1 requires swimming.
## SPEED_FRAME_MS (with SPEED_REAL_TIME=1) stalls each run frame that long: a slow client,
## e.g. 667 for the ~1.5 fps loaded headless one, sends one input per long frame.
## GODOT_TEST_SERVER=127.0.0.1:<port> selects a private server.
const WORLD_WAIT_MS := 180000
const YAW_TOLERANCE := 0.05
const SETTLE_FRAMES := 60
const BACK_FRAMES := 45
const POSITION_TOLERANCE := 0.1

var NAME := OS.get_environment("SPEED_CHARACTER")
var turn_sign := 0.0

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	await process_frame
	var real_time := OS.get_environment("SPEED_REAL_TIME") == "1"
	if not real_time and not is_equal_approx(root.get_process_delta_time(), 1.0 / 60.0):
		fail("Run with --fixed-fps 60, or SPEED_REAL_TIME=1")
		return
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if not server.begins_with("127.0.0.1:"):
		fail("GODOT_TEST_SERVER must explicitly select a 127.0.0.1 server")
		return
	var account := OS.get_environment("SPEED_ACCOUNT")
	var password := OS.get_environment("SPEED_PASSWORD")
	if account == "" or password == "" or NAME == "":
		fail("SPEED_ACCOUNT, SPEED_PASSWORD and SPEED_CHARACTER are required")
		return
	var seconds := float(OS.get_environment("SPEED_RUN_SECONDS")) if OS.get_environment("SPEED_RUN_SECONDS") != "" else 2.5
	var yaw := float(OS.get_environment("SPEED_YAW")) if OS.get_environment("SPEED_YAW") != "" else PI
	var swim := OS.get_environment("SPEED_SWIM") == "1"
	var frame_ms := int(OS.get_environment("SPEED_FRAME_MS"))
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, password, false)
	if error != "":
		fail("connect: " + error)
		return
	var player := await enter_world(client)
	if player == null:
		return
	for _frame in 240:
		await process_frame
	if not await face(player, yaw):
		return
	for _frame in SETTLE_FRAMES:
		await process_frame
	var start := flat(player.position)
	var state: Dictionary = client.account_state()
	print("START client=", player.position, " server=", state.local_server_position, " swimming=", state.local_player_swimming, " server_speed=", state.local_server_speed)
	var run_started := Time.get_ticks_msec()
	push_key(KEY_W, true)
	var max_drift := 0.0
	var run_speed := 0.0
	var ran := 0.0
	var frame := 0
	while ran < seconds:
		OS.delay_msec(frame_ms)
		await process_frame
		ran += root.get_process_delta_time()
		frame += 1
		state = client.account_state()
		if swim and frame > 2 and not state.local_player_swimming:
			push_key(KEY_W, false)
			fail("Left the water at frame %d: %s" % [frame, player.position])
			return
		var drift := flat(player.position).distance_to(flat(state.local_server_position))
		if frame % 15 == 0:
			print("DRIFT frame=%d drift=%.3f server_speed=%.3f" % [frame, drift, float(state.local_server_speed)])
		max_drift = max(max_drift, drift)
		run_speed = max(run_speed, float(state.local_server_speed))
	push_key(KEY_W, false)
	var wall_seconds := (Time.get_ticks_msec() - run_started) / 1000.0
	for _frame in SETTLE_FRAMES:
		await process_frame
	state = client.account_state()
	var walked := flat(player.position).distance_to(start)
	var server_walked := flat(state.local_server_position).distance_to(start)
	var gap := flat(player.position).distance_to(flat(state.local_server_position))
	print("RUN client=", player.position, " server=", state.local_server_position, " walked=%.2f server_walked=%.2f gap=%.3f max_drift=%.3f client_speed=%.3f game_seconds=%.2f wall_seconds=%.2f server_speed=%.3f" % [walked, server_walked, gap, max_drift, walked / ran, ran, wall_seconds, run_speed])
	if walked < 1.0:
		fail("Held W did not move the player: %.2f yd" % walked)
		return
	if gap > POSITION_TOLERANCE:
		fail("Server stopped %.3f yd from the client after the run" % gap)
		return
	var forward_speed := float(state.local_server_speed)
	push_key(KEY_S, true)
	var back_speed := 0.0
	for _frame in BACK_FRAMES:
		await process_frame
		back_speed = float(client.account_state().local_server_speed)
	push_key(KEY_S, false)
	for _frame in SETTLE_FRAMES:
		await process_frame
	state = client.account_state()
	gap = flat(player.position).distance_to(flat(state.local_server_position))
	print("BACK client=", player.position, " server=", state.local_server_position, " gap=%.3f forward_speed=%.3f back_speed=%.3f stopped_speed=%.3f" % [gap, forward_speed, back_speed, float(state.local_server_speed)])
	if absf(back_speed - forward_speed * 0.6) > 0.01:
		fail("Backpedal replicated %.3f, not 0.6 × %.3f" % [back_speed, forward_speed])
		return
	if absf(float(state.local_server_speed) - forward_speed) > 0.01:
		fail("Releasing S sent no stop: the server still replicates %.3f" % float(state.local_server_speed))
		return
	if gap > POSITION_TOLERANCE:
		fail("Server stopped %.3f yd from the client after backpedalling" % gap)
		return
	print("PASS: the server follows the client at %.3f yd/s and applies the stop" % forward_speed)
	client.free()
	quit(0)

func flat(position: Vector3) -> Vector2:
	return Vector2(position.x, position.z)

func facing(player: Node3D) -> float:
	return player.rotation.y + PI / 2.0

## Turn with the real turn keys until the player's forward (sin, cos) has yaw `target`.
func face(player: Node3D, target: float) -> bool:
	for _attempt in 600:
		var error := angle_difference(facing(player), target)
		if abs(error) < YAW_TOLERANCE:
			release_turns()
			await process_frame
			return true
		if turn_sign == 0.0:
			var before := facing(player)
			await hold_key(KEY_RIGHT, 5)
			turn_sign = sign(angle_difference(before, facing(player)))
			if turn_sign == 0.0:
				fail("Turn keys do not change facing")
				return false
			continue
		var right: bool = sign(error) == turn_sign
		push_key(KEY_RIGHT, right)
		push_key(KEY_LEFT, not right)
		await process_frame
		release_turns()
	fail("Could not face %.3f" % target)
	return false

func release_turns() -> void:
	push_key(KEY_RIGHT, false)
	push_key(KEY_LEFT, false)

func enter_world(client: Node) -> Node3D:
	var deadline := Time.get_ticks_msec() + 60000
	var ui = null
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		ui = client.get_node_or_null("CharacterSelectUI")
		if state.screen == "CharacterSelect" and state.reply_received and ui != null:
			break
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return null
	await click_control(ui.find_child("CharCard_0", true, false))
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != NAME:
		fail("Card 0 is %s, not %s" % [selected.text, NAME])
		return null
	await click_control(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen != "InWorld" or state.terrain.pending_count != 0:
			continue
		var player := client.get_node_or_null("WorldUnits/" + NAME) as Node3D
		var camera := client.get_node_or_null("WorldCamera") as Camera3D
		if player == null or camera == null or not camera.current:
			continue
		for _frame in 60:
			await process_frame
		print("WORLD READY map=", state.terrain.map, " player=", player.global_position)
		return player
	fail("Timed out waiting for world: " + str(client.account_state()))
	return null

func hold_key(keycode: Key, frames: int) -> void:
	push_key(keycode, true)
	for _frame in frames:
		await process_frame
	push_key(keycode, false)
	await process_frame

func push_key(keycode: Key, pressed: bool) -> void:
	var key := InputEventKey.new()
	key.physical_keycode = keycode
	key.pressed = pressed
	root.push_input(key, true)

func click_control(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
