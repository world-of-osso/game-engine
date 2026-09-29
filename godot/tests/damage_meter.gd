extends SceneTree

## Retail damage meter at the top left (docs/specs/damage-meter.md). Environment:
##   GODOT_TEST_SERVER             server address (a private test server)
##   DPS_ACCOUNT / DPS_CHARACTER   account (password fbtest) and level-10 mage, placed
##                                 with Northshire's training dummies nearest
##   DPS_SHOTS                     screenshot directory
## Combat 1: Frostbolt the nearest Training Dummy DPS_BOLTS times (default 3) and wait
## for combat to end. The server's Current and Overall sessions and the window's rows
## show exactly the damage the combat log reported. The session menu switches the window
## to Current. Combat 2: the same again is session 2; Current has only its damage,
## Overall the sum of both.

const PASSWORD := "fbtest"
const FROSTBOLT := 116
const BAR_KEYS := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]

var client: Node
var shots := "/tmp/claude/damage-meter/"
var character := ""
var local_id := 0
var log_seq := 0
## Combat log damage by the local player, per combat.
var logged := [0, 0, 0]
var combat_index := 0

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

## Collect the local player's damage lines as they arrive (the log keeps the newest 64).
func _process(_delta: float) -> bool:
	if client == null or not is_instance_valid(client) or local_id == 0:
		return false
	var state: Dictionary = client.damage_meter_state()
	var seq: int = state.combat_log_seq
	var entries: Array = state.combat_log
	var fresh: int = mini(seq - log_seq, entries.size())
	for index in range(entries.size() - fresh, entries.size()):
		var entry: Dictionary = entries[index]
		if entry.damage and entry.source == local_id:
			logged[combat_index] += entry.amount
	log_seq = seq
	return false

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("DPS_ACCOUNT")
	character = OS.get_environment("DPS_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, DPS_ACCOUNT and DPS_CHARACTER are required")
		return
	if OS.get_environment("DPS_SHOTS") != "":
		shots = OS.get_environment("DPS_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not await wait_for(func(s): return s.catalog_ready and s.known.has(FROSTBOLT) and s.level == 10, 60000, "level-10 mage with Frostbolt"):
		return
	local_id = client.account_state().local_player_id
	var stream_ms := int(OS.get_environment("DPS_STREAM_SECS")) * 1000 if OS.get_environment("DPS_STREAM_SECS") != "" else 45000
	var streaming := Time.get_ticks_msec()
	while client.account_state().world_objects.pending > 0 and Time.get_ticks_msec() - streaming < stream_ms:
		await wait_frames(60)
	await wait_frames(30)
	var meter: Dictionary = client.damage_meter_state()
	print("FIXTURE METER_IDLE ", meter)
	if not meter.open or meter.session != "O" or not meter.rows.is_empty():
		fail("The meter should be open on an empty Overall: " + str(meter))
		return
	if not check_top_left():
		return
	await capture("00-idle-overall.png")
	if not await target_dummy():
		return
	# Combat 1.
	combat_index = 1
	if not await fight("combat 1"):
		return
	await capture("01-combat1-in-combat-overall.png")
	if not await wait_session_end(1):
		return
	await capture("02-combat1-ended-overall.png")
	if not check_session(client.damage_meter_state(), 1, logged[1], logged[1]):
		return
	if not check_rows(logged[1]):
		return
	# Switch the window to Current through the session menu.
	if not await open_session_menu():
		return
	await capture("03-session-menu.png")
	if not await choose_session("DamageMeterSessionMenuOption1", "C"):
		return
	if not check_rows(logged[1]):
		return
	await capture("04-combat1-current.png")
	# Combat 2: a new session.
	combat_index = 2
	if not await fight("combat 2"):
		return
	if not await wait_session_end(2):
		return
	var total: int = logged[1] + logged[2]
	if not check_session(client.damage_meter_state(), 2, logged[2], total):
		return
	if not check_rows(logged[2]):
		return
	await capture("05-combat2-current.png")
	if not await open_session_menu():
		return
	if not await choose_session("DamageMeterSessionMenuOption2", "O"):
		return
	if not check_rows(total):
		return
	await capture("06-combat2-overall.png")
	print("FIXTURE LOGGED combat1=%d combat2=%d" % [logged[1], logged[2]])
	print("FIXTURE METER_FINAL ", client.damage_meter_state())
	print("FIXTURE DAMAGE_METER_DONE")
	client.free()
	quit(0)

## The window's root frame sits at the screen's top left.
func check_top_left() -> bool:
	var ui = client.get_node_or_null("DamageMeterUI")
	var frame = ui.find_child("DamageMeter", true, false) if ui != null else null
	if frame == null:
		fail("No DamageMeter frame")
		return false
	var rect: Rect2 = frame.get_global_rect()
	print("FIXTURE METER_RECT ", rect)
	if rect.position.length() > 1.0 or rect.size.x < 100:
		fail("The meter is not at the top left: " + str(rect))
		return false
	return true

func target_dummy() -> bool:
	for attempt in range(10):
		await press(KEY_TAB)
		await wait_frames(4)
		if str(client.target_state().target_name).contains("Training Dummy"):
			break
	if not str(client.target_state().target_name).contains("Training Dummy"):
		fail("Tab did not target a Training Dummy: " + str(client.target_state()))
		return false
	return await face_target()

## Turn with A/D until the target is ahead.
func face_target() -> bool:
	for step in range(200):
		var angle := angle_to_target()
		if absf(angle) < 0.25:
			await wait_frames(10)
			print("FIXTURE FACING %.3f target=%s" % [angle_to_target(), client.target_state()])
			return true
		var key := KEY_A if angle > 0 else KEY_D
		push_key(key, true)
		await wait_frames(2)
		push_key(key, false)
		await wait_frames(2)
	fail("Could not face the target: %.3f" % angle_to_target())
	return false

## DPS_BOLTS Frostbolts; each must land (its damage line arrives) before the next.
func fight(what: String) -> bool:
	var bolts := int(OS.get_environment("DPS_BOLTS")) if OS.get_environment("DPS_BOLTS") != "" else 3
	for bolt in range(bolts):
		var before: int = logged[combat_index]
		if not await cast(FROSTBOLT, "%s Frostbolt %d" % [what, bolt + 1]):
			return false
		if not await wait_until(func(): return logged[combat_index] > before, 8000, "%s Frostbolt %d lands" % [what, bolt + 1]):
			return false
	# The last line's snapshot follows within the server's 1 s interval.
	await wait_real(1.5)
	return true

func wait_session_end(session_id: int) -> bool:
	return await wait_until(func():
		var state: Dictionary = client.damage_meter_state()
		return state.has("current") and state.current.session_id == session_id and not state.current.active,
		15000, "session %d to end with combat" % session_id)

func local_source(session: Dictionary) -> Dictionary:
	for source in session.sources:
		if source.local:
			return source
	return {}

## Current is `session_id` with the local player's `current` damage, Overall has `overall`.
func check_session(state: Dictionary, session_id: int, current: int, overall: int) -> bool:
	print("FIXTURE SESSION %d logged=%d overall_logged=%d state=%s" % [session_id, current, overall, state])
	var mine := local_source(state.current)
	var all := local_source(state.overall)
	if mine.is_empty() or all.is_empty():
		fail("The local player is missing from the meter: " + str(state))
		return false
	if mine.total != current or state.current.total != current:
		fail("Current shows %d, the combat log %d" % [mine.total, current])
		return false
	if all.total != overall:
		fail("Overall shows %d, the combat log %d" % [all.total, overall])
		return false
	var dps: float = float(current) / state.current.duration
	if absf(mine.dps - dps) > 0.01 or state.current.duration < 5.0:
		fail("Current DPS %.3f over %.2f s, expected %.3f" % [mine.dps, state.current.duration, dps])
		return false
	return true

## The window's first row is the local player with `amount` as its value.
func check_rows(amount: int) -> bool:
	var state: Dictionary = client.damage_meter_state()
	var ui = client.get_node_or_null("DamageMeterUI")
	var value = ui.find_child("DamageMeterEntry1Value", true, false) if ui != null else null
	var name = ui.find_child("DamageMeterEntry1Name", true, false) if ui != null else null
	print("FIXTURE ROWS session=%s rows=%s shown=%s|%s" % [state.session, state.rows, name.text if name != null else "?", value.text if value != null else "?"])
	if state.rows.size() != 1:
		fail("Expected one row: " + str(state.rows))
		return false
	var row: Dictionary = state.rows[0]
	if row.name != "1. " + character or not row.value.begins_with(abbreviate(amount) + " ("):
		fail("Row %s does not show %s with %d" % [row, character, amount])
		return false
	if value == null or value.text != row.value or name == null or name.text != row.name:
		fail("The window does not draw the row: " + str(row))
		return false
	return true

## `AbbreviateLargeNumbers` for the amounts a few Frostbolts reach.
func abbreviate(value: int) -> String:
	var digits := str(value)
	if digits.length() >= 6:
		return digits.substr(0, digits.length() - 3) + "K"
	if digits.length() >= 4:
		return digits.substr(0, digits.length() - 3) + "," + digits.substr(digits.length() - 3)
	return digits

func open_session_menu() -> bool:
	var ui = client.get_node_or_null("DamageMeterUI")
	await click(ui.find_child("DamageMeterSessionDropdown", true, false))
	return await wait_until(func(): return client.damage_meter_state().menu_open, 2000, "the session menu opens")

func choose_session(option: String, short_name: String) -> bool:
	var ui = client.get_node_or_null("DamageMeterUI")
	var button = ui.find_child(option, true, false)
	if button == null:
		fail("No menu option " + option)
		return false
	await click(button)
	if not await wait_until(func(): return client.damage_meter_state().session == short_name and not client.damage_meter_state().menu_open, 2000, "the window shows " + short_name):
		return false
	await wait_frames(4)
	return true

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

## Cast `spell` once the GCD and any cast are over and wait for the server to take it.
func cast(spell: int, what: String) -> bool:
	if not await wait_for(func(s): return s.gcd_ms == 0, 5000, "GCD before " + what):
		return false
	if not await press_spell(spell):
		return false
	return await wait_for(func(s): return s.gcd_ms > 0, 3000, what + " accepted")

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
	fail("Timed out waiting for %s: meter=%s target=%s logged=%s" % [what, client.damage_meter_state(), client.target_state(), logged])
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
