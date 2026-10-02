extends SceneTree

## Frame times at character select while the campsite and each selected character's
## model load. Environment:
##   GODOT_TEST_SERVER    server address (a private test server)
##   CHARSELECT_ACCOUNT   account (password fbtest) with at least two characters
##   CHARSELECT_FRAME_MS  longest frame allowed (default 100); fixture policy, not a
##                        product budget
## It waits for the first card's character, selects the second card, then the first
## again, and waits each time for that character's model. No frame from the one that
## switched to character select until the last model shows may exceed
## CHARSELECT_FRAME_MS: a model loads off the main thread and appears once it is ready.

const PASSWORD := "fbtest"
const MODEL_WAIT_MS := 120000

var client: Node
var last_usec := 0
var measuring := false
var frames_ms: Array[float] = []
## Frames over the limit: [seconds since measuring started, ms].
var slow: Array = []
var measure_started_usec := 0
var frame_limit := 100.0

func _initialize() -> void:
	call_deferred("run_test")

func _process(_delta: float) -> bool:
	# Wall clock: Godot caps the process delta, so the delta hides a stall.
	var now := Time.get_ticks_usec()
	if not measuring and measure_started_usec == 0 and client != null \
			and client.account_state().screen == "CharacterSelect":
		# The frame that just ended switched to character select.
		measuring = true
		measure_started_usec = last_usec
	if measuring and last_usec > 0:
		var ms := (now - last_usec) / 1000.0
		frames_ms.append(ms)
		if ms > frame_limit:
			slow.append([(now - measure_started_usec) / 1e6, ms])
	last_usec = now
	return false

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("CHARSELECT_ACCOUNT")
	if server == "" or account == "":
		fail("GODOT_TEST_SERVER and CHARSELECT_ACCOUNT are required")
		return
	var limit := OS.get_environment("CHARSELECT_FRAME_MS")
	frame_limit = float(limit) if limit != "" else 100.0
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	var ui = await character_select_ui()
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return
	if ui.find_child("CharCard_1", true, false) == null:
		fail("The account needs at least two characters")
		return
	var shown := []
	for card in [-1, 1, 0]:
		# The first card's character is selected on arrival; its model may already show.
		var previous = selected_model() if card >= 0 else null
		var started := Time.get_ticks_msec()
		if card >= 0:
			await click(ui.find_child("CharCard_%d" % card, true, false))
		var name: String = ui.find_child("CharSelectCharacterName", true, false).text
		var model = await new_model(previous)
		if model == null:
			fail("%s's model did not show within %d ms" % [name, MODEL_WAIT_MS])
			return
		shown.append("%s after %d ms" % [name, Time.get_ticks_msec() - started])
	measuring = false
	frames_ms.sort()
	print("FIXTURE CHARSELECT_PREVIEW shown=%s frames=%d median_ms=%.1f max_ms=%.1f over_%d_ms=%d" % [
		shown, frames_ms.size(), frames_ms[frames_ms.size() / 2], frames_ms.back(),
		int(frame_limit), slow.size()])
	print("FIXTURE SLOW_FRAMES ", slow.map(func(f): return "%.1fs:%.0fms" % f))
	if not slow.is_empty():
		fail("%d character select frames exceeded %.0f ms" % [slow.size(), frame_limit])
		return
	print("FIXTURE CHARSELECT_PREVIEW_DONE")
	client.free()
	quit(0)

func character_select_ui() -> Node:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		# The session reaches character select before CASC startup ends; its UI after.
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 2 \
				and client.get_node_or_null("CharacterSelectUI") != null:
			return client.get_node("CharacterSelectUI")
	return null

func selected_model() -> Node:
	return client.find_child("SelectedCharacter", true, false)

## The selected character's model once it shows, other than `previous`.
func new_model(previous) -> Node:
	var deadline := Time.get_ticks_msec() + MODEL_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		var model := selected_model()
		if model != null and model != previous:
			return model
		await process_frame
	return null

func click(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await process_frame

func fail(message: String) -> void:
	push_error(message)
	print("FIXTURE FAIL ", message)
	if client != null and is_instance_valid(client):
		client.free()
	quit(1)
