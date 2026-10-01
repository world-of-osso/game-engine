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
	if not await check_right_edge_tooltip(client, bar):
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

## Action buttons use GameTooltip_SetDefaultAnchor (ActionButton.lua:1070-1080, UberTooltips 1):
## the tooltip's BOTTOMRIGHT 9 left of and 85 above UIParent's, in scaled UI units.
func default_anchor_offset(tooltip: Control, scale: float) -> float:
	var rect := tooltip.get_global_rect()
	var expected := Vector2(root.size) - Vector2(9.0, 85.0) * scale
	return rect.end.distance_to(expected)

func game_tooltip(client: Node) -> Control:
	var host := client.get_node_or_null("GameTooltipUI")
	return host.find_child("TooltipFrame", true, false) as Control if host != null else null

func check_scaled_spell_tooltip(client: Node, action: Control) -> bool:
	var physical := action.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = physical
	motion.global_position = physical
	root.push_input(motion, true)
	await wait_frames(4)
	var tooltip := game_tooltip(client)
	if tooltip == null or not tooltip.is_visible_in_tree() or client.tooltip_state().title != "Slam":
		fail("Known Slam action did not reveal its GameTooltip")
		return false
	var scale := 5.0 / 6.0
	if absf(tooltip.get_global_transform().get_scale().x - scale) > 0.01 or default_anchor_offset(tooltip, scale) > 2.0:
		fail("Scaled Slam tooltip %s not at the default anchor" % tooltip.get_global_rect())
		return false
	return await capture_scaled_ui("sound-tooltip", tooltip)

func check_right_edge_tooltip(client: Node, bar: Node) -> bool:
	root.size = Vector2i(800, 720)
	await wait_frames(4)
	var action := bar.find_child("ActionButton12", true, false) as Control
	if action == null or client.spells_state().bar[11] != SLAM:
		fail("Owned action snapshot has no known spell in rightmost button")
		return false
	var point := action.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await wait_frames(4)
	var tooltip := game_tooltip(client)
	if tooltip == null or not tooltip.is_visible_in_tree() or not client.spells_state().tooltip.has("Slam"):
		fail("Rightmost authored Slam button did not reveal its tooltip")
		return false
	var scale := maxf(minf(800.0 / 1920.0, 720.0 / 1080.0), 2.0 / 3.0) * 1.25
	if default_anchor_offset(tooltip, scale) > 2.0 or tooltip.get_global_rect().end.x > root.size.x:
		fail("Scaled tooltip left the default anchor at the narrow viewport: %s vs %s" % [tooltip.get_global_rect(), root.size])
		return false
	if not await capture_scaled_ui("sound-tooltip-800-edge", tooltip):
		return false
	root.size = Vector2i(1280, 720)
	await wait_frames(4)
	return true

func capture_scaled_ui(name: String, control: Control) -> bool:
	var directory := OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	if directory.is_empty():
		return true
	if not directory.is_absolute_path() or not DirAccess.dir_exists_absolute(directory):
		fail("Capture directory must exist and be absolute: " + directory)
		return false
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image == null or image.is_empty():
		fail("Rendered UI capture returned an empty image: " + name)
		return false
	var path := directory.path_join(name + ".png")
	var error := image.save_png(path)
	if error != OK:
		fail("Cannot save rendered UI capture %s: %s" % [path, error_string(error)])
		return false
	print("FIXTURE UI_CAPTURE path=%s viewport=%s image=%s scale=%s rect=%s" % [path, root.size, image.get_size(), control.get_global_transform().get_scale(), control.get_global_rect()])
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
