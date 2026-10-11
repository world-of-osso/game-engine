extends SceneTree
# Private UDP5860 fixture; fb_* credentials supplied by the owned run environment.
# Real auth/roster/native input. No fake cooldown/aura/charge assignment.
var client: Node
var directory := OS.get_environment("COOLDOWN_ASSIST_RUN")
var failed := false

func _initialize() -> void:
	call_deferred("run_fixture")

func fail(message: String) -> void:
	failed = true
	push_error("Cooldown assist fixture: " + message)
	quit(1)

func control(name: String) -> Control:
	return client.find_child(name, true, false) as Control

func key(code: int) -> void:
	for down in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = down
		root.push_input(event, true)
		await process_frame
	await process_frame

func click(name: String) -> void:
	var node := control(name)
	if node == null or not node.is_visible_in_tree():
		fail("missing visible control " + name)
		return
	var point := node.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = down
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
		root.push_input(event, true)
		await process_frame
	await process_frame

func write_json(name: String, value: Dictionary) -> void:
	var file := FileAccess.open(directory.path_join(name + ".json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(value, "\t"))
	file.close()

func capture(name: String) -> void:
	for frame in range(20):
		await process_frame
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(directory.path_join(name + ".png"))
	write_json(name, {"account": client.account_state(), "spells": client.spells_state()})

func wait_screen(screen: String) -> bool:
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().get("screen", "") != screen and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.account_state().get("screen", "") != screen:
		fail("screen timeout " + screen + ": " + str(client.account_state()))
		return false
	return true

func run_fixture() -> void:
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().get("assets_starting", true) and Time.get_ticks_msec() < deadline:
		await process_frame
	var error: String = client.connect_account("127.0.0.1:5860", "fb_cooldown_assist", "fbtest", false)
	if not error.is_empty():
		fail(error)
		return
	if not await wait_screen("CharacterSelect"):
		return
	await click("CharCard_0")
	await click("EnterWorld")
	if not await wait_screen("InWorld"):
		return
	await capture("world-probe")
	await key(KEY_ESCAPE)
	await click("MenuBtnOptions")
	await click("OptionsTabhud")
	await capture("hud-options-probe")
	print("PASS: cooldown private-world/options probe; feature acceptance still pending")
	quit(0)
