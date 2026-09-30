extends SceneTree

## A class's player frame resource bar against a live server (docs/specs/godot-parity-matrix.md).
## Environment:
##   GODOT_TEST_SERVER            server address (a private test server)
##   BAR_ACCOUNT / BAR_CHARACTER  account (password fbtest) and its only character
##   BAR_EXPECT                   "shown" or "hidden"
##   BAR_LIT_PART                 texture part a lit point shows (Retail parentKey, e.g.
##                                IconUncharged, ActiveTexture, Rune_Active, Shard_Icon)
##   BAR_LIT                      points lit on entering the world
##   BAR_SPELL / BAR_ENEMY        optional: Tab to a unit whose name contains BAR_ENEMY and cast
##                                BAR_SPELL (on the action bar) once
##   BAR_LIT_AFTER                points lit once that cast has landed
##   BAR_DIR                      capture directory: the frame on entering, mid-animation
##                                after the cast, and settled
## Points are PlayerSecondaryResourcePip<i><part>; the fixture counts the ones whose lit part
## is drawn. Draw order puts lit points first (Retail sorts ready runes left).

const PASSWORD := "fbtest"
const BAR_KEYS := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]

var client: Node
var character := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("BAR_ACCOUNT")
	character = OS.get_environment("BAR_CHARACTER")
	var expect := OS.get_environment("BAR_EXPECT")
	var dir := OS.get_environment("BAR_DIR")
	if server == "" or account == "" or character == "" or not expect in ["shown", "hidden"] or dir == "":
		fail("GODOT_TEST_SERVER, BAR_ACCOUNT, BAR_CHARACTER, BAR_EXPECT and BAR_DIR are required")
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
	await wait_real(3.0)
	var ui = client.find_child("UnitFramesUI", true, false)
	var frame := ui.find_child("PlayerFrame", true, false) as Control if ui != null else null
	if frame == null or not frame.is_visible_in_tree():
		fail("No player frame")
		return
	var row := ui.find_child("PlayerSecondaryResourceRow", true, false) as Control
	var shown := row != null and row.is_visible_in_tree()
	print("FIXTURE %s spec=%d row=%s" % [character, int(client.spells_state().spec), row.get_global_rect() if shown else "none"])
	await capture(dir + "/%s-frame.png" % character, frame.get_global_rect().grow_individual(20, 10, 20, 60))
	if expect == "hidden":
		if shown:
			fail("Expected no class bar, got " + str(row.get_global_rect()))
			return
		print("PASS: no class bar for ", character)
		quit(0)
		return
	if not shown:
		fail("Expected a class bar")
		return
	var region := row.get_global_rect().grow(16)
	var part := OS.get_environment("BAR_LIT_PART")
	var lit := lit_points(ui, part)
	await capture(dir + "/%s-0.png" % character, region)
	if lit != int(OS.get_environment("BAR_LIT")):
		fail("%d %s lit on entering, expected %s" % [lit, part, OS.get_environment("BAR_LIT")])
		return
	var spell := OS.get_environment("BAR_SPELL")
	if spell != "" and not await cast_and_check(ui, int(spell), part, region, dir):
		return
	print("PASS: class bar for ", character)
	quit(0)

## Points whose `part` is drawn (visible, alpha above zero), counted from the left.
func lit_points(ui: Node, part: String) -> int:
	var count := 0
	for index in range(8):
		var texture := ui.find_child("PlayerSecondaryResourcePip%d%s" % [index, part], true, false) as Control
		if texture != null and texture.is_visible_in_tree() and texture.modulate.a * texture.self_modulate.a > 0.0:
			count += 1
	return count

func cast_and_check(ui: Node, spell: int, part: String, region: Rect2, dir: String) -> bool:
	var enemy := OS.get_environment("BAR_ENEMY")
	var seen := {}
	for attempt in range(150):
		var name := str(client.target_state().target_name)
		seen[name] = true
		if name.contains(enemy):
			break
		await press(KEY_TAB)
		await wait_frames(10)
	if not str(client.target_state().target_name).contains(enemy):
		fail("Could not target %s among %s" % [enemy, seen.keys()])
		return false
	var slot: int = client.spells_state().bar.find(spell)
	if slot < 0 or slot >= BAR_KEYS.size():
		fail("Spell %d not on the first bar row: %s" % [spell, client.spells_state().bar])
		return false
	var want := int(OS.get_environment("BAR_LIT_AFTER"))
	for attempt in range(6):
		var deadline := Time.get_ticks_msec() + 5000
		while Time.get_ticks_msec() < deadline and int(client.spells_state().gcd_ms) > 0:
			await process_frame
		await press(BAR_KEYS[slot])
		deadline = Time.get_ticks_msec() + 4000
		while Time.get_ticks_msec() < deadline and lit_points(ui, part) != want:
			await process_frame
		if lit_points(ui, part) == want:
			break
		# Out of range or a miss: walk toward the target and retry.
		await walk_forward(0.6)
	if lit_points(ui, part) != want:
		fail("%d %s lit after casting %d, expected %d: %s" % [lit_points(ui, part), part, spell, want, client.target_state()])
		return false
	await wait_real(0.25)
	await capture(dir + "/%s-1-animating.png" % character, region)
	await wait_real(1.5)
	await capture(dir + "/%s-1.png" % character, region)
	if lit_points(ui, part) != want:
		fail("%d %s lit once settled, expected %d" % [lit_points(ui, part), part, want])
		return false
	return true

func walk_forward(seconds: float) -> void:
	push_key(KEY_W, true)
	await wait_real(seconds)
	push_key(KEY_W, false)
	await wait_frames(5)

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
	deadline = Time.get_ticks_msec() + 300000
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
