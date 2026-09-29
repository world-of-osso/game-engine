extends SceneTree

## Live proof, over two clients on one server, that another player's model plays the
## locomotion its replicated `PlayerMotion` (Retail `MovementFlags`) selects. Run the
## same script twice:
## - REMOTE_MOTION_ROLE=mover: enters the world, waits for the observer, then runs,
##   backpedals, strafes left and right, walks (Z), jumps in place and jumps running with
##   real key events, stopping between each, and announces every phase in a file.
## - REMOTE_MOTION_ROLE=observer: enters the world, turns to face the mover's character and
##   records its model's animation ID each frame per announced phase. It passes when every
##   moving phase played its clip with changing bones, the jumps played JumpStart 37,
##   Jump 38 and a landing, and every stop returned to Stand 0.
## Environment: GODOT_TEST_SERVER (127.0.0.1:<port>), REMOTE_MOTION_ACCOUNT,
## REMOTE_MOTION_PASSWORD, REMOTE_MOTION_CHARACTER (card 0), REMOTE_MOTION_OTHER (the other
## client's character) and REMOTE_MOTION_SYNC (a directory both clients share). Place both
## characters a few yards apart with `game-server-admin teleport` while they are offline.

const Probe := preload("res://tests/player_locomotion_probe.gd")
const SHOT_DIR := "/tmp/claude"
const READY_FILE := "observer_ready"
const PHASE_FILE := "mover_phase"
const YAW_TOLERANCE := 0.12

## Mover schedule: phase, keys held (pressed at the start, released at the end), ms.
const SCHEDULE := [
	["stand", [], 1500],
	["run", [KEY_W], 1200],
	["stop_run", [], 1500],
	["backpedal", [KEY_S], 1500],
	["stop_backpedal", [], 1500],
	["strafe_left", [KEY_A], 1200],
	["stop_strafe_left", [], 1500],
	["strafe_right", [KEY_D], 1200],
	["stop_strafe_right", [], 1500],
	["walk", [KEY_W], 1500],
	["stop_walk", [], 1500],
	["jump", [], 2000],
	["stop_jump", [], 1000],
	["run_jump", [KEY_W], 1600],
	["stop_run_jump", [], 1500],
]
## Moving phases: the clip other clients must play, with changing bones.
const MOVING := {"run": 5, "backpedal": 13, "strafe_left": 11, "strafe_right": 12, "walk": 4}

var NAME := OS.get_environment("REMOTE_MOTION_CHARACTER")
var OTHER := OS.get_environment("REMOTE_MOTION_OTHER")
var SYNC := OS.get_environment("REMOTE_MOTION_SYNC")
var turn_sign := 0.0
var shot := 0

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":0"):
		fail("GODOT_TEST_SERVER must select a loopback server port")
		return
	var role := OS.get_environment("REMOTE_MOTION_ROLE")
	var account := OS.get_environment("REMOTE_MOTION_ACCOUNT")
	var password := OS.get_environment("REMOTE_MOTION_PASSWORD")
	if account == "" or password == "" or NAME == "" or OTHER == "" or SYNC == "":
		fail("REMOTE_MOTION_ACCOUNT, _PASSWORD, _CHARACTER, _OTHER and _SYNC are required")
		return
	if role != "mover" and role != "observer":
		fail("REMOTE_MOTION_ROLE must be mover or observer")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, password, false)
	if error != "":
		fail("connect: " + error)
		return
	if not await enter_world(client):
		return
	if role == "mover":
		await move(client)
	else:
		await observe(client)

# --- Mover -------------------------------------------------------------------------

func move(client: Node) -> void:
	var deadline := Time.get_ticks_msec() + 180000
	while not FileAccess.file_exists(SYNC.path_join(READY_FILE)):
		if Time.get_ticks_msec() > deadline:
			fail("Observer never became ready")
			return
		await process_frame
	var player := client.get_node("WorldUnits/" + NAME) as Node3D
	var start := player.global_position
	for entry in SCHEDULE:
		var phase: String = entry[0]
		var keys: Array = entry[1]
		# Z toggles walking for the walk phase and back to running after it.
		if phase == "walk" or phase == "stop_walk":
			await tap(KEY_Z)
		announce(phase)
		for key in keys:
			push_key(key, true)
		if phase == "jump":
			await tap(KEY_SPACE)
		if phase == "run_jump":
			await wait_ms(400)
			await tap(KEY_SPACE)
			await wait_ms(int(entry[2]) - 400)
		else:
			await wait_ms(int(entry[2]))
		for key in keys:
			push_key(key, false)
		print("TRACE mover %s ends at %s (%.1f yd from start)" % [phase, player.global_position, player.global_position.distance_to(start)])
	announce("done")
	await wait_ms(2000)
	print("PASS: mover ran its schedule")
	quit(0)

func announce(phase: String) -> void:
	var file := FileAccess.open(SYNC.path_join(PHASE_FILE), FileAccess.WRITE)
	file.store_string(phase)
	file.close()
	print("TRACE phase %s at %d" % [phase, Time.get_ticks_msec()])

func tap(key: Key) -> void:
	push_key(key, true)
	for _frame in 3:
		await process_frame
	push_key(key, false)
	await process_frame

func wait_ms(ms: int) -> void:
	var until := Time.get_ticks_msec() + ms
	while Time.get_ticks_msec() < until:
		await process_frame

# --- Observer ----------------------------------------------------------------------

func observe(client: Node) -> void:
	var player := client.get_node("WorldUnits/" + NAME) as Node3D
	var other := await find_other(client)
	if other == null:
		return
	var probe := Probe.new()
	var bound := probe.bind(other)
	if bound != "":
		fail(OTHER + ": " + bound)
		return
	var deadline := Time.get_ticks_msec() + 20000
	while not await face(player, other):
		if Time.get_ticks_msec() > deadline:
			fail("Could not turn to face " + OTHER)
			return
	release_turn()
	for _frame in 30:
		await process_frame
	await snapshot("ready", other)
	var ready := FileAccess.open(SYNC.path_join(READY_FILE), FileAccess.WRITE)
	ready.close()
	print("TRACE observer ready; %s plays %d" % [OTHER, probe.animation.current_animation_id()])
	var timeline := {}
	var phase := ""
	var proven := {}
	var deadline_done := Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline_done:
		await process_frame
		var announced := read_phase()
		if announced != phase:
			phase = announced
			print("TRACE observer sees phase %s" % phase)
			if phase == "done":
				break
		if phase == "":
			continue
		var id: int = probe.animation.current_animation_id()
		var ids: Array = timeline.get_or_add(phase, [])
		if ids.is_empty() or ids[-1] != id:
			ids.append(id)
			print("TRACE %s: %s plays %d" % [phase, OTHER, id])
		if MOVING.has(phase) and id == MOVING[phase] and not proven.has(phase):
			proven[phase] = await prove_bones_move(probe, other, phase, id)
		if phase in ["jump", "run_jump"] and id == 38 and not proven.has(phase):
			proven[phase] = true
			await snapshot(phase + "-airborne", other)
	if phase != "done":
		fail("Mover did not finish; timeline %s" % timeline)
		return
	var failures := judge(timeline, proven)
	if not failures.is_empty():
		fail("\n".join(failures) + "\ntimeline " + str(timeline))
		return
	await snapshot("stand", other)
	print("PASS: %s played %s" % [OTHER, timeline])
	quit(0)

func judge(timeline: Dictionary, proven: Dictionary) -> Array[String]:
	var failures: Array[String] = []
	for phase in MOVING:
		var ids: Array = timeline.get(phase, [])
		if not ids.has(MOVING[phase]):
			failures.append("%s: never played %d (%s)" % [phase, MOVING[phase], ids])
		elif not proven.get(phase, false):
			failures.append("%s: bones did not change while playing %d" % [phase, MOVING[phase]])
	for phase in ["jump", "run_jump"]:
		# The landing can outlast the phase; it then shows at the start of the stop.
		var ids: Array = timeline.get(phase, []) + timeline.get("stop_" + phase, [])
		var start := ids.find(37)
		var loop := ids.find(38, maxi(start, 0))
		var land := maxi(ids.find(39, maxi(loop, 0)), ids.find(187, maxi(loop, 0)))
		if start < 0 or loop < start or land < loop:
			failures.append("%s: not JumpStart 37 -> Jump 38 -> landing 39/187 (%s)" % [phase, ids])
	for entry in SCHEDULE:
		var phase: String = entry[0]
		if not phase.begins_with("stop_"):
			continue
		var ids: Array = timeline.get(phase, [])
		if ids.is_empty() or ids[-1] != 0:
			failures.append("%s: did not return to Stand 0 (%s)" % [phase, ids])
	return failures

