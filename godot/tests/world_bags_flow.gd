extends "res://tests/world_menu_flow.gd"

# Owned UDP inventory only; all visibility changes come from authored controls/input.
const BAG_WAIT_MS := 5000
const INVENTORY_WAIT_MS := 10000
const BACKPACK := "MainMenuBarBackpackButton"
const BAG_ONE := "CharacterBag0Slot"
const CONTAINER_RIGHT := 16.0
const CONTAINER_BOTTOM := 96.0
const POSITION_TOLERANCE := 2.0

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/diagnostics/native-input-"):
		fail("Bags fixture requires owned loopback peer and isolated canonical defaults")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 180000):
		return
	print("FIXTURE BAGS_LOADING")
	if not await wait_world(client):
		return
	var account: Dictionary = client.account_state()
	if not account.reply_received or account.character_count != 3:
		fail("Bags world did not originate from authenticated fixture roster")
		return
	print("FIXTURE BAGS_READY")
	if not await wait_bag_inventory(client):
		return
	if not bags_closed(client):
		fail("Standalone bags must start closed, without merchant or menu")
		return
	var backpack := await wait_bag_button(client, BACKPACK)
	if backpack == null:
		return
	await click(backpack)
	if not await wait_container(client, 0, true):
		return
	if not check_container(client, 0, 16, 0, "3") or not check_solo_position(client, 0):
		return
	backpack = await wait_bag_button(client, BACKPACK)
	if backpack == null:
		return
	await click(backpack)
	if not await wait_container(client, 0, false):
		return
	var bag_one := await wait_bag_button(client, BAG_ONE)
	if bag_one == null:
		return
	await click(bag_one)
	if not await wait_container(client, 1, true):
		return
	if not check_container(client, 1, 8, 7, "2") or not check_solo_position(client, 1):
		return
	# Re-find after UI synchronization; do not retain a potentially rebuilt control.
	backpack = await wait_bag_button(client, BACKPACK)
	if backpack == null:
		return
	await click(backpack)
	if not await wait_container(client, 0, true) or not await wait_container(client, 1, true):
		return
	var first := authored_control(client, "ContainerFrame0").get_global_rect()
	var second := authored_control(client, "ContainerFrame1").get_global_rect()
	print("BAGS POSITIONS together bag0=", first, " bag1=", second)
	if first.intersects(second):
		fail("Authored standalone bag containers overlap")
		return
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_container(client, 0, false) or not await wait_container(client, 1, false):
		return
	for frame in range(8):
		await process_frame
		if not bags_closed(client):
			fail("Escape opened menu/merchant or reopened bags")
			return
	# Snapshot has no bag2: its authored HUD slot must not invent a container.
	var absent_bag := await wait_bag_button(client, "CharacterBag1Slot")
	if absent_bag == null:
		return
	await click(absent_bag)
	for frame in range(8):
		await process_frame
		var invalid := authored_control(client, "ContainerFrame2")
		if (invalid != null and invalid.is_visible_in_tree()) or not bags_closed(client):
			fail("Absent authoritative bag2 button invented/opened a container or menu")
			return
	if not await wait_bag_inventory(client):
		return
	print("FIXTURE BAGS_DONE")
	# Parent owns deliberate kill/reap; normal engine shutdown is not this test.
	while true:
		await process_frame

func authored_control(client: Node, control_name: String) -> Control:
	return client.find_child(control_name, true, false) as Control

func bags_closed(client: Node) -> bool:
	for index in [0, 1]:
		var container := authored_control(client, "ContainerFrame%s" % index)
		if container != null and container.is_visible_in_tree():
			return false
	return client.get_node_or_null("GameMenuUI") == null and client.get_node_or_null("MerchantUI") == null

func wait_bag_inventory(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + INVENTORY_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var items: Array = client.merchant_state().bags
		if items.size() != 2:
			continue
		var backpack_matches := false
		var bag_matches := false
		for item in items:
			backpack_matches = backpack_matches or (item.bag == 0 and item.slot == 0 and item.item_id == 755 and item.count == 3)
			bag_matches = bag_matches or (item.bag == 1 and item.slot == 7 and item.item_id == 755 and item.count == 2)
		if backpack_matches and bag_matches:
			return true
	fail("Bags authoritative inventory missing: expected candle bag0/slot0 x3 and bag1/slot7 x2")
	return false

func wait_bag_button(client: Node, control_name: String) -> Control:
	var deadline := Time.get_ticks_msec() + BAG_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var button := authored_control(client, control_name)
		if button != null and button.is_visible_in_tree() and button.get_global_rect().has_area():
			return button
	if control_name == BACKPACK:
		fail("RED: missing authored standalone backpack HUD: MainMenuBarBackpackButton not visible in real client host")
	else:
		fail("RED: missing authored equipped bag HUD: " + control_name)
	return null

func wait_container(client: Node, index: int, visible: bool) -> bool:
	var deadline := Time.get_ticks_msec() + BAG_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.get_node_or_null("GameMenuUI") != null or client.get_node_or_null("MerchantUI") != null:
			fail("Standalone bag action opened menu/merchant")
			return false
		var container := authored_control(client, "ContainerFrame%s" % index)
		var observed := container != null and container.is_visible_in_tree()
		if observed == visible:
			return true
	fail("RED: authored ContainerFrame%s visibility did not become %s after physical input" % [index, visible])
	return false

func check_container(client: Node, index: int, capacity: int, occupied_slot: int, count: String) -> bool:
	var container := authored_control(client, "ContainerFrame%s" % index)
	var prefix := "ContainerFrame%sSlot" % index
	for slot in range(capacity):
		if container.find_child(prefix + str(slot), true, false) == null:
			fail("Authored bag slot missing: " + prefix + str(slot))
			return false
	if container.find_child(prefix + str(capacity), true, false) != null:
		fail("Authored bag capacity exceeds authoritative snapshot")
		return false
	var item_prefix := prefix + str(occupied_slot)
	var icon := container.find_child(item_prefix + "Icon", true, false) as Control
	var label := container.find_child(item_prefix + "Count", true, false) as Label
	if icon == null or not icon.is_visible_in_tree() or label == null or not label.is_visible_in_tree() or label.text != count:
		fail("Authored candle card icon/count missing: " + item_prefix)
		return false
	return true

func check_solo_position(client: Node, index: int) -> bool:
	var rect := authored_control(client, "ContainerFrame%s" % index).get_global_rect()
	var viewport := root.get_visible_rect()
	var right := viewport.end.x - rect.end.x
	var bottom := viewport.end.y - rect.end.y
	print("BAGS POSITION solo=", index, " rect=", rect, " right=", right, " bottom=", bottom)
	if absf(right - CONTAINER_RIGHT) > POSITION_TOLERANCE or absf(bottom - CONTAINER_BOTTOM) > POSITION_TOLERANCE:
		fail("Standalone container must match original right16/bottom96 anchor")
		return false
	return true
