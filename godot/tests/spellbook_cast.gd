extends SceneTree

## Spellbook, main action bar and casting against the dev server
## (docs/specs/spellbook-action-bar.md). Environment:
##   GODOT_TEST_SERVER=127.0.0.1:5000
##   SPELL_ACCOUNT / SPELL_CHARACTER   account (password fbtest) and character
##   SPELL_EXPECT_LEVEL                the level the character must be at
##   SPELL_CAST=1                      also cast on the nearest training dummy
##   SPELL_SHOTS                       screenshot directory
## Checks at any level: the spellbook lists the known warrior spells and the class
## spells of later levels as "Level N"; the main bar holds the server's buttons with
## keys 1..=. With SPELL_CAST: Slam without a target fails "You have no target.";
## Tab targets the dummy and auto-attack builds rage; key 1 (Slam) is accepted
## (damage over the dummy, GCD sweep); an immediate second press fails on the GCD;
## Battle Shout (from the spellbook at level 10) starts the GCD without a target.

const PASSWORD := "fbtest"
const SLAM := 1464
const ATTACK := 88163
const CHARGE := 100
const BATTLE_SHOUT := 6673
## `SpellPower` rage cost of Slam, in tenths.
const SLAM_RAGE := 200

var client: Node
var shots := "/tmp/claude/spellbook/"
var character := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5000":
		fail("GODOT_TEST_SERVER must explicitly select 127.0.0.1:5000")
		return
	var account := OS.get_environment("SPELL_ACCOUNT")
	character = OS.get_environment("SPELL_CHARACTER")
	var level := int(OS.get_environment("SPELL_EXPECT_LEVEL"))
	if account == "" or character == "" or level <= 0:
		fail("SPELL_ACCOUNT, SPELL_CHARACTER and SPELL_EXPECT_LEVEL are required")
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
	if not await wait_for(func(s): return s.catalog_ready and not s.known.is_empty() and s.level == level, 60000, "catalog, known spells and level %d" % level):
		return
	var state := spells()
	print("FIXTURE SPELLS level=%d known=%s bar=%s spec=%d" % [state.level, state.known, state.bar, state.spec])
	if not check_known(state, level):
		return
	if not await check_spellbook(level):
		return
	if not await check_action_bar():
		return
	if OS.get_environment("SPELL_CAST") == "1":
		if not await cast_on_dummy(level):
			return
	print("FIXTURE SPELLBOOK_CAST_DONE")
	client.free()
	quit(0)

## Retail class line (SkillLineAbility 840 + SpellLevels): Slam and Attack at 1,
## Charge at 2, Battle Shout at 10.
func check_known(state: Dictionary, level: int) -> bool:
	var known: PackedInt64Array = state.known
	for spell in [SLAM, ATTACK]:
		if not known.has(spell):
			fail("Level %d warrior does not know %d: %s" % [level, spell, known])
			return false
	if known.has(CHARGE) != (level >= 2) or known.has(BATTLE_SHOUT) != (level >= 10):
		fail("Level %d known spells are not the retail list: %s" % [level, known])
		return false
	return true

func check_spellbook(level: int) -> bool:
	push_key(KEY_P, true)
	await wait_frames(2)
	push_key(KEY_P, false)
	if not await wait_for(func(s): return s.spellbook_open and not s.spellbook.is_empty(), 5000, "spellbook open"):
		return false
	await wait_frames(20)
	var entries := {}
	for entry in spells().spellbook:
		entries[entry.id] = entry
	print("FIXTURE SPELLBOOK ", spells().spellbook)
	if not entries.has(SLAM) or entries[SLAM].available_at != 0:
		fail("Slam is not a known spellbook entry: " + str(entries.get(SLAM)))
		return false
	var shout_at := 0 if level >= 10 else 10
	if not entries.has(BATTLE_SHOUT) or entries[BATTLE_SHOUT].available_at != shout_at:
		fail("Battle Shout entry at level %d: %s" % [level, entries.get(BATTLE_SHOUT)])
		return false
	var book := client.get_node("SpellBookUI")
	var name_label := book.find_child("SpellBookItem%dName" % SLAM, true, false) as Label
	if name_label == null or name_label.text != "Slam":
		fail("Slam's spellbook label: " + (name_label.text if name_label else "<none>"))
		return false
	if level < 10:
		var shout_level := book.find_child("SpellBookItem%dRequiredLevel" % BATTLE_SHOUT, true, false) as Label
		if shout_level == null or shout_level.text != "Level 10":
			fail("Battle Shout does not show Level 10")
			return false
	await capture("spellbook-level-%d.png" % level)
	push_key(KEY_ESCAPE, true)
	await wait_frames(2)
	push_key(KEY_ESCAPE, false)
	return await wait_for(func(s): return not s.spellbook_open, 3000, "Escape closes the spellbook")

