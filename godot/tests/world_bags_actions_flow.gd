extends "res://tests/world_bags_flow.gd"

# Separate from bags-cursor acceptance. Real input only; peer owns every mutation.
# Read-only merchant_state().equipment: MainHand item25/count1, startup GUID
# 9170105 replaced by bag sword GUID9170005 only after the peer's Equip delta.
# No equipment mesh parity or raw drag-release coverage.
const ACTION_QUIET_MS := 650
const STARTUP_SWORD_GUID := 9170105
const BAG_SWORD_GUID := 9170005
const INITIAL_ITEMS := [[5, 25, "Worn Shortsword"], [2, 4865, "Ruined Pelt"], [3, 3871, "Plans: Golden Scale Shoulders"]]
const POOR_TEXT := "Do you want to destroy Ruined Pelt?"
const RARE_TEXT := "Do you want to destroy Plans: Golden Scale Shoulders?\n\nType \"DELETE\" into the field to confirm."
const CURSOR_ICON := "CursorItemIcon"

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/diagnostics/native-input-"):
		fail("Bags-actions requires owned loopback peer and unique canonical config")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 180000):
		return
	print("FIXTURE BAGS_ACTIONS_LOADING")
	if not await wait_world(client):
		return
	var account: Dictionary = client.account_state()
	if not account.reply_received or account.character_count != 3:
		fail("Bags-actions must originate from authenticated character17 roster")
		return
	print("FIXTURE BAGS_ACTIONS_READY")
	if not await wait_action_inventory(client, INITIAL_ITEMS):
		return
	if not await wait_startup_equipment(client):
		return
	if not bags_closed(client):
		fail("Bags-actions must start closed without merchant/menu")
		return
	var backpack := await wait_bag_button(client, BACKPACK)
	if backpack == null:
		return
	await click(backpack)
	if not await wait_container(client, 0, true) or not check_action_capacity(client):
		return
	if not await equip_sword(client):
		return
	if not await use_pelt(client):
		return
	if not await poor_destroy(client):
		return
	if not await rare_destroy(client):
		return
	print("FIXTURE BAGS_ACTIONS_RARE_DONE")
	if not await quiet_actions(client, [], false):
		return
	print("FIXTURE BAGS_ACTIONS_DONE")
	# Parent deliberately kills/reaps/drains readers; not engine shutdown proof.
	while true:
		await process_frame

func check_action_capacity(client: Node) -> bool:
	for slot in range(16):
		if authored_control(client, "ContainerFrame0Slot%s" % slot) == null:
			fail("Missing authored bag0 slot%s" % slot)
			return false
	if authored_control(client, "ContainerFrame0Slot16") != null:
		fail("Backpack exceeded authoritative size16")
		return false
	return true

func equip_sword(client: Node) -> bool:
	print("FIXTURE BAGS_ACTIONS_EQUIP_ARM")
	await process_frame
	if not await press_slot(client, 5, MOUSE_BUTTON_RIGHT):
		return false
	# Peer rejects premature/duplicate requests; only its delta may replace startup GUID.
	if not await wait_action_inventory(client, [INITIAL_ITEMS[1], INITIAL_ITEMS[2]]):
		return false
	if not await wait_equipment(client):
		return false
	if not await quiet_actions(client, [INITIAL_ITEMS[1], INITIAL_ITEMS[2]], false):
		return false
	print("FIXTURE BAGS_ACTIONS_EQUIP_DONE")
	return true

# ContainerFrame.lua:1342 UseContainerItem: right-clicking a non-equippable item uses it;
# the peer records the UseItem and changes nothing.
func use_pelt(client: Node) -> bool:
	var remaining := [INITIAL_ITEMS[1], INITIAL_ITEMS[2]]
	print("FIXTURE BAGS_ACTIONS_USE_ARM")
	await process_frame
	if not await press_slot(client, 2, MOUSE_BUTTON_RIGHT):
		return false
	if not await quiet_actions(client, remaining, false):
		return false
	print("FIXTURE BAGS_ACTIONS_USE_DONE")
	return true

func poor_destroy(client: Node) -> bool:
	var remaining := [INITIAL_ITEMS[1], INITIAL_ITEMS[2]]
	if not await pickup_world_drop(client, 2) or not await wait_popup(client, POOR_TEXT, false):
		return false
	if not await quiet_actions(client, remaining, true) or not await press_popup(client, false):
		return false
	if not await wait_popup_closed(client) or not await quiet_actions(client, remaining, false):
		return false
	print("FIXTURE BAGS_ACTIONS_POOR_NO")
	if not await pickup_world_drop(client, 2) or not await wait_popup(client, POOR_TEXT, false):
		return false
	if not await quiet_actions(client, remaining, true):
		return false
	print("FIXTURE BAGS_ACTIONS_POOR_ARM")
	await process_frame
	if not await press_popup(client, true):
		return false
	if not await wait_action_inventory(client, [INITIAL_ITEMS[2]]) or not await wait_popup_closed(client):
		return false
	if not await quiet_actions(client, [INITIAL_ITEMS[2]], false):
		return false
	print("FIXTURE BAGS_ACTIONS_POOR_DONE")
	return true

