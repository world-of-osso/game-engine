extends SceneTree

## Combat animations and spell visuals at a Northshire training dummy
## (docs/wiki/systems/spell-visuals.md). Environment:
##   GODOT_TEST_SERVER           server address (a private test server)
##   SPELL_ACCOUNT / SPELL_CHARACTER   account (password fbtest) and level-10 warrior
##   SPELL_SHOTS                 screenshot directory
##   SPELL_WORLD_TIMEOUT_S       seconds to wait for world objects (default 300)
##   SPELL_SCENARIO=paladin      a level-10 paladin heals itself with Flash of Light, then
##                               casts Judgment and Hammer of Justice on the dummy; each
##                               spell's SoundKits (precast, cast, missile, impact, aura)
##   SPELL_SCENARIO=shout        a level-10 warrior casts Battle Shout: its kit SoundKit
##                               and the warrior's own battle-shout voice
##   SPELL_SCENARIO=mage         a level-10 mage casts Frostbolt instead: the precast
##                               ReadySpellDirected loop (51) with hand models 1598571,
##                               the SpellCastDirected release (53), the missile 1598570
##                               leaving at the clip's $CSL event (200 ms) and flying
##                               distance / 35 yd/s, and the impact model 1599028 with
##                               the dummy's wound (9)
## Tab targets the nearest dummy. Selecting it never attacks (TrinityCore
## HandleSetSelectionOpcode only sets the selection): for SELECTION_SECS neither unit
## plays a melee clip. The warrior right-clicks the dummy in melee range (`AttackSwing`;
## the pick raycasts drawn triangles, so the warrior's own box cannot swallow it, and
## its own body still picks it), Escape clears the target and stops the swings
## (`AttackStop`), a left-click selects the dummy again without attacking, and the
## Attack action (88163, SPELL_ATTR1_INITIATES_COMBAT_ENABLES_AUTO_ATTACK) restarts
## auto-attack. Auto-attack must play the warrior's Attack1H swing (17, Worn Shortsword) and the dummy's
## CombatWound (9). The mage must show no swing (16-19) and the dummy no wound or
## crit reaction until its Frostbolt lands. Slam (key 1) must play the
## one-hand visual's CombatAbility1H01 (818) and put its impact model 1283017 on the
## dummy; Battle Shout (its bar key) must play BattleRoar (55) with its base model
## 1138011 and the buff model 6194303 on the warrior.

const PASSWORD := "fbtest"
const SLAM := 1464
const ATTACK := 88163
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
const FROSTBOLT := 116
const JUDGMENT := 20271
const HAMMER_OF_JUSTICE := 853
const FROST_NOVA := 122
const FLASH_OF_LIGHT := 19750
## Retail SoundKits each spell's kits play (SpellVisualKitEffect type 5, missile
## SoundEntriesID, 12.1.0.69933): Slam 1H cast 57845 and impact 60935; Battle Shout cast
## 114049; Frost Nova cast 350096, aura 350097 (looping) and 350098, aura end 85938;
## Flash of Light precast 349350 (looping), cast 349352 and 349351, impact 349357 and
## 349355; Judgment cast 218258 and 349488, missile 53649 (looping), impact 218257, 221597
## and 224414; Hammer of Justice cast 53854, impact 221582, stun aura 349372 (looping).
const SLAM_SOUNDS := [57845, 60935]
const BATTLE_SHOUT_SOUNDS := [114049]
const JUDGMENT_SOUNDS := [218258, 349488, 53649, 218257, 221597, 224414]
const HAMMER_OF_JUSTICE_SOUNDS := [53854, 221582, 349372]
const FROST_NOVA_SOUNDS := [350096, 350097, 350098]
const FROST_NOVA_END_SOUNDS := [85938]
const FLASH_OF_LIGHT_SOUNDS := [349350, 349352, 349351, 349357, 349355]
## Human male/female CreatureSoundData 49/50 battle shout (kit unit sound 38).
const BATTLE_SHOUT_VOICES := [58088, 58100]
const READY_SPELL_DIRECTED := 51
const SPELL_CAST_DIRECTED := 53
const FROSTBOLT_HANDS := 1598571
const FROSTBOLT_IMPACT := 1599028
## AttackUnarmed, Attack1H, Attack2H, Attack2HL.
const MELEE_SWINGS := [16, 17, 18, 19]
const COMBAT_CRITICAL := 10
## Two unarmed swing intervals: a selection-started auto-attack would swing in this time.
const SELECTION_SECS := 4.5

