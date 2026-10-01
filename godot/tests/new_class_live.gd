extends SceneTree

## Monk, Demon Hunter and Evoker end to end against a private server
## (docs/specs/character-creation.md, new classes). Environment:
##   GODOT_TEST_SERVER   server address (a private test server)
##   NEWCLASS_ACCOUNT    existing account, password fbtest
##   NEWCLASS_RACE       ChrRaces ID, NEWCLASS_CLASS ChrClasses ID, NEWCLASS_NAME new name
##   NEWCLASS_SPELL      spell to cast at the nearest enemy once teleported
##   NEWCLASS_SHOTS      screenshot directory
##   NEWCLASS_PICK       optional "category:option:choice" picked in Customize; with
##                       NEWCLASS_SKINNED (a collection FDID) the preview must show it
##   NEWCLASS_ZOOM       optional camera zoom-in clicks for an extra Customize capture
##   NEWCLASS_FORMS      set for a Dracthyr: switch the preview to the visage form and back
## Creates the character through the real creation screens (race, class, Customize,
## name, Create), enters the world at its start, prints FIXTURE AT_START and waits
## for the orchestrator to teleport it next to a Northshire Training Dummy. Then Tab
## targets the dummy and the bar key of NEWCLASS_SPELL casts it until it deals damage.

const PASSWORD := "fbtest"
const BAR_KEYS := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]

var client: Node
var shots := "/tmp/claude/newclass-live/"
var tag := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func fail(message: String) -> void:
	push_error(message)
	print("FIXTURE FAIL ", message)
	if client != null:
		client.free()
	quit(1)

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("NEWCLASS_ACCOUNT")
	var race := int(OS.get_environment("NEWCLASS_RACE"))
	var klass := int(OS.get_environment("NEWCLASS_CLASS"))
	var name := OS.get_environment("NEWCLASS_NAME")
	var spell := int(OS.get_environment("NEWCLASS_SPELL"))
	if server == "" or account == "" or race == 0 or klass == 0 or name == "" or spell == 0:
		fail("GODOT_TEST_SERVER, NEWCLASS_ACCOUNT/RACE/CLASS/NAME/SPELL are required")
		return
	if OS.get_environment("NEWCLASS_SHOTS") != "":
		shots = OS.get_environment("NEWCLASS_SHOTS")
	tag = "%s-%d-%d" % [name.to_lower(), race, klass]
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Connection: " + error)
		return
	if not await wait_state(func(s): return s.reply_received and s.screen == "CharacterSelect", 20000, "character select"):
		return
	var count: int = client.account_state().character_count
	if not await create_character(race, klass, name):
		return
	if not await wait_state(func(s): return s.screen == "CharacterSelect" and s.character_count == count + 1, 20000, "the created character in the roster"):
		return
	if not await enter_world(name):
		return
	await capture("%s-1-start.png" % tag)
	var start = client.account_state().local_player_position
	print("FIXTURE AT_START ", start)
	# The orchestrator teleports the character next to a Training Dummy.
	if not await wait_state(func(s): return s.local_player_position != null and s.local_player_position.distance_to(start) > 20.0 and s.terrain.pending_count == 0 and not s.terrain.parsed_tiles.is_empty(), 120000, "the teleport"):
		return
	await wait_frames(120)
	print("FIXTURE AT_DUMMY ", client.account_state().local_player_position)
	if not await target_dummy():
		return
	if not await cast_until_damage(spell):
		return
	await wait_frames(30)
	await capture("%s-3-cast.png" % tag)
	print("FIXTURE NEWCLASS_DONE ", tag, " spells=", client.spells_state())
	client.free()
	quit(0)

func create_character(race: int, klass: int, name: String) -> bool:
	await click(client.get_node("CharacterSelectUI").find_child("CreateChar", true, false))
	var ui = client.get_node_or_null("CharacterCreateUI")
	if ui == null:
		fail("Create New Character did not open creation")
		return false
	for button in ["Race_%d" % race, "Class_%d" % klass]:
		var control = ui.find_child(button, true, false)
		if control == null or not control.is_visible_in_tree() or ("disabled" in control and control.disabled):
			fail("%s is not an enabled creation button" % button)
			return false
		await click(control)
	if not shown(ui, "Race_%d_Selected" % race) or not shown(ui, "Class_%d_Selected" % klass):
		fail("Race %d / class %d not selected" % [race, klass])
		return false
	await wait_frames(90)
	await capture("%s-0-race-class.png" % tag)
	await click(ui.find_child("CharCreateNext", true, false))
	var input = ui.find_child("CharCreateNameInput", true, false)
	if input == null or not input.is_visible_in_tree():
		fail("Customize must show the name input")
		return false
	await wait_frames(60)
	var pick := OS.get_environment("NEWCLASS_PICK")
	if pick != "" and not await pick_choice(ui, pick.split(":")):
		return false
	await capture("%s-0-customize.png" % tag)
	if OS.get_environment("NEWCLASS_FORMS") != "" and not await switch_forms(ui):
		return false
	var zoom := int(OS.get_environment("NEWCLASS_ZOOM"))
	if zoom > 0:
		for i in range(zoom):
			await click(ui.find_child("Camera_zoom_in", true, false))
		await wait_frames(90)
		await capture("%s-0-customize-zoom.png" % tag)
	input.text = name
	input.text_changed.emit(name)
	await wait_frames(2)
	await click(ui.find_child("CharCreateButton", true, false))
	return true

