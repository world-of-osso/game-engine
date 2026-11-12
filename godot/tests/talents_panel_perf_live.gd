extends "res://tests/frame_benchmark.gd"

## Measure the live Forever talents panel with the release GDExtension.
## Required: GODOT_TEST_SERVER, BENCH_ACCOUNT, BENCH_CHARACTER,
## BENCH_TALENTS_SCREENSHOT. Optional: BENCH_SEGMENT_S, BENCH_RESOLUTION,
## BENCH_WORLD_TRACE_S, BENCH_WORLD_TRACE_PATH, BENCH_STARTUP_INWORLD,
## BENCH_REQUIRE_LOADING_PREWARM, BENCH_REQUIRE_FAST_FIRST_OPEN.
const ASSET_STARTUP_TIMEOUT_MS := 180000
const SPEC_TIMEOUT_MS := 30000
const OPEN_TIMEOUT_MS := 60000
const SETTLE_SECONDS := 5.0

var world_trace_seconds := 0.0
var world_trace_path := ""
var world_trace_finished := false
var world_trace_prefetched_at_entry := false
var loading_prewarm_seen := false
var loading_started_usec := 0
var prewarm_frame_counts: Dictionary = {}
var prewarm_talent_node_count := 0

class InputEventProbe extends Node:
	var n_pressed_events := 0

	func _input(event: InputEvent) -> void:
		if event is InputEventKey and event.physical_keycode == KEY_N and event.pressed:
			n_pressed_events += 1

func capture_world_trace() -> void:
	world_trace_prefetched_at_entry = client.get_node_or_null("SpellBookPrewarmUI") != null
	print("TALENTS_PANEL_WORLD_TRACE_START ", JSON.stringify({"prefetched_ui_at_entry": world_trace_prefetched_at_entry}))
	for samples in [intervals, client_ms, process_ms, physics_ms, main_cpu_ms]:
		samples.clear()
	recording_systems = false
	last_usec = 0
	var recorded_samples := 0
	var recorded_duration_ms := 0.0
	recording = true
	while recorded_duration_ms < world_trace_seconds * 1000.0:
		await process_frame
		while recorded_samples < intervals.size():
			recorded_duration_ms += intervals[recorded_samples]
			recorded_samples += 1
	recording = false
	world_trace_finished = true
	if intervals.size() != client_ms.size():
		fail("In-world frame trace has mismatched interval and process samples")
		return
	var elapsed_ms := 0.0
	var frames: Array[Dictionary] = []
	for index in range(intervals.size()):
		var interval_ms := intervals[index]
		elapsed_ms += interval_ms
		frames.append({"elapsed_ms": elapsed_ms, "frame_interval_ms": interval_ms,
			"client_process_ms": client_ms[index]})
	var file := FileAccess.open(world_trace_path, FileAccess.WRITE)
	if file == null:
		fail("Could not save in-world frame trace %s: %s" % [world_trace_path, FileAccess.get_open_error()])
		return
	file.store_line(JSON.stringify({"duration_s": world_trace_seconds,
		"prefetched_ui_at_entry": world_trace_prefetched_at_entry,
		"loading_prewarm_seen": loading_prewarm_seen, "frames": frames}))
	file.close()
	print("TALENTS_PANEL_WORLD_TRACE_PASS ", JSON.stringify({"path": world_trace_path,
		"duration_s": world_trace_seconds, "frames": frames.size(), "recorded_ms": elapsed_ms,
		"prefetched_ui_at_entry": world_trace_prefetched_at_entry,
		"loading_prewarm_seen": loading_prewarm_seen}))

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
	var node_count := talent_root_count(book_ui)
	if node_count == 0:
		return {}
	return {"panel": panel, "node_count": node_count,
		"node_control_count": book_ui.find_children("TalentNode*", "Control", true, false).size()}

