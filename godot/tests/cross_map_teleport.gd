extends SceneTree

## Far teleport against a private server. Environment:
##   GODOT_TEST_SERVER               server address (a private test server)
##   XMAP_ACCOUNT / XMAP_CHARACTER   account (password fbtest) and its character
## Enters the world, prints FIXTURE AT_START, and waits for the orchestrator
## (scripts/agent/cross-map-teleport.sh) to teleport the character to another map.
## Once the new map's terrain is loaded it runs 5 more seconds, then prints
## FIXTURE XMAP_DONE. The orchestrator fails the run on any "[panic" in the log.

const PASSWORD := "fbtest"

var client: Node

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
	var account := OS.get_environment("XMAP_ACCOUNT")
	var character := OS.get_environment("XMAP_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, XMAP_ACCOUNT and XMAP_CHARACTER are required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Connection: " + error)
		return
	if not await wait_state(func(s): return s.reply_received and s.screen == "CharacterSelect" and s.character_count >= 1, 20000, "character select"):
		return
	var ui = client.get_node("CharacterSelectUI")
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	for index in range(client.account_state().character_count):
		if selected.text == character:
			break
		await click(ui.find_child("CharCard_%d" % index, true, false))
	if selected.text != character:
		fail("No roster card shows " + character)
		return
	await click(ui.find_child("EnterWorld", true, false))
	if not await wait_state(func(s): return s.screen == "InWorld" and s.local_player_position != null and s.terrain.pending_count == 0 and not s.terrain.parsed_tiles.is_empty(), 120000, "entering the world"):
		return
	await wait_frames(120)
	var start = client.account_state().local_player_position
	print("FIXTURE AT_START ", start)
	if not await wait_state(func(s): return s.screen == "InWorld" and s.local_player_position != null and s.local_player_position.distance_to(start) > 100.0 and s.terrain.pending_count == 0 and not s.terrain.parsed_tiles.is_empty(), 120000, "the far teleport"):
		return
	var deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
	print("FIXTURE XMAP_DONE at ", client.account_state().local_player_position)
	client.free()
	quit(0)

func wait_state(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(client.account_state()):
			return true
	fail("Timed out waiting for %s: %s" % [what, client.account_state()])
	return false

func wait_frames(count: int) -> void:
	for i in range(count):
		await process_frame

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