func check_action_bar() -> bool:
	var bar := client.get_node("MainActionBarUI")
	for index in range(12):
		var hotkey := bar.find_child("ActionButton%dHotKey" % (index + 1), true, false) as Label
		var expected: String = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "="][index]
		if hotkey == null or hotkey.text != expected:
			fail("ActionButton%d key label" % (index + 1))
			return false
	var slots: PackedInt64Array = spells().bar
	if slots[0] != SLAM:
		fail("Button 1 is not Slam: " + str(slots))
		return false
	await capture("action-bar.png")
	return true

func cast_on_dummy(level: int) -> bool:
	# No target: the server rejects Slam with NoTarget.
	await press(KEY_1)
	if not await wait_for(func(s): return s.errors.has("You have no target."), 5000, "NoTarget error"):
		return false
	await capture("no-target.png")
	push_key(KEY_TAB, true)
	await wait_frames(2)
	push_key(KEY_TAB, false)
	if not await wait_until(func(): return str(client.target_state().target_name).contains("Training Dummy"), 5000, "Tab targets the training dummy"):
		return false
	print("FIXTURE TARGET ", client.target_state())
	if not await wait_for(func(s): return s.power >= SLAM_RAGE, 30000, "auto-attack rage for Slam"):
		return false
	var health_before: float = spells().target_health
	var damage_before: int = spells().damage_dealt.size()
	await press(KEY_1)
	if not await wait_for(func(s): return s.gcd_ms > 0 and s.damage_dealt.size() > damage_before, 5000, "Slam accepted: damage and GCD"):
		return false
	print("FIXTURE SLAM ", spells())
	if spells().combat_text == 0:
		fail("No floating combat text for the Slam damage")
		return false
	await capture("slam-hit-gcd.png")
	# Pressed again inside the GCD: rejected.
	var errors_before: int = spells().errors.size()
	await press(KEY_1)
	if not await wait_for(func(s): return s.errors.size() > errors_before, 5000, "second Slam rejected on the GCD"):
		return false
	print("FIXTURE SECOND SLAM ", spells().errors[-1], " dummy health ", health_before, " -> ", spells().target_health)
	await capture("gcd-error.png")
	if level >= 10:
		if not await wait_for(func(s): return s.gcd_ms == 0, 5000, "GCD over"):
			return false
		if not await cast_battle_shout():
			return false
	return true

## Battle Shout from the spellbook: a click on its icon casts it.
func cast_battle_shout() -> bool:
	await press(KEY_P)
	if not await wait_for(func(s): return s.spellbook_open, 3000, "spellbook reopened"):
		return false
	await wait_frames(10)
	var icon := client.get_node("SpellBookUI").find_child("SpellBookItem%dButton" % BATTLE_SHOUT, true, false) as Control
	if icon == null:
		fail("No Battle Shout button")
		return false
	var sent_before: int = spells().sent.size()
	await click(icon)
	if not await wait_for(func(s): return s.sent.size() > sent_before and s.sent[-1] == BATTLE_SHOUT and s.gcd_ms > 0, 5000, "Battle Shout accepted (GCD)"):
		return false
	await capture("battle-shout.png")
	await press(KEY_ESCAPE)
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
	fail("Timed out waiting for %s: %s" % [what, client.target_state()])
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