## Dracthyr AlteredForms: Form_1 shows the visage model, Form_0 the dragon again.
func switch_forms(ui: Node) -> bool:
	var dragon = preview_character()
	var dragon_meshes := preview_mesh_count()
	for form in [1, 0]:
		var button = ui.find_child("Form_%d" % form, true, false)
		if button == null or not button.is_visible_in_tree():
			fail("Customize has no visible Form_%d" % form)
			return false
		await click(button)
		await wait_frames(90)
		var shown := "visage" if form == 1 else "dragon"
		if not shown(ui, "Form_%d_Selected" % form) or shown(ui, "Form_%d_Selected" % (1 - form)):
			fail("Form_%d is not the selected form" % form)
			return false
		print("FIXTURE FORM %s meshes=%d" % [shown, preview_mesh_count()])
		if form == 1 and (preview_character() == dragon or preview_mesh_count() == 0):
			fail("The visage form did not replace the dragon preview")
			return false
		await capture("%s-0-%s.png" % [tag, shown])
	if preview_mesh_count() != dragon_meshes:
		fail("Back on the dragon form the preview has %d meshes, not %d" % [preview_mesh_count(), dragon_meshes])
		return false
	return true

func preview_character() -> Node:
	return client.get_node_or_null("CharacterCreateScene/CreationCharacter")

func preview_mesh_count() -> int:
	var character = preview_character()
	return character.find_children("*", "MeshInstance3D", true, false).size() if character != null else 0

## Select Customize category/option/choice through the dropdown, then check the
## preview for the skinned collection model NEWCLASS_SKINNED.
func pick_choice(ui: Node, ids: PackedStringArray) -> bool:
	for button in ["Category_%s" % ids[0], "OptionToggle_%s" % ids[1], "OptionChoice_%s_%s" % [ids[1], ids[2]]]:
		var control = ui.find_child(button, true, false)
		if control == null or not control.is_visible_in_tree():
			fail("Customize has no visible %s" % button)
			return false
		await click(control)
		await wait_frames(10)
	await wait_frames(60)
	var skinned := OS.get_environment("NEWCLASS_SKINNED")
	if skinned != "":
		var character = client.get_node_or_null("CharacterCreateScene/CreationCharacter")
		var model = character.find_child("SkinnedModel" + skinned, true, false) if character != null else null
		var meshes: Array = model.find_children("*", "MeshInstance3D", true, false) if model != null else []
		var shown := meshes.filter(func(mesh): return mesh.visible)
		print("FIXTURE SKINNED ", skinned, " meshes=", meshes.size(), " visible=", shown.size())
		if shown.is_empty():
			fail("Preview shows no SkinnedModel%s mesh" % skinned)
			return false
	return true

func enter_world(name: String) -> bool:
	var ui = client.get_node_or_null("CharacterSelectUI")
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	for index in range(client.account_state().character_count):
		if selected.text == name:
			break
		await click(ui.find_child("CharCard_%d" % index, true, false))
	if selected.text != name:
		fail("No roster card shows " + name)
		return false
	await click(ui.find_child("EnterWorld", true, false))
	if not await wait_state(func(s): return s.screen == "InWorld" and s.local_player_position != null and s.local_server_position != null and s.terrain.pending_count == 0 and not s.terrain.parsed_tiles.is_empty(), 120000, "entering the world"):
		return false
	await wait_frames(120)
	print("FIXTURE IN_WORLD at ", client.account_state().local_player_position, " spells=", client.spells_state())
	return true

func target_dummy() -> bool:
	for attempt in range(20):
		if str(client.target_state().target_name).contains("Training Dummy"):
			return true
		await press(KEY_TAB)
		await wait_frames(10)
	fail("Tab did not reach a Training Dummy: " + str(client.target_state()))
	return false

func cast_until_damage(spell: int) -> bool:
	var slot: int = client.spells_state().bar.find(spell)
	if slot < 0:
		fail("Spell %d is not on the main bar: %s" % [spell, client.spells_state()])
		return false
	var dealt: int = client.spells_state().damage_dealt.size()
	for attempt in range(3):
		if not await wait_spells(func(s): return s.gcd_ms == 0 and s.casting == 0, 8000, "GCD"):
			return false
		await press(BAR_KEYS[slot])
		if attempt == 0:
			await wait_frames(20)
			await capture("%s-2-casting.png" % tag)
		var deadline := Time.get_ticks_msec() + 6000
		while Time.get_ticks_msec() < deadline:
			await process_frame
			if client.spells_state().damage_dealt.size() > dealt:
				print("FIXTURE DAMAGE ", client.spells_state().damage_dealt, " target=", client.target_state())
				return true
	fail("Spell %d dealt no damage: %s" % [spell, client.spells_state()])
	return false

func wait_state(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(client.account_state()):
			return true
	fail("Timed out waiting for %s: %s" % [what, client.account_state()])
	return false

func wait_spells(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(client.spells_state()):
			return true
	fail("Timed out waiting for %s: %s" % [what, client.spells_state()])
	return false

func shown(ui: Node, name: String) -> bool:
	var node = ui.find_child(name, true, false)
	return node != null and node.is_visible_in_tree()

func wait_frames(count: int) -> void:
	for i in range(count):
		await process_frame

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