const BAR_KEYS := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]

var client: Node
var shots := "/tmp/claude/spellcast-anim/"
var character := ""
var local_id := 0
var target_id := 0
## Action clips seen per unit id, and kit models seen shown.
var seen_actions := {}
var seen_models := {}
var seen_missile := false
## Longest frame since the Frostbolt press: timings land on frame boundaries, so each
## may trail its event by up to two frames (the crossing frame and update order).
var longest_frame := 0.0

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func _process(delta: float) -> bool:
	longest_frame = maxf(longest_frame, delta)
	if client == null or not is_instance_valid(client) or local_id == 0:
		return false
	var visuals: Dictionary = client.spell_visuals_state()
	for id in [local_id, target_id]:
		if id == 0:
			continue
		var action: int = client.unit_action_id(id)
		if action >= 0:
			if not seen_actions.has(id):
				seen_actions[id] = {}
			# First seen, on the effects clock.
			if not seen_actions[id].has(action):
				seen_actions[id][action] = visuals.clock
	for model in visuals.active:
		seen_models[[model.unit, model.model]] = true
	if visuals.missiles > 0:
		seen_missile = true
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
	var scenario := OS.get_environment("SPELL_SCENARIO")
	var mage := scenario == "mage"
	var paladin := scenario == "paladin"
	# The warrior's shout scenario needs no dummy.
	var self_cast := scenario == "shout"
	var signature := FROSTBOLT if mage else (FLASH_OF_LIGHT if paladin else BATTLE_SHOUT)
	if not await wait_for(func(s): return s.catalog_ready and s.known.has(signature) and s.level == 10, 60000, "level-10 spells"):
		return
	local_id = client.account_state().local_player_id
	print("FIXTURE SPELLS known=", spells().known, " bar=", spells().bar)
	# Frame the scene once its doodads have streamed in.
	# A busy machine spawns objects slower (their per-frame time budget); SPELL_WORLD_TIMEOUT_S
	# extends the wait.
	var world_timeout := int(OS.get_environment("SPELL_WORLD_TIMEOUT_S")) if OS.get_environment("SPELL_WORLD_TIMEOUT_S") != "" else 300
	if not await wait_until(func(): return client.account_state().world_objects.pending == 0, world_timeout * 1000, "world objects spawned"):
		print("FIXTURE WORLD_OBJECTS ", client.account_state().world_objects)
		return
	await wait_frames(60)
	await frame_camera()
	# Orbit before targeting: a left-drag press on the world may select or clear a unit.
	if not await orbit_camera():
		return
	await capture("00-idle.png")
	# Flash of Light before any target: it heals the paladin itself.
	if paladin and not (await cast_and_hear(FLASH_OF_LIGHT, FLASH_OF_LIGHT_SOUNDS, 8000) and await loops_stopped(FLASH_OF_LIGHT, 3000)):
		return
	if self_cast:
		if await cast_self_sounds():
			print("FIXTURE SPELLCAST_ANIM_DONE")
			client.free()
			quit(0)
		return
	# Tab cycles nearest-first; take the dummy straight ahead (critters and other
	# dummies may be nearer).
	for attempt in range(8):
		push_key(KEY_TAB, true)
		await wait_frames(2)
		push_key(KEY_TAB, false)
		await wait_frames(6)
		if str(client.target_state().target_name).contains("Training Dummy"):
			target_id = client.target_state().target
			if absf(angle_to_target()) < 0.3:
				break
	if not await wait_until(func(): return str(client.target_state().target_name).contains("Training Dummy"), 3000, "Tab targets a training dummy"):
		print("FIXTURE TAB_FAILED units=", client.account_state().unit_count, " target=", client.target_state())
		return
	target_id = client.target_state().target
	print("FIXTURE TARGET ", client.target_state())
	if not await face_target():
		return
	# The selection alone: no swing, no wound, no auto-attack.
	await wait_frames(int(SELECTION_SECS * 60))
	if not no_melee_yet("after selecting the dummy"):
		return
	await capture("00-selected.png")
	if paladin:
		if await cast_paladin_sounds():
			print("FIXTURE SPELLCAST_ANIM_DONE")
			client.free()
			quit(0)
		return
	if mage:
		if await cast_frostbolt() and await cast_mage_sounds():
			print("FIXTURE SEEN actions=", seen_actions, " models=", seen_models.keys(), " missile=", seen_missile)
			await wait_frames(90)
			print("FIXTURE SPELLCAST_ANIM_DONE")
			client.free()
			quit(0)
		return
	# Right-click the dummy: auto-attack, its swings and the dummy's wound reaction.
	if not await right_click_unit(target_id):
		return
	if not await wait_until(func(): return client.target_state().auto_attack == target_id, 2000, "right-click auto-attack request"):
		return
	if not await wait_until(func(): return saw(local_id, ATTACK_1H) and saw(target_id, COMBAT_WOUND), 15000, "auto-attack Attack1H swing and CombatWound"):
		return
	await capture("01-auto-attack.png")
	# A few more swings on camera before the first ability.
	await wait_frames(int(OS.get_environment("SPELL_SWING_FRAMES")) if OS.get_environment("SPELL_SWING_FRAMES") != "" else 20)
	await capture("02-auto-attack-late.png")
	if not await stop_and_restart_with_attack_action():
		return
	if not await wait_for(func(s): return s.power >= SLAM_RAGE, 30000, "rage for Slam"):
		return
	await press(KEY_1)
	if not await wait_until(func(): return saw(local_id, COMBAT_ABILITY_1H) and seen_models.has([target_id, SLAM_IMPACT]), 5000, "Slam CombatAbility1H01 and impact model on the dummy"):
		return
	await wait_frames(8)
	await capture("03-slam.png")
	await wait_frames(12)
	await capture("04-slam-late.png")
	await wait_frames(30)
	print("FIXTURE SLAM ", client.spell_visuals_state())
	if not await wait_for(func(s): return s.gcd_ms == 0, 5000, "GCD over"):
		return
	await wait_frames(30)
	if not await cast_battle_shout():
		return
	if not await heard(SLAM, SLAM_SOUNDS, 3000) or not await heard(BATTLE_SHOUT, BATTLE_SHOUT_SOUNDS, 3000):
		return
	if not await heard_voice(BATTLE_SHOUT, BATTLE_SHOUT_VOICES, 3000):
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

