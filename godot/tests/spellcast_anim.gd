extends SceneTree

## Combat animations and spell visuals at a Northshire training dummy
## (docs/wiki/systems/spell-visuals.md). Environment:
##   GODOT_TEST_SERVER           server address (a private test server)
##   SPELL_ACCOUNT / SPELL_CHARACTER   account (password fbtest) and level-10 warrior
##   SPELL_SHOTS                 screenshot directory
## Tab targets the nearest dummy; auto-attack must play the warrior's Attack1H swing
## (17, Worn Shortsword) and the dummy's CombatWound (9). Slam (key 1) must play the
## one-hand visual's CombatAbility1H01 (818) and put its impact model 1283017 on the
## dummy; Battle Shout (spellbook) must play BattleRoar (55) with its base model
## 1138011 and the buff model 6194303 on the warrior.

const PASSWORD := "fbtest"
const SLAM := 1464
const BATTLE_SHOUT := 6673
const ATTACK_1H := 17
const COMBAT_WOUND := 9
const COMBAT_ABILITY_1H := 818
const BATTLE_ROAR := 55
const SLAM_IMPACT := 1283017
const SHOUT_BASE := 1138011
const SHOUT_BUFF := 6194303
## `SpellPower` rage cost of Slam, in tenths.
const SLAM_RAGE := 200

