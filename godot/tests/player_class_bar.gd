extends SceneTree

## Player frame class resource bar against a live server (docs/specs/godot-parity-matrix.md).
## Environment:
##   GODOT_TEST_SERVER          server address (a private test server)
##   BAR_ACCOUNT / BAR_CHARACTER  account (password fbtest) and its only character, a mage
##   BAR_EXPECT                 "shown" (Arcane spec 62) or "hidden" (any other spec)
##   BAR_SHOT                   PNG path for a crop of the player frame and the area below it
##   BAR_TARGET                 optional PNG path: Tab-target a unit, capture both frames and
##                              assert the health bars line up as in Retail (player 41..61,
##                              normal target 40..60 below the frame top: PlayerFrame.lua:697,
##                              TargetFrame.lua:419, both frames at y 250 in
##                              EditModePresetLayouts.lua:231-257)
##   BAR_CHARGES                optional directory: Tab to a unit named BAR_ENEMY, cast Arcane
##                              Blast (energizes 1 Arcane Charge) twice and capture the charges
##                              mid-activateAnim and settled; asserts the lit icons follow
## Retail shows Arcane Charges only for the Arcane spec (MageArcaneChargesBar.xml:126-127,
## ClassPowerBar.lua:82-83). When shown, the row sits 11 px below the mana bar, centred 1 px
## left of it (PlayerFrame.lua:716,758; MageArcaneChargesBar.xml:134), clear of its text.

const PASSWORD := "fbtest"
const SPEC_MAGE_ARCANE := 62
const ARCANE_BLAST := 30451
const BAR_KEYS := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]

var client: Node
var character := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("BAR_ACCOUNT")
	character = OS.get_environment("BAR_CHARACTER")
	var expect := OS.get_environment("BAR_EXPECT")
	var shot := OS.get_environment("BAR_SHOT")
	if server == "" or account == "" or character == "" or not expect in ["shown", "hidden"]:
		fail("GODOT_TEST_SERVER, BAR_ACCOUNT, BAR_CHARACTER and BAR_EXPECT are required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Connection: " + error)
		return
	if not await enter_world():
		return
	var deadline := Time.get_ticks_msec() + 20000
	while Time.get_ticks_msec() < deadline and int(client.spells_state().spec) == 0:
		await process_frame
	await wait_frames(30)
	var spec := int(client.spells_state().spec)
	var ui = client.find_child("UnitFramesUI", true, false)
	var frame := ui.find_child("PlayerFrame", true, false) as Control if ui != null else null
	var mana := ui.find_child("PlayerManaBar", true, false) as Control if ui != null else null
	if frame == null or mana == null or not frame.is_visible_in_tree():
		fail("No player frame")
		return
	var row := ui.find_child("PlayerSecondaryResourceRow", true, false) as Control
	var shown := row != null and row.is_visible_in_tree()
	print("FIXTURE spec=%d frame=%s mana=%s row=%s" % [spec, frame.get_global_rect(), mana.get_global_rect(), row.get_global_rect() if shown else "none"])
	if shot != "":
		await capture(shot, frame.get_global_rect().grow_individual(20, 10, 20, 50))
	if expect == "hidden":
		if spec == SPEC_MAGE_ARCANE or shown:
			fail("Expected no class bar for spec %d, shown=%s" % [spec, shown])
			return
	else:
		if spec != SPEC_MAGE_ARCANE or not shown:
			fail("Expected Arcane Charges for spec %d, shown=%s" % [spec, shown])
			return
		var bar := mana.get_global_rect()
		var charges := row.get_global_rect()
		var scale := bar.size.x / 124.0
		if absf(charges.position.y - bar.end.y - 11.0 * scale) > 0.5 or absf(charges.get_center().x - bar.get_center().x + scale) > 0.5:
			fail("Arcane Charges at %s, mana bar at %s" % [charges, bar])
			return
	var both := OS.get_environment("BAR_TARGET")
	if both != "" and not await check_target_alignment(ui, mana, both):
		return
	var charges_dir := OS.get_environment("BAR_CHARGES")
	if charges_dir != "" and not await build_charges(ui, row, charges_dir):
		return
	print("PASS: class bar ", expect, " for spec ", spec)
	quit(0)