## No melee swing by the player, no wound or crit reaction on the dummy, and no
## auto-attack request so far.
func no_melee_yet(when: String) -> bool:
	for swing in MELEE_SWINGS:
		if saw(local_id, swing):
			fail("Melee swing %d %s: %s" % [swing, when, seen_actions])
			return false
	for reaction in [COMBAT_WOUND, COMBAT_CRITICAL]:
		if saw(target_id, reaction):
			fail("Dummy reaction %d %s: %s" % [reaction, when, seen_actions])
			return false
	if client.target_state().auto_attack != null:
		fail("Auto-attack %s: %s" % [when, client.target_state()])
		return false
	print("FIXTURE NO_MELEE ", when, " actions=", seen_actions)
	return true

## Right-click the unit's body in place. The pick must choose it, and the player's own
## body must still pick the player.
func right_click_unit(id: int) -> bool:
	var camera := root.get_viewport().get_camera_3d()
	var point := camera.unproject_position(client.unit_transform(id).origin + Vector3.UP)
	var own := camera.unproject_position(client.unit_transform(local_id).origin + Vector3.UP)
	var screen := Rect2(Vector2.ZERO, Vector2(root.size))
	if not screen.has_point(point) or not screen.has_point(own):
		fail("Off screen: unit at %s, player at %s" % [point, own])
		return false
	if UnitPicker.pick(camera, own) != local_id:
		fail("The player's own body at %s picks %s" % [own, UnitPicker.pick(camera, own)])
		return false
	var picked = UnitPicker.pick(camera, point)
	print("FIXTURE PICK own=", own, " unit=", point, " picked=", picked, " ", client.target_state())
	if picked != id:
		fail("The unit at %s picks %s, not %d" % [point, picked, id])
		return false
	await move_mouse(point)
	await mouse_click(point, MOUSE_BUTTON_RIGHT)
	print("FIXTURE RIGHT_CLICK ", client.target_state())
	return true