func rare_destroy(client: Node) -> bool:
	if not await pickup_world_drop(client, 3) or not await wait_popup(client, RARE_TEXT, true):
		return false
	if not await reject_rare_accept(client, ""):
		return false
	await type_characters("wrong")
	if not await reject_rare_accept(client, "wrong"):
		return false
	await backspaces(5)
	# Unicode is sent as actual characters; cap counts characters, not UTF-8 bytes.
	await type_characters("é".repeat(33))
	if not await reject_rare_accept(client, "é".repeat(32)):
		return false
	await backspaces(1)
	if not edit_matches(client, "é".repeat(31)):
		return false
	await backspaces(31)
	if not await reject_rare_accept(client, ""):
		return false
	print("FIXTURE BAGS_ACTIONS_RARE_INERT")
	await type_characters("delete")
	if not edit_matches(client, "delete") or not yes_enabled(client, true):
		return false
	if not await quiet_actions(client, [INITIAL_ITEMS[2]], true):
		return false
	print("FIXTURE BAGS_ACTIONS_RARE_ARM")
	await process_frame
	if not await press_popup(client, true):
		return false
	return await wait_action_inventory(client, []) and await wait_popup_closed(client)

func reject_rare_accept(client: Node, typed: String) -> bool:
	if not edit_matches(client, typed) or not yes_enabled(client, false):
		return false
	if not await press_popup(client, true):
		return false
	await action_key(KEY_ENTER)
	if not await wait_popup(client, RARE_TEXT, true):
		return false
	return await quiet_actions(client, [INITIAL_ITEMS[2]], true)

func type_characters(value: String) -> void:
	for index in range(value.length()):
		await action_key(0, value.unicode_at(index))

func backspaces(count: int) -> void:
	for index in range(count):
		await action_key(KEY_BACKSPACE)

func action_key(code: int, unicode_value: int = 0) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.unicode = unicode_value
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame

func press_slot(client: Node, slot: int, button: int) -> bool:
	var control := await wait_bag_button(client, "ContainerFrame0Slot%s" % slot)
	if control == null:
		return false
	await pointer_click(control.get_global_rect().get_center(), button)
	return true

func pointer_click(point: Vector2, button: int) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame

func pickup_world_drop(client: Node, slot: int) -> bool:
	if not await press_slot(client, slot, MOUSE_BUTTON_LEFT) or not await wait_action_cursor(client, true):
		return false
	var point := Vector2(960, 600)
	# Exclude authored actionable/mouse-blocking Controls, including container/HUD.
	if blocking_control_at(client, point) != null:
		fail("World-drop point covered by authored Control: " + str(blocking_control_at(client, point)))
		return false
	await pointer_click(point, MOUSE_BUTTON_LEFT)
	return true

func blocking_control_at(node: Node, point: Vector2) -> Control:
	if node is Control:
		var control := node as Control
		if control.is_visible_in_tree() and control.mouse_filter != Control.MOUSE_FILTER_IGNORE and control.get_global_rect().has_point(point):
			return control
	for child in node.get_children():
		var found := blocking_control_at(child, point)
		if found != null:
			return found
	return null

