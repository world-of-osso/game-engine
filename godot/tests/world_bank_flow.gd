extends "res://tests/world_bags_actions_flow.gd"

# No client state setter or direct action invocation. Native pointer/key controls only.
# Owned peer requires exact Banker InteractNpc and every BankChannel action before ack.
const BANKER := "Fixture Banker"
const BANK_WAIT := 10000
var bank_client: Node

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/diagnostics/native-input-"):
		fail("Bank requires authenticated owned loopback peer and isolated config")
		return
	bank_client = load("res://scenes/client.tscn").instantiate()
	root.add_child(bank_client)
	if not await wait_screen(bank_client, "Loading", 180000):
		return
	print("FIXTURE BANK_LOADING")
	if not await wait_world(bank_client):
		return
	if not bank_client.account_state().reply_received or bank_client.account_state().character_count != 3:
		fail("Bank must originate from authenticated character17 roster")
		return
	if not await open_banker():
		return
	if not await wait_bank(func(): return bank_text("BankFrameHeader") == "Supplies" and bank_client.merchant_state().money == 20000000 and bag_linen() == 3, "initial contents/bag/gold"):
		return
	if not visible("ContainerFrame0") or not await grid_layout():
		return
	print("FIXTURE BANK_OPEN")
	# Character deposit/withdraw; server owns bag and bank, not click handlers.
	if not await bank_action("ContainerFrame0Slot0", MOUSE_BUTTON_RIGHT, func(): return bag_linen() == 0 and bank_text("BankFrameItem1Count") == "3"):
		return
	if not await bank_action("BankFrameItem1", MOUSE_BUTTON_RIGHT, func(): return bag_linen() == 3 and bank_text("BankFrameItem1Count") == ""):
		return
	if not await bank_action("BankFrameDepositAllButton", MOUSE_BUTTON_LEFT, func(): return bag_linen() == 3):
		return
	# Purchase tab replaces grid, bag right-click must be inert while selected.
	if not await press_bank("BankFramePurchaseTab") or not visible("BankFramePurchasePromptButton"):
		fail("Bank purchase tab did not show authored prompt")
		return
	await press_bank("ContainerFrame0Slot0", MOUSE_BUTTON_RIGHT)
	await wait_frames(45)
	if bag_linen() != 3 or visible("BankFrameItem1"):
		fail("Purchase prompt leaked bag deposit or slot grid")
		return
	if not await press_bank("BankFramePurchasePromptButton") or not await wait_bank(func(): return visible("StaticPopup1Button1"), "purchase confirmation"):
		return
	if bank_text("StaticPopup1Text") != "Do you want to purchase a Bank tab for:\n1g":
		fail("Purchase confirmation text/price mismatch")
		return
	if not await bank_action("StaticPopup1Button1", MOUSE_BUTTON_LEFT, func(): return bank_text("BankFrameHeader") == "New Tab" and bank_client.merchant_state().money == 19990000):
		return
	# Settings: actual editbox and all five assignments; server retains the icon.
	if not await press_bank("BankFrameTab2", MOUSE_BUTTON_RIGHT) or not visible("BankTabSettingsName"):
		fail("Right side-tab click did not open settings")
		return
	if not await replace_input("BankTabSettingsName", "Renamed"):
		return
	for index in range(5):
		if not await press_bank("BankFrameTabSettingsAssign%s" % index):
			return
	if not await bank_action("BankFrameTabSettingsOkay", MOUSE_BUTTON_LEFT, func(): return bank_text("BankFrameHeader") == "Renamed"):
		return
	# Warband selection is independent from character selected tab.
	if not await press_bank("BankFrameTabSystemTab2") or not await wait_bank(func(): return bank_text("BankFrameHeader") == "Shared", "Warband tab"):
		return
	if not visible("BankFrameMoneyFrameDepositButton") or not visible("BankFrameIncludeReagents"):
		fail("Warband money/Include tradeable reagents missing")
		return
	if not await press_bank("BankFrameTab2") or not await wait_bank(func(): return bank_text("BankFrameHeader") == "Reserve", "second Warband tab"):
		return
	if not await press_bank("BankFrameTabSystemTab1") or bank_text("BankFrameHeader") != "Renamed" or not await press_bank("BankFrameTabSystemTab2"):
		fail("Character/Warband selected tabs were not independent")
		return
	if not await bank_action("ContainerFrame0Slot0", MOUSE_BUTTON_RIGHT, func(): return bag_linen() == 0 and bank_text("BankFrameItem1Count") == "3"):
		return
	if not await bank_action("BankFrameItem1", MOUSE_BUTTON_RIGHT, func(): return bag_linen() == 3 and bank_text("BankFrameItem1Count") == ""):
		return
	# Empty money entry sends nothing. Then concrete gold/silver/copper both ways.
	if not await press_bank("BankFrameMoneyFrameDepositButton") or not await press_bank("BankMoneyPopupAccept"):
		return
	await wait_frames(45)
	if not await money_action(true, ["1", "2", "3"], 19979797, ["4", "2", "3"]):
		return
	if not await money_action(false, ["1", "2", "3"], 19990000, ["3", "", ""]):
		return
	if not await press_bank("BankFrameIncludeReagents") or not await bank_action("BankFrameDepositAllButton", MOUSE_BUTTON_LEFT, func(): return bag_linen() == 3):
		return
	# Server failure must display Retail text without changing bags/money/contents.
	if not await money_action(false, ["9900", "", ""], 19990000, ["3", "", ""], true):
		return
	# Unsolicited account-wide update: no request, actual UI must refresh.
	print("FIXTURE BANK_PEER_REFRESH")
	if not await wait_bank(func(): return bank_text("BankFrameHeader") == "Peer Shared" and bank_text("BankFrameItem3Count") == "3" and money_digits() == ["4", "44", "44"], "peer-only Warband update"):
		return
	# Escape closes bank/backpack, not menu; then close button, then server close.
	await action_key(KEY_ESCAPE)
	if not await wait_closed() or not await open_banker():
		return
	if bank_text("BankFrameHeader") != "Supplies":
		fail("Reopen must start on character bank")
		return
	if not await press_bank("BankFrameCloseButton") or not await wait_closed() or not await open_banker():
		return
	print("FIXTURE BANK_SERVER_CLOSE")
	if not await wait_closed():
		return
	print("FIXTURE BANK_DONE")
	# Parent owns deliberate kill/reap; no shutdown proof.
	while true:
		await process_frame

