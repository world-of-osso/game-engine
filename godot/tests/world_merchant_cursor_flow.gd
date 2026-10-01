extends "res://tests/world_merchant_click_flow.gd"

# Independent mode: inherit real NPC discovery, never execute parent run_test.
const MC_WAIT_MS := 6000
const MC_QUIET_MS := 900
const MC_SOURCE := "MerchantItem1"
const MC_SOURCE_ICON := "MerchantItem1ItemButtonIcon"
const MC_SLOT := "ContainerFrame0Slot0"
const MC_CURSOR := "CursorItemIcon"
const MC_COLOR_TOLERANCE := 0.03
# Original merchant_frame_component: column gap (164,176), below title,
# above bottom controls; ACTION_FRAME = merchant_frame on MerchantFrame itself.
const MC_SALE_BACKGROUND := Vector2(170.0, 145.0)

var mc_pointer := Vector2.ZERO
var mc_sale_texture: Texture2D

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
	print("FIXTURE MERCHANT_CURSOR_BUY_DONE")
	if not await mc_sale_drag(client):
		return
	print("FIXTURE MERCHANT_CURSOR_SPLIT_SEED")
	if not await mc_split_sale(client):
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

func mc_sale_drag(client: Node) -> bool:
	if not mc_embedded_bag(client):
		return false
	var source := mc_control(client, MC_SLOT)
	var frame := mc_control(client, "MerchantFrame")
	var canvas := mc_control(client, "RegistryCanvas")
	var texture := mc_texture(mc_control(client, MC_SLOT + "Icon"))
	if source == null or frame == null or canvas == null or texture == null:
		fail("Sale requires actual bought bag0/slot0 icon and own MerchantFrame/canvas")
		return false
	mc_sale_texture = texture.texture
	print("FIXTURE MERCHANT_CURSOR_SALE_PICKUP_ARM")
	if not await mc_sale_wait_state(client, 1, false, 975, false):
		return false
	if not await mc_sale_quiet_state(client, 1, false, 975, false):
		return false
	var start := source.get_global_rect().get_center()
	var finish := frame.get_global_transform() * MC_SALE_BACKGROUND
	var scale := canvas.get_global_transform().get_scale().x
	if scale <= 0 or start.distance_to(finish) / scale < 4.0 or not mc_sale_background_matches(client, finish):
		fail("Sale target must be own merchant_frame background, >=4 logical px from bag0/slot0")
		return false
	print("MERCHANT CURSOR SALE GEOMETRY source=", source.get_path(), " target=", frame.get_path(), " action=merchant_frame start=", start, " finish=", finish, " scale=", scale)
	print("FIXTURE MERCHANT_CURSOR_SALE_PRESS_ARM")
	mc_motion(start, false)
	await process_frame
	mc_edge(start, true)
	if not await mc_sale_wait_state(client, 1, true, 975, true):
		return false
	if not await mc_sale_quiet_state(client, 1, true, 975, true):
		return false
	print("MERCHANT CURSOR SALE PICKUP bag0/slot0 Linen1 unchanged; original source RGB0.5 alpha1, cursor textured/centered; money975")
	print("FIXTURE MERCHANT_CURSOR_SALE_PICKED_UP")
	print("FIXTURE MERCHANT_CURSOR_SALE_DROP_ARM")
	source = mc_control(client, MC_SLOT)
	if source == null or not source.get_global_rect().has_point(start) or not mc_sale_background_matches(client, finish):
		fail("Sale source/own background moved or became covered while physical Left held")
		return false
	mc_motion(finish, true)
	# Prove source/cursor remain valid at destination before physical release.
	if not await mc_sale_wait_state(client, 1, true, 975, true):
		return false
	if not await mc_sale_quiet_state(client, 1, true, 975, true):
		return false
	if not mc_sale_background_matches(client, finish):
		fail("Sale release background became covered")
		return false
	mc_edge(finish, false)
	if not await mc_sale_wait_state(client, 1, false, 975, false):
		return false
	if not await mc_sale_quiet_state(client, 1, false, 975, false):
		return false
	print("MERCHANT CURSOR SELL COMMIT pre-delta Linen1/money975/source white/cursor hidden; peer must decode guid9182589 count0")
	print("FIXTURE MERCHANT_CURSOR_SELL_COMMIT")
	if not await mc_sale_wait_state(client, 0, false, 988, false):
		return false
	if not await mc_sale_quiet_state(client, 0, false, 988, false):
		return false
	print("MERCHANT CURSOR SALE FINAL bags=[] money988 rendered empty cursor hidden; vendor Linen price25 pack1 unchanged")
	return true