func mouse_click(point: Vector2, button: MouseButton) -> void:
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await wait_frames(2)

## Escape clears the target and stops auto-attack; a left-click reselects the dummy
## without attacking; the Attack action restarts auto-attack.
func stop_and_restart_with_attack_action() -> bool:
	await press(KEY_ESCAPE)
	if not await wait_until(func(): return client.target_state().target == null and client.target_state().auto_attack == null, 2000, "Escape clears the target and stops auto-attack"):
		return false
	# Let the last swing clip finish, then watch two swing intervals.
	await wait_frames(60)
	seen_actions.erase(local_id)
	await wait_frames(int(SELECTION_SECS * 60))
	if saw(local_id, ATTACK_1H):
		fail("Swings after the stop: " + str(seen_actions))
		return false
	await capture("02b-stopped.png")
	var camera := root.get_viewport().get_camera_3d()
	var point := camera.unproject_position(client.unit_transform(target_id).origin + Vector3.UP)
	await move_mouse(point)
	await mouse_click(point, MOUSE_BUTTON_LEFT)
	if not await wait_until(func(): return client.target_state().target == target_id, 2000, "left-click reselects the dummy"):
		return false
	await wait_frames(60)
	if client.target_state().auto_attack != null or saw(local_id, ATTACK_1H):
		fail("Left-click selection attacked: " + str(client.target_state()))
		return false
	var attack_slot: int = spells().bar.find(ATTACK)
	if attack_slot < 0:
		fail("Attack is not on the main bar: " + str(spells().bar))
		return false
	await press(BAR_KEYS[attack_slot])
	if not await wait_until(func(): return client.target_state().auto_attack == target_id and saw(local_id, ATTACK_1H), 5000, "Attack action restarts auto-attack"):
		return false
	print("FIXTURE STOP_AND_ATTACK_ACTION ", client.target_state())
	await capture("02c-attack-action.png")
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
		# Let the follow distance settle before the next step.
		await wait_frames(40)
	await wait_frames(30)

func orbit_camera() -> bool:
	# Drag from the sky strip: a press over a unit would select it.
	var center := Vector2(640, 60)
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

## Frostbolt from its action button: the precast loop and hand models hold while the
## cast bar fills, then the release, the missile and the impact on the dummy.
func cast_frostbolt() -> bool:
	var slot: int = spells().bar.find(FROSTBOLT)
	if slot < 0:
		fail("Frostbolt is not on the main bar: " + str(spells().bar))
		return false
	longest_frame = 0.0
	print("FIXTURE FROSTBOLT_PRESS clock=%.3f wall_ms=%d" % [client.spell_visuals_state().clock, int(Time.get_unix_time_from_system() * 1000.0)])
	await press(BAR_KEYS[slot])
	if not await wait_until(func(): return saw(local_id, READY_SPELL_DIRECTED) and seen_models.has([local_id, FROSTBOLT_HANDS]), 3000, "Frostbolt precast loop and hand models"):
		return false
	await wait_frames(20)
	await capture("10-frostbolt-precast.png")
	if not await wait_until(func(): return saw(local_id, SPELL_CAST_DIRECTED) and seen_missile, 5000, "Frostbolt release and missile"):
		return false
	# Frostbolt starts no auto-attack (no SPELL_ATTR1/ATTR2 auto-attack attribute).
	await capture("11-frostbolt-missile.png")
	if not await wait_until(func(): return seen_models.has([target_id, FROSTBOLT_IMPACT]) and saw(target_id, COMBAT_WOUND), 5000, "Frostbolt impact on the dummy"):
		return false
	await wait_frames(4)
	await capture("12-frostbolt-impact.png")
	var visuals: Dictionary = client.spell_visuals_state()
	print("FIXTURE FROSTBOLT ", visuals)
	print_cast_timeline(visuals)
	if not no_reaction_before_impact(visuals.flights):
		return false
	return check_frostbolt_flight(visuals.flights) and check_frostbolt_sounds(visuals)

