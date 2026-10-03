extends SceneTree

## Live steady flight on a private server with real key events: the character summons the
## Golden Gryphon (32235), rides it (its visual on the mount's MountMain attachment 0), holds
## Space to jump, take off and climb (`JumpOrAscendStart`), flies forward over the terrain,
## hangs in the air with no key held, holds X down to the ground (`SitStandOrDescendStart`)
## and lands, then uses the mount again to dismount. The server-replicated height (what
## other clients draw) must follow the flyer.
## Environment: GODOT_TEST_SERVER (private server), FLY_ACCOUNT / FLY_CHARACTER (password
## fbtest, card 0) knowing 32235 and Expert Riding 34090:
##   game-server-admin learn-spell <name> 34090
##   game-server-admin learn-spell <name> 32235
## The client needs at least 4 fps: the server applies at most 0.25 s of movement per input
## (game-server `MAX_INPUT_STEP_SECS`), so a slower client outruns it.
## FLY_SHOTS: screenshot directory. FLY_READY_FILE: when set, wait in the world until that
## file exists (a driver creates it after teaching the online character the spells).

const PASSWORD := "fbtest"
const GOLDEN_GRYPHON := 32235
## `FLIGHT_SPEED` × Mount Speed Mod: Standard Flying Mount 86459 (+220%).
const STANDARD_FLIGHT := 7.0 * 3.2

var client: Node
var player: Node3D
var character := ""
var shots := "/tmp/claude/flying-mount-live"
var shot := 0

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(960, 540)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("FLY_ACCOUNT")
	character = OS.get_environment("FLY_CHARACTER")
	if server == "" or server == "127.0.0.1:5000" or account == "" or character == "":
		fail("GODOT_TEST_SERVER (a private server), FLY_ACCOUNT and FLY_CHARACTER are required")
		return
	if OS.get_environment("FLY_SHOTS") != "":
		shots = OS.get_environment("FLY_SHOTS")
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
	var ready_file := OS.get_environment("FLY_READY_FILE")
	if ready_file != "" and not await wait_until(func(): return FileAccess.file_exists(ready_file), 60000, ready_file):
		return
	# Noon light and a close orbit, so the screenshots show the rider on the mount.
	client.set_world_minutes(720.0)
	client.set_camera_orbit(0.0, -0.3, 9.0)
	await wait_frames(30)
	trace("ground")
	await snapshot("ground")

	var sent: String = client.use_spell(GOLDEN_GRYPHON)
	if sent != "":
		fail("mount cast: " + sent)
		return
	if not await wait_until(func(): return mounted(), 30000, "mounted on the gryphon"):
		return
	await wait_frames(60)
	trace("mounted")
	await snapshot("mounted")

	var ground := player.position.y
	push_key(KEY_SPACE, true)
	var climb_start := Time.get_ticks_msec()
	for _sample in 6:
		await wait_seconds(0.25)
		trace("climbing")
	if not await wait_until(func(): return player.position.y > ground + 30.0, 60000, "climb 30 yards"):
		return
	var climb_secs := (Time.get_ticks_msec() - climb_start) / 1000.0
	push_key(KEY_SPACE, false)
	print("TRACE climbed 30 yards in %.2f wall s (flight speed %.1f yd/s)" % [climb_secs, STANDARD_FLIGHT])
	trace("climbed")
	await snapshot("climbed")

	push_key(KEY_W, true)
	await wait_seconds(3.0)
	push_key(KEY_W, false)
	trace("flew-forward")
	await snapshot("flew-forward")

	var hover := player.position.y
	await wait_seconds(2.0)
	if absf(player.position.y - hover) > 0.01:
		fail("fell while hovering: %s -> %s" % [hover, player.position.y])
		return
	if not await wait_until(func(): return server_close(), 30000, "server height follows the flyer"):
		return
	trace("hovering")
	await snapshot("hovering")

	push_key(KEY_X, true)
	if not await wait_until(func(): return on_ground(), 120000, "land"):
		return
	push_key(KEY_X, false)
	await wait_frames(30)
	trace("landed")
	await snapshot("landed")

	sent = client.use_spell(GOLDEN_GRYPHON)
	if sent != "":
		fail("dismount: " + sent)
		return
	if not await wait_until(func(): return not mounted(), 30000, "dismounted"):
		return
	await wait_frames(60)
	trace("dismounted")
	await snapshot("dismounted")
	print("FLYING_MOUNT_LIVE PASS")
	quit(0)

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
	return server != null and absf(server.y - player.position.y) < 0.5

func on_ground() -> bool:
	var height = client.terrain_height_at(player.position.x, player.position.z)
	return height != null and player.position.y - float(height) < 0.1

func trace(label: String) -> void:
	var state: Dictionary = client.account_state()
	var height = client.terrain_height_at(player.position.x, player.position.z)
	print("TRACE t=%d %s client=%s server=%s server_speed=%s terrain=%s mounted=%s fps=%s" % [
		Time.get_ticks_msec(), label, player.position, state.local_server_position,
		state.local_server_speed, height, mounted(), Engine.get_frames_per_second()])

func snapshot(label: String) -> void:
	await RenderingServer.frame_post_draw
	var path := "%s/flying-mount-%02d-%s.png" % [shots, shot, label]
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
