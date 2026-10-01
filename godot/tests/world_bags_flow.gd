extends "res://tests/world_menu_flow.gd"

# Owned UDP inventory only; all visibility changes come from authored controls/input.
const BAG_WAIT_MS := 5000
const INVENTORY_WAIT_MS := 10000
const BACKPACK := "MainMenuBarBackpackButton"
const BAG_ONE := "CharacterBag0Slot"
const CONTAINER_RIGHT := 16.0
const CONTAINER_BOTTOM := 96.0
const POSITION_TOLERANCE := 2.0
const EXPECTED_ITEMS := [
	[0, 0, 755, "Melted Candle", 3],
	[0, 1, 2589, "Linen Cloth", 3],
	[0, 2, 4865, "Ruined Pelt", 1],
	[1, 7, 755, "Melted Candle", 2],
]

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
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
	await right_click_control(backpack)
	for frame in range(8):
		await process_frame
		if not bags_closed(client):
			fail("RED: right backpack click toggled bags; original toggle_bag_frame accepts left press only")
			return
	await click(backpack)
	if not await wait_container(client, 0, true):
		return
	if not check_container(client, 0, 16, 0, "3") or not check_solo_position(client, 0):
		return
	if not await check_item_hover_tooltips(client):
		return
	backpack = await wait_bag_button(client, BACKPACK)
	if backpack == null:
		return
	await click(backpack)
	if not await wait_container(client, 0, false):
		return
	if not await wait_item_tooltip_hidden(client):
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
	if first.intersects(second) or absf(root.get_visible_rect().end.y - first.end.y - CONTAINER_BOTTOM) > POSITION_TOLERANCE or absf(first.position.y - second.end.y - 8.0) > POSITION_TOLERANCE:
		fail("Authored standalone bags must stack by bag index with the original eight-unit gap")
		return
	if not await capture_bags():
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