func mc_split_sale(client: Node) -> bool:
	if not mc_embedded_bag(client):
		return false
	if not await mc_split_wait(client, 5, false, 988, false) or not await mc_split_quiet(client, 5, false, 988, false):
		return false
	var source := mc_control(client, MC_SLOT)
	if source == null:
		fail("Split sale requires seeded own bag0/slot0")
		return false
	var start := source.get_global_rect().get_center()
	print("FIXTURE MERCHANT_CURSOR_SPLIT_PICKER_ARM")
	mc_motion(start, false)
	await process_frame
	mc_split_key(KEY_SHIFT, 0, true)
	mc_split_shift_edge(start, true)
	await process_frame
	mc_split_shift_edge(start, false)
	mc_split_key(KEY_SHIFT, 0, false)
	# Picker alone has no cursor.source(): original source stays WHITE.
	if not await mc_split_wait(client, 5, false, 988, false, "1") or not await mc_split_quiet(client, 5, false, 988, false, "1"):
		return false
	# No bag-split max diagnostic exists. Prove authored max5 physically:
	# four increments reach5; another stays5; four decrements restore1.
	for amount in [2, 3, 4, 5, 5]:
		await mc_split_tap(KEY_UP)
		if not await mc_split_wait(client, 5, false, 988, false, str(amount)):
			return false
	if not await mc_split_quiet(client, 5, false, 988, false, "5"):
		return false
	for amount in [4, 3, 2, 1]:
		await mc_split_tap(KEY_DOWN)
		if not await mc_split_wait(client, 5, false, 988, false, str(amount)):
			return false
	print("MERCHANT CURSOR SPLIT PICKER original BOTTOMRIGHT=owner TOPRIGHT, 172x96; text1/max5, cursor empty/source white/Linen5/money988")
	print("FIXTURE MERCHANT_CURSOR_SPLIT_PICKER_OPEN")
	mc_split_key(KEY_2, 50, true)
	mc_split_key(KEY_2, 50, false)
	if not await mc_split_wait(client, 5, false, 988, false, "2"):
		return false
	await mc_split_tap(KEY_ENTER)
	if not await mc_split_wait(client, 5, true, 988, true) or not await mc_split_quiet(client, 5, true, 988, true):
		return false
	print("FIXTURE MERCHANT_CURSOR_SPLIT_HELD")
	var frame := mc_control(client, "MerchantFrame")
	if frame == null:
		fail("Split sale lost own MerchantFrame")
		return false
	var finish := frame.get_global_transform() * MC_SALE_BACKGROUND
	if not mc_sale_background_matches(client, finish):
		fail("Split sale requires uncovered own merchant background")
		return false
	# Enter produced an already-held split, not a fresh pickup drag origin.
	mc_motion(finish, false)
	if not await mc_split_wait(client, 5, true, 988, true) or not await mc_split_quiet(client, 5, true, 988, true):
		return false
	if not mc_sale_background_matches(client, finish):
		fail("Split sale press background moved/became covered")
		return false
	print("FIXTURE MERCHANT_CURSOR_SPLIT_SELL_PRESS_ARM")
	mc_edge(finish, true)
	# Prove sale on PRESS, before release; peer still withholds both updates.
	if not await mc_split_wait(client, 5, false, 988, false) or not await mc_split_quiet(client, 5, false, 988, false):
		return false
	mc_edge(finish, false)
	if not await mc_split_quiet(client, 5, false, 988, false):
		return false
	print("MERCHANT CURSOR SPLIT SELL COMMIT Linen5/money988/source white/cursor hidden; guid9182590 count2 only peer-visible")
	print("FIXTURE MERCHANT_CURSOR_SPLIT_SELL_COMMIT")
	if not await mc_split_wait(client, 3, false, 1014, false) or not await mc_split_quiet(client, 3, false, 1014, false):
		return false
	print("MERCHANT CURSOR SPLIT FINAL rendered Linen3/money1014/cursor hidden/modal hidden; exact peer totals buys1/sells2")
	return true

