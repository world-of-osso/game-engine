extends SceneTree

# Private-server test: Evasion 5277 resolves authored aura kit 43868 / alpha 0.5.
const SPELL := 5277
var client: Node
var character := ""
var output := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("ALPHA_ACCOUNT")
	character = OS.get_environment("ALPHA_CHARACTER")
	output = OS.get_environment("ALPHA_SHOTS")
	if server.is_empty() or account.is_empty() or character.is_empty() or output.is_empty():
		fail("private server, account, character and output are required")
		return
	DirAccess.make_dir_recursive_absolute(output)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error: String = client.connect_account(server, account, "fbtest", false)
	if not error.is_empty():
		fail("connect: " + error)
		return
	if not await enter_world():
		return
	if not await wait_until(func(): return client.spells_state().known.has(SPELL), 60000, "known Evasion"):
		return
	if not await wait_until(func(): return mesh_transparency() >= 0.0, 60000, "player mesh"):
		return
	var baseline := mesh_transparency()
	if absf(baseline) > 0.001:
		fail("expected opaque player before cast: " + str(baseline))
		return
	await screenshot("before.png")
	await press(KEY_P)
	if not await wait_until(func(): return client.spells_state().spellbook_open, 5000, "spellbook"):
		return
	var icon: Control = client.get_node("SpellBookUI").find_child("SpellBookItem%dButton" % SPELL, true, false)
	if icon == null:
		fail("Evasion spellbook button missing")
		return
	await click(icon)
	await press(KEY_ESCAPE)
	if not await wait_until(func(): return has_aura(), 15000, "replicated Evasion aura"):
		return
	if not await wait_until(func(): return absf(mesh_transparency() - 0.5) < 0.001, 10000, "held alpha 0.5"):
		return
	print("EVASION_ALPHA active aura=true transparency=", mesh_transparency())
	await screenshot("active.png")
	if not await wait_until(func(): return not has_aura(), 30000, "Evasion aura removal"):
		return
	if not await wait_until(func(): return absf(mesh_transparency()) < 0.001, 10000, "restored opacity"):
		return
	print("EVASION_ALPHA removed aura=false transparency=", mesh_transparency())
	await screenshot("after.png")
	print("PASS native Evasion aura alpha and restoration")
	client.free()
	quit(0)

func has_aura() -> bool:
	for aura in client.aura_state().buffs:
		if aura.spell_id == SPELL:
			return true
	return false

func mesh_transparency() -> float:
	var visual: Node = client.get_node_or_null("WorldUnits/" + character + "/NpcVisualRoot")
	if visual == null:
		return -1.0
	for mesh in visual.find_children("*", "MeshInstance3D", true, false):
		if mesh.is_visible_in_tree():
			return mesh.transparency
	return -1.0

func enter_world() -> bool:
	if not await wait_until(func(): return client.account_state().screen == "CharacterSelect" and client.account_state().character_count >= 1 and client.get_node_or_null("CharacterSelectUI") != null, 30000, "character select"):
		return false
	var ui: Node = client.get_node("CharacterSelectUI")
	var enter: Control = ui.find_child("EnterWorld", true, false)
	if not await wait_until(func(): return enter.is_visible_in_tree(), 180000, "character preview"):
		return false
	await click(ui.find_child("CharCard_0", true, false))
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != character:
		fail("unexpected character: " + str(selected.text))
		return false
	await click(enter)
	if not await wait_until(func(): return in_world(client.account_state()), 120000, "in world"):
		return false
	await wait_frames(60)
	return true

func in_world(state: Dictionary) -> bool:
	return state.screen == "InWorld" and state.local_player_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty()

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func wait_until(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("timeout waiting for %s: %s" % [what, client.account_state()])
	return false

func press(code: Key) -> void:
	for down in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = down
		root.push_input(event, true)
		await process_frame

func click(control: Control) -> void:
	if control == null:
		fail("missing control")
		return
	var point := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(2)
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = down
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func screenshot(name: String) -> void:
	await RenderingServer.frame_post_draw
	if root.get_texture().get_image().save_png(output.path_join(name)) != OK:
		fail("screenshot failed: " + name)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
