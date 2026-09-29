extends "res://tests/world_menu_flow.gd"

const SLAM := 1464

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/native-reset-fixture-"):
		fail("Spell-click fixture requires owned UDP endpoint and isolated options")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 15000):
		return
	print("FIXTURE SPELL_CLICK_LOADING")
	if not await wait_world(client):
		return
	var sound := client.get_node_or_null("NativeSound")
	var effects := sound.get_node_or_null("Effects") as AudioStreamPlayer if sound != null else null
	if effects == null:
		fail("Authenticated GameClient has no owned Effects channel")
		return
	var completed := [0]
	effects.finished.connect(func(): completed[0] += 1)
	if not await wait_spells(client):
		return
	var bar := client.get_node_or_null("MainActionBarUI")
	var action := bar.find_child("ActionButton1", true, false) as Control if bar != null else null
	if action == null or not await assert_click(client, effects, completed, action, "action bar"):
		return
	push_key(KEY_P, true)
	await process_frame
	push_key(KEY_P, false)
	if not await wait_book(client):
		return
	var book := client.get_node_or_null("SpellBookUI")
	var icon := book.find_child("SpellBookItem%dButton" % SLAM, true, false) as Control if book != null else null
	if icon == null or not await assert_click(client, effects, completed, icon, "spellbook"):
		return
	if not await assert_quiet_input(effects, completed, icon, MOUSE_BUTTON_RIGHT):
		return
	var before: int = completed[0]
	push_key(KEY_1, true)
	await process_frame
	push_key(KEY_1, false)
	await wait_frames(8)
	if effects.is_playing() or completed[0] != before:
		fail("Keyboard cast played pointer-only effect")
		return
	push_key(KEY_P, true)
	await process_frame
	push_key(KEY_P, false)
	await wait_frames(8)
	push_key(KEY_P, true)
	await process_frame
	push_key(KEY_P, false)
	if not await wait_book(client):
		return
	await wait_frames(8)
	if effects.is_playing() or completed[0] != before:
		fail("Reopening spellbook replayed stale pointer effect")
		return
	print("FIXTURE SPELL_CLICK_DONE")
	client.free()
	quit(0)

func wait_spells(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + 30000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.spells_state()
		if state.catalog_ready and state.known.has(SLAM) and state.bar[0] == SLAM and state.level == 10:
			return true
	fail("Replicated known spells/action bar or authored catalog missing: " + str(client.spells_state()))
	return false

func wait_book(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.spells_state().spellbook_open and client.get_node_or_null("SpellBookUI") != null:
			return true
	fail("Authored spellbook did not open")
	return false

func pointer(control: Control, button: MouseButton, down: bool) -> void:
	var point := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	root.push_input(motion, true)
	var event := InputEventMouseButton.new()
	event.position = point
	event.button_index = button
	event.pressed = down
	root.push_input(event, true)

func assert_click(client: Node, effects: AudioStreamPlayer, completed: Array, control: Control, label: String) -> bool:
	await wait_frames(8)
	if not control.is_visible_in_tree() or effects.is_playing():
		fail(label + " not eligible or Effects not quiet before press")
		return false
	var before: int = completed[0]
	pointer(control, MOUSE_BUTTON_LEFT, true)
	await process_frame
	var playing := effects.is_playing()
	if not playing and completed[0] == before:
		await process_frame
	if (not playing and completed[0] != before + 1) or absf(effects.volume_linear - 0.44) > 0.001:
		fail(label + " left press did not play owned Effects at seeded master*effects*.55 before release")
		return false
	pointer(control, MOUSE_BUTTON_LEFT, false)
	await wait_frames(8)
	if effects.is_playing() or completed[0] != before + 1:
		fail(label + " release replayed effect or first effect did not finish")
		return false
	print("PASS: spell-click %s (%s)" % [label, "active" if playing else "finished"])
	return true

func assert_quiet_input(effects: AudioStreamPlayer, completed: Array, control: Control, button: MouseButton) -> bool:
	var before: int = completed[0]
	pointer(control, button, true)
	await process_frame
	pointer(control, button, false)
	await wait_frames(8)
	if effects.is_playing() or completed[0] != before:
		fail("Right pointer input played Effects")
		return false
	return true

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame
