extends SceneTree

## Lost server link while spell kit models are attached to units, against a private
## server. Environment:
##   GODOT_TEST_SERVER                       server address (a private test server)
##   LINKLOSS_ACCOUNT / LINKLOSS_CHARACTER   account (password fbtest) and a level-10
##                                           warrior that knows Battle Shout
##   LINKLOSS_ROUNDS                         link losses to survive (default 1)
## Each round casts Battle Shout and, while its kit models are shown on the warrior's
## model, prints FIXTURE LINK_READY <round> and stalls the main thread for STALL_MS while
## the orchestrator (scripts/agent/link-loss.sh) stops the server past the client's link
## timeout. The next frame's account step sees the lost link before spell visuals advance,
## so the kit models are still shown when every unit despawns (freeing them with their
## unit's model) and the world resets in that same frame, as after the 15 s stall in
## data/diagnostics/portraitstill-2026-10-02/red. The client then reconnects into the
## world. After the last round it prints FIXTURE LINKLOSS_DONE. The orchestrator fails the
## run on any "[panic" in the log.

const PASSWORD := "fbtest"
const BATTLE_SHOUT := 6673
## Longer than the orchestrator's 13 s server stop, so the link times out mid-stall.
const STALL_MS := 15000

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
	var account := OS.get_environment("LINKLOSS_ACCOUNT")
	var character := OS.get_environment("LINKLOSS_CHARACTER")
	var rounds := int(OS.get_environment("LINKLOSS_ROUNDS")) if OS.get_environment("LINKLOSS_ROUNDS") != "" else 1
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, LINKLOSS_ACCOUNT and LINKLOSS_CHARACTER are required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Connection: " + error)
		return
	if not await wait_until(func(): return client.get_node_or_null("CharacterSelectUI") != null and client.account_state().character_count >= 1, 60000, "character select"):
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
	for round in range(1, rounds + 1):
		if not await wait_state(in_world, 120000, "the world (round %d)" % round):
			return
		var refused: String = client.use_spell(BATTLE_SHOUT)
		if refused != "":
			fail("Battle Shout: " + refused)
			return
		if not await wait_until(func(): return not client.spell_visuals_state().active.is_empty(), 10000, "an active kit model (round %d)" % round):
			return
		print("FIXTURE LINK_READY %d active=%s" % [round, client.spell_visuals_state().active])
		OS.delay_msec(STALL_MS)
		if not await wait_state(func(s): return s.reconnect_phase != "Inactive", 60000, "the lost link (round %d)" % round):
			return
		print("FIXTURE LINK_LOST %d" % round)
	if not await wait_state(in_world, 120000, "the world after the last reconnect"):
		return
	await wait_frames(60)
	print("FIXTURE LINKLOSS_DONE")
	client.free()
	quit(0)

func in_world(s: Dictionary) -> bool:
	return s.screen == "InWorld" and s.reconnect_phase == "Inactive" and s.local_player_position != null and s.terrain.pending_count == 0 and not s.terrain.parsed_tiles.is_empty()

func wait_state(predicate: Callable, timeout_ms: int, what: String) -> bool:
	return await wait_until(func(): return predicate.call(client.account_state()), timeout_ms, what)

func wait_until(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("Timed out waiting for %s: %s; kits started: %s" % [what, client.account_state(), client.spell_visuals_state().started])
	return false

func click(target: Control) -> void:
	var point := target.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await wait_frames(3)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame
