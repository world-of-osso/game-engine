extends SceneTree

## Mage auras on the HUD (docs/specs/buff-frame.md, TargetFrame auras) against a private
## server. Environment:
##   GODOT_TEST_SERVER             server address (a private test server)
##   AURA_ACCOUNT / AURA_CHARACTER account (password fbtest) and level-10 Human mage,
##                                 placed with a Blackrock Spy straight ahead
##   AURA_SHOTS                    screenshot directory
## Arcane Intellect on the mage shows in BuffFrame at the top right and, targeting
## itself (F1), as a large TargetFrame buff. Tab to the spy: Frostbolt's Chilled, then
## Polymorph, show as large debuff icons in the TargetFrame aura container with their
## icons, and their reverse cooldown swipe grows.

const PASSWORD := "fbtest"
const ARCANE_INTELLECT := 1459
const FROSTBOLT := 116
const CHILLED := 205708
const POLYMORPH := 118
## `SpellMisc.SpellIconFileDataID` of each aura spell.
const ICONS := {1459: 135932, 205708: 135846, 118: 136071}
const BAR_KEYS := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]

var client: Node
var shots := "/tmp/claude/auras-live/"
var character := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("AURA_ACCOUNT")
	character = OS.get_environment("AURA_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, AURA_ACCOUNT and AURA_CHARACTER are required")
		return
	if OS.get_environment("AURA_SHOTS") != "":
		shots = OS.get_environment("AURA_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not await wait_for(func(s): return s.catalog_ready and s.known.has(ARCANE_INTELLECT) and s.known.has(FROSTBOLT) and s.known.has(POLYMORPH), 60000, "mage with Arcane Intellect, Frostbolt and Polymorph"):
		return
	await wait_frames(120)
	# Arcane Intellect with nothing selected lands on the mage.
	if not await cast(ARCANE_INTELLECT, "Arcane Intellect"):
		return
	if not await wait_until(func(): return button(auras().buffs, ARCANE_INTELLECT) != null, 5000, "Arcane Intellect in BuffFrame"):
		return
	await wait_frames(10)
	var buff: Dictionary = button(auras().buffs, ARCANE_INTELLECT)
	print("FIXTURE BUFF ", buff)
	if not check_icon(buff, "BuffButton"):
		return
	# BuffFrame: TOPRIGHT -255,-10 of the HUD canvas; the first icon 15 px further left. The
	# in-world HUD scale is max(min(w / 1920, h / 1080), 2/3) at UI Scale 1.
	var scale := maxf(minf(root.size.x / 1920.0, root.size.y / 1080.0), 2.0 / 3.0)
	var right := root.size.x - (255.0 + 15.0) * scale
	if absf(buff.rect[0] + buff.rect[2] - right) > 1.5 or absf(buff.rect[1] - 10.0 * scale) > 1.5 or absf(buff.rect[2] - 30.0 * scale) > 1.0:
		fail("BuffButton0 icon rect %s, expected right %.1f top %.1f size %.1f" % [buff.rect, right, 10.0 * scale, 30.0 * scale])
		return
	await capture("01-arcane-intellect-buffframe.png")
	# Self-target: TargetFrame shows the mage's own buff, large (its own cast).
	await press(KEY_F1)
	if not await wait_until(func(): return button(auras().target_buffs, ARCANE_INTELLECT) != null, 3000, "Arcane Intellect on the self-target TargetFrame"):
		return
	await wait_frames(6)
	var own: Dictionary = button(auras().target_buffs, ARCANE_INTELLECT)
	print("FIXTURE SELF_TARGET ", own)
	if not check_icon(own, "TargetBuff") or not own.large or absf(own.rect[2] - 21.0 * scale) > 1.0:
		fail("Self-target Arcane Intellect is not a large 21 px icon: " + str(own))
		return
	await capture("02-self-target-arcane-intellect.png")
	await press(KEY_ESCAPE)
	# The spy: Frostbolt chills it.
	for attempt in range(10):
		await press(KEY_TAB)
		await wait_frames(4)
		if str(client.target_state().target_name).contains("Blackrock Spy"):
			break
	if not str(client.target_state().target_name).contains("Blackrock Spy"):
		fail("Tab did not reach the Blackrock Spy: " + str(client.target_state()))
		return
	if not await cast(FROSTBOLT, "Frostbolt"):
		return
	if not await wait_until(func(): return button(auras().target_debuffs, CHILLED) != null, 6000, "Chilled on the spy's TargetFrame"):
		return
	await wait_frames(20)
	var chilled: Dictionary = button(auras().target_debuffs, CHILLED)
	print("FIXTURE CHILLED ", chilled)
	if not check_icon(chilled, "TargetDebuff") or not chilled.large:
		fail("Chilled is not the mage's large debuff: " + str(chilled))
		return
	await capture("03-frostbolt-chilled.png")
	# Polymorph joins it; the mage's debuffs sort by aura instance.
	if not await cast(POLYMORPH, "Polymorph"):
		return
	if not await wait_until(func(): return button(auras().target_debuffs, POLYMORPH) != null, 5000, "Polymorph on the spy's TargetFrame"):
		return
	await wait_frames(20)
	var state := auras()
	var polymorph: Dictionary = button(state.target_debuffs, POLYMORPH)
	print("FIXTURE POLYMORPH ", state)
	if not check_icon(polymorph, "TargetDebuff") or not polymorph.large:
		fail("Polymorph is not the mage's large debuff: " + str(polymorph))
		return
	var chilled_now = button(state.target_debuffs, CHILLED)
	if chilled_now != null and chilled_now.rect[0] >= polymorph.rect[0]:
		fail("Chilled (applied first) must precede Polymorph: " + str(state.target_debuffs))
		return
	await capture("04-polymorph-and-chilled.png")
	var swipe_before: float = polymorph.swipe
	await wait_real(6.0)
	polymorph = button(auras().target_debuffs, POLYMORPH)
	print("FIXTURE SWIPE %.3f -> %.3f" % [swipe_before, polymorph.swipe])
	if polymorph == null or not (polymorph.swipe > swipe_before + 0.05):
		fail("Polymorph's swipe did not grow: %s -> %s" % [swipe_before, polymorph])
		return
	await capture("05-polymorph-swipe-later.png")
	print("FIXTURE AURAS_LIVE_DONE")
	client.free()
	quit(0)

func auras() -> Dictionary:
	return client.aura_state()

func button(list: Array, spell: int):
	for entry in list:
		if entry.spell_id == spell:
			return entry
	return null

## Shown, with the spell's icon, as a button of `prefix`.
func check_icon(entry, prefix: String) -> bool:
	if entry == null or not str(entry.name).begins_with(prefix) or not entry.visible or entry.texture_fdid != ICONS[entry.spell_id] or entry.rect[2] <= 0.0:
		fail("Aura button %s is not a visible %s with icon %s" % [entry, prefix, ICONS.get(entry.spell_id if entry != null else 0)])
		return false
	return true

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
	fail("Timed out waiting for %s: auras=%s target=%s" % [what, auras(), client.target_state()])
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
