extends "res://tests/frame_benchmark.gd"

## Measure the live Forever talents panel with the release GDExtension.
## Required: GODOT_TEST_SERVER, BENCH_ACCOUNT, BENCH_CHARACTER,
## BENCH_TALENTS_SCREENSHOT. Optional: BENCH_SEGMENT_S, BENCH_RESOLUTION.
const ASSET_STARTUP_TIMEOUT_MS := 180000
const SPEC_TIMEOUT_MS := 30000
const OPEN_TIMEOUT_MS := 60000
const SETTLE_SECONDS := 5.0

class InputEventProbe extends Node:
	var n_pressed_events := 0

	func _input(event: InputEvent) -> void:
		if event is InputEventKey and event.physical_keycode == KEY_N and event.pressed:
			n_pressed_events += 1

func select_specialization(spec_id: int) -> bool:
	var error: String = client.set_specialization(spec_id)
	if not error.is_empty():
		fail("Could not request specialization %d: %s" % [spec_id, error])
		return false
	var deadline := Time.get_ticks_msec() + SPEC_TIMEOUT_MS
	while Time.get_ticks_msec() < deadline:
		if int(client.spells_state().get("spec", 0)) == spec_id:
			return true
		await process_frame
	fail("Server did not confirm specialization %d" % spec_id)
	return false

func send_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.physical_keycode = code
	event.keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func find_talents_panel() -> Dictionary:
	var book_ui := client.get_node_or_null("SpellBookUI")
	if book_ui == null:
		return {}
	var panel := book_ui.find_child("ClassTalentsFrame", true, false) as Control
	if panel == null or not panel.is_visible_in_tree():
		return {}
	var node_count := book_ui.find_children("TalentNode*", "Control", true, false).size()
	if node_count == 0:
		return {}
	return {"panel": panel, "node_count": node_count}

func wait_for_startup_world(character: String) -> bool:
	var started_usec := Time.get_ticks_usec()
	var max_process_frame_ms := 0.0
	var deadline := Time.get_ticks_msec() + 900000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		max_process_frame_ms = maxf(max_process_frame_ms, Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0)
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and Readiness.is_ready(state) and state.local_player_position != null:
			print("TALENTS_PANEL_INWORLD_READY ", JSON.stringify({
				"elapsed_ms": float(Time.get_ticks_usec() - started_usec) / 1000.0,
				"max_process_frame_ms": max_process_frame_ms
			}))
			return true
		if Time.get_ticks_msec() % 10000 < 20:
			print("TALENTS_PANEL_STARTUP_WAIT ", JSON.stringify({"screen": state.screen, "assets_starting": state.assets_starting}))
	fail("Startup did not enter a ready world: " + str(client.account_state()))
	return false

func wait_for_asset_startup() -> bool:
	var deadline := Time.get_ticks_msec() + ASSET_STARTUP_TIMEOUT_MS
	while Time.get_ticks_msec() < deadline and client.account_state().assets_starting:
		await process_frame
	if client.account_state().assets_starting:
		fail("CASC asset startup did not finish within %d ms" % ASSET_STARTUP_TIMEOUT_MS)
		return false
	return true

