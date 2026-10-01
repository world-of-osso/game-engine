extends SceneTree

## Live crit text and crit sound of a Human warrior's swings at a Northshire Blackrock
## Worg (creature 49871) against a private server, reusing `melee_sounds_live.gd`'s
## targeting and Attack start. Environment as there (MELEE_SECS defaults to 300), and
## MELEE_MOB, the target's name (default Blackrock Worg; a Training Dummy does not wander).
## Required, 12.1.0.69933: a swing the server sends as `CriticalHit` arrives as
## "Hit { critical: true }", swooshes the crit kit 236 (WeaponSwingSounds2 Medium crit),
## lands an impact kit no plain hit used (WeaponImpactSounds `CritImpactSoundID`; 53249
## on the flesh worg), and its
## floating combat text reads "<amount>!" at 1.5x the plain size.

const PASSWORD := "fbtest"
## Center distance the fixture accepts (the server swings within 5 yd).
const MELEE_REACH := 4.5
const ATTACK := 88163
const BAR_KEYS := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]
const WARRIOR_SWING := 235
const WARRIOR_SWING_CRIT := 236
const WARRIOR_IMPACT := 53248
const WARRIOR_IMPACT_CRIT := 53249
const WARRIOR_PARRIED := 53263
var MOB := OS.get_environment("MELEE_MOB") if OS.get_environment("MELEE_MOB") != "" else "Blackrock Worg"
const MOB_WOUND := [11908, 11909]
const MOB_SWING := [233, 234]
const MISS_WHOOSH := 7080
const WARRIOR_EXERTION := 2941
const MOB_EXERTION := [11907]
const MOB_IMPACT := [1014]
const MOB_PARRIED := 1019
const WARRIOR_WOUND := 2942
const MOB_DEATH := 11910
## The widest $CSS → $CAH gap of the attack clips (HumanMale HD Attack2H 133 ms). Sounds
## start on frame boundaries, so a measured gap may also span two of the longest frames.
const LAND_GAP := 0.133

var client: Node
var character := ""
var local_id := 0
var target_id := 0
## Every worg engaged.
var mobs: Array = []
## Longest frame seen: the timing slack of every measured sound gap.
var longest_frame := 0.0
## Every sound start and melee event seen, keyed so the bounded snapshot lists merge.
var sounds := {}
var melee := {}
## Floating combat texts seen: crit ("N!") texts with their font sizes, and plain sizes.
var crit_texts := {}
var plain_sizes := {}

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func _process(delta: float) -> bool:
	if client == null or not is_instance_valid(client) or local_id == 0:
		return false
	if not mobs.is_empty():
		longest_frame = maxf(longest_frame, delta)
	var visuals: Dictionary = client.spell_visuals_state()
	for start in visuals.sounds:
		var key := "%.4f|%s|%d|%d|%d" % [start.at, start.source, start.unit, start.sound_kit, start.fdid]
		if not sounds.has(key):
			sounds[key] = start
			print("FIXTURE SOUND %s unit=%s kit=%d fdid=%d at=%.3f" % [start.source, who(start.unit), start.sound_kit, start.fdid, start.at])
	for label in client.find_children("CombatText*", "Label3D", false, false):
		var text: String = label.text
		if text.ends_with("!"):
			if not crit_texts.has(text):
				crit_texts[text] = label.font_size
				print("FIXTURE CRIT_TEXT %s size=%d" % [text, label.font_size])
		elif text.is_valid_int():
			plain_sizes[label.font_size] = true
	for seen in visuals.melee:
		var key := "%.4f|%d|%d" % [seen.at, seen.attacker, seen.target]
		if not melee.has(key):
			melee[key] = seen
			print("FIXTURE MELEE %s -> %s %s at=%.3f" % [who(seen.attacker), who(seen.target), seen.result, seen.at])
	return false

func who(id: int) -> String:
	if id == local_id:
		return "warrior"
	if mobs.has(id):
		return "worg%d" % mobs.find(id)
	return str(id)

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("MELEE_ACCOUNT")
	character = OS.get_environment("MELEE_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, MELEE_ACCOUNT and MELEE_CHARACTER are required")
		return
	var secs := int(OS.get_environment("MELEE_SECS")) if OS.get_environment("MELEE_SECS") != "" else 300
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not await wait_until(func(): return client.spells_state().catalog_ready and client.spells_state().bar.has(ATTACK), 60000, "Attack on the action bar"):
		print("FIXTURE SPELLS ", client.spells_state())
		return
	local_id = client.account_state().local_player_id
	var deadline := Time.get_ticks_msec() + secs * 1000
	while Time.get_ticks_msec() < deadline and not complete():
		# A killed worg respawns at its spawn point (spawntimesecs 120).
		if not await engage_worg():
			await wait_frames(300)
			continue
		# Fight while swings keep coming; a dead or wandered-off worg (10 yd) is replaced.
		var last := melee.size()
		var quiet_since := Time.get_ticks_msec()
		while Time.get_ticks_msec() < deadline and not complete() and Time.get_ticks_msec() - quiet_since < 8000:
			await wait_frames(30)
			if melee.size() != last:
				last = melee.size()
				quiet_since = Time.get_ticks_msec()
	# Let the last swing's sounds land.
	await wait_frames(60)
	print("FIXTURE SUMMARY ", summary(), " longest_frame=%.3f" % longest_frame)
	if not complete():
		fail("Missing melee sounds after %d s: %s" % [secs, summary()])
		return
	for size in crit_texts.values():
		for plain in plain_sizes.keys():
			if size * 2 != plain * 3:
				fail("Crit text size %d is not 1.5x the plain %d" % [size, plain])
				return
	if not check_landing():
		return
	print("FIXTURE MELEE_OUTCOMES_DONE")
	client.free()
	quit(0)

## Tab to a Blackrock Worg within 3.5 yd (the server swings within 5) and start
## auto-attack on it with the Attack action. Tab cycles the visible units nearest
## first; worgs wander, so the caller retries until one is in range.
func engage_worg() -> bool:
	var distance := INF
	for attempt in range(60):
		await press(KEY_TAB)
		await wait_frames(6)
		var state: Dictionary = client.target_state()
		if state.target != null and str(state.target_name).contains(MOB):
			distance = client.unit_transform(local_id).origin.distance_to(client.unit_transform(state.target).origin)
			if distance < MELEE_REACH:
				break
	if distance >= MELEE_REACH:
		print("FIXTURE NO_WORG_IN_RANGE ", client.target_state())
		return false
	target_id = client.target_state().target
	if not mobs.has(target_id):
		mobs.append(target_id)
	print("FIXTURE TARGET ", client.target_state(), " distance=%.2f" % distance)
	await press(BAR_KEYS[client.spells_state().bar.find(ATTACK)])
	return await wait_until(func(): return client.target_state().auto_attack == target_id, 3000, "Attack starts auto-attack")

## Sound starts of `source` on one of `units` with a SoundKit in `kits`.
func starts(source: String, units: Array, kits: Array) -> Array:
	return sounds.values().filter(func(s): return s.source == source and units.has(s.unit) and kits.has(s.sound_kit))

func avoided_swings() -> Array:
	return melee.values().filter(func(m): return avoided(m) and (m.attacker == local_id or mobs.has(m.attacker)))

func avoided(event: Dictionary) -> bool:
	return event.result == "Miss" or event.result == "Dodge"

func crit_events() -> Array:
	return melee.values().filter(func(m): return m.attacker == local_id and m.result == "Hit { critical: true }")

func complete() -> bool:
	return not crit_events().is_empty() \
		and not starts("swing", [local_id], [WARRIOR_SWING_CRIT]).is_empty() \
		and crit_impact_kits().size() > 0 \
		and not crit_texts.is_empty()

## Impact kits on the mob right after a crit swing that no plain hit used.
func crit_impact_kits() -> Array:
	var hit_kits := {}
	var crit_kits := {}
	for event in melee.values():
		if event.attacker != local_id:
			continue
		var impacts: Array = sounds.values().filter(func(s): return s.source == "impact" and s.unit == event.target and s.at >= event.at)
		impacts.sort_custom(func(a, b): return a.at < b.at)
		if impacts.is_empty():
			continue
		if event.result == "Hit { critical: true }":
			crit_kits[impacts[0].sound_kit] = true
		elif event.result == "Hit { critical: false }":
			hit_kits[impacts[0].sound_kit] = true
	return crit_kits.keys().filter(func(kit): return not hit_kits.has(kit))

