extends "res://tests/world_bags_flow.gd"

# Physical HUD input only. Inventory changes originate exclusively at the owned UDP peer.
const SOURCE_SLOT := "ContainerFrame0Slot0"
const SWAP_SLOT := "ContainerFrame1Slot0"
const SPLIT_SLOT := "ContainerFrame1Slot1"
const CURSOR := "CursorItemIcon"
const SPLIT_FRAME := "StackSplitFrame"
const CURSOR_QUIET_MS := 900
const COLOR_TOLERANCE := 0.03

var cursor_pointer := Vector2.ZERO

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/diagnostics/native-input-"):
		fail("Bags cursor requires owned loopback peer and isolated defaults")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 180000):
		return
	print("FIXTURE BAGS_CURSOR_LOADING")
	if not await wait_world(client):
		return
	var account: Dictionary = client.account_state()
	if not account.reply_received or account.character_count != 3:
		fail("Bags cursor world must originate from authenticated fixture roster")
		return
	print("FIXTURE BAGS_CURSOR_READY")
	if not await wait_inventory(client, [[0, 0, 3]]):
		return
	if not bags_closed(client):
		fail("Bags cursor must start with closed bags and no merchant/menu")
		return
	for control_name in [BACKPACK, BAG_ONE]:
		var button := await wait_bag_button(client, control_name)
		if button == null:
			return
		await click(button)
	if not await wait_container(client, 0, true) or not await wait_container(client, 1, true):
		return
	if not check_container(client, 0, 16, 0, "3") or not empty_second_bag(client):
		return
	if not await local_cursor_cases(client):
		return
	if not await stale_cursor_case(client):
		return
	if not await swap_cursor_case(client):
		return
	if not await split_cursor_case(client):
		return
	print("FIXTURE BAGS_CURSOR_SPLIT_DONE")
	if not await quiet_inventory(client, [[0, 0, 1], [1, 1, 2]]):
		return
	print("FIXTURE BAGS_CURSOR_DONE")
	# Deliberate parent kill/reap/readers drain, never normal shutdown acceptance.
	while true:
		await process_frame

func empty_second_bag(client: Node) -> bool:
	for slot in range(8):
		if authored_control(client, "ContainerFrame1Slot%s" % slot) == null:
			fail("Missing authored slot in authoritative eight-slot bag1")
			return false
		if not slot_render_matches(client, 1, slot, 0, false):
			fail("Initial bag1 must render empty")
			return false
	if authored_control(client, "ContainerFrame1Slot8") != null:
		fail("Bag1 rendered beyond authoritative capacity")
		return false
	return true

func local_cursor_cases(client: Node) -> bool:
	if not await click_slot(client, SOURCE_SLOT) or not await wait_cursor(client, true):
		return false
	if not await wait_slot_render(client, 0, 0, 3, true):
		return false
	if not inventory_matches(client, [[0, 0, 3]]):
		fail("Pickup locally mutated authoritative inventory")
		return false
	print("FIXTURE BAGS_CURSOR_PICKUP")
	if not await click_slot(client, SOURCE_SLOT) or not await wait_cursor(client, false):
		return false
	if not await wait_slot_render(client, 0, 0, 3, false):
		return false
	if not await quiet_inventory(client, [[0, 0, 3]]):
		return false
	print("FIXTURE BAGS_CURSOR_RETURN")
	print("FIXTURE BAGS_CURSOR_ESCAPE_START")
	if not await click_slot(client, SOURCE_SLOT) or not await wait_cursor(client, true):
		return false
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_cursor(client, false) or not await wait_slot_render(client, 0, 0, 3, false):
		return false
	if not await quiet_inventory(client, [[0, 0, 3]]):
		return false
	print("FIXTURE BAGS_CURSOR_ESCAPE")
	return true

