extends SceneTree

## Hunter pet frame and pet action bar against a private server (game-server
## pet_control.rs, docs/specs/pet-bar.md). Environment:
##   GODOT_TEST_SERVER      server address (a private test server)
##   PET_ACCOUNT / PET_CHARACTER  account (password fbtest) and a hunter whose tamed wolf
##                          was out at logout, standing near other creatures
##   PET_SHOTS              screenshot directory
## Login brings the wolf back with its bar; Ctrl-1 (BONUSACTIONBUTTON1, Attack) sends it at
## the Tab target; a click on PetActionButton2 (Follow) brings it back.

const PASSWORD := "fbtest"
const COMMAND_FOLLOW := 1
const PET_REACH := 6.0
const MELEE := 6.0

var client: Node
var shots := "/tmp/claude/pet-bar-live/"
var character := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("PET_ACCOUNT")
	character = OS.get_environment("PET_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, PET_ACCOUNT and PET_CHARACTER are required")
		return
	if OS.get_environment("PET_SHOTS") != "":
		shots = OS.get_environment("PET_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not await wait_until(func(): return client.account_state().local_player_health > 0.0, 60000, "a living hunter"):
		return
	if not await wait_until(func(): return bar().get("shown", false) and bar().get("local_pet") != null and bar().local_pet == bar().pet, 60000, "the pet bar of the hunter's own pet"):
		return
	await wait_frames(60)
	print("FIXTURE BAR ", bar())
	print("FIXTURE PORTRAIT ", client.unit_portrait_state("PetPortrait"))
	await capture("01-pet-frame-and-bar.png")
	if not await attack():
		return
	if not await follow():
		return
	print("FIXTURE PET_BAR_DONE")
	client.free()
	quit(0)

## Tab to a creature that is not the pet, then Ctrl-1: the wolf runs to it and bites.
func attack() -> bool:
	var pet: int = bar().pet
	for attempt in range(12):
		await press(KEY_TAB)
		await wait_frames(6)
		var target = client.target_state().target
		if target != null and int(target) != pet:
			break
	var target = client.target_state().target
	if target == null or int(target) == pet:
		fail("Tab found no target: %s" % client.target_state())
		return false
	print("FIXTURE TARGET ", client.target_state().target_name, " ", target)
	await press_ctrl(KEY_1)
	if not await wait_until(func(): return bar().get("pet_in_combat", false), 10000, "the Attack order (PET_IN_COMBAT)"):
		return false
	print("FIXTURE ATTACK_SENT ", bar().sent)
	if not await wait_until(func(): return unit_node(pet) != null and unit_node(int(target)) != null and unit_node(pet).global_position.distance_to(unit_node(int(target)).global_position) <= MELEE, 30000, "the wolf at its target"):
		return false
	await wait_real(2.5)
	print("FIXTURE ATTACKING bar=", bar(), " target=", client.target_state())
	await capture("02-wolf-attacks-target.png")
	return true

## Click Follow: the wolf stops attacking and comes back to the hunter.
func follow() -> bool:
	var pet: int = bar().pet
	var ui := client.get_node("PetActionBarUI")
	await click(ui.find_child("PetActionButton2", true, false))
	if not await wait_until(func(): return bar().command_state == COMMAND_FOLLOW and not bar().pet_in_combat, 10000, "Follow (no PET_IN_COMBAT)"):
		return false
	if not await wait_until(func(): return unit_node(pet) != null and unit_node(pet).global_position.distance_to(player().global_position) <= PET_REACH, 30000, "the wolf back at the hunter"):
		return false
	await wait_real(1.0)
	print("FIXTURE FOLLOWED bar=", bar(), " gap=%.1f" % unit_node(pet).global_position.distance_to(player().global_position))
	await capture("03-wolf-follows-back.png")
	return true

func bar() -> Dictionary:
	return client.pet_bar_state()

func player() -> Node3D:
	return client.get_node("WorldUnits/" + character) as Node3D

func unit_node(server_id: int) -> Node3D:
	for child in client.get_node("WorldUnits").get_children():
		var pick := child.find_child("UnitPick", true, false)
		if pick != null and int(pick.get_meta("unit_server_id", -1)) == server_id:
			return child as Node3D
	return null

func wait_until(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("Timed out waiting for %s: bar=%s target=%s" % [what, bar(), client.target_state()])
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
	push_key(code, true, false)
	await wait_frames(2)
	push_key(code, false, false)
	await wait_frames(2)

## Ctrl held around `code`, as a player presses Ctrl-1.
func press_ctrl(code: Key) -> void:
	push_key(KEY_CTRL, true, true)
	await wait_frames(2)
	push_key(code, true, true)
	await wait_frames(2)
	push_key(code, false, true)
	await wait_frames(2)
	push_key(KEY_CTRL, false, false)
	await wait_frames(2)

func push_key(code: Key, pressed: bool, ctrl: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	event.ctrl_pressed = ctrl
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