func run_test() -> void:
	root.size = parse_resolution(OS.get_environment("BENCH_RESOLUTION"))
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("BENCH_ACCOUNT")
	var character := OS.get_environment("BENCH_CHARACTER")
	var screenshot := OS.get_environment("BENCH_TALENTS_SCREENSHOT")
	if server.is_empty() or account.is_empty() or character.is_empty() or screenshot.is_empty():
		fail("GODOT_TEST_SERVER, BENCH_ACCOUNT, BENCH_CHARACTER and BENCH_TALENTS_SCREENSHOT are required")
		return
	var segment_s := float(OS.get_environment("BENCH_SEGMENT_S")) if OS.get_environment("BENCH_SEGMENT_S") != "" else 10.0
	RenderingServer.viewport_set_measure_render_time(root.get_viewport_rid(), true)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if OS.get_environment("BENCH_STARTUP_INWORLD") == "1":
		if not await wait_for_startup_world(character):
			return
	else:
		if not await wait_for_asset_startup():
			return
		var error: String = client.connect_account(server, account, PASSWORD, false)
		if not error.is_empty():
			fail("Fixture connection: " + error)
			return
		if not await enter_world(character):
			return
	player = client.get_node_or_null("WorldUnits/" + character) as Node3D
	if player == null:
		fail("No local player node WorldUnits/" + character)
		return
	var spec_id := int(OS.get_environment("BENCH_SPEC_ID"))
	if spec_id > 0 and not await select_specialization(spec_id):
		return
	var ready_spells: Dictionary = client.spells_state()
	if int(ready_spells.get("level", 0)) != 90 or int(ready_spells.get("spec", 0)) <= 0:
		fail("Talents requires a level-90 player with an active spec: " + str(ready_spells))
		return
	var skin_error: String = client.preview_world_skin(true)
	if not skin_error.is_empty() or client.account_state().ui_skin != "Forever":
		fail("Could not select Forever: " + skin_error)
		return
	client.set_world_minutes(WORLD_MINUTES)
	client.set_camera_orbit(0.0, PITCH, DISTANCE)
	await create_timer(SETTLE_SECONDS).timeout
	if not Readiness.is_ready(client.account_state()):
		fail("World work queued before measurement: " + str(client.account_state().world_objects))
		return
	var focus := root.gui_get_focus_owner()
	var spell_state: Dictionary = client.spells_state()
	print("TALENTS_PANEL_BENCH_READY ", JSON.stringify({"character": character, "ui_skin": client.account_state().ui_skin,
		"server": server, "world_minutes": client.world_minutes(), "focus": str(focus.get_path()) if focus != null else "none",
		"spell_spec": spell_state.get("spec"), "spellbook_open": spell_state.get("spellbook_open"),
		"catalog_ready": spell_state.get("catalog_ready"), "input_processing": client.is_processing_input(),
		"unhandled_input_processing": client.is_processing_unhandled_input()}))
	await measure("talents_closed", segment_s, func(_time: float): pass, true)
	root.gui_release_focus()
	var input_probe := InputEventProbe.new()
	input_probe.set_process_input(true)
	root.add_child(input_probe)
	var keypress_usec := Time.get_ticks_usec()
	send_key(KEY_N, true)
	print("TALENTS_PANEL_KEY_STATE ", JSON.stringify({"input_pressed": Input.is_key_pressed(KEY_N),
		"probe_presses": input_probe.n_pressed_events, "book_open": client.spells_state().spellbook_open}))
	var panel: Control
	var node_count := 0
	var panel_state: Dictionary = {}
	var open_wait_frames := 0
	var lookup_total_usec := 0
	var lookup_max_usec := 0
	var deadline := Time.get_ticks_msec() + OPEN_TIMEOUT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		open_wait_frames += 1
		var lookup_started_usec := Time.get_ticks_usec()
		panel_state = find_talents_panel()
		var lookup_usec := Time.get_ticks_usec() - lookup_started_usec
		lookup_total_usec += lookup_usec
		lookup_max_usec = maxi(lookup_max_usec, lookup_usec)
		if not panel_state.is_empty():
			panel = panel_state.panel
			node_count = panel_state.node_count
			await RenderingServer.frame_post_draw
			break
	send_key(KEY_N, false)
	input_probe.queue_free()
	if panel == null or not panel.is_visible_in_tree() or node_count == 0:
		var book := client.find_child("SpellBookRoot", true, false)
		fail("N did not render talents: screen=%s book=%s panel=%s nodes=%d focus=%s spells=%s" % [
			client.account_state().screen, book != null, panel != null and panel.is_visible_in_tree(), node_count,
			str(root.gui_get_focus_owner()), str(client.spells_state())])
		return
	var first_frame_ms := float(Time.get_ticks_usec() - keypress_usec) / 1000.0
	print("TALENTS_PANEL_FIRST_FRAME ", JSON.stringify({"open_ms": first_frame_ms, "talent_nodes": node_count,
		"wait_frames": open_wait_frames, "lookup_total_ms": float(lookup_total_usec) / 1000.0,
		"lookup_max_ms": float(lookup_max_usec) / 1000.0, "ui_skin": client.account_state().ui_skin}))
	var first_frame := root.get_texture().get_image()
	if first_frame.save_png(screenshot + "-first.png") != OK:
		fail("Could not save first talents frame")
		return
	await create_timer(2.0).timeout
	await RenderingServer.frame_post_draw
	var settled_frame := root.get_texture().get_image()
	if settled_frame.save_png(screenshot + "-settled.png") != OK:
		fail("Could not save settled talents frame")
		return
	await measure("talents_open", segment_s, func(_time: float): pass, true)
	send_key(KEY_N, true)
	await process_frame
	send_key(KEY_N, false)
	var close_deadline := Time.get_ticks_msec() + OPEN_TIMEOUT_MS
	while Time.get_ticks_msec() < close_deadline and client.spells_state().spellbook_open:
		await process_frame
	if client.spells_state().spellbook_open:
		fail("N did not close the talents panel after measurement")
		return
	await measure("talents_closed_warm", segment_s, func(_time: float): pass, true)
	var reopen_started_usec := Time.get_ticks_usec()
	send_key(KEY_N, true)
	var reopen_deadline := Time.get_ticks_msec() + OPEN_TIMEOUT_MS
	panel_state = {}
	while Time.get_ticks_msec() < reopen_deadline:
		await process_frame
		panel_state = find_talents_panel()
		if not panel_state.is_empty():
			break
	send_key(KEY_N, false)
	if panel_state.is_empty() or panel_state.node_count != node_count:
		fail("Reopening Talents did not show the complete tree: " + str(panel_state))
		return
	print("TALENTS_PANEL_REOPEN_PASS ", JSON.stringify({"open_ms": float(Time.get_ticks_usec() - reopen_started_usec) / 1000.0,
		"talent_nodes": panel_state.node_count}))
	send_key(KEY_N, true)
	await process_frame
	send_key(KEY_N, false)
	print("TALENTS_PANEL_PERF_PASS")
	client.free()
	quit(0)
