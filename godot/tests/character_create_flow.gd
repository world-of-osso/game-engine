extends SceneTree

# Real-server character creation driven through the shared original rules: the
# authored race backdrop and selected character scene, race selection, Customize mode, authored random names, the empty-name error (no request
# is sent, so the shared server gains no character) and Back navigation.

var client: Node

func _initialize() -> void:
	call_deferred("run")

func fail(message: String) -> void:
	push_error(message)
	client.queue_free()
	quit(1)

func press(ui: Node, name: String) -> bool:
	var button = ui.find_child(name, true, false)
	if button == null:
		return false
	button.emit_signal("pressed")
	await process_frame
	await process_frame
	return true

func shown(ui: Node, name: String) -> bool:
	var node = ui.find_child(name, true, false)
	return node != null and node.is_visible_in_tree()

func scene_node(name: String) -> Node:
	return client.get_node_or_null("CharacterCreateScene/" + name)

func character_meshes() -> int:
	var character = scene_node("CreationCharacter")
	if character == null:
		return 0
	return character.find_children("*", "MeshInstance3D", true, false).size()

func run() -> void:
	root.size = Vector2i(1280, 720)
	var server = OS.get_environment("GODOT_TEST_SERVER")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if server.is_empty():
		fail("GODOT_TEST_SERVER must select a local server")
		return
	var error = client.connect_account(server, "admin", "admin", false)
	if error != "":
		fail(error)
		return
	var deadline = Time.get_ticks_msec() + 15000
	while client.account_state().screen != "CharacterSelect":
		if Time.get_ticks_msec() > deadline:
			fail("Timed out waiting for character selection")
			return
		await process_frame
	await press(client.get_node("CharacterSelectUI"), "CreateChar")
	var ui = client.get_node_or_null("CharacterCreateUI")
	if ui == null or client.account_state().screen != "CharacterCreate":
		fail("Create New Character must open creation")
		return
	if not shown(ui, "Race_1_Selected") or shown(ui, "Race_3_Selected"):
		fail("Creation must start on the original default Human")
		return
	# ChrRaces.CreateScreenFileDataID: Human and Dwarf 623712, Orc 623714.
	var camera = scene_node("Camera")
	if scene_node("CharCreateBackdrop_623712") == null or camera == null or not camera.current:
		fail("Human creation must show the authored backdrop through its camera")
		return
	var human = scene_node("CreationCharacter")
	if character_meshes() == 0:
		fail("Creation must show the selected character model")
		return
	await press(ui, "Race_3")
	if shown(ui, "Race_1_Selected") or not shown(ui, "Race_3_Selected"):
		fail("Race click must move the selection ring to Dwarf")
		return
	if scene_node("CreationCharacter") == human or character_meshes() == 0:
		fail("Dwarf selection must replace the displayed character")
		return
	if scene_node("CharCreateBackdrop_623712") == null:
		fail("Dwarf must keep the shared Alliance backdrop")
		return
	await press(ui, "Race_2")
	if scene_node("CharCreateBackdrop_623714") == null or scene_node("CharCreateBackdrop_623712") != null:
		fail("Orc selection must swap to the Horde backdrop")
		return
	await press(ui, "Race_3")
	await press(ui, "CharCreateNext")
	var input = ui.find_child("CharCreateNameInput", true, false)
	if input == null or not input.is_visible_in_tree():
		fail("Customize must show the name input")
		return
	if await press(ui, "CharCreateRandomName") and input.text.is_empty():
		fail("Random name must fill the name input from NameGen.csv")
		return
	input.text = ""
	input.text_changed.emit("")
	await process_frame
	await press(ui, "CharCreateButton")
	if ui.frame_text("CharCreateError") != "Please enter a name" or not shown(ui, "CharCreateError"):
		fail("Creating without a name must show the original error: " + ui.frame_text("CharCreateError"))
		return
	await press(ui, "CharCreateBack")
	if shown(ui, "CharCreateNameInput") or not shown(ui, "Race_3_Selected"):
		fail("Back from Customize must return to race/class with the selection kept")
		return
	await press(ui, "CharCreateBack")
	if client.account_state().screen != "CharacterSelect":
		fail("Back from race/class must return to character selection")
		return
	if client.get_node_or_null("CharacterCreateScene") != null:
		fail("Leaving creation must remove its scene")
		return
	print("PASS: creation backdrop/character scene, race selection, customize, random name, empty-name error and back")
	client.queue_free()
	quit(0)
