extends "res://tests/world_merchant_click_flow.gd"

# Source oracles: tooltip_frame/mod.rs merchant_tooltip + owner_anchor_position;
# shared item_tooltip.rs and local ItemSparse4865 SellPrice5, quality0.
# Vendor/buyback names and quality intentionally differ from ItemSparse, proving
# peer cell content is not accidentally replaced by the catalog formatter.
const MT_WAIT_MS := 6000
const MT_QUIET_MS := 900
const MT_TOLERANCE := 2.0
const MT_VENDOR_ITEMS := ["Fixture Linen Bundle", "Fixture Single Pelt"]
const MT_BUYBACK_ITEMS := ["Fixture Returned Linen", "Fixture Returned Pelt"]
var mt_npc: Variant
var mt_bag_rect := Rect2()
var mt_frame_rect := Rect2()
var mt_pointer := Vector2.ZERO
var mt_refreshed := false
var mt_buyback := false

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/diagnostics/native-input-"):
		fail("Merchant tooltips require owned UDP endpoint and isolated root defaults")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 180000):
		return
	print("FIXTURE MERCHANT_TOOLTIPS_LOADING")
	if not await wait_world(client):
		return
	var account: Dictionary = client.account_state()
	if not account.reply_received or account.character_count != 3:
		fail("Merchant tooltips require authenticated selected fixture roster")
		return
	print("FIXTURE MERCHANT_TOOLTIPS_READY")
	var vendor := await find_vendor(client)
	if vendor.is_empty() or not await open_vendor(client, vendor):
		return
	mt_npc = vendor.id
	var bag := mt_control(client, "ContainerFrame0")
	var frame := mt_control(client, "MerchantFrame")
	if bag == null or frame == null or not bag.is_visible_in_tree() or not frame.is_visible_in_tree():
		fail("Tooltip setup requires actual embedded merchant bag/frame")
		return
	mt_bag_rect = bag.get_global_rect()
	mt_frame_rect = frame.get_global_rect()
	if not await mt_wait_authority(client):
		return
	print("FIXTURE MERCHANT_TOOLTIPS_VENDOR_OPEN")
	if not await mt_vendor_hovers(client) or not await mt_buyback_hovers(client) or not await mt_bag_hovers(client):
		return
	print("FIXTURE MERCHANT_TOOLTIPS_DONE")
	# Parent owns forced kill/reap/readers drain. Never claim normal shutdown.
	while true:
		await process_frame
		if not mt_authority(client, false) or not mt_hidden(client):
			fail("Tooltip terminal close drain changed")
			return

func open_vendor(client: Node, vendor: Dictionary) -> bool:
	pointer_at(vendor.point, MOUSE_BUTTON_RIGHT, true)
	await process_frame
	if client.target_state().target != vendor.id or client.target_state().auto_attack != null:
		fail("Tooltip vendor right-click targeted an attack rather than owned interaction")
		return false
	pointer_at(vendor.point, MOUSE_BUTTON_RIGHT, false)
	var deadline := Time.get_ticks_msec() + MT_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.merchant_state()
		if state.open and state.npc == vendor.id and state.vendor_name == VENDOR and state.items == MT_VENDOR_ITEMS and state.buyback == MT_BUYBACK_ITEMS:
			return true
	fail("Tooltip setup did not receive seeded owned vendor/buyback: " + str(client.merchant_state()))
	return false

func mt_vendor_hovers(client: Node) -> bool:
	var bundle := mt_expected("Fixture Linen Bundle", Color(0.12, 1.0, 0.0, 1.0), [["Stack Count", "5"], ["In Stock", "7"], ["Item ID: 2589", ""]])
	if not await mt_hover(client, "MerchantItem1", bundle, "VENDOR"):
		return false
	var single := mt_expected("Fixture Single Pelt", Color(0.62, 0.62, 0.62, 1.0), [["Item ID: 4865", ""]])
	if not await mt_hover(client, "MerchantItem2", single, "SINGLE"):
		return false
	if not await mt_hover(client, "MerchantItem3", {}, "EMPTY_CELL"):
		return false
	# Re-show before peer refresh; same physical pointer, not a synthetic callback.
	if not await mt_hover(client, "MerchantItem1", bundle, "REFRESH_ARM"):
		return false
	mt_refreshed = true
	bundle.lines = [["Stack Count", "2"], ["In Stock", "0"], ["Item ID: 2589", ""]]
	return await mt_expect_stable(client, "MerchantItem1", bundle, true, "REFRESHED")