func mc_split_key(code: Key, unicode: int, down: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.unicode = unicode
	event.pressed = down
	root.push_input(event, true)

func mc_split_tap(code: Key) -> void:
	mc_split_key(code, 0, true)
	await process_frame
	mc_split_key(code, 0, false)
	await process_frame

func mc_split_shift_edge(point: Vector2, down: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
	event.shift_pressed = true
	event.pressed = down
	mc_pointer = point
	root.push_input(event, true)

func mc_split_control(client: Node, control_name: String) -> Control:
	var ui := client.get_node_or_null("CursorItemUI")
	return ui.find_child(control_name, true, false) as Control if ui != null else null

func mc_split_picker_matches(client: Node, amount: String) -> bool:
	var picker := mc_split_control(client, "StackSplitFrame")
	if amount.is_empty():
		return picker == null or not picker.is_visible_in_tree()
	var source := mc_control(client, MC_SLOT)
	var canvas := mc_control(client, "RegistryCanvas")
	var label := mc_split_control(client, "StackSplitText") as Label
	if picker == null or not picker.is_visible_in_tree() or source == null or canvas == null or label == null or not label.is_visible_in_tree() or label.text != amount:
		return false
	# Original frame_state and authored SSF geometry, NOT native calibration.
	# Bag picker BOTTOMRIGHT at owner TOPRIGHT, width172 height96.
	var scale := canvas.get_global_transform().get_scale()
	if scale.x <= 0 or scale.y <= 0:
		return false
	var owner_rect := source.get_global_rect()
	var expected_size := Vector2(172.0, 96.0) * scale
	var expected_position := Vector2(owner_rect.end.x, owner_rect.position.y) - expected_size
	var actual := picker.get_global_rect()
	return actual.position.distance_to(expected_position) <= 1.0 and actual.size.distance_to(expected_size) <= 1.0

func mc_split_render_matches(client: Node, count: int) -> bool:
	for slot in range(16):
		var prefix := "ContainerFrame0Slot%s" % slot
		var texture := mc_texture(mc_control(client, prefix + "Icon"))
		var label := mc_control(client, prefix + "Count") as Label
		if slot == 0:
			if texture == null or texture.texture != mc_sale_texture or label == null or not label.is_visible_in_tree() or label.text != str(count):
				return false
		elif texture != null or (label != null and label.is_visible_in_tree()):
			return false
	return true

func mc_split_state_matches(client: Node, count: int, held: bool, money: int, locked: bool, amount: String) -> bool:
	var state: Dictionary = client.merchant_state()
	if not state.open or state.items != ["Linen Cloth"] or state.money != money or state.bags.size() != 1 or state.split_open or client.get_node_or_null("GameMenuUI") != null:
		return false
	var item: Dictionary = state.bags[0]
	if item.bag != 0 or item.slot != 0 or item.item_id != 2589 or item.name != "Linen Cloth" or item.count != count:
		return false
	var popup := client.find_child("StaticPopup1", true, false) as Control
	if popup != null and popup.is_visible_in_tree():
		return false
	if held:
		var cursor_texture := mc_texture(client.find_child(MC_CURSOR, true, false) as Control)
		if cursor_texture == null or cursor_texture.texture != mc_sale_texture:
			return false
	return mc_split_picker_matches(client, amount) and mc_cursor_matches(client, held) and mc_source_matches(client) and mc_split_render_matches(client, count) and mc_sale_source_matches(client, count, locked)

func mc_split_wait(client: Node, count: int, held: bool, money: int, locked: bool, amount: String = "") -> bool:
	var deadline := Time.get_ticks_msec() + MC_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if mc_split_state_matches(client, count, held, money, locked, amount):
			return true
	var picker := mc_split_control(client, "StackSplitFrame")
	var source := mc_control(client, MC_SLOT)
	fail("Split sale state missing count%s held%s money%s locked%s pickertext%s; picker_rect=%s owner_rect=%s state=%s; anchor defect is unproved until native run" % [count, held, money, locked, amount, picker.get_global_rect() if picker != null else Rect2(), source.get_global_rect() if source != null else Rect2(), client.merchant_state()])
	return false

func mc_split_quiet(client: Node, count: int, held: bool, money: int, locked: bool, amount: String = "") -> bool:
	var deadline := Time.get_ticks_msec() + MC_QUIET_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if not mc_split_state_matches(client, count, held, money, locked, amount):
			fail("Split sale quiet phase changed authored picker/source/cursor/rendered count/inventory/money: " + str(client.merchant_state()))
			return false
	return true

func mc_sale_background_matches(client: Node, point: Vector2) -> bool:
	var ui := client.get_node_or_null("MerchantUI")
	var frame := mc_control(client, "MerchantFrame")
	if ui == null or frame == null or not ui.is_ancestor_of(frame) or not frame.is_visible_in_tree() or not frame.get_global_rect().has_point(point):
		return false
	if (frame.get_global_transform() * MC_SALE_BACKGROUND).distance_to(point) > 0.1:
		return false
	# Projected mouse-enabled/action controls STOP; visual parts IGNORE.
	# Reject title, buttons, cells, backpack or any other actionable coverage.
	for node in ui.find_children("*", "Control", true, false):
		var control := node as Control
		if control != frame and control.is_visible_in_tree() and control.mouse_filter != Control.MOUSE_FILTER_IGNORE and control.get_global_rect().has_point(point):
			return false
	return frame.mouse_filter == Control.MOUSE_FILTER_STOP

func mc_sale_source_matches(client: Node, count: int, locked: bool) -> bool:
	if count == 0:
		return mc_texture(mc_control(client, MC_SLOT + "Icon")) == null
	var texture := mc_texture(mc_control(client, MC_SLOT + "Icon"))
	if texture == null or texture.texture != mc_sale_texture:
		return false
	# Same original shared BagFrameComponent oracle as world_bags_cursor_flow:
	# ImagePart vertex color is self_modulate, multiplied by ancestor modulate.
	var tint := texture.self_modulate
	var ancestor: Node = texture
	while ancestor != null and ancestor != client:
		if ancestor is CanvasItem:
			tint *= (ancestor as CanvasItem).modulate
		ancestor = ancestor.get_parent()
	var expected_rgb := 0.5 if locked else 1.0
	return absf(tint.r - expected_rgb) <= MC_COLOR_TOLERANCE and absf(tint.g - expected_rgb) <= MC_COLOR_TOLERANCE and absf(tint.b - expected_rgb) <= MC_COLOR_TOLERANCE and absf(tint.a - 1.0) <= MC_COLOR_TOLERANCE

func mc_sale_state_matches(client: Node, count: int, held: bool, money: int, locked: bool) -> bool:
	return mc_state_matches(client, count, held, money) and mc_sale_source_matches(client, count, locked)

func mc_sale_wait_state(client: Node, count: int, held: bool, money: int, locked: bool) -> bool:
	var deadline := Time.get_ticks_msec() + MC_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if mc_sale_state_matches(client, count, held, money, locked):
			return true
	var texture := mc_texture(mc_control(client, MC_SLOT + "Icon"))
	var observed_tint := str(texture.self_modulate) if texture != null else "missing"
	fail("Merchant sale state missing: count%s held%s money%s locked%s source_self_modulate=%s pointer=%s state=%s; RED meaning requires actual native run" % [count, held, money, locked, observed_tint, mc_pointer, client.merchant_state()])
	return false

func mc_sale_quiet_state(client: Node, count: int, held: bool, money: int, locked: bool) -> bool:
	var deadline := Time.get_ticks_msec() + MC_QUIET_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if not mc_sale_state_matches(client, count, held, money, locked):
			fail("Merchant sale quiet phase changed source lock/texture/count/cursor/inventory/money/modal: " + str(client.merchant_state()))
			return false
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