func summary() -> Dictionary:
	return {
		"melee": melee.size(),
		"crits": crit_events().size(),
		"crit_swooshes": starts("swing", [local_id], [WARRIOR_SWING_CRIT]).size(),
		"crit_impact_kits": crit_impact_kits(),
		"crit_texts": crit_texts,
		"plain_text_sizes": plain_sizes.keys(),
	}

## Each melee event's attacker swooshes after it; a landed swing's impact follows that
## swoosh within LAND_GAP, an avoided swing has no impact before the attacker's next
## swoosh.
func check_landing() -> bool:
	var events: Array = melee.values()
	events.sort_custom(func(a, b): return a.at < b.at)
	for index in range(events.size()):
		var event: Dictionary = events[index]
		var attacker: int = event.attacker
		var victim: int = event.target
		if attacker != local_id and not mobs.has(attacker):
			continue
		var next_at := INF
		for later in events.slice(index + 1):
			if later.attacker == attacker:
				next_at = later.at
				break
		var swooshes: Array = sounds.values().filter(func(s): return s.source == "swing" and s.unit == attacker and s.at >= event.at and s.at < next_at)
		if swooshes.is_empty():
			# The final swing may still be in its clip when the worg dies.
			if next_at == INF:
				continue
			fail("No swoosh for %s's %s at %.3f" % [who(attacker), event.result, event.at])
			return false
		var swoosh: Dictionary = swooshes[0]
		# From the event, not the swoosh: a first-use swoosh file (e.g. the worg's first
		# Light kit 233) starts when its async load ends, which can trail its own impact.
		var impacts: Array = sounds.values().filter(func(s): return s.source == "impact" and s.unit == victim and s.at >= event.at and s.at < next_at)
		if avoided(event):
			if swoosh.sound_kit != MISS_WHOOSH:
				fail("Avoided swing at %.3f swooshed %d, not the miss whoosh" % [event.at, swoosh.sound_kit])
				return false
			if not impacts.is_empty():
				fail("Avoided swing at %.3f struck: %s" % [event.at, impacts])
				return false
			print("FIXTURE AVOIDED %s swoosh kit=%d fdid=%d at=%.3f, no impact" % [who(attacker), swoosh.sound_kit, swoosh.fdid, swoosh.at])
			continue
		if impacts.is_empty():
			if next_at == INF:
				continue
			fail("No impact for %s's %s at %.3f" % [who(attacker), event.result, event.at])
			return false
		var gap: float = impacts[0].at - swoosh.at
		if absf(gap) > LAND_GAP + 2.0 * longest_frame:
			fail("%s's impact %.3f s after its swoosh (longest frame %.3f s)" % [who(attacker), gap, longest_frame])
			return false
		print("FIXTURE LANDED %s %s swoosh=%d/%d at=%.3f impact=%d/%d gap=%.3f" % [who(attacker), event.result, swoosh.sound_kit, swoosh.fdid, swoosh.at, impacts[0].sound_kit, impacts[0].fdid, gap])
	return true

func enter_world() -> bool:
	# Asset startup attaches the character select UI asynchronously.
	var deadline := Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 and client.get_node_or_null("CharacterSelectUI") != null:
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
	var world_timeout := int(OS.get_environment("MELEE_WORLD_TIMEOUT_S")) if OS.get_environment("MELEE_WORLD_TIMEOUT_S") != "" else 300
	deadline = Time.get_ticks_msec() + world_timeout * 1000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD at ", state.local_player_position)
			await wait_frames(120)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func wait_until(condition: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		if condition.call():
			return true
		await process_frame
	fail("Timed out waiting for " + what)
	return false

func press(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		root.push_input(event, true)
		await wait_frames(2)

func click(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await wait_frames(2)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