## The mage never swings nor auto-attacks, and the dummy's first wound or crit reaction comes with the
## Frostbolt landing, not before (timed on the effects clock, so a slow frame rate does
## not reorder them).
func no_reaction_before_impact(flights: Array) -> bool:
	for swing in MELEE_SWINGS:
		if saw(local_id, swing):
			fail("Melee swing %d by the mage: %s" % [swing, seen_actions])
			return false
	var flight: Dictionary = flights.filter(func(f): return f.spell == FROSTBOLT and f.caster == local_id)[-1]
	var landed: float = flight.released_at + flight.flight_time
	for reaction in [COMBAT_WOUND, COMBAT_CRITICAL]:
		if saw(target_id, reaction) and seen_actions[target_id][reaction] < landed - 0.001:
			fail("Dummy reaction %d at %.3f, before the Frostbolt landed at %.3f" % [reaction, seen_actions[target_id][reaction], landed])
			return false
	if client.target_state().auto_attack != null:
		fail("Frostbolt started auto-attack: %s" % client.target_state())
		return false
	print("FIXTURE NO_MELEE before the Frostbolt impact actions=", seen_actions, " landed=%.3f" % landed)
	return true

## Without a target: Battle Shout buffs the warrior (its cast kit's SoundKit and the
## warrior's own shout voice).
func cast_self_sounds() -> bool:
	return await cast_and_hear(BATTLE_SHOUT, BATTLE_SHOUT_SOUNDS, 5000) and await heard_voice(BATTLE_SHOUT, BATTLE_SHOUT_VOICES, 3000)

## On the dummy: Judgment (cast, looping missile sound that stops on landing, impact) and
## Hammer of Justice (cast, impact, the stun aura's looping sound until the stun ends).
func cast_paladin_sounds() -> bool:
	if not await cast_and_hear(JUDGMENT, JUDGMENT_SOUNDS, 8000) or not await loops_stopped(JUDGMENT, 3000):
		return false
	if not await wait_for(func(s): return s.gcd_ms == 0 and s.casting == 0, 8000, "GCD over after Judgment"):
		return false
	await wait_frames(30)
	return await cast_and_hear(HAMMER_OF_JUSTICE, HAMMER_OF_JUSTICE_SOUNDS, 5000) and await loops_stopped(HAMMER_OF_JUSTICE, 15000)

## After Frostbolt: Frost Nova (cast, aura sounds on the rooted dummy, aura-end sound).
func cast_mage_sounds() -> bool:
	if not await wait_for(func(s): return s.gcd_ms == 0 and s.casting == 0, 8000, "GCD over after Frostbolt"):
		return false
	await wait_frames(30)
	if not await cast_and_hear(FROST_NOVA, FROST_NOVA_SOUNDS, 5000):
		return false
	# The root breaks after its duration (or damage): the aura's loop stops and its end
	# kit sounds.
	if not await heard(FROST_NOVA, FROST_NOVA_END_SOUNDS, 15000):
		return false
	return await loops_stopped(FROST_NOVA, 3000)

## Press `spell`'s bar key, then require its kits' SoundKits.
func cast_and_hear(spell: int, sound_kits: Array, timeout_ms: int) -> bool:
	var slot: int = spells().bar.find(spell)
	if slot < 0:
		fail("Spell %d is not on the main bar: %s" % [spell, spells().bar])
		return false
	await press(BAR_KEYS[slot])
	return await heard(spell, sound_kits, timeout_ms)

## Every one of `sound_kits` played for `spell` (kit or missile sounds), each logged with
## its file, unit, source and times.
func heard(spell: int, sound_kits: Array, timeout_ms: int) -> bool:
	var played := func() -> Array:
		return client.spell_visuals_state().sounds.filter(func(s): return s.spell == spell)
	if not await wait_until(func(): return sound_kits.all(func(kit): return played.call().any(func(s): return s.sound_kit == kit)), timeout_ms, "spell %d SoundKits %s" % [spell, sound_kits]):
		print("FIXTURE SOUNDS_AT_FAILURE ", played.call())
		return false
	for sound in played.call():
		print_sound(sound)
	return true