func right_click_control(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	Input.warp_mouse(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_RIGHT
		event.pressed = pressed
		event.position = point
		event.global_position = point
		Input.parse_input_event(event)
		await process_frame

func capture_bags(filename: String = "standalone-bags.png", tooltip_client: Node = null, tooltip_title: String = "") -> bool:
	var directory := OS.get_environment("GODOT_BAGS_SCREENSHOTS")
	if directory.is_empty():
		return true
	if not directory.contains("/data/diagnostics/"):
		fail("Bag capture path must be persistent owned diagnostics")
		return false
	var error := DirAccess.make_dir_recursive_absolute(directory)
	if error != OK:
		fail("Cannot create bag capture directory: %s" % error)
		return false
	await RenderingServer.frame_post_draw
	if tooltip_client != null:
		var panel := authored_control(tooltip_client, "TooltipFrame")
		var title := authored_control(tooltip_client, "TooltipTitle") as Label
		if panel == null or not panel.is_visible_in_tree() or title == null or not title.is_visible_in_tree() or title.text != tooltip_title:
			fail("Item tooltip must remain shown in capture: " + filename)
			return false
	error = root.get_texture().get_image().save_png(directory.path_join(filename))
	if error != OK:
		fail("Cannot save authored bag capture: %s" % error)
		return false
	return true

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
		if bag_inventory_matches(client):
			return true
	fail("Bags authoritative inventory must remain exactly %s; observed=%s" % [EXPECTED_ITEMS, client.merchant_state().bags])
	return false

func bag_inventory_matches(client: Node) -> bool:
	var items: Array = client.merchant_state().bags
	if items.size() != EXPECTED_ITEMS.size():
		return false
	for wanted in EXPECTED_ITEMS:
		var matches := 0
		for item in items:
			if item.bag == wanted[0] and item.slot == wanted[1] and item.item_id == wanted[2] and item.name == wanted[3] and item.count == wanted[4]:
				matches += 1
		if matches != 1:
			return false
	return true

func check_item_hover_tooltips(client: Node) -> bool:
	var backpack_rect := authored_control(client, "ContainerFrame0").get_global_rect()
	# Names below are the original tooltip screen's desired native contract, not
	# existing native controls. Physical motion must produce the mounted projection.
	if not await hover_bag_slot(client, 1) or not await wait_item_tooltip(client, "Linen Cloth", Color(1.0, 1.0, 1.0, 1.0), "39", "Item ID: 2589"):
		return false
	if not await quiet_tooltip_inventory(client, true, backpack_rect):
		return false
	if not await capture_bags("standalone-bags-linen-tooltip.png", client, "Linen Cloth"):
		return false
	if not await hover_bag_slot(client, 2) or not await wait_item_tooltip(client, "Ruined Pelt", Color(0.62, 0.62, 0.62, 1.0), "5", "Item ID: 4865"):
		return false
	if not await quiet_tooltip_inventory(client, true, backpack_rect):
		return false
	if not await capture_bags("standalone-bags-poor-tooltip.png", client, "Ruined Pelt"):
		return false
	if not await hover_bag_slot(client, 3) or not await wait_item_tooltip_hidden(client):
		return false
	if not await quiet_tooltip_inventory(client, true, backpack_rect):
		return false
	# Re-show before leaving the whole bag, so away-hide is not a vacuous check.
	if not await hover_bag_slot(client, 1) or not await wait_item_tooltip(client, "Linen Cloth", Color.WHITE, "39", "Item ID: 2589"):
		return false
	await move_tooltip_pointer(Vector2(64.0, 64.0))
	if not await wait_item_tooltip_hidden(client) or not await quiet_tooltip_inventory(client, true, backpack_rect):
		return false
	# Close via keyboard while the captured physical pointer remains on the item.
	if not await hover_bag_slot(client, 1) or not await wait_item_tooltip(client, "Linen Cloth", Color.WHITE, "39", "Item ID: 2589"):
		return false
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_container(client, 0, false) or not await wait_item_tooltip_hidden(client):
		return false
	if not await quiet_tooltip_inventory(client, false, backpack_rect):
		return false
	# Restore the original flow before its existing backpack-close click.
	var backpack := await wait_bag_button(client, BACKPACK)
	if backpack == null:
		return false
	await click(backpack)
	if not await wait_container(client, 0, true):
		return false
	if not check_container(client, 0, 16, 0, "3") or not check_solo_position(client, 0):
		return false
	# Re-show before the original backpack-close click too.
	if not await hover_bag_slot(client, 1) or not await wait_item_tooltip(client, "Linen Cloth", Color.WHITE, "39", "Item ID: 2589"):
		return false
	print("BAGS TOOLTIP title/quality, stack price, placement, empty/away/close hide and unchanged authority checked")
	return await quiet_tooltip_inventory(client, true, backpack_rect)

func hover_bag_slot(client: Node, slot: int) -> bool:
	var control := authored_control(client, "ContainerFrame0Slot%s" % slot)
	if control == null or not control.is_visible_in_tree() or not control.get_global_rect().has_area():
		fail("Missing real authored backpack hover slot: %s" % slot)
		return false
	await move_tooltip_pointer(control.get_global_rect().get_center())
	return true

func move_tooltip_pointer(point: Vector2) -> void:
	Input.warp_mouse(point)
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame

func wait_item_tooltip(client: Node, title_text: String, title_color: Color, copper: String, item_id_line: String) -> bool:
	var deadline := Time.get_ticks_msec() + BAG_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var panel := authored_control(client, "TooltipFrame")
		var title := authored_control(client, "TooltipTitle") as Label
		if panel == null or not panel.is_visible_in_tree() or title == null or not title.is_visible_in_tree() or title.text != title_text:
			continue
		if not title.get_theme_color("font_color").is_equal_approx(title_color):
			fail("Original item tooltip quality color for %s: expected=%s observed=%s" % [title_text, title_color, title.get_theme_color("font_color")])
			return false
		for expected in [["TooltipLine0Left", "Sell Price:"], ["TooltipLine0MoneyAmount0", copper], ["TooltipLine1Left", item_id_line]]:
			var label := authored_control(client, expected[0]) as Label
			if label == null or not label.is_visible_in_tree() or label.text != expected[1]:
				fail("Original item tooltip %s must show %s" % [expected[0], expected[1]])
				return false
		var coin := authored_control(client, "TooltipLine0MoneyCoin0")
		if coin == null or not coin.is_visible_in_tree():
			fail("Original stack sell price requires its authored copper coin")
			return false
		for unexpected in ["TooltipLine0MoneyAmount1", "TooltipLine2Left"]:
			var extra := authored_control(client, unexpected)
			if extra != null and extra.is_visible_in_tree():
				fail("Simple item tooltip has unexpected extra money/line: " + unexpected)
				return false
		var owner_slot := 1 if title_text == "Linen Cloth" else 2
		if not check_item_tooltip_rect(client, panel, owner_slot):
			return false
		print("BAGS TOOLTIP observed title=", title_text, " color=", title.get_theme_color("font_color"), " copper=", copper, " id=", item_id_line)
		return true
	fail("RED: physical authored bag hover did not project original TooltipFrame/TooltipTitle for " + title_text)
	return false

func check_item_tooltip_rect(client: Node, panel: Control, owner_slot: int) -> bool:
	var owner := authored_control(client, "ContainerFrame0Slot%s" % owner_slot)
	if owner == null or not owner.is_visible_in_tree():
		fail("Tooltip placement requires its visible authored slot owner")
		return false
	# Original tooltip_frame/mod.rs: width260; max(34, 2*8 + 16 + 2*14).
	# Sell Price and the appended Item ID are exactly two lines for both items.
	# Convert logical units using the owner's canvas transform, not tooltip size.
	var transform := owner.get_global_transform()
	var scale := Vector2(transform.x.length(), transform.y.length())
	if scale.x <= 0.0 or not is_equal_approx(scale.x, scale.y):
		fail("Authored bag owner must have positive uniform logical UI scale")
		return false
	var expected_size := Vector2(260.0, maxf(34.0, 2.0 * 8.0 + 16.0 + 2.0 * 14.0)) * scale
	var viewport := root.get_visible_rect()
	var owner_rect := owner.get_global_rect()
	if owner_rect.end.x < viewport.get_center().x:
		fail("Fixture tooltip owners must exercise original right-edge left placement")
		return false
	# Original owner_anchor_position: left of owner, above owner; then clamp.
	var expected_position := owner_rect.position - expected_size
	expected_position.x = clampf(expected_position.x, viewport.position.x, maxf(viewport.position.x, viewport.end.x - expected_size.x))
	expected_position.y = clampf(expected_position.y, viewport.position.y, maxf(viewport.position.y, viewport.end.y - expected_size.y))
	var observed := panel.get_global_rect()
	if not viewport.encloses(observed):
		fail("Full item tooltip panel must fit viewport: panel=%s viewport=%s" % [observed, viewport])
		return false
	if absf(observed.size.x - expected_size.x) > POSITION_TOLERANCE or absf(observed.size.y - expected_size.y) > POSITION_TOLERANCE or absf(observed.position.x - expected_position.x) > POSITION_TOLERANCE or absf(observed.position.y - expected_position.y) > POSITION_TOLERANCE:
		fail("Original left/above/clamped tooltip geometry: expected=%s observed=%s owner=%s scale=%s" % [Rect2(expected_position, expected_size), observed, owner_rect, scale])
		return false
	return true

func wait_item_tooltip_hidden(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + BAG_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var panel := authored_control(client, "TooltipFrame")
		if panel != null and panel.is_visible_in_tree():
			continue
		for frame in range(8):
			await process_frame
			# Include coins, extra denominations, right text, ID and any other
			# descendants, even if they lack the authored Tooltip name prefix.
			var controls := client.find_children("Tooltip*", "Control", true, false)
			var current_panel := authored_control(client, "TooltipFrame")
			if current_panel != null:
				controls.append_array(current_panel.find_children("*", "Control", true, false))
			for control in controls:
				if control.is_visible_in_tree():
					fail("Item tooltip remained/reappeared visible after empty/away/close: " + str(control.name))
					return false
		return true
	fail("RED: item tooltip did not hide after empty/away/close")
	return false

func quiet_tooltip_inventory(client: Node, backpack_open: bool, backpack_rect: Rect2) -> bool:
	for frame in range(8):
		await process_frame
		var state: Dictionary = client.merchant_state()
		var cursor_icon := authored_control(client, "CursorItemIcon")
		var popup := authored_control(client, "StaticPopup1")
		if not bag_inventory_matches(client) or state.cursor != "" or state.split_open or (cursor_icon != null and cursor_icon.is_visible_in_tree()) or (popup != null and popup.is_visible_in_tree()):
			fail("Item hover changed exact authoritative inventory/cursor/split/popup: %s" % state)
			return false
		var container := authored_control(client, "ContainerFrame0")
		var other := authored_control(client, "ContainerFrame1")
		if container == null or container.is_visible_in_tree() != backpack_open or (other != null and other.is_visible_in_tree()) or client.get_node_or_null("GameMenuUI") != null or client.get_node_or_null("MerchantUI") != null:
			fail("Item hover changed standalone container/unrelated-window visibility")
			return false
		if backpack_open and container.get_global_rect() != backpack_rect:
			fail("Item hover changed authored backpack geometry")
			return false
	return true

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
