extends "res://tests/world_bags_actions_flow.gd"

# Actual authenticated wire response -> authored UI -> physical requests.
# No production setter/action helper, no invented inventory or Gold mutation.
func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0") or not config.contains("/data/diagnostics/native-input-"):
		fail("Trade requires owned authenticated peer and isolated config")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 180000):
		return
	print("FIXTURE TRADE_LOADING")
	if not await wait_world(client):
		return
	var account: Dictionary = client.account_state()
	if not account.reply_received or account.character_count != 3:
		fail("Trade must originate from authenticated fixture roster")
		return
	print("FIXTURE TRADE_READY")
	if not await wait_trade_invitation(client):
		return
	print("FIXTURE TRADE_ACCEPT_ARM")
	await process_frame
	if not await press_popup(client, true):
		return
	if not await wait_trade_open(client, false) or not check_trade_slots(client):
		return
	print("FIXTURE TRADE_CONFIRM_ARM")
	await process_frame
	if not await trade_press(client, "TradeFrameTradeButton") or not await wait_trade_open(client, true):
		return
	var accept := authored_control(client, "TradeFrameTradeButton") as BaseButton
	if accept == null or not accept.disabled:
		fail("Accepted side must disable Trade button")
		return
	print("FIXTURE TRADE_UNACCEPT_ARM")
	await process_frame
	if not await trade_press(client, "TradeFrameCancelButton") or not await wait_trade_open(client, false):
		return
	print("FIXTURE TRADE_COMPLETE_ARM")
	await process_frame
	if not await trade_press(client, "TradeFrameTradeButton") or not await wait_trade_closed(client):
		return
	print("FIXTURE TRADE_DONE")
	while true:
		await process_frame

func wait_trade_invitation(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + INVENTORY_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var popup := authored_control(client, "StaticPopup1")
		var text := authored_control(client, "StaticPopup1Text") as Label
		if popup != null and popup.is_visible_in_tree() and text != null and text.text == "Trade with Remote Fixture?":
			return true
	fail("RED: authenticated TradeStateUpdate PendingIncoming did not produce authored TRADE invitation")
	return false

func wait_trade_open(client: Node, accepted: bool) -> bool:
	var deadline := Time.get_ticks_msec() + INVENTORY_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var frame := authored_control(client, "TradeFrame")
		var name_label := authored_control(client, "TradeFrameRecipientNameText") as Label
		var highlight := authored_control(client, "TradeHighlightPlayerTop")
		var highlighted := highlight != null and highlight.is_visible_in_tree()
		if frame != null and frame.is_visible_in_tree() and name_label != null and name_label.text == "Remote Fixture" and highlighted == accepted:
			return true
	fail("Trade snapshot failed to show authored open frame/accepted highlight=%s" % accepted)
	return false

func check_trade_slots(client: Node) -> bool:
	for side in ["Player", "Recipient"]:
		for slot in range(1, 8):
			if authored_control(client, "Trade%sItem%sItemButton" % [side, slot]) == null:
				fail("Missing one of seven authored %s trade slots: %s" % [side, slot])
				return false
		var label := authored_control(client, "Trade%sItemEnchantText" % side) as Label
		if label == null or label.text != "Will not be traded":
			fail("Seventh slot non-traded label missing")
			return false
	var item_name := authored_control(client, "TradePlayerItem1Name") as Label
	if item_name == null or item_name.text != "Linen Cloth":
		fail("Trade slot did not display peer-only item snapshot")
		return false
	return true

func trade_press(client: Node, control_name: String) -> bool:
	var control := authored_control(client, control_name)
	if control == null or not control.is_visible_in_tree():
		fail("Missing physical trade control: " + control_name)
		return false
	await pointer_click(control.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	return true

func wait_trade_closed(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + INVENTORY_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var frame := authored_control(client, "TradeFrame")
		var backpack := authored_control(client, "ContainerFrame0")
		if (frame == null or not frame.is_visible_in_tree()) and (backpack == null or not backpack.is_visible_in_tree()):
			return true
	fail("Authoritative Trade complete did not close trade/backpack")
	return false
