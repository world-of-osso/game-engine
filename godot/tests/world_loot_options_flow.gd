extends "res://tests/world_menu_flow.gd"

# Test-only owned UDP peer. No direct connection, loot setter or TakeAll calls.
const CORPSE := "Fixture Corpse"
const ITEM_LINE := "You receive loot: [Melted Candle]x2"
const MONEY_LINE := "You loot 1 Gold, 5 Silver, 2 Copper"
const REQUEST_MS := 10000

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/diagnostics/native-input-"):
		fail("Loot fixture requires owned loopback peer and isolated options")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 15000):
		return
	print("FIXTURE LOOT_LOADING")
	if not await wait_world(client):
		return
	var corpse := await find_corpse(client)
	if corpse.is_empty():
		return
	print("FIXTURE LOOT_READY")
	# Ordered reliable CorpseLootable must follow completed model replication.
	await create_timer(0.5).timeout
	for case in range(4):
		var default_auto: bool = case >= 2
		var shift: bool = case == 1 or case == 3
		if not await set_auto_loot(client, config, default_auto):
			return
		if case > 0:
			print("FIXTURE LOOT_REARM")
			await create_timer(0.5).timeout
		corpse = await find_corpse(client)
		if corpse.is_empty():
			return
		var item_before := chat_count(client, ITEM_LINE)
		var money_before := chat_count(client, MONEY_LINE)
		await corpse_click(corpse.point, shift)
		print("FIXTURE LOOT_CLICKED")
		if default_auto != shift:
			if not await wait_chat(client, ITEM_LINE, item_before + 1) or not await wait_chat(client, MONEY_LINE, money_before + 1):
				return
			if not await wait_hidden(client):
				return
		else:
			if not await wait_rows(client, ["Melted Candle", "1 Gold\n5 Silver\n2 Copper"]):
				return
			var host := loot_host(client)
			if not check_label(host, "LootFrameElement1ItemCount", "2") or not check_label(host, "LootFrameElement1QualityText", "Poor"):
				return
			await click(host.find_child("LootFrameElement1", true, false) as Control)
			if not await wait_rows(client, ["1 Gold\n5 Silver\n2 Copper"]):
				return
			if not await wait_chat(client, ITEM_LINE, item_before + 1):
				return
			if case == 0:
				var close := loot_host(client).find_child("LootFrameCloseButton", true, false) as Control
				if close == null:
					fail("Authored LootFrame close button missing")
					return
				await click(close)
				if not await wait_hidden(client):
					return
			else:
				await click(loot_host(client).find_child("LootFrameElement1", true, false) as Control)
				if not await wait_chat(client, MONEY_LINE, money_before + 1) or not await wait_hidden(client):
					return
				print("FIXTURE LOOT_EMPTY")
				await create_timer(0.25).timeout
		# Bounded duplicate/mismatched-message check: no additional content lines.
		await create_timer(0.25).timeout
		if chat_count(client, ITEM_LINE) != item_before + 1 or chat_count(client, MONEY_LINE) != money_before + (0 if case == 0 else 1):
			fail("Loot content chat not exactly once for actual taken slots, case=%s" % case)
			return
	print("FIXTURE LOOT_DONE")
	client.free()
	quit(0)

