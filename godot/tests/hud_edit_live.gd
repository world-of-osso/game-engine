extends SceneTree
# Private-server process fixture. All edits use native key/mouse events, never model writes.
# GODOT_HUDEDIT_RUN: owned evidence directory; GODOT_HUDEDIT_PHASE: edit | relog.
# GODOT_HUDEDIT_SERVER / ACCOUNT / PASSWORD: explicit disposable private endpoint.
var client: Node
var directory := OS.get_environment("GODOT_HUDEDIT_RUN")
var phase := OS.get_environment("GODOT_HUDEDIT_PHASE")
var failed := false

func _initialize() -> void:
	call_deferred("run_fixture")

func fail(message: String) -> void:
	failed = true
	push_error("HUD edit fixture: " + message)
	quit(1)

func control(name: String) -> Control:
	return client.find_child(name, true, false) as Control

func rectangle(name: String) -> Array:
	var node := control(name)
	if node == null:
		return []
	var rect := node.get_global_rect()
	return [rect.position.x, rect.position.y, rect.size.x, rect.size.y]

func write_json(name: String, value: Dictionary) -> void:
	var file := FileAccess.open(directory.path_join(name + ".json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(value, "\t"))
	file.close()

func capture(name: String) -> void:
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(directory.path_join(name + ".png"))
	write_json(name, {"account": client.account_state(), "player": rectangle("PlayerFrame"), "chat": rectangle("ChatFrame1"), "editor": client.get_node_or_null("HudEditUI") != null})

func key(code: int) -> void:
	for down in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = down
		root.push_input(event, true)
		await process_frame
	await process_frame

func motion(point: Vector2, relative: Vector2, down: bool) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	event.relative = relative
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
	root.push_input(event, true)
	await process_frame

func button(point: Vector2, down: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
	event.pressed = down
	root.push_input(event, true)
	await process_frame

func click(name: String) -> void:
	var node := control(name)
	if node == null:
		fail("missing click control " + name)
		return
	var point := node.get_global_rect().get_center()
	await motion(point, Vector2.ZERO, false)
	await button(point, true)
	await button(point, false)
	await process_frame

func drag(name: String, delta: Vector2) -> void:
	var node := control(name)
	if node == null:
		fail("missing drag root " + name)
		return
	var start := node.get_global_rect().position + Vector2(20, 20)
	await motion(start, Vector2.ZERO, false)
	await button(start, true)
	for step in range(1, 9):
		await motion(start + delta * float(step) / 8.0, delta / 8.0, true)
	await button(start + delta, false)
	await process_frame

func equal_rect(a: Array, b: Array) -> bool:
	if a.size() != 4 or b.size() != 4:
		return false
	for axis in range(4):
		if abs(float(a[axis]) - float(b[axis])) > 0.1:
			return false
	return true

func run_fixture() -> void:
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().get("assets_starting", true) and Time.get_ticks_msec() < deadline:
		await process_frame
	var error: String = client.connect_account(OS.get_environment("GODOT_HUDEDIT_SERVER"), OS.get_environment("GODOT_HUDEDIT_ACCOUNT"), OS.get_environment("GODOT_HUDEDIT_PASSWORD"), false)
	if not error.is_empty():
		fail(error)
		return
	deadline = Time.get_ticks_msec() + 180000
	while client.account_state().screen != "CharacterSelect" and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.get_node_or_null("CharacterSelectUI") == null:
		fail("character select timeout: " + str(client.account_state()))
		return
	await click("CharCard_0")
	await click("EnterWorld")
	deadline = Time.get_ticks_msec() + 180000
	while (client.account_state().screen != "InWorld" or control("PlayerFrame") == null or control("ChatFrame1") == null) and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.account_state().screen != "InWorld" or control("PlayerFrame") == null or control("ChatFrame1") == null:
		fail("in-world HUD timeout: " + str(client.account_state()))
		return
	for frame in range(20):
		await process_frame
	if phase == "relog":
		await prove_relog()
	else:
		await prove_edits()
	if not failed:
		print("PASS: HUD edit ", phase)
		quit(0)

func prove_edits() -> void:
	await capture("live-before")
	var before_player := rectangle("PlayerFrame")
	var before_chat := rectangle("ChatFrame1")
	await key(KEY_F10)
	if control("EditModeSelection_player_frameLabel") == null or control("EditModeSelection_chat_frameLabel") == null:
		fail("F10 did not mount labelled player/chat movers")
		return
	await capture("live-edit-mode")
	await drag("PlayerFrame", Vector2(260, -180))
	await drag("ChatFrame1", Vector2(64, -100))
	var moved_player := rectangle("PlayerFrame")
	var moved_chat := rectangle("ChatFrame1")
	if equal_rect(before_player, moved_player) or equal_rect(before_chat, moved_chat):
		fail("native drag did not move both roots")
		return
	await click("EditModeManagerFrameSave")
	await key(KEY_ESCAPE)
	if client.get_node_or_null("HudEditUI") != null:
		fail("Escape did not exit edit mode")
		return
	if not equal_rect(moved_player, rectangle("PlayerFrame")) or not equal_rect(moved_chat, rectangle("ChatFrame1")):
		fail("Save then Escape did not retain moved frames")
		return
	write_json("expected-placements", {"player": moved_player, "chat": moved_chat, "authored_player": before_player, "authored_chat": before_chat})
	await capture("live-saved")
	# Reset must restore authored position, not bake the previously applied override.
	await key(KEY_F10)
	await click("EditModeSelection_player_frameLabel")
	await click("EditModeManagerFrameReset")
	if not equal_rect(before_player, rectangle("PlayerFrame")) or not equal_rect(moved_chat, rectangle("ChatFrame1")):
		fail("Reset Selected changed another frame or failed authored restoration")
		return
	await capture("live-reset-draft")
	await key(KEY_ESCAPE)
	if not equal_rect(moved_player, rectangle("PlayerFrame")):
		fail("Escape did not discard unsaved reset")
		return
	await capture("live-final-saved")

func prove_relog() -> void:
	var expected: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(directory.path_join("expected-placements.json")))
	if not equal_rect(expected.player, rectangle("PlayerFrame")) or not equal_rect(expected.chat, rectangle("ChatFrame1")):
		fail("fresh-process relog did not restore both saved placements")
		return
	if client.get_node_or_null("HudEditUI") != null:
		fail("relog unexpectedly entered edit mode")
		return
	await capture("live-relog")
	write_json("persistence-proof", {"passed": true, "phase": "fresh-process-relog", "player": rectangle("PlayerFrame"), "chat": rectangle("ChatFrame1"), "expected": expected, "character_id": client.account_state().selected_character_id})
