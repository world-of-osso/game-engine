extends SceneTree

## Hunter Tame Beast, Dismiss Pet and Call Pet against a private server
## (game-server docs/specs/hunter-taming.md). Environment:
##   GODOT_TEST_SERVER          server address (a private test server)
##   TAME_ACCOUNT / TAME_CHARACTER  account (password fbtest) and hunter (level 5+) placed
##                              near a Diseased Young Wolf in Northshire
##   TAME_PHASE                 "tame": Tab to the wolf, Tame Beast (6 s channel), the pet
##                              follows. "dismiss" (relogged): the pet is back, Dismiss Pet.
##                              "call" (relogged again): no pet out, Call Pet 1 brings the
##                              wolf back and it follows.
##   TAME_SHOTS                 screenshot directory
## Spells not on the main bar are cast from the spellbook. The pet follows the server's
## hunter, so the catch-up check measures against `local_server_position`. Live run
## 2026-10-02 (data/diagnostics/tamebeast-2026-10-02): level-60 hunters at -9281 96.7 68.5,
## where nearby Mangy Wolves were killed through the admin socket as they attacked.

const PASSWORD := "fbtest"
const TAME_BEAST := 1515
const CALL_PET_1 := 883
const DISMISS_PET := 2641
const WOLF := "Diseased Young Wolf"
const BAR_KEYS := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]
## A pet out stands within this of its owner (both combat reaches + PET_FOLLOW_DIST, + slack).
const PET_REACH := 6.0