func talent_root_count(root_node: Node) -> int:
	var total := 0
	for control in root_node.find_children("TalentNode*", "Control", true, false):
		var name := str(control.name)
		if name.trim_prefix("TalentNode").is_valid_int():
			total += 1
	return total

func spend_art_count(counts: Dictionary) -> int:
	var total := 0
	for control_name in counts:
		if str(control_name).ends_with("SpendArt"):
			total += int(counts[control_name])
	return total

func named_control_counts(root_node: Node) -> Dictionary:
	var counts: Dictionary = {}
	for control in root_node.find_children("*", "Control", true, false):
		var control_name := str(control.name)
		counts[control_name] = int(counts.get(control_name, 0)) + 1
	return counts

func total_control_count(counts: Dictionary) -> int:
	var total := 0
	for count in counts.values():
		total += int(count)
	return total

func control_name_delta(before: Dictionary, after: Dictionary) -> Array[String]:
	var delta: Array[String] = []
	for control_name in after:
		var extra := int(after[control_name]) - int(before.get(control_name, 0))
		for _index in range(maxi(extra, 0)):
			delta.append(str(control_name))
	delta.sort()
	return delta

func capture_loading_prewarm_tree() -> bool:
	var prewarm := client.get_node_or_null("SpellBookPrewarmUI") as CanvasLayer
	if prewarm == null:
		return true
	var loading := client.get_node_or_null("LoadingUI") as CanvasLayer
	loading_prewarm_seen = true
	if prewarm_frame_counts.is_empty():
		prewarm_frame_counts = named_control_counts(prewarm)
		prewarm_talent_node_count = talent_root_count(prewarm)
		if prewarm_talent_node_count != 88:
			fail("Loading prewarm has %d numeric TalentNode roots; expected 88" % prewarm_talent_node_count)
			return false
		print("TALENTS_PANEL_LOADING_PREWARM ", JSON.stringify({
			"visible": prewarm.is_visible(), "visible_in_tree": prewarm.is_visible_in_tree(),
			"layer": prewarm.get_layer(), "loading_visible": loading != null and loading.is_visible(),
			"loading_layer": loading.get_layer() if loading != null else null,
			"talent_nodes": prewarm_talent_node_count,
			"controls": total_control_count(prewarm_frame_counts),
			"spend_art": spend_art_count(prewarm_frame_counts)
		}))
	return true

func rendering_pipeline_counts() -> Dictionary:
	return {
		"canvas": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_CANVAS),
		"draw": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_DRAW),
		"specialization": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_SPECIALIZATION)
	}

func pipeline_compilation_delta(before: Dictionary, after: Dictionary) -> Dictionary:
	var delta: Dictionary = {}
	for kind in after:
		delta[kind] = int(after[kind]) - int(before.get(kind, 0))
	return delta