var client: Node
var shots := "/tmp/claude/spellcast-anim/"
var character := ""
var local_id := 0
var target_id := 0
## Action clips seen per unit id, and kit models seen shown.
var seen_actions := {}
var seen_models := {}

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func _process(_delta: float) -> bool:
	if client == null or not is_instance_valid(client) or local_id == 0:
		return false
	for id in [local_id, target_id]:
		if id == 0:
			continue
		var action: int = client.unit_action_id(id)
		if action >= 0:
			if not seen_actions.has(id):
				seen_actions[id] = {}
			seen_actions[id][action] = true
	for model in client.spell_visuals_state().active:
		seen_models[[model.unit, model.model]] = true
	return false

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("SPELL_ACCOUNT")
	character = OS.get_environment("SPELL_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, SPELL_ACCOUNT and SPELL_CHARACTER are required")
		return
	if OS.get_environment("SPELL_SHOTS") != "":
		shots = OS.get_environment("SPELL_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not await wait_for(func(s): return s.catalog_ready and s.known.has(BATTLE_SHOUT), 60000, "level-10 warrior spells"):
		return
	local_id = client.account_state().local_player_id
	await frame_camera()
	# Orbit before targeting: a left-drag press on the world may select or clear a unit.
	if not await orbit_camera():
		return
	await capture("00-idle.png")
	push_key(KEY_TAB, true)
	await wait_frames(2)
	push_key(KEY_TAB, false)
	if not await wait_until(func(): return str(client.target_state().target_name).contains("Training Dummy"), 5000, "Tab targets the training dummy"):
		return
	target_id = client.target_state().target
	print("FIXTURE TARGET ", client.target_state())
	if not await face_target():
		return
	# Auto-attack: swings and the dummy's wound reaction.
	if not await wait_until(func(): return saw(local_id, ATTACK_1H) and saw(target_id, COMBAT_WOUND), 15000, "auto-attack Attack1H swing and CombatWound"):
		return
	await capture("01-auto-attack.png")
	await wait_frames(20)
	await capture("02-auto-attack-late.png")
	if not await wait_for(func(s): return s.power >= SLAM_RAGE, 30000, "rage for Slam"):
		return
	await press(KEY_1)
	if not await wait_until(func(): return saw(local_id, COMBAT_ABILITY_1H) and seen_models.has([target_id, SLAM_IMPACT]), 5000, "Slam CombatAbility1H01 and impact model on the dummy"):
		return
	await wait_frames(8)
	await capture("03-slam.png")
	await wait_frames(12)
	await capture("04-slam-late.png")
	print("FIXTURE SLAM ", client.spell_visuals_state())
	if not await wait_for(func(s): return s.gcd_ms == 0, 5000, "GCD over"):
		return
	await wait_frames(30)
	if not await cast_battle_shout():
		return
	print("FIXTURE SEEN actions=", seen_actions, " models=", seen_models.keys())
	await wait_frames(90)
	print("FIXTURE SPELLCAST_ANIM_DONE")
	client.free()
	quit(0)

## Signed angle about +Y from the warrior's facing (model +X) to the target.
func angle_to_target() -> float:
	var me: Transform3D = client.unit_transform(local_id)
	var target: Transform3D = client.unit_transform(target_id)
	var forward := me.basis.x
	var to_target := target.origin - me.origin
	forward.y = 0
	to_target.y = 0
	return forward.signed_angle_to(to_target, Vector3.UP)

## The fixture places the warrior facing the dummy (turning in place does not persist
## across server rotation updates yet); require it within 0.3 rad of straight ahead.
func face_target() -> bool:
	var angle := angle_to_target()
	print("FIXTURE FACING angle=%.3f" % angle)
	if absf(angle) > 0.3:
		fail("The warrior does not face the dummy: %.3f rad" % angle)
		return false
	return true

func saw(id: int, action: int) -> bool:
	return seen_actions.has(id) and seen_actions[id].has(action)

## Close third-person framing orbited to the warrior's front-left, so the swing, the
## cast and the dummy are all in view: zoom in with the wheel, then orbit with left-drag
## (which leaves the facing) until the camera sits SPELL_ORBIT radians from behind.
func frame_camera() -> void:
	var center := Vector2(640, 360)
	var goal := float(OS.get_environment("SPELL_CAMERA_DISTANCE")) if OS.get_environment("SPELL_CAMERA_DISTANCE") != "" else 5.0
	for step in range(40):
		if client.account_state().camera_distance <= goal:
			break
		var wheel := InputEventMouseButton.new()
		wheel.position = center
		wheel.global_position = center
		wheel.button_index = MOUSE_BUTTON_WHEEL_UP
		wheel.factor = 1.0
		wheel.pressed = true
		root.push_input(wheel, true)
		# Let the follow distance settle before the next step.
		await wait_frames(40)
	await wait_frames(30)

func orbit_camera() -> bool:
	var center := Vector2(640, 360)
	var offset := float(OS.get_environment("SPELL_ORBIT")) if OS.get_environment("SPELL_ORBIT") != "" else 1.7
	for attempt in range(60):
		var state: Dictionary = client.account_state()
		var behind: float = state.local_player_facing - PI
		var error := wrapf(state.camera_yaw - (behind + offset), -PI, PI)
		if absf(error) < 0.05:
			print("FIXTURE CAMERA yaw=%.3f facing=%.3f distance=%.2f pitch=%.3f" % [state.camera_yaw, state.local_player_facing, state.camera_distance, state.camera_pitch])
			return true
		# Left-drag turns the camera yaw by -0.01 rad per pixel (look_sensitivity).
		var pixels := clampf(error / 0.01, -60.0, 60.0)
		await drag(center, center + Vector2(pixels, 0), MOUSE_BUTTON_LEFT)
	fail("Camera orbit did not converge: " + str(client.account_state()))
	return false

func drag(from: Vector2, to: Vector2, button: MouseButton) -> void:
	await move_mouse(from)
	var down := InputEventMouseButton.new()
	down.position = from
	down.global_position = from
	down.button_index = button
	down.pressed = true
	root.push_input(down, true)
	await wait_frames(2)
	var steps := 4
	for step in range(1, steps + 1):
		var point := from.lerp(to, float(step) / steps)
		var motion := InputEventMouseMotion.new()
		motion.position = point
		motion.global_position = point
		motion.relative = (to - from) / steps
		motion.button_mask = MOUSE_BUTTON_MASK_LEFT if button == MOUSE_BUTTON_LEFT else MOUSE_BUTTON_MASK_RIGHT
		root.push_input(motion, true)
		await wait_frames(1)
	var up := InputEventMouseButton.new()
	up.position = to
	up.global_position = to
	up.button_index = button
	up.pressed = false
	root.push_input(up, true)
	await wait_frames(2)

## Battle Shout from the spellbook: a click on its icon casts it.
func cast_battle_shout() -> bool:
	await press(KEY_P)
	if not await wait_for(func(s): return s.spellbook_open, 3000, "spellbook opened"):
		return false
	await wait_frames(10)
	var icon := client.get_node("SpellBookUI").find_child("SpellBookItem%dButton" % BATTLE_SHOUT, true, false) as Control
	if icon == null:
		fail("No Battle Shout button")
		return false
	await click(icon)
	await press(KEY_ESCAPE)
	if not await wait_until(func(): return saw(local_id, BATTLE_ROAR) and seen_models.has([local_id, SHOUT_BASE]) and seen_models.has([local_id, SHOUT_BUFF]), 5000, "Battle Shout BattleRoar, base and buff models"):
		return false
	await wait_frames(6)
	await capture("05-battle-shout.png")
	await wait_frames(20)
	await capture("06-battle-shout-late.png")
	print("FIXTURE SHOUT ", client.spell_visuals_state())
	return true

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
	fail("Timed out waiting for %s: actions=%s models=%s visuals=%s" % [what, seen_actions, seen_models.keys(), client.spell_visuals_state()])
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
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(shots + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