## `spell`'s kits played the caster's own voice, one of `sound_kits`.
func heard_voice(spell: int, sound_kits: Array, timeout_ms: int) -> bool:
	var voiced := func() -> Array:
		return client.spell_visuals_state().sounds.filter(func(s): return s.spell == spell and s.source == "voice" and s.unit == local_id and sound_kits.has(s.sound_kit))
	if not await wait_until(func(): return not voiced.call().is_empty(), timeout_ms, "spell %d caster voice %s" % [spell, sound_kits]):
		return false
	for sound in voiced.call():
		print_sound(sound)
	return true

## `spell`'s looping sounds (precast, missile, aura) all stopped with their kit.
func loops_stopped(spell: int, timeout_ms: int) -> bool:
	var running := func() -> Array:
		return client.spell_visuals_state().sounds.filter(func(s): return s.spell == spell and s.looping and s.stopped_at < 0.0)
	if not await wait_until(func(): return running.call().is_empty(), timeout_ms, "spell %d looping sounds to stop" % spell):
		print("FIXTURE LOOPS_RUNNING ", running.call())
		return false
	for sound in client.spell_visuals_state().sounds.filter(func(s): return s.spell == spell and s.looping):
		print("FIXTURE LOOP_STOPPED spell=%d sound_kit=%d fdid=%d source=%s at=%.3f stopped_at=%.3f" % [sound.spell, sound.sound_kit, sound.fdid, sound.source, sound.at, sound.stopped_at])
	return true

func print_sound(sound: Dictionary) -> void:
	print("FIXTURE SPELL_SOUND spell=%d kit=%d sound_kit=%d fdid=%d unit=%d source=%s looping=%s at=%.3f stopped_at=%.3f" % [sound.spell, sound.kit, sound.sound_kit, sound.fdid, sound.unit, sound.source, sound.looping, sound.at, sound.stopped_at])

## When the client saw the Frostbolt cast start (first replicated CastState, with the
## server's elapsed time) and resolve (SpellGo), on the effects clock and wall clock.
func print_cast_timeline(visuals: Dictionary) -> void:
	for seen in visuals.casts:
		if seen.spell == FROSTBOLT:
			print("FIXTURE FROSTBOLT_CAST %s at=%.3f wall_ms=%d elapsed=%.3f duration=%.3f" % ["SpellGo" if seen.go else "CastState", seen.at, seen.wall_ms, seen.elapsed, seen.duration])

