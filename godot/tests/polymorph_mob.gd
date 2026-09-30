extends SceneTree

## Polymorph 118 on a Northshire Blackrock Spy (humanoid) by a level-10 mage
## (docs/wiki/systems/spell-visuals.md, Polymorph section). Environment:
##   GODOT_TEST_SERVER               server address (a private test server)
##   POLY_ACCOUNT / POLY_CHARACTER   account (password fbtest) and level-10 mage,
##                                   placed with the spy straight ahead
##   POLY_SHOTS                      screenshot directory
##   POLY_GRAB                       optional directory for a real-time recording: a JPEG
##                                   every 100 ms of wall-clock time from the idle framing on,
##                                   with `frames.txt` (ffmpeg concat durations). Movie Maker
##                                   (`--write-movie`) steps 1/30 s of game time per frame while
##                                   the server runs on wall-clock time, so its server events
##                                   come early whenever rendering is slower than 30 fps.
## Tab targets the spy. Frostbolt pulls it: it runs in and swings at the mage (the
## mage's CombatWound 9). Polymorph from the bar shows its precast and cast bar, then
## the server's TRANSFORM swaps the spy's display for the Polymorphed Sheep (856 or
## 857) in place: same unit node, still selected, still nameplated. The sheep stands
## (0) or walks (4) and never swings for SHEEP_SECS. A second Frostbolt breaks it: the
## spy's own display returns and it swings again.
## Also asserted: the neutral (yellow) spy's plate turns hostile red once the player is on
## its threat list (CompactUnitFrame.lua:674, :879); the pull's damage stays while it
## fights unpolymorphed (Creature::Update regenerates only `!IsEngaged() ||
## IsPolymorphed()`); Chilled from the Frostbolt halves its run speed (speed_run
## 0.857143 × 7 = 6 yd/s → 3 yd/s) while it chases, and once Chilled's 8 s are over the
## sheep wanders at its walk speed (speed_walk 1 × 2.5 yd/s, ConfusedMovementGenerator
## SetWalk). Speeds are measured over wall-clock time, so only in real-time runs.

const PASSWORD := "fbtest"
const POLYMORPH := 118
const FROSTBOLT := 116
const SHEEP_DISPLAYS := [856, 857]
const ANIM_STAND := 0
const ANIM_WALK := 4
## AttackUnarmed, Attack1H, Attack2H, Attack2HL, Attack1HPierce, Attack2HLoosePierce.
const MELEE_SWINGS := [16, 17, 18, 19, 199, 200]
const COMBAT_WOUND := 9
## Real seconds (the server's clock: Movie Maker frames are 1/30 s of game time each).
const SHEEP_SECS := 5.0
const ANIM_RUN := 5
## Run speed under Chilled 205708 (-50%).
const CHASE_SPEED := 3.0
const WALK_SPEED := 2.5
## Two creature regeneration intervals (CREATURE_REGEN_INTERVAL 2 s) and a tick.
const ENGAGED_REGEN_SECS := 4.5
const HOSTILE := Color(1, 0, 0, 1)
## Measured target movement: distance and time over frames showing `speed_anims`.
var speed_anims: Array = []
var speed_distance := 0.0
var speed_secs := 0.0
var speed_last = null
## Playback rates of the measured clip (`unit_display().animation_rate`).
var speed_rates := {}
var SPEED_TRACE := OS.get_environment("POLY_SPEED_TRACE") != ""

const BAR_KEYS := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]

