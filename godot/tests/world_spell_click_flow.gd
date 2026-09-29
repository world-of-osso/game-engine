extends "res://tests/world_sound_flow.gd"

const SLAM := 1464

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/native-reset-fixture-"):
		fail("Spell-click fixture requires owned UDP endpoint and isolated options")
		return
	if not seed_scaled_sound_options(config):
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
	if action == null or not await check_scaled_spell_tooltip(client, action):
		return
	if not await assert_click(client, effects, completed, action, "action bar"):
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
	if not await check_action_bar_visibility(client, bar, action, effects, completed):
		return
	before = completed[0]
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
	if not await check_cast_audio(client, sound):
		return
	print("FIXTURE SPELL_CLICK_DONE")
	client.free()
	quit(0)

func seed_scaled_sound_options(config: String) -> bool:
	var path := config.path_join("world-of-osso/options_settings.ron")
	var settings := FileAccess.get_file_as_string(path)
	if not settings.begins_with("(sound:"):
		fail("Sound-click fixture has no isolated options to scale")
		return false
	var options := FileAccess.open(path, FileAccess.WRITE)
	if options == null:
		fail("Cannot seed owned sound-click UI scale")
		return false
	options.store_string(settings.replace("(sound:", "(graphics:(uiScale:1.25),sound:"))
	options.close()
	return true

func check_scaled_spell_tooltip(client: Node, action: Control) -> bool:
	var physical := action.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = physical
	motion.global_position = physical
	root.push_input(motion, true)
	await wait_frames(4)
	var host := client.get_node_or_null("SpellTooltipUI")
	var tooltip := host.find_child("SpellTooltip", true, false) as Control if host != null else null
	if tooltip == null or not tooltip.is_visible_in_tree():
		fail("Known Slam action did not reveal authored spell tooltip")
		return false
	var scale := 5.0 / 6.0
	var rect := tooltip.get_global_rect()
	var expected := Vector2(action.get_global_rect().position.x, action.get_global_rect().position.y - rect.size.y - 4.0 * scale)
	if absf(tooltip.get_global_transform().get_scale().x - scale) > 0.01 or rect.position.distance_to(expected) > 2.0:
		fail("Scaled spell tooltip %s not above live Slam action at %s" % [rect, expected])
		return false
	return true

func check_action_bar_visibility(client: Node, bar: Node, action: Control, effects: AudioStreamPlayer, completed: Array) -> bool:
	if not action.is_visible_in_tree() or client.spells_state().bar[0] != SLAM:
		fail("Replicated main bar not visible with Slam in slot 1")
		return false
	var original_id := bar.get_instance_id()
	push_key(KEY_P, true)
	await process_frame
	push_key(KEY_P, false)
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client):
		return false
	await click_menu_action(client, "MenuBtnOptions")
	await process_frame
	await click_option(client, "OptionsTabhud")
	await click_option(client, "ToggleSwitchshow_action_barsLeftHit")
	await process_frame
	if action.is_visible_in_tree() or bar.get_instance_id() != original_id:
		fail("Committed HUD toggle did not hide cached whole main bar immediately")
		return false
	await click_option(client, "OptionsDoneButton")
	if client.get_node_or_null("GameMenuUI") != null:
		await click_menu_action(client, "MenuBtnResume")
	var sent_before: int = client.spells_state().sent.size()
	push_key(KEY_1, true)
	await process_frame
	push_key(KEY_1, false)
	await wait_frames(2)
	var state: Dictionary = client.spells_state()
	if action.is_visible_in_tree() or state.bar[0] != SLAM or state.sent.size() != sent_before + 1 or state.sent[-1] != SLAM:
		fail("Hidden bar changed slot or blocked keyboard SpellCastIntent: " + str(state))
		return false
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client):
		return false
	await click_menu_action(client, "MenuBtnOptions")
	await process_frame
	await click_option(client, "OptionsTabhud")
	await click_option(client, "ToggleSwitchshow_action_barsRightHit")
	await process_frame
	if not action.is_visible_in_tree() or bar.get_instance_id() != original_id or client.spells_state().bar[0] != SLAM:
		fail("Restoring HUD toggle did not restore cached main bar and slot")
		return false
	await click_option(client, "OptionsDoneButton")
	if client.get_node_or_null("GameMenuUI") != null:
		await click_menu_action(client, "MenuBtnResume")
	if not await assert_click(client, effects, completed, action, "restored action bar"):
		return false
	push_key(KEY_P, true)
	await process_frame
	push_key(KEY_P, false)
	if not await wait_book(client):
		return false
	print("PASS: whole main bar hide, hidden key cast, restored cached slot and click")
	return true