func read_phase() -> String:
	var path := SYNC.path_join(PHASE_FILE)
	if not FileAccess.file_exists(path):
		return ""
	return FileAccess.get_file_as_string(path).strip_edges()

func find_other(client: Node) -> Node3D:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var other := client.get_node_or_null("WorldUnits/" + OTHER) as Node3D
		if other != null and other.find_child("PlayerModel", true, false) != null:
			print("TRACE found %s at %s" % [OTHER, other.global_position])
			return other
	fail("%s never appeared with a PlayerModel" % OTHER)
	return null

func prove_bones_move(probe: Probe, other: Node3D, phase: String, id: int) -> bool:
	var before := probe.capture_pose()
	await snapshot(phase + "-a", other)
	for _frame in 12:
		await process_frame
	await snapshot(phase + "-b", other)
	var changed := probe.changed_from(before)
	print("TRACE %s: %s bones %s while playing %d" % [phase, OTHER, "changed" if changed else "DID NOT change", id])
	return changed

## Turn the character toward `target` with the arrow keys; whether it already faces it.
func face(player: Node3D, target: Node3D) -> bool:
	await process_frame
	var d := target.global_position - player.global_position
	var error := angle_difference(player.rotation.y + PI / 2.0, atan2(d.x, d.z))
	if turn_sign == 0.0:
		var before := player.rotation.y
		push_key(KEY_RIGHT, true)
		for _frame in 10:
			await process_frame
		push_key(KEY_RIGHT, false)
		await process_frame
		turn_sign = 1.0 if angle_difference(before, player.rotation.y) >= 0.0 else -1.0
		return false
	push_key(KEY_RIGHT, abs(error) > YAW_TOLERANCE and sign(error) == turn_sign)
	push_key(KEY_LEFT, abs(error) > YAW_TOLERANCE and sign(error) == -turn_sign)
	return abs(error) <= YAW_TOLERANCE

func release_turn() -> void:
	push_key(KEY_RIGHT, false)
	push_key(KEY_LEFT, false)

func snapshot(label: String, target: Node3D) -> void:
	await RenderingServer.frame_post_draw
	DirAccess.make_dir_recursive_absolute(SHOT_DIR)
	var path := "%s/remote-motion-%02d-%s.png" % [SHOT_DIR, shot, label]
	shot += 1
	root.get_texture().get_image().save_png(path)
	var camera := root.get_camera_3d()
	print("TRACE screenshot %s, %s at pixel %s" % [path, OTHER, camera.unproject_position(target.global_position)])

# --- Shared ------------------------------------------------------------------------

func enter_world(client: Node) -> bool:
	if not await wait_until(client, func(state): return state.screen == "CharacterSelect" and state.reply_received, 20000, "CharacterSelect"):
		return false
	var ui = client.get_node("CharacterSelectUI")
	await click_control(ui.find_child("CharCard_0", true, false))
	await process_frame
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != NAME:
		fail("Card 0 is %s, not %s" % [selected.text, NAME])
		return false
	await click_control(ui.find_child("EnterWorld", true, false))
	var ready := func(state):
		return state.screen == "InWorld" and state.selected_character_name == NAME \
			and state.local_player_position != null and state.terrain.pending_count == 0 \
			and not state.terrain.parsed_tiles.is_empty()
	if not await wait_until(client, ready, 120000, "InWorld with terrain"):
		return false
	for _frame in 60:
		await process_frame
	print("TRACE entered as ", NAME)
	return true

func wait_until(client: Node, predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(client.account_state()):
			return true
	fail("Timed out waiting for %s: %s" % [what, client.account_state()])
	return false

func push_key(keycode: Key, pressed: bool) -> void:
	var key := InputEventKey.new()
	key.physical_keycode = keycode
	key.keycode = keycode
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