func stale_cursor_case(client: Node) -> bool:
	if not await click_slot(client, SOURCE_SLOT) or not await wait_cursor(client, true):
		return false
	if not await wait_slot_render(client, 0, 0, 3, true):
		return false
	print("FIXTURE BAGS_CURSOR_STALE")
	if not await wait_inventory(client, []) or not await wait_cursor(client, false):
		return false
	if not await wait_slot_render(client, 0, 0, 0, false):
		return false
	if not await quiet_inventory(client, []):
		return false
	print("FIXTURE BAGS_CURSOR_STALE_CLEARED")
	if not await wait_inventory(client, [[0, 0, 3]]):
		return false
	return await wait_slot_render(client, 0, 0, 3, false)

func swap_cursor_case(client: Node) -> bool:
	if not await click_slot(client, SOURCE_SLOT) or not await wait_cursor(client, true):
		return false
	if not await wait_slot_render(client, 0, 0, 3, true):
		return false
	if not inventory_matches(client, [[0, 0, 3]]):
		fail("Swap pickup locally mutated inventory")
		return false
	print("FIXTURE BAGS_CURSOR_SWAP_ARM")
	await process_frame
	if not await click_slot(client, SWAP_SLOT):
		return false
	if not await wait_inventory(client, [[1, 0, 3]]) or not await wait_cursor(client, false):
		return false
	if not await wait_slot_render(client, 0, 0, 0, false) or not await wait_slot_render(client, 1, 0, 3, false):
		return false
	if not await quiet_inventory(client, [[1, 0, 3]]):
		return false
	print("FIXTURE BAGS_CURSOR_SWAP_DONE")
	if not await wait_inventory(client, [[0, 0, 3]]):
		return false
	return await wait_slot_render(client, 0, 0, 3, false) and await wait_slot_render(client, 1, 0, 0, false)

func split_cursor_case(client: Node) -> bool:
	push_key(KEY_SHIFT, true)
	if not await click_slot(client, SOURCE_SLOT, true):
		push_key(KEY_SHIFT, false)
		return false
	push_key(KEY_SHIFT, false)
	if not await wait_split_frame(client, true):
		return false
	if not await wait_cursor(client, false):
		return false
	push_digit_two()
	await process_frame
	var amount := authored_control(client, "StackSplitText") as Label
	if amount == null or amount.text != "2":
		fail("Authored StackSplitText did not accept physical digit2")
		return false
	push_key(KEY_ENTER, true)
	await process_frame
	push_key(KEY_ENTER, false)
	if not await wait_split_frame(client, false) or not await wait_cursor(client, true):
		return false
	if not await wait_slot_render(client, 0, 0, 3, true):
		return false
	if not inventory_matches(client, [[0, 0, 3]]):
		fail("Split picker/Enter locally changed authoritative source")
		return false
	# Cursor amount2 is proved by the decoded exact SplitItem, not a test UI setter.
	print("FIXTURE BAGS_CURSOR_SPLIT_ARM")
	await process_frame
	if not await click_slot(client, SPLIT_SLOT):
		return false
	if not await wait_inventory(client, [[0, 0, 1], [1, 1, 2]]) or not await wait_cursor(client, false):
		return false
	return await wait_slot_render(client, 0, 0, 1, false) and await wait_slot_render(client, 1, 1, 2, false)

func click_slot(client: Node, control_name: String, shift: bool = false) -> bool:
	var control := await wait_bag_button(client, control_name)
	if control == null:
		return false
	var point := control.get_global_rect().get_center()
	cursor_pointer = point
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.shift_pressed = shift
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await process_frame
	return true

func push_digit_two() -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_2
		event.physical_keycode = KEY_2
		event.unicode = 50
		event.pressed = pressed
		root.push_input(event, true)

func wait_cursor(client: Node, visible: bool) -> bool:
	var deadline := Time.get_ticks_msec() + BAG_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var icon := authored_control(client, CURSOR)
		var shown := icon != null and icon.is_visible_in_tree()
		if shown != visible:
			continue
		if not visible:
			return true
		var texture := rendered_texture(icon)
		if texture == null:
			continue
		var rect := icon.get_global_rect()
		if rect.has_area() and rect.get_center().distance_to(cursor_pointer) <= POSITION_TOLERANCE:
			return true
	fail("RED: missing/incorrect CursorItemIcon after actual slot input; expected visible=%s, textured and centered on pointer" % visible)
	return false