## Frostbolt's SoundKits (SpellVisualKitEffect type 5): the precast kit 81575 starts
## 85501 (precast_start) and loops 85500 (precast_loop) when the cast replicates, the cast kit
## 81337 plays 85502 (cast) at SpellGo and the impact kit 80718 plays 85503 (impact)
## as the missile lands; the loop stops at SpellGo (PrecastEnd).
func check_frostbolt_sounds(visuals: Dictionary) -> bool:
	var frame_s := 2.0 * longest_frame
	var flight: Dictionary = visuals.flights.filter(func(f): return f.spell == FROSTBOLT and f.caster == local_id)[-1]
	var go: float = flight.released_at - flight.release_delay
	var landed: float = flight.released_at + flight.flight_time
	var expected := {
		85501: [1631391, 1631394, local_id, false],
		85500: [1631387, 1631390, local_id, true],
		85502: [1631379, 1631382, local_id, false],
		85503: [1631383, 1631386, target_id, false],
	}
	var at := {}
	for sound in visuals.sounds:
		if sound.spell != FROSTBOLT:
			continue
		print("FIXTURE FROSTBOLT_SOUND kit=%d sound_kit=%d fdid=%d unit=%d looping=%s at=%.3f (SpellGo %+.3f) stopped_at=%.3f" % [sound.kit, sound.sound_kit, sound.fdid, sound.unit, sound.looping, sound.at, sound.at - go, sound.stopped_at])
		var want: Array = expected.get(sound.sound_kit, [])
		if want.is_empty() or sound.fdid < want[0] or sound.fdid > want[1] or sound.unit != want[2] or sound.looping != want[3]:
			fail("Unexpected Frostbolt sound: " + str(sound))
			return false
		at[sound.sound_kit] = sound.at
		if sound.looping:
			at["loop_stop"] = sound.stopped_at
	if at.size() != expected.size() + 1:
		fail("Frostbolt sound kits played: %s, expected %s" % [at.keys(), expected.keys()])
		return false
	var cast_seen: Array = visuals.casts.filter(func(c): return c.spell == FROSTBOLT and c.unit == local_id and not c.go)
	if cast_seen.is_empty():
		fail("The client never saw Frostbolt's CastState: " + str(visuals.casts))
		return false
	var precast: float = cast_seen[-1].at
	if absf(at[85500] - precast) > 0.001 or absf(at[85501] - precast) > 0.001:
		fail("Precast sounds %.3f/%.3f did not start with the replicated cast at %.3f" % [at[85500], at[85501], precast])
		return false
	if at.loop_stop < 0.0 or absf(at.loop_stop - go) > frame_s:
		fail("Precast loop stopped at %.3f, not with the cast at SpellGo %.3f" % [at.loop_stop, go])
		return false
	if absf(at[85502] - go) > frame_s:
		fail("Cast sound %.3f is not at SpellGo %.3f" % [at[85502], go])
		return false
	if absf(at[85503] - landed) > frame_s:
		fail("Impact sound %.3f is not at the landing %.3f" % [at[85503], landed])
		return false
	var loops := client.find_children("SpellSound*", "", true, false).filter(func(node): return int(str(node.name).trim_prefix("SpellSound")) in range(1631387, 1631391))
	if not loops.is_empty():
		fail("Precast loop still playing after the cast: " + str(loops))
		return false
	return true

## The missile leaves at SpellCastDirected's `$CSL` release event (200 ms into the clip
## the SpellGo starts) and flies distance / 35 yd/s (`SpellMisc.Speed`), each within
## two of the longest frames since the press.
func check_frostbolt_flight(flights: Array) -> bool:
	const RELEASE_EVENT_S := 0.2
	var frame_s := 2.0 * longest_frame
	var mine := flights.filter(func(f): return f.spell == FROSTBOLT and f.caster == local_id and f.flight_time >= 0.0)
	if mine.is_empty():
		fail("No finished Frostbolt flight: " + str(flights))
		return false
	var flight: Dictionary = mine[-1]
	var expected: float = flight.distance / flight.speed
	print("FIXTURE FROSTBOLT_FLIGHT release_delay=%.3f distance=%.2f speed=%.1f flight=%.3f expected=%.3f longest_frame=%.3f" % [flight.release_delay, flight.distance, flight.speed, flight.flight_time, expected, longest_frame])
	if flight.release_delay < RELEASE_EVENT_S - 0.001 or flight.release_delay > RELEASE_EVENT_S + frame_s:
		fail("Frostbolt left %.3f s after SpellGo, not at the 200 ms release event" % flight.release_delay)
		return false
	if flight.flight_time < expected - 0.001 or flight.flight_time > expected + frame_s:
		fail("Frostbolt flew %.3f s over %.2f yd, not distance / speed" % [flight.flight_time, flight.distance])
		return false
	return true

## Battle Shout from its action button (the server's bar holds it at "-").
func cast_battle_shout() -> bool:
	var slot: int = spells().bar.find(BATTLE_SHOUT)
	if slot < 0:
		fail("Battle Shout is not on the main bar: " + str(spells().bar))
		return false
	await press(BAR_KEYS[slot])
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
	# Movie frame of the still, for cutting a recording (`--write-movie`, fixed fps).
	print("FIXTURE MARK %s frame=%d" % [file, Engine.get_frames_drawn()])
	await RenderingServer.frame_post_draw
	var started := Time.get_ticks_msec()
	var image := root.get_texture().get_image()
	var error := image.save_png(shots + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))
	print("FIXTURE CAPTURED %s in %d ms" % [file, Time.get_ticks_msec() - started])

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
