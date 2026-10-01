extends "res://tests/world_merchant_click_flow.gd"

# Independent mode: inherit real NPC discovery, never execute parent run_test.
const MC_WAIT_MS := 6000
const MC_QUIET_MS := 900
const MC_SOURCE := "MerchantItem1"
const MC_SOURCE_ICON := "MerchantItem1ItemButtonIcon"
const MC_SLOT := "ContainerFrame0Slot0"
const MC_CURSOR := "CursorItemIcon"

var mc_pointer := Vector2.ZERO

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/diagnostics/native-input-"):
		fail("Merchant cursor requires owned UDP endpoint and isolated root defaults")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 180000):
		return
	print("FIXTURE MERCHANT_CURSOR_LOADING")
	if not await wait_world(client):
		return
	var account: Dictionary = client.account_state()
	if not account.reply_received or account.character_count != 3:
		fail("Merchant cursor requires authenticated selected fixture roster")
		return
	print("FIXTURE MERCHANT_CURSOR_READY")
	var vendor := await find_vendor(client)
	if vendor.is_empty() or not await open_vendor(client, vendor):
		return
	if not await mc_wait_state(client, 0, false, 1000) or not mc_embedded_bag(client):
		return
	print("FIXTURE MERCHANT_CURSOR_VENDOR_OPEN")
	if not await mc_buy_drag(client):
		return
	print("FIXTURE MERCHANT_CURSOR_DONE")
	# Parent owns deliberate kill/reap/readers drain; not shutdown proof.
	while true:
		await process_frame

func open_vendor(client: Node, vendor: Dictionary) -> bool:
	# Parent predicate is Fixture Bread; this independent catalog is Linen Cloth.
	pointer_at(vendor.point, MOUSE_BUTTON_RIGHT, true)
	await process_frame
	if client.target_state().target != vendor.id or client.target_state().auto_attack != null:
		fail("Vendor right-click targeted attack rather than owned interaction")
		return false
	pointer_at(vendor.point, MOUSE_BUTTON_RIGHT, false)
	var deadline := Time.get_ticks_msec() + MC_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.merchant_state()
		if state.open and state.vendor_name == VENDOR and state.npc == vendor.id and state.items == ["Linen Cloth"]:
			var ui := client.get_node_or_null("MerchantUI")
			if ui != null and ui.find_child("MerchantFrame", true, false) != null:
				return true
	fail("Owned Linen vendor did not open: " + str(client.merchant_state()))
	return false

func mc_buy_drag(client: Node) -> bool:
	var source := mc_control(client, MC_SOURCE)
	var target := mc_control(client, MC_SLOT)
	var canvas := mc_control(client, "RegistryCanvas")
	if source == null or target == null or canvas == null:
		fail("Missing actual merchant vendor cell/backpack/canvas")
		return false
	var start := source.get_global_rect().get_center()
	var finish := target.get_global_rect().get_center()
	var scale := canvas.get_global_transform().get_scale().x
	if scale <= 0 or start.distance_to(finish) / scale < 4.0:
		fail("Merchant drag must travel >=4 logical px at actual canvas scale")
		return false
	print("MERCHANT CURSOR GEOMETRY source=", source.get_path(), " target=", target.get_path(), " start=", start, " finish=", finish, " scale=", scale)
	print("FIXTURE MERCHANT_CURSOR_PICKUP_ARM")
	mc_motion(start, false)
	await process_frame
	mc_edge(start, true)
	if not await mc_wait_state(client, 0, true, 1000):
		return false
	if not await mc_quiet_state(client, 0, true, 1000):
		return false
	print("MERCHANT CURSOR PICKUP source Linen unchanged, cursor textured/centered, bags=[] money=1000")
	print("FIXTURE MERCHANT_CURSOR_PICKED_UP")
	print("FIXTURE MERCHANT_CURSOR_DROP_ARM")
	# Press remains held across frames; this is a qualifying physical release.
	target = mc_control(client, MC_SLOT)
	if target == null or not target.get_global_rect().has_point(finish):
		fail("Embedded destination moved/disappeared during physical pickup")
		return false
	mc_motion(finish, true)
	await process_frame
	mc_edge(finish, false)
	if not await mc_wait_state(client, 0, false, 1000):
		return false
	if not await mc_quiet_state(client, 0, false, 1000):
		return false
	print("MERCHANT CURSOR COMMIT pre-delta bags=[] money=1000 cursor hidden")
	print("FIXTURE MERCHANT_CURSOR_COMMIT")
	if not await mc_wait_state(client, 1, false, 975):
		return false
	if not await mc_quiet_state(client, 1, false, 975):
		return false
	print("MERCHANT CURSOR FINAL bag0/slot0 Linen2589 count1 money975 rendered icon, cursor hidden; GUID only peer-visible")
	return true

