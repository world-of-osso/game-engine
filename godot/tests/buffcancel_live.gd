extends SceneTree

## Real native input and replicated removal on a private server only.
## GODOT_TEST_SERVER, AURA_ACCOUNT, AURA_CHARACTER, AURA_SHOTS are required.
const SPELL := 1459
const BAR_KEYS := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]
var client: Node
var shots: String
var failed := false

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	shots = OS.get_environment("AURA_SHOTS")
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("AURA_ACCOUNT")
	if endpoint != "127.0.0.1:5298" or account != "fb_buffcancel" or shots.is_empty():
		fail("Private buffcancel endpoint/account/shots required")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error: String = client.connect_account(endpoint, account, "fbtest", false)
	if not error.is_empty():
		fail(error)
		return
	if not await wait_until(func(): return client.account_state().screen == "CharacterSelect", 60000, "character select"):
		return
	var select := client.get_node("CharacterSelectUI")
	await click(select.find_child("CharCard_0", true, false), MOUSE_BUTTON_LEFT)
	var selected: String = select.find_child("CharSelectCharacterName", true, false).text
	if selected != OS.get_environment("AURA_CHARACTER"):
		fail("Unexpected character " + selected)
		return
	await click(select.find_child("EnterWorld", true, false), MOUSE_BUTTON_LEFT)
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and client.account_state().local_server_position != null, 180000, "in world"):
		return
	if not await wait_until(func(): return client.spells_state().catalog_ready and client.spells_state().known.has(SPELL), 60000, "Arcane Intellect catalog"):
		return
	await frames(60)
	var slot: int = client.spells_state().bar.find(SPELL)
	if slot < 0:
		fail("Arcane Intellect missing from bar " + str(client.spells_state()))
		return
	await press(BAR_KEYS[slot])
	if not await wait_until(func(): return buff() != null, 10000, "replicated self buff"):
		return
	var entry: Dictionary = buff()
	if entry.texture_fdid != 135932 or not entry.visible:
		fail("Wrong buff icon " + str(entry))
		return
	var point := Vector2(entry.rect[0] + entry.rect[2] / 2.0, entry.rect[1] + entry.rect[3] / 2.0)
	await move_mouse(point)
	if not await wait_until(func(): return client.tooltip_state().visible and client.tooltip_state().title == "Arcane Intellect", 5000, "native aura tooltip"):
		return
	var tooltip: Dictionary = client.tooltip_state()
	if tooltip.lines.size() < 2 or not str(tooltip.lines[-1]).contains("remaining") or not str(tooltip.lines[0]).contains("Intellect"):
		fail("Missing aura description/time " + str(tooltip))
		return
	print("BUFFCANCEL HOVER ", tooltip)
	await capture("01-hover")
	await move_mouse(Vector2(960, 540))
	if not await wait_until(func(): return not client.tooltip_state().visible, 5000, "tooltip leave"):
		return
	await move_mouse(point)
	# Left release and right press must not cancel. Right release must.
	await mouse(point, MOUSE_BUTTON_LEFT, true)
	await mouse(point, MOUSE_BUTTON_LEFT, false)
	await frames(10)
	if buff() == null:
		fail("Left click cancelled aura")
		return
	await mouse(point, MOUSE_BUTTON_RIGHT, true)
	await frames(20)
	if buff() == null:
		fail("Right press cancelled before RightButtonUp")
		return
	await capture("02-right-held")
	await mouse(point, MOUSE_BUTTON_RIGHT, false)
	if not await wait_until(func(): return buff() == null, 5000, "server-authoritative aura removal"):
		return
	if not await wait_until(func(): return not client.tooltip_state().visible, 5000, "removed aura tooltip hides"):
		return
	print("BUFFCANCEL REMOVED ", client.aura_state())
	await capture("03-cancelled")
	print("BUFFCANCEL LIVE PASS")
	client.free()
	quit(0)

func buff():
	for entry in client.aura_state().buffs:
		if entry.spell_id == SPELL:
			return entry
	return null

func wait_until(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("Timed out " + what + ": " + str(client.account_state()))
	return false

func press(key: Key) -> void:
	for down in [true, false]:
		var event := InputEventKey.new()
		event.keycode = key
		event.physical_keycode = key
		event.pressed = down
		root.push_input(event, true)
		await frames(2)

func move_mouse(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await frames(3)

func mouse(point: Vector2, button: MouseButton, down: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = button
	event.pressed = down
	root.push_input(event, true)
	await frames(3)

func click(control: Control, button: MouseButton) -> void:
	var point := control.get_global_rect().get_center()
	await move_mouse(point)
	await mouse(point, button, true)
	await mouse(point, button, false)

func capture(name: String) -> void:
	await RenderingServer.frame_post_draw
	var error := root.get_texture().get_image().save_png(shots.path_join(name + ".png"))
	if error != OK:
		fail("Capture failed " + str(error))
	var file := FileAccess.open(shots.path_join(name + ".json"), FileAccess.WRITE)
	file.store_string(JSON.stringify({"auras": client.aura_state(), "tooltip": client.tooltip_state(), "account": client.account_state()}, "\t"))

func frames(count: int) -> void:
	for _frame in range(count):
		await process_frame

func fail(message: String) -> void:
	failed = true
	push_error(message)
	quit(1)