var client: Node
var shots := "/tmp/claude/tame-beast-live/"
var character := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("TAME_ACCOUNT")
	character = OS.get_environment("TAME_CHARACTER")
	var phase := OS.get_environment("TAME_PHASE")
	if server == "" or account == "" or character == "" or not phase in ["tame", "dismiss", "call"]:
		fail("GODOT_TEST_SERVER, TAME_ACCOUNT, TAME_CHARACTER and TAME_PHASE=tame|call are required")
		return
	if OS.get_environment("TAME_SHOTS") != "":
		shots = OS.get_environment("TAME_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not await wait_for(func(s): return s.catalog_ready and s.known.has(TAME_BEAST) and s.known.has(CALL_PET_1) and s.known.has(DISMISS_PET), 60000, "hunter knowing Tame Beast, Call Pet 1 and Dismiss Pet"):
		return
	if not await wait_until(func(): return client.account_state().local_player_health > 0.0, 60000, "a living hunter"):
		return
	await wait_frames(120)
	print("FIXTURE BAR ", spells().bar)
	if phase == "tame":
		await tame()
	elif phase == "dismiss":
		await dismiss()
	else:
		await call_back()

func tame() -> void:
	for attempt in range(12):
		await press(KEY_TAB)
		await wait_frames(6)
		if str(client.target_state().target_name) == WOLF:
			break
	if str(client.target_state().target_name) != WOLF:
		fail("Tab did not reach a %s: %s" % [WOLF, client.target_state()])
		return
	var wild = client.target_state().target
	print("FIXTURE WILD_WOLF ", wild, " ", client.nameplate_rules(wild))
	await capture("01-wolf-targeted.png")
	var pressed_at := Time.get_ticks_msec()
	if not await press_spell(TAME_BEAST):
		return
	if not await wait_for(func(s): return s.casting == TAME_BEAST, 30000, "Tame Beast channel"):
		return
	print("FIXTURE CHANNEL seen %d ms after the click" % (Time.get_ticks_msec() - pressed_at))
	await capture("02-tame-beast-channel.png")
	if not await wait_for(func(s): return s.casting == 0, 30000, "Tame Beast channel end"):
		return
	var pet := await wait_pet(30000, "the tamed wolf next to the hunter")
	if pet == null:
		return
	print("FIXTURE TAMED errors=", spells().errors)
	await capture("03-tamed-pet.png")
	if not await follows(pet, "04-pet-follows.png"):
		return
	print("FIXTURE TAME_PHASE_DONE")
	client.free()
	quit(0)

## After relogging with the pet out: it is back at login; Dismiss Pet (3 s) removes it.
func dismiss() -> void:
	var pet := await wait_pet(30000, "the pet out at logout back at login")
	if pet == null:
		return
	await capture("05-relogged-pet-back.png")
	if not await press_spell(DISMISS_PET):
		return
	if not await wait_until(func(): return pet_near() == null, 60000, "the pet gone after Dismiss Pet"):
		return
	print("FIXTURE DISMISSED errors=", spells().errors)
	await capture("06-pet-dismissed.png")
	print("FIXTURE DISMISS_PHASE_DONE")
	client.free()
	quit(0)

func call_back() -> void:
	await wait_real(3.0)
	if pet_near() != null:
		fail("A pet is out after relogging with it dismissed")
		return
	await capture("07-relogged-no-pet.png")
	if not await press_spell(CALL_PET_1):
		return
	var pet := await wait_pet(30000, "Call Pet 1 bringing the wolf")
	if pet == null:
		return
	await capture("08-called-pet.png")
	if not await follows(pet, "09-called-pet-follows.png"):
		return
	print("FIXTURE CALL_PHASE_DONE")
	client.free()
	quit(0)

func server_hunter():
	return client.account_state().local_server_position

func player() -> Node3D:
	return client.get_node("WorldUnits/" + character) as Node3D

## A unit named after the wolf, friendly to the hunter (its faction), within `PET_REACH`.
func pet_near() -> Node3D:
	var me := player()
	for child in client.get_node("WorldUnits").get_children():
		var unit := child as Node3D
		if unit != null and str(unit.get_meta("unit_name", "")) == WOLF and unit.global_position.distance_to(me.global_position) <= PET_REACH and reaction(unit) == "Friendly":
			return unit
	return null

func reaction(unit: Node3D) -> String:
	var pick := unit.find_child("UnitPick", true, false)
	if pick == null:
		return ""
	return str(client.nameplate_rules(pick.get_meta("unit_server_id")).get("reaction", ""))

func wait_pet(timeout_ms: int, what: String) -> Node3D:
	if not await wait_until(func(): return pet_near() != null, timeout_ms, what):
		return null
	var pet := pet_near()
	print("FIXTURE PET %s at %.1f yd" % [pet.name, pet.global_position.distance_to(player().global_position)])
	return pet

## Run forward 2 s: the pet runs after the hunter and catches up within `PET_REACH`.
func follows(pet: Node3D, shot: String) -> bool:
	var start := player().global_position
	var pet_start := pet.global_position
	push_key(KEY_W, true)
	await wait_real(2.0)
	push_key(KEY_W, false)
	var moved := player().global_position.distance_to(start)
	print("FIXTURE FOLLOW hunter moved %.1f yd, pet %.1f yd behind" % [moved, pet.global_position.distance_to(player().global_position)])
	if moved < 5.0:
		fail("The hunter did not run: %.1f yd" % moved)
		return false
	# The server's hunter is the one the pet follows.
	if not await wait_until(func(): return server_hunter() != null and pet.global_position.distance_to(server_hunter()) <= PET_REACH, 30000, "the pet catching up"):
		return false
	print("FIXTURE CAUGHT_UP pet ran %.1f yd, %.1f yd from the server's hunter %s (client hunter %s)" % [pet.global_position.distance_to(pet_start), pet.global_position.distance_to(server_hunter()), server_hunter(), player().global_position])
	await capture(shot)
	return true

func spells() -> Dictionary:
	return client.spells_state()

## Its main bar key, else a click on its spellbook button (paging through the book).
func press_spell(spell: int) -> bool:
	var slot: int = spells().bar.find(spell)
	if slot >= 0:
		await press(BAR_KEYS[slot])
		return true
	await press(KEY_P)
	if not await wait_until(func(): return spells().spellbook_open, 3000, "spellbook"):
		return false
	await wait_frames(10)
	var button := book_button(spell)
	for page in range(8):
		if button != null:
			break
		await click(client.get_node("SpellBookUI").find_child("SpellBookNextPageButton", true, false))
		await wait_frames(10)
		button = book_button(spell)
	if button == null:
		fail("Spell %d is neither on the main bar %s nor in the spellbook %s" % [spell, spells().bar, spells().spellbook])
		return false
	await click(button)
	await press(KEY_P)
	return true

func book_button(spell: int) -> Control:
	var ui := client.get_node_or_null("SpellBookUI")
	return ui.find_child("SpellBookItem%dButton" % spell, true, false) as Control if ui != null else null

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
	fail("Timed out waiting for %s: spells=%s target=%s" % [what, spells(), client.target_state()])
	return false

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	var ui = null
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		ui = client.get_node_or_null("CharacterSelectUI")
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 and ui != null:
			break
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	for card in range(client.account_state().character_count):
		await click(ui.find_child("CharCard_%d" % card, true, false))
		if selected.text == character:
			break
	if selected.text != character:
		fail("No character card is %s" % character)
		return false
	await click(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + 300000
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

func click(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
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
