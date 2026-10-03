extends SceneTree

## Hunter pet stance persistence, autocast overlay and hotkeys against a private server
## (game-server docs/specs/pet-control.md, docs/specs/pet-bar.md). Environment:
##   GODOT_TEST_SERVER        server address (a private test server)
##   PET_ACCOUNT / PET_CHARACTER  account (password fbtest) and a hunter whose wolf was out
##                            at logout, standing near a Mangy Wolf
##   PET_PHASE                "fight": the bar shows Ctrl-N hotkeys and Bite's autocast;
##                            Defensive is clicked, then Ctrl-1 sends the wolf at the Tab
##                            target, which it autocasts Bite on (the server's combat log
##                            is checked by the caller). "relog": after relogging the wolf
##                            is back Defensive.
##   PET_SHOTS                screenshot directory
##   PET_SETTLE_SECS          seconds to wait in the world before the fight phase acts

const PASSWORD := "fbtest"
const BITE := 17253
const REACT_DEFENSIVE := 1
const TARGET := "Mangy Wolf"
const MELEE := 6.0

var client: Node
var shots := "/tmp/claude/pet-spells-live/"
var character := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("PET_ACCOUNT")
	character = OS.get_environment("PET_CHARACTER")
	var phase := OS.get_environment("PET_PHASE")
	if server == "" or account == "" or character == "" or not phase in ["fight", "relog"]:
		fail("GODOT_TEST_SERVER, PET_ACCOUNT, PET_CHARACTER and PET_PHASE=fight|relog are required")
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
	if not await wait_until(func(): return bar().get("shown", false) and bar().get("local_pet") != null and bar().local_pet == bar().pet, 60000, "the pet bar of the hunter's own pet"):
		return
	await wait_frames(60)
	print("FIXTURE BAR ", bar())
	if phase == "fight":
		await fight()
	else:
		await relog()

func fight() -> void:
	# PET_SETTLE_SECS: time for the caller to clear creatures that attacked on arrival.
	await wait_real(float(OS.get_environment("PET_SETTLE_SECS")))
	if not await wait_until(func(): return bar().get("shown", false) and bar().get("local_pet") != null, 30000, "the pet still out after settling"):
		return
	var slot: int = bite_slot()
	if slot < 0:
		fail("No Bite on the bar: %s" % bar())
		return
	if int(bar().autocast[slot]) != 2:
		fail("Bite's autocast is not on: %s" % bar())
		return
	var ui := client.get_node("PetActionBarUI")
	for button in range(1, 11):
		var hotkey := ui.find_child("PetActionButton%dHotKey" % button, true, false)
		print("FIXTURE HOTKEY %d '%s' visible=%s" % [button, hotkey.text if hotkey != null else "", hotkey != null and hotkey.is_visible_in_tree()])
	for part in ["", "HotKey", "Icon", "NormalTexture", "AutoCastCorners", "AutoCastShine"]:
		var node := ui.find_child("PetActionButton%d%s" % [slot + 1, part], true, false) as Control
		if node != null:
			print("FIXTURE LAYER %s z=%d relative=%s visible=%s rect=%s class=%s text=%s parent=%s" % [part, node.z_index, node.z_as_relative, node.is_visible_in_tree(), node.get_global_rect(), node.get_class(), node.get("text"), node.get_parent().name])
	var shine := ui.find_child("PetActionButton%dAutoCastShine" % (slot + 1), true, false)
	print("FIXTURE BITE_SHINE visible=", shine != null and shine.is_visible_in_tree())
	await capture("01-bar-hotkeys-autocast.png")
	# Defensive (PetActionButton9).
	await click(ui.find_child("PetActionButton9", true, false))
	if not await wait_until(func(): return int(bar().react_state) == REACT_DEFENSIVE, 10000, "Defensive"):
		return
	print("FIXTURE DEFENSIVE bar=", bar())
	for attempt in range(12):
		await press(KEY_TAB)
		await wait_frames(6)
		if str(client.target_state().target_name) == TARGET:
			break
	if str(client.target_state().target_name) != TARGET:
		fail("Tab did not reach a %s: %s" % [TARGET, client.target_state()])
		return
	var target: int = int(client.target_state().target)
	print("FIXTURE TARGET ", TARGET, " ", target)
	await press_ctrl(KEY_1)
	if not await wait_until(func(): return bar().get("pet_in_combat", false), 10000, "the Attack order (PET_IN_COMBAT)"):
		return
	var pet: int = bar().pet
	if not await wait_until(func(): return unit_node(pet) != null and unit_node(target) != null and unit_node(pet).global_position.distance_to(unit_node(target).global_position) <= MELEE, 30000, "the wolf at its target"):
		return
	print("FIXTURE AT_TARGET ", Time.get_unix_time_from_system())
	# Bite's 3 s cooldown: autocast bites at least twice in 5 s at its target.
	if not await wait_until(func(): return pet_bites(pet).size() >= 2, 10000, "the wolf autocasting Bite twice"):
		return
	await capture("02-wolf-bites-mangy-wolf.png")
	print("FIXTURE BITES ", pet_bites(pet), " target=", client.target_state())
	print("FIXTURE FIGHT_DONE bar=", bar())
	client.free()
	quit(0)

func relog() -> void:
	print("FIXTURE RELOG react_state=", bar().react_state)
	await capture("03-relogged-defensive.png")
	if int(bar().react_state) != REACT_DEFENSIVE:
		fail("The wolf came back %s, not Defensive" % bar().react_state)
		return
	print("FIXTURE RELOG_DONE")
	client.free()
	quit(0)

## Bite damage the owner's combat log got from its pet.
func pet_bites(pet: int) -> Array:
	var bites := []
	for entry in client.damage_meter_state().combat_log:
		if entry.damage and int(entry.source) == pet and int(entry.spell_id) == BITE:
			bites.append(int(entry.amount))
	return bites

func bite_slot() -> int:
	var buttons: PackedInt64Array = bar().buttons
	for slot in range(buttons.size()):
		if (int(buttons[slot]) & 0xFFFFFF) == BITE:
			return slot
	return -1

func bar() -> Dictionary:
	return client.pet_bar_state()

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