func wait_world(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.selected_character_name == NAME and state.unit_count == 3:
			var terrain: Dictionary = state.terrain
			if terrain.map == "azeroth" and terrain.pending_count == 0 and terrain.failures.is_empty() and not terrain.parsed_tiles.is_empty():
				return true
	fail("Owned corpse world not ready: " + str(client.account_state()))
	return false

func find_corpse(client: Node) -> Dictionary:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var unit := client.get_node_or_null("WorldUnits/" + CORPSE) as Node3D
		var model := unit.get_node_or_null("NpcVisualRoot/NpcModel") if unit != null else null
		var camera := root.get_camera_3d()
		var area := unit.find_child("UnitPick", true, false) as Area3D if unit != null else null
		if model == null or camera == null or area == null:
			continue
		for node in model.find_children("*", "MeshInstance3D", true, false):
			var mesh := node as MeshInstance3D
			if mesh.mesh == null or not mesh.is_visible_in_tree():
				continue
			var center := mesh.global_transform * mesh.get_aabb().get_center()
			var point := camera.unproject_position(center)
			var id = area.get_meta("unit_server_id")
			if camera.is_position_in_frustum(center) and Rect2(Vector2.ZERO, Vector2(root.size)).has_point(point) and UnitPicker.pick(camera, point) == id:
				return {"id": id, "point": point}
	fail("Cached corpse model not visible and UnitPicker-pickable at actual mesh")
	return {}

func corpse_click(point: Vector2, shift: bool) -> void:
	if shift:
		push_key(KEY_SHIFT, true)
		await process_frame
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.shift_pressed = shift
	root.push_input(motion, true)
	await process_frame
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_RIGHT
		event.pressed = down
		event.shift_pressed = shift
		root.push_input(event, true)
		await process_frame
	if shift:
		push_key(KEY_SHIFT, false)
		await process_frame

func set_auto_loot(client: Node, config: String, enabled: bool) -> bool:
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client):
		return false
	await click_menu_action(client, "MenuBtnOptions")
	var menu := client.get_node_or_null("GameMenuUI")
	var tab := menu.find_child("OptionsTabhud", true, false) as Control
	if tab == null:
		fail("Authored HUD Options tab missing")
		return false
	await click(tab)
	var toggle := menu.find_child("ToggleSwitchauto_loot", true, false) as Control
	if toggle == null or not toggle.is_visible_in_tree():
		fail("Authored Auto Loot control missing")
		return false
	# Select both sides so initial defaults `()` also get an observable save.
	for selection in [not enabled, enabled]:
		var side := menu.find_child("ToggleSwitchauto_lootRightHit" if selection else "ToggleSwitchauto_lootLeftHit", true, false) as Control
		if side == null:
			fail("Authored Auto Loot segment missing")
			return false
		await click(side)
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var saved := FileAccess.get_file_as_string(config + "/world-of-osso/options_settings.ron")
		var expression := RegEx.new()
		if expression.compile("auto_loot\\s*:\\s*" + ("true" if enabled else "false")) != OK:
			fail("Invalid canonical Auto Loot persistence oracle")
			return false
		if expression.search(saved) != null:
			await click_menu_action(client, "OptionsDoneButton")
			# Done returns to the game menu; Resume closes the overlay.
			if client.get_node_or_null("GameMenuUI") != null:
				await click_menu_action(client, "MenuBtnResume")
			return await wait_menu_closed(client, null)
	fail("Authored Auto Loot selection did not save canonical boolean")
	return false

func loot_host(client: Node) -> Node:
	# Find the authored external frame, without assuming a future host class/API.
	return client.find_child("LootFrame", true, false)

func check_label(host: Node, name: String, expected: String) -> bool:
	var label := host.find_child(name, true, false) as Label if host != null else null
	if label == null or not label.is_visible_in_tree() or label.text != expected:
		fail("Authored loot label %s expected %s" % [name, expected])
		return false
	return true

func wait_rows(client: Node, expected: Array) -> bool:
	var deadline := Time.get_ticks_msec() + REQUEST_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var frame := loot_host(client) as Control
		if frame == null or not frame.is_visible_in_tree():
			continue
		var names: Array[String] = []
		for index in range(1, 7):
			var label := frame.find_child("LootFrameElement%sText" % index, true, false) as Label
			if label != null and label.is_visible_in_tree():
				names.append(label.text)
		if names == expected:
			return true
	fail("LootResponse/SlotRemoved did not produce authored rows: " + str(expected))
	return false

func wait_hidden(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + REQUEST_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var frame := loot_host(client) as Control
		if frame == null or not frame.is_visible_in_tree():
			return true
	fail("Closed/empty loot frame remained visible")
	return false

func chat_count(client: Node, text: String) -> int:
	var host := client.get_node_or_null("ChatFrameUI")
	if host == null:
		return 0
	var rows := {}
	for node in host.find_children("*", "Label", true, false):
		var label := node as Label
		if not label.is_visible_in_tree():
			continue
		var name := str(label.name)
		var row := ""
		if name.begins_with("ChatFrame1MessagesRow"):
			row = name.substr(0, name.find("Run"))
		elif name.begins_with("ChatFrame1Link"):
			row = "ChatFrame1MessagesRow" + name.substr(14).split("_")[0]
		else:
			continue
		rows[row] = rows.get(row, "") + label.text
	var count := 0
	for value in rows.values():
		if str(value).strip_edges() == text:
			count += 1
	return count

func wait_chat(client: Node, text: String, count: int) -> bool:
	var deadline := Time.get_ticks_msec() + REQUEST_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if chat_count(client, text) == count:
			return true
	fail("LootSlotRemoved chat count expected %s for %s" % [count, text])
	return false
