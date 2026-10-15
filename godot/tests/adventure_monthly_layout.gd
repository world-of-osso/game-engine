extends "res://tests/adventure_journeys_watch.gd"
# Genuine private-realm rows and physical input only. Collect independent source failures.
var failures: Array[String] = []

func verify(condition: bool, detail: String) -> void:
	if not condition:
		failures.append(detail)
		print("MONTHLY_SOURCE_FAIL " + detail)

func shift_click(name: String) -> void:
	if not await move_to(name):
		return
	var point := control(name).get_global_rect().get_center()
	var key := InputEventKey.new()
	key.keycode = KEY_SHIFT
	key.pressed = true
	Input.parse_input_event(key)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		event.shift_pressed = true
		Input.parse_input_event(event)
		await process_frame
	key.pressed = false
	Input.parse_input_event(key)
	await settle()

func wait_tracking_change(previous: int) -> bool:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		if int(client.adventure_guide_state().tracked_activity_count) != previous:
			return true
		await process_frame
	return false

func first_visible_activity() -> Control:
	var viewport := control("EncounterJournalContent").get_global_rect()
	for node in client.find_children("JournalActivity*", "Button", true, false):
		var suffix := str(node.name).trim_prefix("JournalActivity")
		if suffix.is_valid_int() and node.is_visible_in_tree() and viewport.has_point(node.get_global_rect().get_center()):
			return node
	return null

func search_activity(value: String) -> bool:
	if not await click("EncounterJournalSearchBox"):
		return false
	for character in value:
		var event := InputEventKey.new()
		event.unicode = character.unicode_at(0)
		event.pressed = true
		Input.parse_input_event(event)
		await process_frame
	await settle()
	return true

func verify_tree(scale: float) -> bool:
	if not await search_activity("Complete 5 Mythic+ Dungeons"):
		return false
	verify(control("JournalActivity6") != null, "authored Mythic+ parent6 absent after search")
	verify(control("JournalActivity81") == null, "source child81 must initially be collapsed")
	if control("JournalActivity6") == null or not await click("JournalActivity6"):
		return false
	var child := control("JournalActivity81")
	var parent := control("JournalActivity6")
	verify(child != null, "physical parent click did not reveal source child81")
	if child == null:
		return false
	verify(child.get_global_rect().size.distance_to(Vector2(521,48) * scale) < 2.0, "source child row521x48")
	verify(absf(child.get_global_rect().position.x - parent.get_global_rect().position.x - 32.0 * scale) < 2.0, "source child indent32")
	if not await move_to("JournalActivity81") or not await shot("monthly-expanded-source-tree"):
		return false
	if not await click("JournalActivity6"):
		return false
	verify(control("JournalActivity81") == null, "physical parent collapse did not hide child81")
	return await click("EncounterJournalSearchBoxClearButton")

func visible_filter_names() -> Array[String]:
	var names: Array[String] = []
	for node in client.find_children("JournalActivityTag*", "Button", true, false):
		if node.is_visible_in_tree():
			names.append(str(node.name))
	return names

func verify_filter_scroll() -> bool:
	var before := visible_filter_names()
	for index in 8:
		if not await wheel("EncounterJournalFilterList", MOUSE_BUTTON_WHEEL_DOWN):
			return false
	verify(visible_filter_names() != before, "physical tag-list wheel did not reach later categories")
	if not await shot("monthly-options-scrolled"):
		return false
	for index in 8:
		if not await wheel("EncounterJournalFilterList", MOUSE_BUTTON_WHEEL_UP):
			return false
	verify(visible_filter_names() == before, "physical tag-list wheel did not restore categories")
	return true

func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	shots = OS.get_environment("ADVENTURE_GUIDE_SHOTS")
	if server != "127.0.0.1:5771" or not account.begins_with("fb_") or shots.is_empty():
		stop("Explicit private5771/fb_* account and shots required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().assets_starting and Time.get_ticks_msec() < deadline:
		await process_frame
	var error: String = client.connect_account(server, account, OS.get_environment("ADVENTURE_GUIDE_PASSWORD"), false)
	if not error.is_empty():
		stop(error)
		return
	if not await enter_world_when_ready():
		return
	await journal_key()
	deadline = Time.get_ticks_msec() + 90000
	while (not client.adventure_guide_state().loaded or not client.adventure_guide_state().activity_snapshot_received) and Time.get_ticks_msec() < deadline:
		await process_frame
	if not await click("EncounterJournalMonthlyActivitiesTab"):
		return
	var pane := control("EncounterJournalMonthlyActivitiesFrame")
	if pane == null:
		stop("Source MonthlyActivities inset missing")
		return
	var scale := pane.get_global_rect().size.x / 792.0
	verify(pane.clip_contents, "native monthly pane must clip authored overhang")
	for entry in [["JournalMonthlyThresholdContainer", Rect2(46,74,663,33)], ["EncounterJournalFilterList", Rect2(5,137,220,290)], ["EncounterJournalContent", Rect2(241,138,521,291)]]:
		var node := control(entry[0])
		var expected: Rect2 = entry[1]
		verify(node != null and node.get_global_rect().position.distance_to(pane.get_global_rect().position + expected.position * scale) < 2.0 and node.get_global_rect().size.distance_to(expected.size * scale) < 2.0, "source rect " + entry[0])
	verify(control("JournalActivityFilterAll") == null and control("JournalActivityFilterTracked") == null, "invented toolbar remains")
	var row := first_visible_activity()
	if row == null:
		stop("No authentic visible activity row")
		return
	var row_name := str(row.name)
	verify(row.get_global_rect().size.distance_to(Vector2(521,78) * scale) < 2.0, "source parent row521x78 " + row_name)
	verify(control(row_name + "Track") == null, "invented per-row Track button remains")
	if not await move_to(row_name):
		return
	await settle()
	var tooltip: Dictionary = client.tooltip_state()
	var title := label_text(row_name + "Name")
	verify(tooltip.get("visible", false) and tooltip.get("title", "") == title and str(tooltip.get("lines", "")).contains("Shift click"), "source activity tooltip missing")
	print("MONTHLY_TOOLTIP_STATE row=%s tooltip=%s" % [row_name, JSON.stringify(tooltip)])
	if not await shot("monthly-source-rows-tooltip"):
		return
	var before := int(client.adventure_guide_state().tracked_activity_count)
	await shift_click(row_name)
	var changed := await wait_tracking_change(before)
	verify(changed, "physical Shift-click did not produce authoritative tracking change")
	print("MONTHLY_TRACKING_STATE row=%s before=%s after=%s" % [row_name, before, client.adventure_guide_state().tracked_activity_count])
	if changed:
		if not await shot("monthly-tracking-ack"):
			return
		var tracked := int(client.adventure_guide_state().tracked_activity_count)
		await shift_click(row_name)
		verify(await wait_tracking_change(tracked) and int(client.adventure_guide_state().tracked_activity_count) == before, "second Shift-click did not restore tracking state")
	if not await verify_tree(scale) or not await verify_filter_scroll():
		return
	if not failures.is_empty():
		stop("Monthly source failures: " + "; ".join(failures))
		return
	print("MONTHLY_LAYOUT_PASS skin=%s genuine_rows/source_geometry/tooltip/ShifttrackACK/restore" % client.account_state().ui_skin)
	quit(0)