func wait_split_frame(client: Node, visible: bool) -> bool:
	var deadline := Time.get_ticks_msec() + BAG_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var frame := authored_control(client, SPLIT_FRAME)
		if (frame != null and frame.is_visible_in_tree()) == visible:
			return true
	fail("RED: authored StackSplitFrame visibility did not become %s" % visible)
	return false

func inventory_matches(client: Node, expected: Array) -> bool:
	var items: Array = client.merchant_state().bags
	if items.size() != expected.size():
		return false
	for wanted in expected:
		var matches := 0
		for item in items:
			if item.bag == wanted[0] and item.slot == wanted[1] and item.item_id == 755 and item.count == wanted[2]:
				matches += 1
		if matches != 1:
			return false
	return true

func wait_inventory(client: Node, expected: Array) -> bool:
	var deadline := Time.get_ticks_msec() + INVENTORY_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if inventory_matches(client, expected):
			return true
	fail("Cursor authoritative inventory mismatch: expected=%s observed=%s" % [expected, client.merchant_state().bags])
	return false

func quiet_inventory(client: Node, expected: Array) -> bool:
	var deadline := Time.get_ticks_msec() + CURSOR_QUIET_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var cursor := authored_control(client, CURSOR)
		var cursor_shown := cursor != null and cursor.is_visible_in_tree()
		if cursor_shown or not inventory_matches(client, expected) or not containers_remain_open(client):
			fail("Cursor quiet phase revived cursor, changed inventory or closed bags/opened menu/merchant")
			return false
	return true

func containers_remain_open(client: Node) -> bool:
	if client.get_node_or_null("GameMenuUI") != null or client.get_node_or_null("MerchantUI") != null:
		return false
	for index in [0, 1]:
		var container := authored_control(client, "ContainerFrame%s" % index)
		if container == null or not container.is_visible_in_tree():
			return false
	return true

func rendered_texture(control: Control) -> TextureRect:
	if control == null:
		return null
	var direct := control as TextureRect
	if direct != null and direct.is_visible_in_tree() and direct.texture != null:
		return direct
	for descendant in control.find_children("*", "TextureRect", true, false):
		var texture := descendant as TextureRect
		if texture.is_visible_in_tree() and texture.texture != null:
			return texture
	return null

func slot_render_matches(client: Node, bag: int, slot: int, count: int, locked: bool) -> bool:
	var prefix := "ContainerFrame%sSlot%s" % [bag, slot]
	var icon := authored_control(client, prefix + "Icon")
	var texture := rendered_texture(icon)
	var label := authored_control(client, prefix + "Count") as Label
	if count == 0:
		return texture == null and (label == null or not label.is_visible_in_tree())
	if texture == null:
		return false
	# Projection places ImagePart vertex color on TextureRect.self_modulate.
	var tint := texture.self_modulate
	var ancestor: Node = texture
	while ancestor != null and ancestor != client:
		if ancestor is CanvasItem:
			tint *= (ancestor as CanvasItem).modulate
		ancestor = ancestor.get_parent()
	var expected_rgb := 0.5 if locked else 1.0
	if absf(tint.r - expected_rgb) > COLOR_TOLERANCE or absf(tint.g - expected_rgb) > COLOR_TOLERANCE or absf(tint.b - expected_rgb) > COLOR_TOLERANCE or absf(tint.a - 1.0) > COLOR_TOLERANCE:
		return false
	if count == 1:
		return label == null or not label.is_visible_in_tree()
	return label != null and label.is_visible_in_tree() and label.text == str(count)

func wait_slot_render(client: Node, bag: int, slot: int, count: int, locked: bool) -> bool:
	var deadline := Time.get_ticks_msec() + BAG_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if slot_render_matches(client, bag, slot, count, locked):
			return true
	fail("Cursor slot renderer mismatch bag%s/slot%s count=%s locked=%s" % [bag, slot, count, locked])
	return false