var client: Node
var shots := "/tmp/claude/polymorph-mob/"
var character := ""
var local_id := 0
var target_id := 0
var seen_actions := {}
var seen_models := {}
## Locomotion clips seen on the target per display id.
var seen_animations := {}
var grab_dir := ""
var grab_last_ms := -1
var grab_index := 0
var grab_list := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func _process(delta: float) -> bool:
	if client == null or not is_instance_valid(client) or local_id == 0:
		return false
	measure_speed(delta)
	for id in [local_id, target_id]:
		if id == 0:
			continue
		var action: int = client.unit_action_id(id)
		if action >= 0:
			if not seen_actions.has(id):
				seen_actions[id] = {}
			seen_actions[id][action] = true
	if target_id != 0:
		var display: Dictionary = client.unit_display(target_id)
		if not display.is_empty() and display.animation >= 0:
			if not seen_animations.has(display.display_id):
				seen_animations[display.display_id] = {}
			seen_animations[display.display_id][display.animation] = true
	var visuals: Dictionary = client.spell_visuals_state()
	for model in visuals.active:
		seen_models[[model.unit, model.model]] = true
	grab_frame()
	return false

## One JPEG per 100 ms of wall-clock time; each listed with the time until the next.
func grab_frame() -> void:
	if grab_dir == "" or grab_last_ms < 0:
		return
	var now := Time.get_ticks_msec()
	if grab_index > 0 and now - grab_last_ms < 100:
		return
	if grab_index > 0:
		grab_list += "duration %.3f\n" % ((now - grab_last_ms) / 1000.0)
	var file := "grab-%05d.jpg" % grab_index
	root.get_texture().get_image().save_jpg(grab_dir + file, 0.9)
	grab_list += "file '%s'\n" % file
	grab_index += 1
	grab_last_ms = now

