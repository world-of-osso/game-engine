extends SceneTree

## Live Flight Style toggle and Skyriding takeoff (part 4) on a private server: the character
## enters the world on Flight Style: Steady, so the Golden Gryphon (32235) flies steady (no
## Skyriding bar, no vigor). Switch Flight Style 436854 puts it on Skyriding: mounting again
## pages the main bar to the Skyriding bar with six Skyriding Charges. A takeoff from the
## ground (Space, Space held) casts Skyward Ascent 372610 through the server, which spends
## one charge, and its SpellGo launches the glider.
## Environment: GODOT_TEST_SERVER (private server), SKY_ACCOUNT / SKY_CHARACTER (password
## fbtest, card 0), SKY_SHOTS (screenshot directory). SKY_READY_FILE: wait in the world until
## that file exists (the driver creates it after, on the online character:
##   game-server-admin learn-spell <name> 34090    (Expert Riding)
##   game-server-admin learn-spell <name> 32235    (Golden Gryphon)
##   game-server-admin learn-spell <name> 376777   (Skyriding Basics)
##   game-server-admin learn-spell <name> 436854   (Switch Flight Style)
## ).

const PASSWORD := "fbtest"
const GOLDEN_GRYPHON := 32235
const SWITCH_FLIGHT_STYLE := 436854
const SKYRIDING_BAR := [372608, 372610, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]

var client: Node
var player: Node3D
var character := ""
var shots := "/tmp/claude/skyriding-style-live"
var shot := 0

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
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

	# Steady: the gryphon flies steady, no Skyriding bar, no vigor.
	if not await mount():
		return
	await wait_seconds(2.0)
	if Array(client.spells_state().bar) == SKYRIDING_BAR or Array(client.spells_state().vigor) != [0, 0]:
		fail("Skyriding on Flight Style: Steady")
		return
	trace("steady mounted")
	await snapshot("steady-mounted")
	if not await dismount():
		return

	var errors_before: int = client.spells_state().errors.size()
	var sent: String = client.use_spell(SWITCH_FLIGHT_STYLE)
	if sent != "":
		fail("Switch Flight Style: " + sent)
		return
	# A 5 s cast (SpellCastTimes 6).
	await wait_seconds(6.0)
	if client.spells_state().errors.size() > errors_before:
		fail("Switch Flight Style failed: " + str(client.spells_state().errors))
		return
	trace("switched")

	# Skyriding: the next mount pages the Skyriding bar with six charges.
	if not await mount():
		return
	if not await wait_until(func(): return Array(client.spells_state().bar) == SKYRIDING_BAR, 10000, "the Skyriding bar"):
		return
	if not await wait_until(func(): return Array(client.spells_state().vigor) == [6, 6], 5000, "six full charges"):
		return
	await wait_frames(30)
	trace("skyriding mounted")
	await snapshot("skyriding-mounted")

	# Takeoff: Space jumps, Space held in the air casts Skyward Ascent.
	var ground := player.position.y
	errors_before = client.spells_state().errors.size()
	push_key(KEY_SPACE, true)
	if not await wait_until(func(): return player.position.y > ground + 8.0, 10000, "the launch"):
		return
	push_key(KEY_SPACE, false)
	if not await wait_until(func(): return Array(client.spells_state().vigor) == [6, 5], 5000, "five charges after the takeoff"):
		return
	if client.spells_state().errors.size() > errors_before:
		fail("takeoff failed: " + str(client.spells_state().errors))
		return
	await wait_seconds(1.0)
	trace("takeoff rose %.1f" % (player.position.y - ground))
	await snapshot("takeoff")
	print("SKYRIDING_STYLE_LIVE PASS")
	quit(0)

func mount() -> bool:
	var sent: String = client.use_spell(GOLDEN_GRYPHON)
	if sent != "":
		fail("mount cast: " + sent)
		return false
	return await wait_until(func(): return mounted(), 30000, "mounted on the gryphon")

func dismount() -> bool:
	var sent: String = client.use_spell(GOLDEN_GRYPHON)
	if sent != "":
		fail("dismount: " + sent)
		return false
	return await wait_until(func(): return not mounted(), 10000, "dismounted")

func trace(label: String) -> void:
	print("TRACE t=%d %s mounted=%s y=%.2f bar=%s vigor=%s errors=%s" % [Time.get_ticks_msec(), label,
		mounted(), player.position.y, client.spells_state().bar, client.spells_state().vigor,
		client.spells_state().errors])

func snapshot(label: String) -> void:
	await RenderingServer.frame_post_draw
	var path := "%s/skyriding-style-%02d-%s.png" % [shots, shot, label]
	shot += 1
	root.get_texture().get_image().save_png(path)
	print("TRACE screenshot ", path)

## The gryphon aura is on the player and its visual rides the mount's saddle.
func mounted() -> bool:
	var has_aura := false
	for buff in client.aura_state().buffs:
		if buff.spell_id == GOLDEN_GRYPHON:
			has_aura = true
	var saddle := player.find_child("Attachment0", true, false)
	return has_aura and saddle != null and saddle.get_child_count() > 0

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