func mt_buyback_hovers(client: Node) -> bool:
	if not await mt_tab(client, "MerchantFrameTab2", "Buyback"):
		return false
	var returned := mt_expected("Fixture Returned Linen", Color(0.64, 0.21, 0.93, 1.0), [["Stack Count", "3"], ["Item ID: 2589", ""]])
	if not await mt_hover(client, "MerchantItem1", returned, "BUYBACK"):
		return false
	var single := mt_expected("Fixture Returned Pelt", Color.WHITE, [["Item ID: 4865", ""]])
	if not await mt_hover(client, "MerchantItem2", single, "BUYBACK_SINGLE"):
		return false
	return await mt_hover(client, "MerchantItem3", {}, "BUYBACK_EMPTY")

func mt_bag_hovers(client: Node) -> bool:
	# Original hovered_item_tooltip isn't limited to the Merchant tab.
	var bag := mt_expected("Ruined Pelt", Color(0.62, 0.62, 0.62, 1.0), [["Sell Price:", ""], ["Item ID: 4865", ""]])
	bag.copper = "10"
	bag.bag = true
	if not await mt_hover(client, "ContainerFrame0Slot0", bag, "BAG"):
		return false
	if not await mt_hover(client, "ContainerFrame0Slot1", {}, "BAG_EMPTY"):
		return false
	# Re-show before leaving controls; away-hide cannot pass vacuously.
	if not await mt_move_to(client, "ContainerFrame0Slot0") or not await mt_wait_projection(client, "ContainerFrame0Slot0", bag, true):
		return false
	await mt_motion(Vector2(64.0, 64.0))
	if not await mt_expect_stable(client, "", {}, true, "AWAY"):
		return false
	if not await mt_tab(client, "MerchantFrameTab1", "Merchant"):
		return false
	# No invented SellJunk hover text or unchecked per-item Repair contract.
	if not await mt_hover(client, "MerchantSellAllJunkButton", {}, "SERVICE"):
		return false
	# Peer closes after900ms visible bag hover, with pointer stationary on it.
	if not await mt_hover(client, "ContainerFrame0Slot0", bag, "CLOSE_ARM"):
		return false
	return await mt_expect_stable(client, "", {}, false, "")

func mt_expected(title: String, color: Color, lines: Array) -> Dictionary:
	return {"title": title, "color": color, "lines": lines, "copper": "", "bag": false}

func mt_control(client: Node, name: String) -> Control:
	var ui := client.get_node_or_null("MerchantUI")
	return ui.find_child(name, true, false) as Control if ui != null else null

func mt_move_to(client: Node, owner_name: String) -> bool:
	var owner := mt_control(client, owner_name)
	if owner == null or not owner.is_visible_in_tree() or not owner.get_global_rect().has_area():
		fail("Tooltip setup missing actual visible merchant owner " + owner_name)
		return false
	await mt_motion(owner.get_global_rect().get_center())
	return true

func mt_motion(point: Vector2) -> void:
	mt_pointer = point
	Input.warp_mouse(point)
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	root.push_input(event, true)
	await process_frame

func mt_tab(client: Node, name: String, wanted: String) -> bool:
	var tab := mt_control(client, name)
	if tab == null or not tab.is_visible_in_tree():
		fail("Missing authored merchant tab " + name)
		return false
	var point := tab.get_global_rect().get_center()
	pointer_at(point, MOUSE_BUTTON_LEFT, true)
	await process_frame
	pointer_at(point, MOUSE_BUTTON_LEFT, false)
	var deadline := Time.get_ticks_msec() + MT_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var title := mt_control(client, "MerchantFrameTitleText") as Label
		var expected_title := "Merchant Buyback" if wanted == "Buyback" else VENDOR
		if title != null and title.is_visible_in_tree() and title.text == expected_title:
			mt_buyback = wanted == "Buyback"
			return true
	fail("Authored tab didn't select " + wanted)
	return false

