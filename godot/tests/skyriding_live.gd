extends SceneTree

## Live Skyriding flight physics (part 1) on a private server with real key and mouse
## events: the character on Flight Style: Skyriding summons the Golden Gryphon (32235),
## jumps and presses Space again to launch (`shared::skyriding::LAUNCH_SPEED`), rises
## without lift to its apex and spreads its wings, dives with the right mouse button held
## and the camera pitched down (gaining speed past steady flight), pulls up (losing it),
## glides level, then dives onto the ground and lands. Every sample at 10 fps or more checks
## the server's position (what other clients draw) stays within a yard of the client's path.
## Launch from a high point (the driver `set-position`s the character on the peak east of
## Northshire, WoW (-8793, 213, 382)) so the dive has room.
## Environment: GODOT_TEST_SERVER (private server), SKY_ACCOUNT / SKY_CHARACTER (password
## fbtest, card 0). SKY_READY_FILE: wait in the world until that file exists (the driver
## creates it after, on the online character:
##   game-server-admin learn-spell <name> 34090
##   game-server-admin learn-spell <name> 32235
##   game-server-admin flight-style <name> skyriding
## ). SKY_SHOTS: screenshot directory.

const PASSWORD := "fbtest"
const GOLDEN_GRYPHON := 32235
## `FLIGHT_SPEED` × Skyriding 406095's MOD_INCREASE_MOUNTED_FLIGHT_SPEED (+310%): the fixed
## speed a steady flyer would have on this aura.
const STEADY_EQUIVALENT := 7.0 * 4.1
const SAMPLE_SECS := 0.1

var client: Node
var player: Node3D
var character := ""
var shots := "/tmp/claude/skyriding-live"
var shot := 0
var worst_gap := 0.0
var top_speed := 0.0
var last_position := Vector3.ZERO

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(960, 540)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("SKY_ACCOUNT")
	character = OS.get_environment("SKY_CHARACTER")
	if server == "" or server == "127.0.0.1:5000" or account == "" or character == "":
		fail("GODOT_TEST_SERVER (a private server), SKY_ACCOUNT and SKY_CHARACTER are required")
		return
	if OS.get_environment("SKY_SHOTS") != "":
		shots = OS.get_environment("SKY_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("connect: " + error)
		return
	if not await enter_world():
		return
	player = client.get_node("WorldUnits/" + character) as Node3D
	var ready_file := OS.get_environment("SKY_READY_FILE")
	if ready_file != "" and not await wait_until(func(): return FileAccess.file_exists(ready_file), 120000, ready_file):
		return
	client.set_world_minutes(720.0)
	client.set_camera_orbit(PI, -0.3, 12.0)
	await wait_frames(30)

	var sent: String = client.use_spell(GOLDEN_GRYPHON)
	if sent != "":
		fail("mount cast: " + sent)
		return
	if not await wait_until(func(): return mounted(), 30000, "mounted on the gryphon"):
		return
	await wait_frames(60)
	trace("mounted")
	await snapshot("mounted")

	# Space jumps; held in the air it launches.
	var ground := player.position.y
	push_key(KEY_SPACE, true)
	if not await wait_until(func(): return player.position.y > ground + 3.0, 10000, "launch"):
		return
	push_key(KEY_SPACE, false)
	var apex := ground
	var launch_deadline := Time.get_ticks_msec() + 4000
	while Time.get_ticks_msec() < launch_deadline:
		await sample_flight()
		if player.position.y < apex:
			break
		apex = player.position.y
	print("TRACE apex %.2f yards over the takeoff ground" % (apex - ground))
	trace("apex")
	await snapshot("apex")
	if apex - ground < 18.0:
		fail("launch reached only %.2f yards" % (apex - ground))
		return

	# Dive: right mouse held, camera looking down.
	client.set_camera_orbit(PI, -0.8, 12.0)
	push_mouse(MOUSE_BUTTON_RIGHT, true)
	var dive_start := player.position.y
	var dive_samples := 0
	while dive_samples < 10 or (height_over_ground() > 30.0 and dive_start - player.position.y < 150.0):
		await sample_flight()
		dive_samples += 1
	var dive_speed := top_speed
	print("TRACE dive top speed %.1f yd/s (steady flight on this aura %.1f)" % [dive_speed, STEADY_EQUIVALENT])
	trace("dived")
	await snapshot("dived")

	# Pull up, then glide level.
	client.set_camera_orbit(PI, 0.35, 12.0)
	var pull_speed_start := current_speed
	var pull_start_y := player.position.y
	for _sample in 15:
		await sample_flight()
	var pull_speed_end := current_speed
	print("TRACE pull-up speed %.1f -> %.1f yd/s, climbed %.1f yards" % [
		pull_speed_start, pull_speed_end, player.position.y - pull_start_y])
	trace("pulled-up")
	await snapshot("pulled-up")
	client.set_camera_orbit(PI, 0.0, 12.0)
	for _sample in 20:
		await sample_flight()
	trace("glided")
	await snapshot("glided")

	# Down onto the ground.
	client.set_camera_orbit(PI, -0.5, 12.0)
	var landing_deadline := Time.get_ticks_msec() + 30000
	while not on_ground() and Time.get_ticks_msec() < landing_deadline:
		await sample_flight()
	push_mouse(MOUSE_BUTTON_RIGHT, false)
	if not on_ground():
		fail("never landed")
		return
	if not await wait_until(func(): return server_close(), 10000, "server at the landing spot"):
		return
	trace("landed")
	await snapshot("landed")
	print("TRACE worst server gap off the client's path %.2f yards, worst lag %.3f s, top speed %.1f yd/s" % [
		worst_gap, worst_lag, top_speed])
	if worst_gap > 1.0:
		fail("the server left the client's path by %.2f yards" % worst_gap)
		return
	if dive_speed <= STEADY_EQUIVALENT:
		fail("dive never passed steady flight speed: %.1f" % dive_speed)
		return
	if pull_speed_end >= pull_speed_start:
		fail("pulling up kept its speed: %.1f -> %.1f" % [pull_speed_start, pull_speed_end])
		return
	print("SKYRIDING_LIVE PASS")
	quit(0)

var current_speed := 0.0
var last_usec := 0
## The client's path over the last 5 s: [usec, position] per frame.
var path := []
var worst_lag := 0.0

## One sample: the client's speed since the last sample, and how far the server's position
## (what other clients draw) is off the client's path (`gap`) and how long ago the client
## was there (`lag`).
func sample_flight() -> void:
	if last_usec == 0:
		last_usec = Time.get_ticks_usec()
		last_position = player.position
	var deadline := Time.get_ticks_msec() + int(SAMPLE_SECS * 1000.0)
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var frame_usec := Time.get_ticks_usec()
		path.append([frame_usec, player.position])
		while path.size() > 2 and frame_usec - path[0][0] > 5000000:
			path.pop_front()
	var now := Time.get_ticks_usec()
	current_speed = player.position.distance_to(last_position) / ((now - last_usec) / 1000000.0)
	last_usec = now
	last_position = player.position
	top_speed = maxf(top_speed, current_speed)
	var server = client.account_state().local_server_position
	if server == null:
		return
	var off := INF
	var lag := 0.0
	for i in range(1, path.size()):
		var a: Vector3 = path[i - 1][1]
		var b: Vector3 = path[i][1]
		var t := 0.0
		if a.distance_squared_to(b) > 0.0:
			t = clampf((server - a).dot(b - a) / a.distance_squared_to(b), 0.0, 1.0)
		var distance: float = (a.lerp(b, t)).distance_to(server)
		if distance < off:
			off = distance
			lag = (now - lerpf(path[i - 1][0], path[i][0], t)) / 1000000.0
	# Below 10 fps a frame reports more travel than the server applies per input
	# (game-server `MAX_INPUT_STEP_SECS`), and its catch-up runs straight to the newest
	# report, cutting the curve between sparse reports.
	if Engine.get_frames_per_second() >= 10.0:
		worst_gap = maxf(worst_gap, off)
		worst_lag = maxf(worst_lag, lag)
	print("TRACE sample client=%s server=%s gap=%.2f lag=%.3f speed=%.1f over_ground=%.1f fps=%s" % [
		player.position, server, off, lag, current_speed, height_over_ground(),
		Engine.get_frames_per_second()])

func height_over_ground() -> float:
	var height = client.terrain_height_at(player.position.x, player.position.z)
	return INF if height == null else player.position.y - float(height)

## The gryphon aura is on the player and its visual rides the mount's saddle.
func mounted() -> bool:
	var has_aura := false
	for buff in client.aura_state().buffs:
		if buff.spell_id == GOLDEN_GRYPHON:
			has_aura = true
	var saddle := player.find_child("Attachment0", true, false)
	return has_aura and saddle != null and saddle.get_child_count() > 0

func server_close() -> bool:
	var server = client.account_state().local_server_position
	return server != null and (server as Vector3).distance_to(player.position) < 0.5

## Landed, on terrain or a WMO: the server moves the player at its mounted run speed
## again (2 × 7) instead of its flight speed.
func on_ground() -> bool:
	var speed = client.account_state().local_server_speed
	return speed != null and float(speed) < STEADY_EQUIVALENT - 1.0 and current_speed < 0.5

func trace(label: String) -> void:
	var state: Dictionary = client.account_state()
	print("TRACE t=%d %s client=%s server=%s server_speed=%s over_ground=%.1f mounted=%s fps=%s" % [
		Time.get_ticks_msec(), label, player.position, state.local_server_position,
		state.local_server_speed, height_over_ground(), mounted(), Engine.get_frames_per_second()])

func snapshot(label: String) -> void:
	await RenderingServer.frame_post_draw
	var path := "%s/skyriding-%02d-%s.png" % [shots, shot, label]
	shot += 1
	root.get_texture().get_image().save_png(path)
	print("TRACE screenshot ", path)

func enter_world() -> bool:
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
	await click(ui.find_child("CharCard_0", true, false))
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != character:
		fail("Card 0 is %s, not %s" % [selected.text, character])
		return false
	await click(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and state.local_server_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD at ", state.local_player_position)
			await wait_frames(60)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func wait_until(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	trace("timeout")
	fail("Timed out waiting for %s" % what)
	return false

func wait_frames(count: int) -> void:
	for _frame in count:
		await process_frame

func wait_seconds(seconds: float) -> void:
	var deadline := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < deadline:
		await process_frame

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func push_mouse(button: MouseButton, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = Vector2(root.size) * 0.5
	event.button_index = button
	event.pressed = pressed
	root.push_input(event, true)

func click(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