func press_popup(client: Node, accept: bool) -> bool:
	var control := authored_control(client, "StaticPopup1Button1" if accept else "StaticPopup1Button2")
	if control == null or not control.is_visible_in_tree():
		fail("Missing authored popup Yes/No button")
		return false
	await pointer_click(control.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	return true

func yes_enabled(client: Node, enabled: bool) -> bool:
	var button := authored_control(client, "StaticPopup1Button1") as BaseButton
	if button == null or button.disabled == enabled:
		fail("StaticPopup1 Yes enabled state expected %s" % enabled)
		return false
	return true

func edit_matches(client: Node, expected: String) -> bool:
	var label := authored_control(client, "StaticPopup1EditBoxText") as Label
	if label == null or label.text != expected:
		fail("Popup Unicode/backspace/cap typing expected=%s observed=%s" % [expected, label.text if label != null else "missing"])
		return false
	return true

func wait_popup(client: Node, expected_text: String, edit: bool) -> bool:
	var deadline := Time.get_ticks_msec() + BAG_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var frame := authored_control(client, "StaticPopup1")
		if frame == null or not frame.is_visible_in_tree():
			continue
		var text := authored_control(client, "StaticPopup1Text") as Label
		var editbox := authored_control(client, "StaticPopup1EditBox")
		var has_edit := editbox != null and editbox.is_visible_in_tree()
		if text == null or text.text != expected_text or has_edit != edit:
			fail("Original destroy popup text/edit mismatch: %s" % (text.text if text != null else "missing"))
			return false
		for pair in [["StaticPopup1Button1", "Yes"], ["StaticPopup1Button2", "No"]]:
			var button := authored_control(client, pair[0]) as Button
			var caption: Label = null
			if button != null:
				caption = button.find_child("Text", true, false) as Label
			if caption == null or caption.text != pair[1]:
				fail("Original popup button caption mismatch: " + pair[0])
				return false
		return true
	fail("RED: world click after pickup did not open original authored StaticPopup1")
	return false

func wait_popup_closed(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + BAG_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var frame := authored_control(client, "StaticPopup1")
		if frame == null or not frame.is_visible_in_tree():
			return await wait_action_cursor(client, false)
	fail("Destroy popup remained visible after Yes/No")
	return false

func cursor_shown(client: Node) -> bool:
	var icon := authored_control(client, CURSOR_ICON)
	return icon != null and icon.is_visible_in_tree()

func wait_action_cursor(client: Node, visible: bool) -> bool:
	var deadline := Time.get_ticks_msec() + BAG_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if cursor_shown(client) == visible:
			return true
	fail("Bags-actions cursor expected visible=%s" % visible)
	return false

func action_inventory_matches(client: Node, expected: Array) -> bool:
	var items: Array = client.merchant_state().bags
	if items.size() != expected.size():
		return false
	for wanted in expected:
		var matches := 0
		for item in items:
			if item.bag == 0 and item.slot == wanted[0] and item.item_id == wanted[1] and item.name == wanted[2] and item.count == 1:
				matches += 1
		if matches != 1:
			return false
	return true

func wait_action_inventory(client: Node, expected: Array) -> bool:
	var deadline := Time.get_ticks_msec() + INVENTORY_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if action_inventory_matches(client, expected):
			return true
	fail("Bags-actions authoritative inventory expected=%s observed=%s" % [expected, client.merchant_state().bags])
	return false

func wait_startup_equipment(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + INVENTORY_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.merchant_state()
		var popup := authored_control(client, "StaticPopup1")
		var split := authored_control(client, "StackSplitFrame")
		if not action_inventory_matches(client, INITIAL_ITEMS) or not bags_closed(client) or cursor_shown(client) or state.split_open:
			fail("Bags-actions startup equipment wait changed bags/windows/carried cursor/split: %s" % state)
			return false
		if (popup != null and popup.is_visible_in_tree()) or (split != null and split.is_visible_in_tree()):
			fail("Bags-actions startup equipment wait opened popup/split")
			return false
		if equipment_matches(client, STARTUP_SWORD_GUID):
			print("BAGS ACTIONS STARTUP_EQUIPMENT MainHand item25 guid%s count1 unchanged bags/no popup/no carried cursor/no split" % STARTUP_SWORD_GUID)
			return true
	fail("RED: startup EquipmentSnapshot requires exactly MainHand item25 guid%s count1 before EQUIP_ARM; observed=%s" % [STARTUP_SWORD_GUID, client.merchant_state()])
	return false

func equipment_matches(client: Node, expected_guid: int = BAG_SWORD_GUID) -> bool:
	var state: Dictionary = client.merchant_state()
	if not state.has("equipment") or not (state.equipment is Array):
		return false
	var entries: Array = state.equipment
	if entries.size() != 1:
		return false
	var item: Dictionary = entries[0]
	return item.get("slot") == "MainHand" and item.get("item_id") == 25 and item.get("item_guid") == expected_guid and item.get("count") == 1

func wait_equipment(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + INVENTORY_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if equipment_matches(client):
			return true
	fail("Read-only merchant_state().equipment requires exactly MainHand sword25 guid9170005 count1 after authoritative delta; observed=%s" % client.merchant_state())
	return false

func quiet_actions(client: Node, expected: Array, cursor: bool) -> bool:
	var deadline := Time.get_ticks_msec() + ACTION_QUIET_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if not action_inventory_matches(client, expected) or not equipment_matches(client) or cursor_shown(client) != cursor:
			fail("Bags-actions quiet interval changed authoritative inventory/equipment/cursor")
			return false
		var container := authored_control(client, "ContainerFrame0")
		if container == null or not container.is_visible_in_tree() or client.get_node_or_null("GameMenuUI") != null or client.get_node_or_null("MerchantUI") != null:
			fail("Bags-actions quiet interval closed backpack/opened unrelated window")
			return false
	return true