func mt_hover(client: Node, owner: String, expected: Dictionary, marker: String) -> bool:
	return await mt_move_to(client, owner) and await mt_expect_stable(client, owner, expected, true, marker)

func mt_wait_authority(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + MT_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if mt_authority(client, true):
			return true
	fail("Tooltip setup seed/authority mismatch: " + str(client.merchant_state()))
	return false

func mt_expect_stable(client: Node, owner: String, expected: Dictionary, open: bool, marker: String) -> bool:
	if not await mt_wait_projection(client, owner, expected, open):
		return false
	var deadline := Time.get_ticks_msec() + MT_QUIET_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if not mt_authority(client, open) or not mt_projection(client, owner, expected):
			fail("Tooltip content/geometry/authority changed during900ms: owner=%s expected=%s state=%s" % [owner, expected, client.merchant_state()])
			return false
	if not marker.is_empty():
		print("MERCHANT TOOLTIPS OBSERVED owner=", owner, " expected=", expected, " pointer=", mt_pointer, " stable_ms=", MT_QUIET_MS)
		print("FIXTURE MERCHANT_TOOLTIPS_" + marker)
	return true

func mt_wait_projection(client: Node, owner: String, expected: Dictionary, open: bool) -> bool:
	var deadline := Time.get_ticks_msec() + MT_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if mt_authority(client, open) and mt_projection(client, owner, expected):
			return true
	fail("Merchant tooltip behavioral boundary: physical hover/display/placement/hide missing; owner=%s expected=%s state=%s" % [owner, expected, client.merchant_state()])
	return false

func mt_authority(client: Node, open: bool) -> bool:
	var state: Dictionary = client.merchant_state()
	if state.open != open or state.money != 1000 or state.split_open or state.bags.size() != 1:
		return false
	var item: Dictionary = state.bags[0]
	if item.bag != 0 or item.slot != 0 or item.item_id != 4865 or item.count != 2 or item.name != "Ruined Pelt":
		return false
	for name in ["CursorItemIcon", "StackSplitFrame", "StaticPopup1"]:
		for node in client.find_children(name, "Control", true, false):
			if node.is_visible_in_tree():
				return false
	if client.get_node_or_null("GameMenuUI") != null:
		return false
	if not open:
		for name in ["MerchantFrame", "ContainerFrame0"]:
			for node in client.find_children(name, "Control", true, false):
				if node.is_visible_in_tree():
					return false
		return true
	var bag := mt_control(client, "ContainerFrame0")
	var frame := mt_control(client, "MerchantFrame")
	if state.npc != mt_npc or state.items != MT_VENDOR_ITEMS or state.buyback != MT_BUYBACK_ITEMS or bag == null or frame == null:
		return false
	if not bag.is_visible_in_tree() or bag.get_global_rect() != mt_bag_rect or frame.get_global_rect() != mt_frame_rect:
		return false
	var title := mt_control(client, "MerchantFrameTitleText") as Label
	if title == null or not title.is_visible_in_tree() or title.text != ("Merchant Buyback" if mt_buyback else VENDOR):
		return false
	for name in ["MerchantItem1ItemButtonIcon", "MerchantItem2ItemButtonIcon", "ContainerFrame0Slot0Icon"]:
		var icon := mt_texture(mt_control(client, name))
		if icon == null:
			return false
	if not mt_buyback:
		var count := mt_control(client, "MerchantItem1ItemButtonCount") as Label
		var stock := mt_control(client, "MerchantItem1ItemButtonStock") as Label
		return count != null and stock != null and count.is_visible_in_tree() and stock.is_visible_in_tree() and count.text == ("2" if mt_refreshed else "5") and stock.text == ("(0)" if mt_refreshed else "(7)")
	return true

func mt_hidden(client: Node) -> bool:
	for node in client.find_children("Tooltip*", "Control", true, false):
		if node.is_visible_in_tree():
			return false
	return true

func mt_projection(client: Node, owner: String, expected: Dictionary) -> bool:
	if expected.is_empty():
		return mt_hidden(client)
	var panels := client.find_children("TooltipFrame", "Control", true, false)
	var visible: Array[Control] = []
	for panel in panels:
		if panel.is_visible_in_tree():
			visible.append(panel)
	if visible.size() != 1:
		return false
	var panel := visible[0]
	var host := panel.get_parent()
	var title := host.find_child("TooltipTitle", true, false) as Label
	if title == null or not title.is_visible_in_tree() or title.text != expected.title or not title.get_theme_color("font_color").is_equal_approx(expected.color):
		return false
	for index in range(expected.lines.size()):
		for side in [0, 1]:
			var label := host.find_child("TooltipLine%d%s" % [index, "Left" if side == 0 else "Right"], true, false) as Label
			if label == null or not label.is_visible_in_tree() or label.text != expected.lines[index][side]:
				return false
			var color := Color(0.92, 0.89, 0.82, 1.0)
			if side == 0:
				if expected.lines[index][0].begins_with("Item ID:"):
					color = Color(0.5, 0.5, 0.5, 1.0)
				elif expected.bag:
					color = Color.WHITE
				else:
					color = Color(0.72, 0.72, 0.72, 1.0)
			if not label.get_theme_color("font_color").is_equal_approx(color):
				return false
	var extra := host.find_child("TooltipLine%dLeft" % expected.lines.size(), true, false) as Control
	if extra != null and extra.is_visible_in_tree():
		return false
	var amount := host.find_child("TooltipLine0MoneyAmount0", true, false) as Label
	if not expected.copper.is_empty():
		var coin := mt_texture(host.find_child("TooltipLine0MoneyCoin0", true, false) as Control)
		if amount == null or not amount.is_visible_in_tree() or amount.text != expected.copper or coin == null:
			return false
	elif amount != null and amount.is_visible_in_tree():
		return false
	return mt_geometry(client, panel, owner, expected)

func mt_texture(control: Control) -> TextureRect:
	if control == null or not control.is_visible_in_tree():
		return null
	for node in control.find_children("*", "TextureRect", true, false):
		var texture := node as TextureRect
		if texture.is_visible_in_tree() and texture.texture != null:
			return texture
	return null

func mt_geometry(client: Node, panel: Control, owner_name: String, expected: Dictionary) -> bool:
	var owner := mt_control(client, owner_name)
	if owner == null or not owner.is_visible_in_tree():
		return false
	var scale := owner.get_global_transform().get_scale()
	if scale.x <= 0.0 or not is_equal_approx(scale.x, scale.y):
		return false
	# Retail GameTooltip fits its widest line (game_tooltip/render.rs tooltip_size).
	var rect: PackedFloat32Array = client.tooltip_state().rect
	var size := Vector2(rect[2], rect[3]) * scale
	var owner_rect := owner.get_global_rect()
	var viewport := root.get_visible_rect()
	# Existing merchant ANCHOR_RIGHT: tooltip BOTTOMLEFT at owner's TOPRIGHT.
	var position := Vector2(owner_rect.end.x, owner_rect.position.y - size.y)
	if expected.bag and owner_rect.end.x >= viewport.get_center().x:
		position.x = owner_rect.position.x - size.x
	position.x = clampf(position.x, viewport.position.x, maxf(viewport.position.x, viewport.end.x - size.x))
	position.y = clampf(position.y, viewport.position.y, maxf(viewport.position.y, viewport.end.y - size.y))
	var observed := panel.get_global_rect()
	return viewport.encloses(observed) and observed.position.distance_to(position) <= MT_TOLERANCE and observed.size.distance_to(size) <= MT_TOLERANCE
