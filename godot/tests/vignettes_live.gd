extends SceneTree

## Creature vignettes on the minimap and the world map against a private server.
## Environment:
##   GODOT_TEST_SERVER        server address (a private test server)
##   VIGNETTES_ACCOUNT / VIGNETTES_CHARACTER  account (password fbtest) and its only
##                            character, standing within 100 yd of a vignette creature
##   VIGNETTES_SHOTS          screenshot directory
## Enters the world, waits for a vignette blip on the minimap, opens the world map (M)
## and waits for its vignette pin, capturing both.

const PASSWORD := "fbtest"

var client: Node
var shots := "/tmp/claude/vignettes-live/"
var character := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("VIGNETTES_ACCOUNT")
	character = OS.get_environment("VIGNETTES_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, VIGNETTES_ACCOUNT and VIGNETTES_CHARACTER are required")
		return
	if OS.get_environment("VIGNETTES_SHOTS") != "":
		shots = OS.get_environment("VIGNETTES_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not await wait_until(func(): return client.minimap_state().get("vignettes", 0) >= 1, 30000, "a minimap vignette"):
		return
	print("FIXTURE MINIMAP ", client.minimap_state())
	await capture("01-minimap-vignette.png")
	push_key(KEY_M, true)
	await wait_frames(2)
	push_key(KEY_M, false)
	if not await wait_until(func(): return vignette_pin() != {}, 15000, "a world map vignette pin"):
		return
	print("FIXTURE WORLD_MAP ", client.world_map_state().map_name, " ", vignette_pin())
	await capture("02-world-map-vignette.png")
	for sample in range(10):
		await wait_frames(30)
		print("FIXTURE SAMPLE %d minimap=%s pin=%s units=%s" % [sample, client.minimap_state().get("vignettes", 0), vignette_pin(), unit_names()])
	print("FIXTURE VIGNETTES_LIVE_DONE")
	client.free()
	quit(0)

func unit_names() -> Array:
	var names := []
	var units = client.get_node_or_null("WorldUnits")
	if units != null:
		for unit in units.get_children():
			names.append(str(unit.name))
	return names

func vignette_pin() -> Dictionary:
	var state: Dictionary = client.world_map_state()
	if not state.get("open", false):
		return {}
	for pin in state.get("pins", []):
		if str(pin.type).begins_with("Vignette"):
			return pin
	return {}

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
		if state.screen == "InWorld" and state.local_player_position != null and state.local_server_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD at ", state.local_player_position)
			await wait_frames(60)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func wait_until(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("Timed out waiting for %s: minimap=%s" % [what, client.minimap_state()])
	return false

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

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
		event.button_mask = 1 if pressed else 0
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(shots + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
