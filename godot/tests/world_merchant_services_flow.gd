extends "res://tests/world_merchant_click_flow.gd"

# Independent authenticated owned-UDP mode; never execute parent run_test.
const SERVICES_NPC := 4294966979
const SERVICES_WAIT_MS := 6000
const SERVICES_QUIET_MS := 900
# Observed local ItemSparse 12.1.0.69933 row4865 SellPrice5; peer input only.
const SERVICES_PELT_PRICE := 5
const SERVICES_FINAL_GOLD := 984 + 2 * SERVICES_PELT_PRICE
const SERVICES_REPAIR := "MerchantRepairAllButton"
const SERVICES_JUNK := "MerchantSellAllJunkButton"

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/diagnostics/native-input-"):
		fail("Services SETUP requires owned UDP endpoint and isolated root defaults")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 180000):
		return
	print("FIXTURE MERCHANT_SERVICES_LOADING")
	if not await wait_world(client):
		return
	var account: Dictionary = client.account_state()
	if not account.reply_received or account.character_count != 3:
		fail("Services SETUP requires authenticated selected fixture roster")
		return
	print("FIXTURE MERCHANT_SERVICES_READY")
	var vendor := await find_vendor(client)
	if vendor.is_empty() or vendor.id != SERVICES_NPC or not await open_vendor(client, vendor):
		return
	if not await services_wait(client, 2, 1000, 16):
		return
	print("FIXTURE MERCHANT_SERVICES_VENDOR_OPEN")
	if not await services_repair(client):
		return
	if not await services_junk(client):
		return
	print("FIXTURE MERCHANT_SERVICES_DONE")
	# Parent deliberately kills/reaps the owned client. Not shutdown proof.
	while true:
		await process_frame
		if not services_matches(client, 0, SERVICES_FINAL_GOLD, 0):
			fail("Services terminal authority/no-popup/render state changed")
			return

func open_vendor(client: Node, vendor: Dictionary) -> bool:
	pointer_at(vendor.point, MOUSE_BUTTON_RIGHT, true)
	await process_frame
	if client.target_state().target != vendor.id or client.target_state().auto_attack != null:
		fail("Services SETUP right-click must target owned vendor, not attack")
		return false
	pointer_at(vendor.point, MOUSE_BUTTON_RIGHT, false)
	var deadline := Time.get_ticks_msec() + SERVICES_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.merchant_state()
		if state.open and state.npc == SERVICES_NPC and state.vendor_name == VENDOR and state.items == ["Linen Cloth"]:
			return true
	fail("Services SETUP owned vendor did not open: " + str(client.merchant_state()))
	return false

func services_repair(client: Node) -> bool:
	if not await services_quiet(client, 2, 1000, 16):
		return false
	print("FIXTURE MERCHANT_SERVICES_REPAIR_ARM")
	if not await services_tap(client, SERVICES_REPAIR):
		return false
	if not await services_quiet(client, 2, 1000, 16):
		return false
	print("MERCHANT SERVICES REPAIR pre-commit Poor2/Linen3/Gold1000 cost16/no popup900ms")
	print("FIXTURE MERCHANT_SERVICES_REPAIR_COMMIT")
	if not await services_wait(client, 2, 984, 0) or not await services_quiet(client, 2, 984, 0):
		return false
	print("FIXTURE MERCHANT_SERVICES_REPAIR_DONE")
	# Physically press the rendered disabled Repair All. Peer rejects any request.
	if not await services_tap(client, SERVICES_REPAIR):
		return false
	return await services_quiet(client, 2, 984, 0)

func services_junk(client: Node) -> bool:
	print("FIXTURE MERCHANT_SERVICES_JUNK_ARM")
	if not await services_tap(client, SERVICES_JUNK):
		return false
	if not await services_quiet(client, 2, 984, 0):
		return false
	print("MERCHANT SERVICES JUNK pre-commit Poor2/Linen3/Gold984/no confirmation900ms; no Yes/No keys sent")
	print("FIXTURE MERCHANT_SERVICES_JUNK_COMMIT")
	if not await services_wait(client, 0, SERVICES_FINAL_GOLD, 0):
		return false
	if not await services_quiet(client, 0, SERVICES_FINAL_GOLD, 0):
		return false
	# Physically press the rendered disabled junk service; no extra request.
	if not await services_tap(client, SERVICES_JUNK):
		return false
	if not await services_quiet(client, 0, SERVICES_FINAL_GOLD, 0):
		return false
	print("MERCHANT SERVICES FINAL rendered PoorEmpty/Linen3/Gold", SERVICES_FINAL_GOLD, " Junkdisabled/no extra request900ms; fixture price, not server-pricing proof")
	return true

func services_control(client: Node, name: String) -> Control:
	var ui := client.get_node_or_null("MerchantUI")
	return ui.find_child(name, true, false) as Control if ui != null else null

func services_texture(control: Control) -> TextureRect:
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

func services_no_modal(client: Node) -> bool:
	return services_modal_reason(client).is_empty()

func services_modal_reason(client: Node) -> String:
	if client.merchant_state().split_open:
		return "merchant split_open=true"
	if client.get_node_or_null("GameMenuUI") != null:
		return "GameMenuUI root exists"
	for name in ["CursorItemIcon", "StackSplitFrame", "MerchantRepairItemButton"]:
		var control := client.find_child(name, true, false) as Control
		if control != null and control.is_visible_in_tree():
			return services_modal_path(control)
	for node in root.find_children("*", "Window", true, false):
		if (node as Window).visible:
			return "visible Window: " + str(node.get_path())
	for node in client.find_children("*", "Control", true, false):
		var name := str(node.name).to_lower()
		if name.contains("popup") or name.contains("confirmation") or name.contains("picker"):
			if (node as Control).is_visible_in_tree():
				return services_modal_path(node)
	return ""

func services_modal_path(control: Node) -> String:
	var layers: Array[String] = []
	var ancestor := control.get_parent()
	while ancestor != null:
		if ancestor is CanvasLayer:
			layers.append("%s visible=%s" % [ancestor.get_path(), (ancestor as CanvasLayer).visible])
		ancestor = ancestor.get_parent()
	return "visible Control: %s; canvas_layers=%s" % [control.get_path(), layers]

func services_tap(client: Node, name: String) -> bool:
	var control := services_control(client, name)
	var ui := client.get_node_or_null("MerchantUI")
	if control == null or ui == null or not ui.is_ancestor_of(control):
		fail("Services SETUP missing own physical control " + name)
		return false
	var rect := control.get_global_rect()
	if not control.is_visible_in_tree() or control.mouse_filter != Control.MOUSE_FILTER_STOP or not rect.has_area() or not root.get_visible_rect().encloses(rect):
		fail("Services SETUP button must be visible mouseSTOP inside viewport: " + name)
		return false
	var point := rect.get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	root.push_input(motion, true)
	await process_frame
	services_edge(point, true)
	await process_frame
	var no_modal_on_press := services_no_modal(client)
	services_edge(point, false)
	await process_frame
	if not no_modal_on_press or not services_no_modal(client):
		fail("Direct services physical Left opened confirmation/cursor/picker/menu: " + name)
		return false
	return true

func services_edge(point: Vector2, down: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
	event.pressed = down
	root.push_input(event, true)

func services_button_matches(client: Node, name: String, enabled: bool) -> bool:
	var button := services_control(client, name)
	var texture := services_texture(services_control(client, name + "Icon"))
	if button == null or not button.is_visible_in_tree() or texture == null:
		return false
	var tint := texture.self_modulate
	var ancestor: Node = texture
	while ancestor != null and ancestor != client:
		if ancestor is CanvasItem:
			tint *= (ancestor as CanvasItem).modulate
		ancestor = ancestor.get_parent()
	var expected := Color.WHITE if enabled else Color(0.4, 0.4, 0.4, 1.0)
	return tint.is_equal_approx(expected)

func services_inventory_matches(state: Dictionary, poor: int, money: int, cost: int) -> bool:
	if not state.open or state.npc != SERVICES_NPC or state.vendor_name != VENDOR or state.items != ["Linen Cloth"]:
		return false
	if state.money != money or state.repair_cost != cost or state.bags.size() != (2 if poor > 0 else 1):
		return false
	var buyback: Array = ["Ruined Pelt"] if poor == 0 else []
	if state.buyback != buyback:
		return false
	var expected := {2589: [1, 3, "Linen Cloth"]}
	if poor > 0:
		expected[4865] = [0, poor, "Ruined Pelt"]
	for item in state.bags:
		if not expected.has(item.item_id):
			return false
		var values: Array = expected[item.item_id]
		if item.bag != 0 or item.slot != values[0] or item.count != values[1] or item.name != values[2]:
			return false
		expected.erase(item.item_id)
	return expected.is_empty()

func services_bag_matches(client: Node, poor: int) -> bool:
	var bag := services_control(client, "ContainerFrame0")
	if bag == null or not bag.is_visible_in_tree():
		return false
	for slot in range(16):
		var prefix := "ContainerFrame0Slot%s" % slot
		var texture := services_texture(services_control(client, prefix + "Icon"))
		var count := services_control(client, prefix + "Count") as Label
		var amount := 3 if slot == 1 else (poor if slot == 0 else 0)
		if (texture != null) != (amount > 0):
			return false
		if amount > 0:
			if count == null or not count.is_visible_in_tree() or count.text != str(amount):
				return false
		elif count != null and count.is_visible_in_tree():
			return false
	return true

func services_money_matches(client: Node, money: int) -> bool:
	# SmallMoneyFrame suppresses zero copper: Gold1000 is just silver10.
	var first := services_control(client, "MerchantMoneyFrameAmount0") as Label
	var second := services_control(client, "MerchantMoneyFrameAmount1") as Label
	var extra := services_control(client, "MerchantMoneyFrameAmount2") as Label
	if first == null or not first.is_visible_in_tree() or first.text != str(money / 100):
		return false
	if money % 100 == 0:
		if second != null and second.is_visible_in_tree():
			return false
	elif second == null or not second.is_visible_in_tree() or second.text != str(money % 100):
		return false
	return extra == null or not extra.is_visible_in_tree()

func services_matches(client: Node, poor: int, money: int, cost: int) -> bool:
	if not services_no_modal(client) or not services_inventory_matches(client.merchant_state(), poor, money, cost):
		return false
	if not services_bag_matches(client, poor) or not services_money_matches(client, money):
		return false
	return services_button_matches(client, SERVICES_REPAIR, cost > 0) and services_button_matches(client, SERVICES_JUNK, poor > 0)

func services_wait(client: Node, poor: int, money: int, cost: int) -> bool:
	var deadline := Time.get_ticks_msec() + SERVICES_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if not services_no_modal(client):
			fail("Direct services rejected modal oracle: " + services_modal_reason(client))
			return false
		if services_matches(client, poor, money, cost):
			return true
	fail("Services state timeout Poor%s/Gold%s/cost%s: %s; inspect peer counts, setup is not request RED" % [poor, money, cost, client.merchant_state()])
	return false

func services_quiet(client: Node, poor: int, money: int, cost: int) -> bool:
	var deadline := Time.get_ticks_msec() + SERVICES_QUIET_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if not services_matches(client, poor, money, cost):
			fail("Services900ms quiet changed bags/Gold/cost/render/enablement or opened confirmation: " + str(client.merchant_state()))
			return false
	return true