func finish_grab() -> void:
	if grab_dir == "" or grab_index == 0:
		return
	var list := FileAccess.open(grab_dir + "frames.txt", FileAccess.WRITE)
	list.store_string(grab_list + "duration 0.1\n")
	list.close()

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("POLY_ACCOUNT")
	character = OS.get_environment("POLY_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, POLY_ACCOUNT and POLY_CHARACTER are required")
		return
	if OS.get_environment("POLY_SHOTS") != "":
		shots = OS.get_environment("POLY_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not await wait_for(func(s): return s.catalog_ready and s.known.has(POLYMORPH) and s.known.has(FROSTBOLT) and s.level == 10, 60000, "level-10 mage with Polymorph"):
		return
	local_id = client.account_state().local_player_id
	# Nearby objects stream first; the far valley keeps streaming for many minutes, so
	# frame the scene after POLY_STREAM_SECS (default 90) even if some are pending.
	var stream_ms := int(OS.get_environment("POLY_STREAM_SECS")) * 1000 if OS.get_environment("POLY_STREAM_SECS") != "" else 90000
	var streaming := Time.get_ticks_msec()
	while client.account_state().world_objects.pending > 0 and Time.get_ticks_msec() - streaming < stream_ms:
		await wait_frames(60)
	print("FIXTURE STREAMING ", client.account_state().world_objects)
	await wait_frames(60)
	await frame_camera()
	if not await orbit_camera():
		return
	await capture("00-idle.png")
	grab_dir = OS.get_environment("POLY_GRAB")
	if grab_dir != "":
		DirAccess.make_dir_recursive_absolute(grab_dir)
		grab_last_ms = Time.get_ticks_msec()
	for attempt in range(10):
		push_key(KEY_TAB, true)
		await wait_frames(2)
		push_key(KEY_TAB, false)
		await wait_frames(6)
		if str(client.target_state().target_name).contains("Blackrock Spy") and absf(angle_to_target()) < 0.4:
			break
	if not await wait_until(func(): return str(client.target_state().target_name).contains("Blackrock Spy"), 3000, "Tab targets the Blackrock Spy"):
		return
	target_id = client.target_state().target
	var native: Dictionary = client.unit_display(target_id)
	print("FIXTURE TARGET ", client.target_state(), " display=", native, " angle=%.3f" % angle_to_target())
	if SHEEP_DISPLAYS.has(native.display_id):
		fail("The spy already shows a sheep: " + str(native))
		return
	var neutral := plate()
	print("FIXTURE NEUTRAL_PLATE ", neutral)
	if neutral.is_empty() or neutral.color.is_equal_approx(HOSTILE):
		fail("The unpulled spy's plate is not neutral: " + str(neutral))
		return
	# Pull it: the spy runs in and swings at the mage.
	start_speed([ANIM_RUN])
	if not await cast(FROSTBOLT, "pull Frostbolt"):
		return
	if not await wait_until(func(): return saw_swing(target_id) and saw(local_id, COMBAT_WOUND), 20000, "the spy swings at the mage"):
		return
	if not check_speed("chase", CHASE_SPEED, 2.6):
		return
	# Run (949470.skel sequence 5, movespeed 7) paced to 3 yd/s: 3/7.
	if not check_rates("chase Run", 0.35, 0.55):
		return
	if not await wait_until(func(): return plate_hostile(), 3000, "the pulled spy's plate turns hostile"):
		return
	await capture("01-spy-attacks.png")
	# Engaged and not polymorphed: the pull's damage stays.
	var wounded: float = plate().fraction
	await wait_real(ENGAGED_REGEN_SECS)
	print("FIXTURE ENGAGED_HEALTH %.4f -> %.4f" % [wounded, plate().fraction])
	if wounded >= 1.0 or plate().fraction > wounded:
		fail("The engaged spy healed: %.4f -> %.4f" % [wounded, plate().fraction])
		return
	# Polymorph: precast and cast bar, then the sheep.
	await wait_for(func(s): return s.gcd_ms == 0, 5000, "GCD over")
	seen_actions.erase(local_id)
	if not await press_spell(POLYMORPH):
		return
	await wait_frames(25)
	await capture("02-polymorph-casting.png")
	print("FIXTURE POLYMORPH_CASTING actions=", seen_actions.get(local_id, {}), " visuals=", client.spell_visuals_state())
	var before: Transform3D = client.unit_transform(target_id)
	if not await wait_until(func(): return SHEEP_SHOWN(), 5000, "the spy turns into a sheep"):
		return
	var poly_frame := Engine.get_frames_drawn()
	print("FIXTURE SHEEP frame=%d display=%s target=%s nameplates=%s" % [poly_frame, client.unit_display(target_id), client.target_state(), client.nameplate_state()])
	await wait_frames(6)
	await capture("03-sheep.png")
	# Swapped in place: the unit keeps its position (the sheep wanders at most 2 yd).
	var moved: float = (client.unit_transform(target_id).origin - before.origin).length()
	if moved > 2.5:
		fail("The sheep appeared %.2f yd away" % moved)
		return
	if client.target_state().target != target_id or client.target_state().circle_on != target_id:
		fail("The sheep lost its selection: " + str(client.target_state()))
		return
	# The sheep never swings: clear the spy's seen actions, watch SHEEP_SECS.
	seen_actions.erase(target_id)
	await wait_real(SHEEP_SECS)
	await capture("04-sheep-wanders.png")
	start_speed([ANIM_WALK])
	await wait_real(SHEEP_SECS)
	if not check_speed("sheep walk", WALK_SPEED, 1.75):
		return
	# Walk (1377131.m2 sequence 4, movespeed 1.111) paced to 2.5 yd/s: 2.25.
	if not check_rates("sheep Walk", 2.0, 2.5):
		return
	if not plate_hostile():
		fail("The sheep's plate is not hostile: " + str(plate()))
		return
	if saw_swing(target_id):
		fail("The sheep swung: " + str(seen_actions))
		return
	if not SHEEP_SHOWN():
		fail("Polymorph ended early: " + str(client.unit_display(target_id)))
		return
	var sheep_clips: Dictionary = seen_animations.get(client.unit_display(target_id).display_id, {})
	print("FIXTURE SHEEP_CLIPS ", sheep_clips, " actions=", seen_actions.get(target_id, {}))
	if not (sheep_clips.has(ANIM_STAND) or sheep_clips.has(ANIM_WALK)):
		fail("The sheep plays neither Stand nor Walk: " + str(sheep_clips))
		return
	await capture("05-sheep-late.png")
	# Break it with Frostbolt: the spy returns and fights.
	seen_actions.erase(target_id)
	if not await cast(FROSTBOLT, "breaking Frostbolt"):
		return
	if not await wait_until(func(): return client.unit_display(target_id).display_id == native.display_id and client.unit_display(target_id).visual, 8000, "the spy's own display returns"):
		return
	print("FIXTURE BROKEN frame=%d display=%s" % [Engine.get_frames_drawn(), client.unit_display(target_id)])
	await wait_frames(4)
	await capture("06-broken.png")
	if not await wait_until(func(): return saw_swing(target_id), 20000, "the spy swings again"):
		return
	await capture("07-spy-attacks-again.png")
	print("FIXTURE SEEN actions=", seen_actions, " animations=", seen_animations, " models=", seen_models.keys())
	await wait_real(2.0)
	finish_grab()
	print("FIXTURE POLYMORPH_MOB_DONE")
	client.free()
	quit(0)

## Ground distance the target covers per frame while it plays one of `speed_anims`, over
## wall-clock time (the client runs at a few frames per second in the headless cage).
func measure_speed(_delta: float) -> void:
	if speed_anims.is_empty() or target_id == 0:
		speed_last = null
		return
	var origin: Vector3 = client.unit_transform(target_id).origin
	var now := Time.get_ticks_usec()
	var display: Dictionary = client.unit_display(target_id)
	var animation: int = display.animation
	if speed_anims.has(animation) and display.has("animation_rate"):
		speed_rates[snappedf(display.animation_rate, 0.01)] = true
	if speed_last != null and speed_anims.has(animation):
		var step := Vector2(origin.x - speed_last[1].x, origin.z - speed_last[1].z).length()
		speed_distance += step
		speed_secs += (now - speed_last[0]) / 1000000.0
		if SPEED_TRACE:
			print("FIXTURE STEP t=%.3f anim=%d step=%.3f" % [now / 1000000.0, animation, step])
	speed_last = [now, origin]

func start_speed(anims: Array) -> void:
	speed_anims = anims
	speed_distance = 0.0
	speed_secs = 0.0
	speed_last = null
	speed_rates = {}

## The measured speed (yd/s), or a failure when it is above `expected` by over 10% or
## below `slowest` (starts and stops inside the window lower the average: the sheep's
## legs are at most 4 yd, and its clip changes a replication tick apart from its moves).
func check_speed(what: String, expected: float, slowest: float) -> bool:
	speed_anims = []
	var speed := speed_distance / speed_secs if speed_secs > 0.0 else 0.0
	print("FIXTURE SPEED %s %.2f yd/s over %.2f s (%.2f yd), expected %.2f, clip rates %s" % [what, speed, speed_secs, speed_distance, expected, speed_rates.keys()])
	if speed_secs < 0.5 or speed > expected * 1.1 or speed < slowest:
		fail("%s speed %.2f yd/s over %.2f s, expected %.2f (at least %.2f)" % [what, speed, speed_secs, expected, slowest])
		return false
	return true

## Every playback rate the measured clip showed lies in [low, high].
func check_rates(what: String, low: float, high: float) -> bool:
	if speed_rates.is_empty():
		fail("%s: no clip rate seen" % what)
		return false
	for rate in speed_rates:
		if rate < low or rate > high:
			fail("%s clip rate %.2f outside [%.2f, %.2f]: %s" % [what, rate, low, high, speed_rates.keys()])
			return false
	return true

func plate() -> Dictionary:
	return client.nameplate_state().get(target_id, {})

func plate_hostile() -> bool:
	var view := plate()
	return not view.is_empty() and view.color.is_equal_approx(HOSTILE)

func SHEEP_SHOWN() -> bool:
	var display: Dictionary = client.unit_display(target_id)
	return not display.is_empty() and SHEEP_DISPLAYS.has(display.display_id) and display.visual

func saw(id: int, action: int) -> bool:
	return seen_actions.has(id) and seen_actions[id].has(action)

func saw_swing(id: int) -> bool:
	for swing in MELEE_SWINGS:
		if saw(id, swing):
			return true
	return false

func angle_to_target() -> float:
	var me: Transform3D = client.unit_transform(local_id)
	var target = client.unit_transform(client.target_state().target) if client.target_state().target != null else null
	if target == null:
		return PI
	var forward := me.basis.x
	var to_target: Vector3 = target.origin - me.origin
	forward.y = 0
	to_target.y = 0
	return forward.signed_angle_to(to_target, Vector3.UP)

func press_spell(spell: int) -> bool:
	var slot: int = spells().bar.find(spell)
	if slot < 0:
		fail("Spell %d is not on the main bar: %s" % [spell, spells().bar])
		return false
	await press(BAR_KEYS[slot])
	return true

## Cast `spell` once the GCD is over and wait for the server to take it.
func cast(spell: int, what: String) -> bool:
	if not await wait_for(func(s): return s.gcd_ms == 0, 5000, "GCD before " + what):
		return false
	if not await press_spell(spell):
		return false
	return await wait_for(func(s): return s.gcd_ms > 0, 3000, what + " accepted")

## Close framing from the mage's front-left (as spellcast_anim.gd).
func frame_camera() -> void:
	var center := Vector2(640, 360)
	var goal := float(OS.get_environment("POLY_CAMERA_DISTANCE")) if OS.get_environment("POLY_CAMERA_DISTANCE") != "" else 7.0
	for step in range(40):
		var distance: float = client.account_state().camera_distance
		if absf(distance - goal) < 0.6:
			break
		var wheel := InputEventMouseButton.new()
		wheel.position = center
		wheel.global_position = center
		wheel.button_index = MOUSE_BUTTON_WHEEL_UP if distance > goal else MOUSE_BUTTON_WHEEL_DOWN
		wheel.factor = 1.0
		wheel.pressed = true
		root.push_input(wheel, true)
		await wait_frames(40)
	await wait_frames(30)

func orbit_camera() -> bool:
	var center := Vector2(640, 60)
	var offset := float(OS.get_environment("POLY_ORBIT")) if OS.get_environment("POLY_ORBIT") != "" else -1.2
	for attempt in range(60):
		var state: Dictionary = client.account_state()
		var behind: float = state.local_player_facing - PI
		var error := wrapf(state.camera_yaw - (behind + offset), -PI, PI)
		if absf(error) < 0.05:
			print("FIXTURE CAMERA yaw=%.3f facing=%.3f distance=%.2f" % [state.camera_yaw, state.local_player_facing, state.camera_distance])
			return true
		var pixels := clampf(error / 0.01, -60.0, 60.0)
		await drag(center, center + Vector2(pixels, 0))
	fail("Camera orbit did not converge: " + str(client.account_state()))
	return false

func drag(from: Vector2, to: Vector2) -> void:
	await move_mouse(from)
	var down := InputEventMouseButton.new()
	down.position = from
	down.global_position = from
	down.button_index = MOUSE_BUTTON_LEFT
	down.pressed = true
	root.push_input(down, true)
	await wait_frames(2)
	for step in range(1, 5):
		var point := from.lerp(to, float(step) / 4)
		var motion := InputEventMouseMotion.new()
		motion.position = point
		motion.global_position = point
		motion.relative = (to - from) / 4
		motion.button_mask = MOUSE_BUTTON_MASK_LEFT
		root.push_input(motion, true)
		await wait_frames(1)
	var up := InputEventMouseButton.new()
	up.position = to
	up.global_position = to
	up.button_index = MOUSE_BUTTON_LEFT
	up.pressed = false
	root.push_input(up, true)
	await wait_frames(2)

func spells() -> Dictionary:
	return client.spells_state()

func wait_for(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(spells()):
			return true
	fail("Timed out waiting for %s: %s" % [what, spells()])
	return false

func wait_until(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("Timed out waiting for %s: actions=%s animations=%s display=%s target=%s" % [what, seen_actions, seen_animations, client.unit_display(target_id) if target_id != 0 else {}, client.target_state()])
	return false

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 20000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1:
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