func wait_for_startup_world(character: String) -> bool:
	var started_usec := Time.get_ticks_usec()
	var max_process_frame_ms := 0.0
	var deadline := Time.get_ticks_msec() + 900000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		max_process_frame_ms = maxf(max_process_frame_ms, Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0)
		var state: Dictionary = client.account_state()
		if state.screen == "Loading" and loading_started_usec == 0:
			loading_started_usec = Time.get_ticks_usec()
		if state.screen == "Loading" and OS.get_environment("BENCH_REQUIRE_LOADING_PREWARM") == "1":
			if not capture_loading_prewarm_tree():
				return false
		if state.screen == "InWorld" and world_trace_seconds > 0.0 and not world_trace_finished:
			await capture_world_trace()
		if state.screen == "InWorld" and Readiness.is_ready(state) and state.local_player_position != null \
				and (world_trace_seconds <= 0.0 or world_trace_finished):
			if OS.get_environment("BENCH_REQUIRE_LOADING_PREWARM") == "1":
				var prewarm := client.get_node_or_null("SpellBookPrewarmUI") as CanvasLayer
				if prewarm == null or prewarm.is_visible_in_tree():
					fail("Talent prewarm remained visible after Loading")
					return false
			var loading_ms := float(Time.get_ticks_usec() - loading_started_usec) / 1000.0 if loading_started_usec > 0 else -1.0
			print("TALENTS_PANEL_INWORLD_READY ", JSON.stringify({
				"elapsed_ms": float(Time.get_ticks_usec() - started_usec) / 1000.0,
				"loading_ms": loading_ms,
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
	world_trace_seconds = float(OS.get_environment("BENCH_WORLD_TRACE_S")) if OS.get_environment("BENCH_WORLD_TRACE_S") != "" else 0.0
	world_trace_path = OS.get_environment("BENCH_WORLD_TRACE_PATH")
	if world_trace_seconds > 0.0 and world_trace_path.is_empty():
		fail("BENCH_WORLD_TRACE_PATH is required when BENCH_WORLD_TRACE_S is set")
		return
	if world_trace_seconds > 0.0 and OS.get_environment("BENCH_STARTUP_INWORLD") != "1":
		fail("BENCH_WORLD_TRACE_S requires BENCH_STARTUP_INWORLD=1")
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
	world_trace_prefetched_at_entry = client.get_node_or_null("SpellBookPrewarmUI") != null
	if OS.get_environment("BENCH_REQUIRE_LOADING_PREWARM") == "1" \
			and (not loading_prewarm_seen or not world_trace_prefetched_at_entry):
		fail("Talent UI was not prewarmed on the Loading screen before InWorld")
		return
	player = client.get_node_or_null("WorldUnits/" + character) as Node3D
	if player == null:
		fail("No local player node WorldUnits/" + character)
		return
	var spec_id := int(OS.get_environment("BENCH_SPEC_ID"))
	var active_spec_id := int(client.spells_state().get("spec", 0))
	if spec_id > 0 and active_spec_id != spec_id and not await select_specialization(spec_id):
		return
	var ready_spells: Dictionary = client.spells_state()
	if int(ready_spells.get("level", 0)) != 90 or int(ready_spells.get("spec", 0)) <= 0:
		fail("Talents requires a level-90 player with an active spec: " + str(ready_spells))
		return
	if client.account_state().ui_skin != "Forever":
		fail("Selected character layout must use Forever; got " + str(client.account_state().ui_skin))
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
	var pipeline_counts_before_open := rendering_pipeline_counts()
	var pipeline_counts_after_open: Dictionary = {}
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
	var tree_visible_ms := -1.0
	var draw_wait_ms := -1.0
	var first_frame_ms := -1.0
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
			tree_visible_ms = float(Time.get_ticks_usec() - keypress_usec) / 1000.0
			var draw_started_usec := Time.get_ticks_usec()
			await RenderingServer.frame_post_draw
			pipeline_counts_after_open = rendering_pipeline_counts()
			draw_wait_ms = float(Time.get_ticks_usec() - draw_started_usec) / 1000.0
			first_frame_ms = float(Time.get_ticks_usec() - keypress_usec) / 1000.0
			break
	send_key(KEY_N, false)
	input_probe.queue_free()
	if panel == null or not panel.is_visible_in_tree() or node_count == 0:
		var book := client.find_child("SpellBookRoot", true, false)
		fail("N did not render talents: screen=%s book=%s panel=%s nodes=%d focus=%s spells=%s" % [
			client.account_state().screen, book != null, panel != null and panel.is_visible_in_tree(), node_count,
			str(root.gui_get_focus_owner()), str(client.spells_state())])
		return
	var book_ui := client.get_node_or_null("SpellBookUI")
	if book_ui != null and not prewarm_frame_counts.is_empty():
		var visible_frame_counts := named_control_counts(book_ui)
		var added_frame_names := control_name_delta(prewarm_frame_counts, visible_frame_counts)
		var removed_frame_names := control_name_delta(visible_frame_counts, prewarm_frame_counts)
		var added_root_nodes := node_count - prewarm_talent_node_count
		var prewarm_spend_art := spend_art_count(prewarm_frame_counts)
		var added_spend_art := spend_art_count(visible_frame_counts) - prewarm_spend_art
		print("TALENTS_PANEL_PREWARM_DIFF ", JSON.stringify({
			"prewarm_controls": total_control_count(prewarm_frame_counts),
			"visible_controls": total_control_count(visible_frame_counts),
			"added_controls": added_frame_names.size(),
			"added_names": added_frame_names,
			"removed_controls": removed_frame_names.size(),
			"removed_names": removed_frame_names,
			"prewarm_talent_nodes": prewarm_talent_node_count,
			"visible_talent_nodes": node_count,
			"prewarm_spend_art": prewarm_spend_art,
			"added_root_nodes": added_root_nodes,
			"added_spend_art": added_spend_art
		}))
		if OS.get_environment("BENCH_REQUIRE_FAST_FIRST_OPEN") == "1" \
				and (prewarm_spend_art == 0 or prewarm_talent_node_count != 88 or node_count != 88 \
				or added_frame_names.size() != 0 or removed_frame_names.size() != 0 \
				or added_root_nodes != 0 or added_spend_art != 0):
			fail("First-open UI tree differs from Forever Loading prewarm: art=%d added=%s removed=%s" % [
				prewarm_spend_art, added_frame_names, removed_frame_names])
			return
	var dropdown := book_ui.find_child("TalentLoadoutDropDown", true, false) as Control if book_ui != null else null
	var search := book_ui.find_child("TalentSearchBox", true, false) as Control if book_ui != null else null
	var caption := book_ui.find_child("TalentLoadoutName", true, false) as Label if book_ui != null else null
	if dropdown == null or search == null or caption == null:
		fail("Talents footer is missing the loadout caption or SearchBox")
		return
	var dropdown_rect := dropdown.get_global_rect()
	var search_rect := search.get_global_rect()
	var ui_scale := search_rect.size.y / 30.0
	var dropdown_width := dropdown_rect.size.x / ui_scale
	var dropdown_gap := (search_rect.position.x - dropdown_rect.end.x) / ui_scale
	if absf(dropdown_width - 200.0) > 0.5 or absf(dropdown_gap - 20.0) > 0.5:
		fail("Loadout footer geometry differs from Retail: width=%s gap=%s" % [dropdown_width, dropdown_gap])
	if caption.text != "Default Loadout" or caption.get_line_count() != 1:
		fail("Default Loadout caption is clipped: %s lines=%s" % [caption.text, caption.get_line_count()])
	print("TALENTS_PANEL_FOOTER_PASS ", JSON.stringify({"dropdown_width": dropdown_width,
		"gap_to_search": dropdown_gap, "caption": caption.text, "caption_lines": caption.get_line_count()}))
	print("TALENTS_PANEL_FIRST_FRAME ", JSON.stringify({"open_ms": first_frame_ms, "tree_visible_ms": tree_visible_ms,
		"draw_wait_ms": draw_wait_ms, "pipeline_compilations_before": pipeline_counts_before_open,
		"pipeline_compilations_after": pipeline_counts_after_open,
		"pipeline_compilations_delta": pipeline_compilation_delta(pipeline_counts_before_open, pipeline_counts_after_open),
		"talent_nodes": node_count,
		"talent_node_controls": panel_state.node_control_count, "wait_frames": open_wait_frames,
		"lookup_total_ms": float(lookup_total_usec) / 1000.0,
		"lookup_max_ms": float(lookup_max_usec) / 1000.0, "ui_skin": client.account_state().ui_skin}))
	if OS.get_environment("BENCH_REQUIRE_FAST_FIRST_OPEN") == "1" and first_frame_ms >= 100.0:
		fail("First talents frame took %.3f ms; budget is under 100 ms" % first_frame_ms)
		return
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