func check_target_alignment(ui: Node, mana: Control, path: String) -> bool:
	for attempt in range(20):
		if str(client.target_state().target_name) != "":
			break
		push_key(KEY_TAB, true)
		await wait_frames(2)
		push_key(KEY_TAB, false)
		await wait_frames(20)
	var target := ui.find_child("TargetFrame", true, false) as Control
	if target == null or not target.is_visible_in_tree():
		fail("No target frame: " + str(client.target_state()))
		return false
	await wait_frames(10)
	var player_health := (ui.find_child("PlayerHealthBar", true, false) as Control).get_global_rect()
	var target_health := (ui.find_child("TargetHealthBar", true, false) as Control).get_global_rect()
	var scale := mana.get_global_rect().size.x / 124.0
	print("FIXTURE target=%s player_health=%s target_health=%s" % [client.target_state().target_name, player_health, target_health])
	var centre := float(root.size.x) / 2.0
	var player_frame := (ui.find_child("PlayerFrame", true, false) as Control).get_global_rect()
	await capture(path, player_frame.merge(target.get_global_rect()).grow_individual(20, 20, 20, 50))
	if absf(target_health.end.y - (player_health.end.y - scale)) > 0.5:
		fail("Health bar bottoms: player %.2f, target %.2f" % [player_health.end.y, target_health.end.y])
		return false
	if absf((target_health.get_center().x - centre) - (centre - player_health.get_center().x)) > 0.5:
		fail("Health bars not mirrored about x %.1f" % centre)
		return false
	return true

func build_charges(ui: Node, row: Control, dir: String) -> bool:
	var enemy := OS.get_environment("BAR_ENEMY")
	for attempt in range(30):
		if str(client.target_state().target_name).contains(enemy):
			break
		await press(KEY_TAB)
		await wait_frames(10)
	if not str(client.target_state().target_name).contains(enemy):
		fail("Could not target %s: %s" % [enemy, client.target_state()])
		return false
	var region := row.get_global_rect().grow(12)
	await capture(dir + "/charges-0.png", region)
	for charge in [1, 2]:
		var slot: int = client.spells_state().bar.find(ARCANE_BLAST)
		if slot < 0:
			fail("Arcane Blast not on the bar: %s known=%s" % [client.spells_state().bar, client.spells_state().known.has(ARCANE_BLAST)])
			return false
		var deadline := Time.get_ticks_msec() + 5000
		while Time.get_ticks_msec() < deadline and int(client.spells_state().gcd_ms) > 0:
			await process_frame
		await press(BAR_KEYS[slot])
		var lit := ui.find_child("PlayerSecondaryResourcePip%dArcaneIcon" % (charge - 1), true, false) as Control
		deadline = Time.get_ticks_msec() + 8000
		while Time.get_ticks_msec() < deadline and not (lit != null and lit.is_visible_in_tree()):
			await process_frame
			lit = ui.find_child("PlayerSecondaryResourcePip%dArcaneIcon" % (charge - 1), true, false) as Control
		if lit == null or not lit.is_visible_in_tree():
			fail("Arcane Blast %d gave no charge: %s" % [charge, client.target_state()])
			return false
		await wait_real(0.3)
		await capture(dir + "/charges-%d-activating.png" % charge, region)
		await wait_real(1.2)
		await capture(dir + "/charges-%d.png" % charge, region)
		for index in range(4):
			var icon := ui.find_child("PlayerSecondaryResourcePip%dArcaneIcon" % index, true, false) as Control
			if icon.is_visible_in_tree() != (index < charge):
				fail("Charge %d icon shown=%s with %d charges" % [index, icon.is_visible_in_tree(), charge])
				return false
	print("FIXTURE charges built")
	return true

func press(code: Key) -> void:
	push_key(code, true)
	await wait_frames(2)
	push_key(code, false)
	await wait_frames(2)

func wait_real(seconds: float) -> void:
	var deadline := Time.get_ticks_msec() + int(seconds * 1000)
	while Time.get_ticks_msec() < deadline:
		await process_frame

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

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
		if state.screen == "InWorld" and state.local_player_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			await wait_frames(60)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

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

func capture(path: String, region: Rect2) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var crop := Rect2i(region).intersection(Rect2i(Vector2i.ZERO, image.get_size()))
	var error := image.get_region(crop).save_png(path)
	if error != OK:
		fail("Could not save " + path + ": " + str(error))

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