func mc_control(client: Node, control_name: String) -> Control:
	var ui := client.get_node_or_null("MerchantUI")
	return ui.find_child(control_name, true, false) as Control if ui != null else null

func mc_embedded_bag(client: Node) -> bool:
	var ui := client.get_node_or_null("MerchantUI")
	var bag := mc_control(client, "ContainerFrame0")
	if ui == null or bag == null or not ui.is_ancestor_of(bag) or not bag.is_visible_in_tree():
		fail("Destination must be own MerchantUI embedded backpack")
		return false
	for slot in range(16):
		var control := bag.find_child("ContainerFrame0Slot%s" % slot, true, false) as Control
		if control == null or not control.is_visible_in_tree() or not control.get_global_rect().has_area():
			fail("Embedded authoritative bag0 capacity16 missing slot%s" % slot)
			return false
	if bag.find_child("ContainerFrame0Slot16", true, false) != null:
		fail("Embedded bag exceeds authoritative capacity16")
		return false
	return true

func mc_motion(point: Vector2, held: bool) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	event.relative = point - mc_pointer
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if held else 0
	mc_pointer = point
	root.push_input(event, true)

func mc_edge(point: Vector2, down: bool) -> void:
	mc_pointer = point
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
	event.pressed = down
	root.push_input(event, true)

func mc_texture(control: Control) -> TextureRect:
	if control == null:
		return null
	var direct := control as TextureRect
	if direct != null and direct.is_visible_in_tree() and direct.texture != null:
		return direct
	for node in control.find_children("*", "TextureRect", true, false):
		var texture := node as TextureRect
		if texture.is_visible_in_tree() and texture.texture != null:
			return texture
	return null

func mc_cursor_matches(client: Node, held: bool) -> bool:
	var icon := client.find_child(MC_CURSOR, true, false) as Control
	var shown := icon != null and icon.is_visible_in_tree()
	if shown != held:
		return false
	if not held:
		return true
	return mc_texture(icon) != null and icon.get_global_rect().has_area() and icon.get_global_rect().get_center().distance_to(mc_pointer) <= 2.0

func mc_source_matches(client: Node) -> bool:
	var name_label := mc_control(client, "MerchantItem1Name") as Label
	var count := mc_control(client, "MerchantItem1ItemButtonCount") as Label
	var price := mc_control(client, "MerchantItem1MoneyFrameAmount0") as Label
	var source := mc_control(client, MC_SOURCE)
	return source != null and source.is_visible_in_tree() and name_label != null and name_label.is_visible_in_tree() and name_label.text == "Linen Cloth" and mc_texture(mc_control(client, MC_SOURCE_ICON)) != null and (count == null or not count.is_visible_in_tree()) and price != null and price.is_visible_in_tree() and price.text == "25"

func mc_inventory_matches(state: Dictionary, count: int, money: int) -> bool:
	if state.money != money or state.bags.size() != count:
		return false
	if count == 0:
		return true
	var item: Dictionary = state.bags[0]
	return item.bag == 0 and item.slot == 0 and item.item_id == 2589 and item.name == "Linen Cloth" and item.count == 1

func mc_bag_render_matches(client: Node, count: int) -> bool:
	for slot in range(16):
		var prefix := "ContainerFrame0Slot%s" % slot
		var texture := mc_texture(mc_control(client, prefix + "Icon"))
		var label := mc_control(client, prefix + "Count") as Label
		var occupied := slot == 0 and count == 1
		if (texture != null) != occupied or (label != null and label.is_visible_in_tree()):
			return false
	return true

func mc_state_matches(client: Node, count: int, held: bool, money: int) -> bool:
	var state: Dictionary = client.merchant_state()
	if not state.open or state.items != ["Linen Cloth"] or state.split_open or client.get_node_or_null("GameMenuUI") != null:
		return false
	for control_name in ["StaticPopup1", "StackSplitFrame"]:
		var control := client.find_child(control_name, true, false) as Control
		if control != null and control.is_visible_in_tree():
			return false
	return mc_inventory_matches(state, count, money) and mc_cursor_matches(client, held) and mc_source_matches(client) and mc_bag_render_matches(client, count)

func mc_wait_state(client: Node, count: int, held: bool, money: int) -> bool:
	var deadline := Time.get_ticks_msec() + MC_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if mc_state_matches(client, count, held, money):
			return true
	fail("Merchant cursor state missing: count%s held%s money%s pointer%s state=%s; pickup held=true is intended first RED" % [count, held, money, mc_pointer, client.merchant_state()])
	return false

func mc_quiet_state(client: Node, count: int, held: bool, money: int) -> bool:
	var deadline := Time.get_ticks_msec() + MC_QUIET_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if not mc_state_matches(client, count, held, money):
			fail("Merchant cursor quiet phase changed source/cursor/inventory/money/modal: " + str(client.merchant_state()))
			return false
	return true