func check_cast_audio(client: Node, sound: Node) -> bool:
	# Keyboard already requested SLAM; the server has not confirmed a cast yet.
	var cast_root := sound.get_node_or_null("CastSpells")
	if cast_root != null and cast_root.get_child_count() != 0:
		fail("CastStart played for request without replicated CastState")
		return false
	if cast_root == null:
		fail("CastSpells spatial channel missing")
		return false
	var starts := [0]
	cast_root.child_entered_tree.connect(func(_node: Node): starts[0] += 1)
	print("FIXTURE SPELL_CLICK_CAST_REQUEST_QUIET")
	if not await expect_cast(client, cast_root, starts, 1, 0.6):
		return false
	print("FIXTURE SPELL_CLICK_CAST_REPEAT")
	await wait_frames(12)
	if starts[0] != 1:
		fail("Repeated active CastState replayed CastStart")
		return false
	print("FIXTURE SPELL_CLICK_CAST_INACTIVE")
	var inactive_deadline := Time.get_ticks_msec() + 5000
	while client.spells_state().casting != 0 and Time.get_ticks_msec() < inactive_deadline:
		await process_frame
	if client.spells_state().casting != 0:
		fail("Inactive CastState did not clear local cast")
		return false
	print("FIXTURE SPELL_CLICK_CAST_RETRIGGER")
	if not await expect_cast(client, cast_root, starts, 2, 0.6):
		return false
	print("FIXTURE SPELL_CLICK_CAST_MUTING")
	if not await mute_cast_options(client):
		return false
	print("FIXTURE SPELL_CLICK_CAST_MUTED")
	if not await expect_cast(client, cast_root, starts, 3, 0.0):
		return false
	var removal_start := Time.get_ticks_msec()
	print("FIXTURE SPELL_CLICK_CAST_REMOVAL")
	var removal_deadline := removal_start + 130
	while client.get_node_or_null("WorldUnits/" + NAME) != null and Time.get_ticks_msec() < removal_deadline:
		await process_frame
	if client.get_node_or_null("WorldUnits/" + NAME) != null or cast_root.get_child_count() != 0:
		fail("Replicated removal did not stop active CastStart before 140ms sample completed")
		return false
	print("PASS: authoritative CastStart/repeat/reset/gain/mute/removal")
	return true

func expect_cast(client: Node, cast_root: Node, starts: Array, expected: int, gain: float) -> bool:
	var deadline := Time.get_ticks_msec() + 5000
	while starts[0] < expected and Time.get_ticks_msec() < deadline:
		await process_frame
	if starts[0] != expected:
		fail("Expected exactly %d confirmed CastStart emitters, got %d" % [expected, starts[0]])
		return false
	var player := client.get_node_or_null("WorldUnits/" + NAME) as Node3D
	for child in cast_root.get_children():
		if child is AudioStreamPlayer3D and child.stream is AudioStreamWAV and child.stream.data.size() == 12348 and player != null and child.global_position.distance_to(player.global_position) < 0.1 and absf(child.volume_linear - gain) < 0.001:
			return true
	fail("CastStart missing spatial player at master*effects*.75 = %.3f" % gain)
	return false

func mute_cast_options(client: Node) -> bool:
	if client.spells_state().spellbook_open:
		push_key(KEY_P, true)
		await process_frame
		push_key(KEY_P, false)
		await wait_frames(2)
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client):
		return false
	await click_menu_action(client, "MenuBtnOptions")
	await process_frame
	await click_option(client, "OptionsTabsound")
	await click_option(client, "ToggleSwitchmutedRightHit")
	return true

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