func bank_text(name: String) -> String:
	var control := authored_control(bank_client, name)
	return control.text if control is Label and control.is_visible_in_tree() else ""

func visible(name: String) -> bool:
	var control := authored_control(bank_client, name)
	return control != null and control.is_visible_in_tree()

func bag_linen() -> int:
	var items: Array = bank_client.merchant_state().bags
	if items.is_empty():
		return 0
	if items.size() == 1 and items[0].bag == 0 and items[0].slot == 0 and items[0].item_id == 2589:
		return items[0].count
	return -1

func money_digits() -> Array:
	return [bank_text("BankFrameMoneyFrameMoneyAmount0"), bank_text("BankFrameMoneyFrameMoneyAmount1"), bank_text("BankFrameMoneyFrameMoneyAmount2")]

func authority_view() -> Array:
	var counts := []
	for index in range(98):
		counts.append(bank_text("BankFrameItem%sCount" % (index + 1)))
	return [bag_linen(), bank_client.merchant_state().money, bank_text("BankFrameHeader"), counts, money_digits()]

func bank_action(control: String, button: int, acknowledged: Callable) -> bool:
	var before := authority_view()
	print("FIXTURE BANK_ARM")
	await process_frame
	if not await press_bank(control, button):
		return false
	var deadline := Time.get_ticks_msec() + 900
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if authority_view() != before:
			fail("Bank mutated authoritative bags/money/contents before peer ack: " + control)
			return false
	print("FIXTURE BANK_PREACK")
	return await wait_bank(acknowledged, "authoritative " + control)

func money_action(deposit: bool, digits: Array, gold: int, stored: Array, failure := false) -> bool:
	if not await press_bank("BankFrameMoneyFrameDepositButton" if deposit else "BankFrameMoneyFrameWithdrawButton"):
		return false
	for index in range(3):
		if not await replace_input(["BankMoneyGold", "BankMoneySilver", "BankMoneyCopper"][index], digits[index]):
			return false
	var result := await bank_action("BankMoneyPopupAccept", MOUSE_BUTTON_LEFT, func(): return bank_client.merchant_state().money == gold and money_digits() == stored and (not failure or has_error("You don't have enough money.")))
	return result

func has_error(message: String) -> bool:
	for label in bank_client.find_children("*", "Label", true, false):
		if label.is_visible_in_tree() and label.text == message:
			return true
	return false

func replace_input(name: String, value: String) -> bool:
	if not await press_bank(name):
		return false
	# Existing registry editboxes support Home/End and Backspace, no setters.
	await action_key(KEY_END)
	await backspaces(20)
	await type_characters(value)
	return true

func press_bank(name: String, button: int = MOUSE_BUTTON_LEFT) -> bool:
	var control := authored_control(bank_client, name)
	if control == null or not control.is_visible_in_tree():
		fail("Missing authored native Bank control: " + name)
		return false
	await pointer_click(control.get_global_rect().get_center(), button)
	await wait_frames(2)
	return true

func wait_bank(predicate: Callable, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + BANK_WAIT
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			await wait_frames(2)
			return true
	fail("Native Bank missing " + what)
	return false

func wait_closed() -> bool:
	if not await wait_bank(func(): return not visible("BankFrame") and not visible("ContainerFrame0"), "bank/backpack close"):
		return false
	await wait_frames(45)
	if bank_client.get_node_or_null("GameMenuUI") != null:
		fail("Bank close opened GameMenu")
		return false
	return true

func grid_layout() -> bool:
	var frame := authored_control(bank_client, "BankFrame")
	var scale := frame.get_global_transform().get_scale().x
	if not frame.size.is_equal_approx(Vector2(738, 460)):
		fail("BankFrame must retain original 738x460 layout")
		return false
	for index in range(98):
		var slot := authored_control(bank_client, "BankFrameItem%s" % (index + 1))
		var column := index / 7
		var row := index % 7
		var expected := frame.get_global_rect().position + Vector2(26 + column * 45 + (column / 2) * 11, 63 + row * 47) * scale
		if slot == null or not slot.is_visible_in_tree() or slot.get_global_rect().position.distance_to(expected) > 2:
			fail("Native Bank column-major slot %s missing/misplaced" % index)
			return false
	return authored_control(bank_client, "BankFrameItem99") == null

func open_banker() -> bool:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var camera := root.get_viewport().get_camera_3d()
		var units := bank_client.get_node_or_null("WorldUnits")
		if camera == null or units == null:
			continue
		for unit in units.get_children():
			if str(unit.name) != BANKER:
				continue
			var area := unit.find_child("UnitPick", true, false) as Area3D
			if area == null:
				continue
			var center := (area.get_child(0) as Node3D).global_position
			var point := camera.unproject_position(center)
			var id = area.get_meta("unit_server_id")
			if camera.is_position_in_frustum(center) and UnitPicker.pick(camera, point) == id:
				await pointer_click(point, MOUSE_BUTTON_RIGHT)
				return await wait_bank(func(): return visible("BankFrame") and bank_text("BankFrameTitleText") == "Bank", "Banker InteractionOpened/contents consumer (functional RED if absent)")
	fail("Owned authenticated banker is not ray-pickable")
	return false
