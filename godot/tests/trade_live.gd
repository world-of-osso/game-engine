extends SceneTree

## Two-client player trade against a private server (docs/specs/trade-frame.md). Run
## once per side; the sides meet at barrier files. Environment:
##   GODOT_TEST_SERVER   private server address
##   TRADE_ROLE          "a" (initiates, offers Linen + gold) or "b" (Peacebloom + silver)
##   TRADE_ACCOUNT / TRADE_CHARACTER / TRADE_PARTNER   (password fbtest)
##   TRADE_SYNC          shared barrier directory
##   TRADE_SHOTS         screenshot directory
##   TRADE_ADMIN         game-server-admin binary (side b fills its bags with it)
## Real input only: world click and TargetFrame menu, `/trade`, popup Yes, bag
## right-clicks, typed money, Trade/Cancel/Escape. Every result is the server's.

const PASSWORD := "fbtest"
const LINEN := 2589
const WOOL := 2592
const PEACEBLOOM := 2447
const HEARTHSTONE := 6948
const FILLER := 25
const WAIT_MS := 60000

var client: Node
var role := ""
var partner := ""
var sync_dir := ""
var shots := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	role = OS.get_environment("TRADE_ROLE")
	partner = OS.get_environment("TRADE_PARTNER")
	sync_dir = OS.get_environment("TRADE_SYNC")
	shots = OS.get_environment("TRADE_SHOTS")
	var character := OS.get_environment("TRADE_CHARACTER")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("TRADE_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail("connect: " + error)
		return
	if not await enter_world(character):
		return
	if not await barrier("world"):
		return
	for step in [Callable(self, "trade_items_and_money"), Callable(self, "cancel_by_button"), Callable(self, "soulbound_refused"), Callable(self, "full_bags_refused")]:
		if not await step.call():
			return
	print("FIXTURE TRADE_LIVE_DONE role=", role)
	client.free()
	quit(0)

# --- scenarios -------------------------------------------------------------

func trade_items_and_money() -> bool:
	var before := money()
	var wool_before := bag_count(WOOL)
	if role == "a":
		if not await target_partner_by_click() or not await menu_trade():
			return false
	if not await open_trade("1"):
		return false
	var item := LINEN if role == "a" else PEACEBLOOM
	if not await offer(item) or not await type_money(10000 if role == "a" else 5000):
		return false
	# Side a also drops Wool from the cursor on the "Will not be traded" slot.
	if role == "a" and not await place_on_slot(WOOL, 7):
		return false
	if not await wait_trade(func(t): return t.window_open and slot_item(t.other, 0) == (PEACEBLOOM if role == "a" else LINEN) and t.other.gold == (5000 if role == "a" else 10000) and slot_item(t.player if role == "a" else t.other, 6) == WOOL, "both offers"):
		return false
	await capture("1-offers")
	if not await barrier("offered"):
		return false
	if role == "a":
		await press("TradeFrameTradeButton")
	if not await wait_trade(func(t): return (t.player if role == "a" else t.other).accepted, "a accepted"):
		return false
	await capture("2-a-accepted")
	if not await barrier("accepted"):
		return false
	# Any change after an accept resets both accepts.
	if role == "a" and not await type_money(20000):
		return false
	if not await wait_trade(func(t): return not t.player.accepted and not t.other.accepted and (t.player if role == "a" else t.other).gold == 20000, "accepts reset by the money change"):
		return false
	await capture("3-accept-reset")
	if not await barrier("reset"):
		return false
	if role == "a":
		await press("TradeFrameTradeButton")
	if not await barrier("a-confirmed"):
		return false
	if role == "b":
		if not await wait_trade(func(t): return t.other.accepted, "a accepted again"):
			return false
		await press("TradeFrameTradeButton")
	if not await wait_until(func(): return not client.trade_state().window_open and bag_count(LINEN if role == "b" else PEACEBLOOM) > 0, "trade complete"):
		return false
	var delta := money() - before
	var expected := -20000 + 5000 if role == "a" else 20000 - 5000
	if delta != expected or bag_count(item) != 0 or bag_count(WOOL) != wool_before:
		return fail("exchange mismatch: money delta %d (want %d), %d left of %d, wool %d (was %d)" % [delta, expected, bag_count(item), item, bag_count(WOOL), wool_before])
	if not await wait_error("Trade complete."):
		return false
	await capture("4-complete")
	print("FIXTURE TRADE_COMPLETE role=%s money_delta=%d bags=%s" % [role, delta, client.merchant_state().bags])
	return await barrier("done-1")

func cancel_by_button() -> bool:
	if role == "a" and not await send_line("/trade"):
		return false
	if not await open_trade("2"):
		return false
	if role == "b":
		await press("TradeFrameCancelButton")
	if not await wait_until(func(): return not client.trade_state().window_open and not client.trade_state().has("phase"), "cancelled trade closed"):
		return false
	if not await wait_error("Trade canceled."):
		return false
	await capture("5-cancelled")
	return await barrier("done-2")

func soulbound_refused() -> bool:
	if role == "a" and not await send_line("/trade"):
		return false
	if not await open_trade("3"):
		return false
	if role == "a":
		if not await offer(HEARTHSTONE) or not await wait_error("You can't trade a soulbound item."):
			return false
	if not await barrier("soulbound-offered"):
		return false
	if not await wait_trade(func(t): return t.window_open and slot_item(t.player, 0) == 0 and slot_item(t.other, 0) == 0, "no soulbound item offered"):
		return false
	await capture("6-soulbound-refused")
	if not await barrier("soulbound-seen"):
		return false
	if role == "a":
		await tap(KEY_ESCAPE)
	if not await wait_until(func(): return not client.trade_state().window_open, "escape closed"):
		return false
	return await barrier("done-3")

func full_bags_refused() -> bool:
	if role == "b" and not await fill_bags():
		return false
	if not await barrier("filled"):
		return false
	var wool_before := bag_count(WOOL)
	if role == "a" and not await send_line("/trade"):
		return false
	if not await open_trade("4"):
		return false
	if role == "a":
		if not await offer(WOOL):
			return false
	if not await wait_trade(func(t): return slot_item(t.player if role == "a" else t.other, 0) == WOOL, "wool offered"):
		return false
	if not await barrier("wool-offered"):
		return false
	if role == "a":
		await press("TradeFrameTradeButton")
	if not await barrier("a-confirmed-4"):
		return false
	if role == "b":
		if not await wait_trade(func(t): return t.other.accepted, "a accepted wool"):
			return false
		await press("TradeFrameTradeButton")
	var text := "Trade failed, target doesn't have enough space." if role == "a" else "Trade failed, you don't have enough space."
	if not await wait_error(text):
		return false
	if not await wait_trade(func(t): return t.window_open and not t.player.accepted and not t.other.accepted, "refused trade stays open"):
		return false
	if bag_count(WOOL) != wool_before:
		return fail("wool moved on a refused trade")
	await capture("7-bags-full")
	if not await barrier("full-seen"):
		return false
	if role == "a":
		await press("TradeFrameCancelButton")
	if not await wait_until(func(): return not client.trade_state().window_open, "closed after full bags"):
		return false
	return await barrier("done-4")

# --- steps -----------------------------------------------------------------

## Side b answers the TRADE popup with Yes; both wait for the window and backpack.
func open_trade(tag: String) -> bool:
	if role == "b":
		var text := "Trade with %s?" % partner
		if not await wait_until(func(): return popup_text() == text, "TRADE popup '%s'" % text):
			return false
		await capture(tag + "-popup")
		await click(control("StaticPopup1Button1"))
	if not await wait_trade(func(t): return t.window_open and t.other.name == partner, "open trade window"):
		return false
	# Retail TradeFrame does not open bags; the player opens the backpack to offer.
	if not await open_backpack():
		return false
	return await barrier("open-" + tag)

func open_backpack() -> bool:
	var backpack := control("ContainerFrame0")
	if backpack != null and backpack.is_visible_in_tree():
		return true
	await press("MainMenuBarBackpackButton")
	return await wait_until(func(): var frame := control("ContainerFrame0"); return frame != null and frame.is_visible_in_tree(), "backpack opened")

func target_partner_by_click() -> bool:
	var id := partner_id()
	if id == 0:
		return fail("partner %s not replicated: %s" % [partner, client.trade_state()])
	var transform = client.unit_transform(id)
	var camera := root.get_viewport().get_camera_3d()
	var point := camera.unproject_position(transform.origin + Vector3(0, 1.0, 0))
	await click_at(point, MOUSE_BUTTON_LEFT)
	return await wait_until(func(): return client.target_state().target_name == partner, "partner targeted by click")

func menu_trade() -> bool:
	var frame := control("TargetFrame")
	if frame == null:
		return fail("no TargetFrame")
	await click_at(frame.get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	var item := control("UnitFrameContextMenuTrade")
	if item == null or not item.is_visible_in_tree():
		return fail("unit menu has no Trade entry")
	await capture("0-unit-menu")
	await click(item)
	return true

## Right-click the bag item (ContainerFrame.lua: with TradeFrame shown it is offered).
func offer(item_id: int) -> bool:
	for entry in client.merchant_state().bags:
		if entry.item_id == item_id and entry.bag == 0:
			await click_at(control("ContainerFrame0Slot%d" % entry.slot).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
			return true
	return fail("no %d in the backpack: %s" % [item_id, client.merchant_state().bags])

## Left-click the bag item onto the cursor, then click trade slot `slot` (1-7).
func place_on_slot(item_id: int, slot: int) -> bool:
	for entry in client.merchant_state().bags:
		if entry.item_id == item_id and entry.bag == 0:
			await click(control("ContainerFrame0Slot%d" % entry.slot))
			await press("TradePlayerItem%dItemButton" % slot)
			return await wait_trade(func(t): return slot_item(t.player, slot - 1) == item_id, "%d in trade slot %d" % [item_id, slot])
	return fail("no %d in the backpack: %s" % [item_id, client.merchant_state().bags])

## Type the amount in the gold/silver boxes and press Enter.
func type_money(copper: int) -> bool:
	var gold := control("TradePlayerInputMoneyFrameGold") as LineEdit
	var silver := control("TradePlayerInputMoneyFrameSilver") as LineEdit
	if gold == null or silver == null:
		return fail("no money boxes")
	await click(gold)
	gold.select_all()
	await type_text(str(copper / 10000))
	await click(silver)
	silver.select_all()
	await type_text(str(copper / 100 % 100))
	await tap(KEY_ENTER)
	return await wait_trade(func(t): return t.player.gold == copper, "offered money %d" % copper)

func fill_bags() -> bool:
	var admin := OS.get_environment("TRADE_ADMIN")
	var character := OS.get_environment("TRADE_CHARACTER")
	for attempt in range(20):
		var output := []
		var code := OS.execute(admin, ["grant-item", character, str(FILLER), "1"], output, true)
		print("FIXTURE FILL ", code, " ", output)
		if code != 0:
			break
	return await wait_until(func(): return backpack_full(), "backpack full")

# --- observation -----------------------------------------------------------

func partner_id() -> int:
	return int(client.trade_state().players.get(partner, 0))

func backpack_full() -> bool:
	var used := 0
	for entry in client.merchant_state().bags:
		if entry.bag == 0:
			used += 1
	return used >= 16

func bag_count(item_id: int) -> int:
	var total := 0
	for entry in client.merchant_state().bags:
		if entry.item_id == item_id:
			total += entry.count
	return total

func money() -> int:
	return client.merchant_state().money

func slot_item(party: Dictionary, slot: int) -> int:
	var item = party.slots[slot]
	return item.item_id if item is Dictionary else 0

func popup_text() -> String:
	var popup := control("StaticPopup1")
	var text := control("StaticPopup1Text") as Label
	return text.text if popup != null and popup.is_visible_in_tree() and text != null else ""

func control(name: String) -> Control:
	return client.find_child(name, true, false) as Control

func wait_error(text: String) -> bool:
	return await wait_until(func(): return error_shown(client.get_node_or_null("UIErrors"), text), "UIErrorsFrame '%s'" % text)

func error_shown(node: Node, text: String) -> bool:
	if node == null:
		return false
	if node is Label and node.is_visible_in_tree() and node.text == text:
		return true
	for child in node.get_children():
		if error_shown(child, text):
			return true
	return false

func wait_trade(predicate: Callable, what: String) -> bool:
	return await wait_until(func(): var t: Dictionary = client.trade_state(); return t.has("player") and predicate.call(t), what)

func wait_until(predicate: Callable, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	await capture("fail")
	return fail("%s timed out waiting for %s: trade=%s" % [role, what, client.trade_state()])

func barrier(name: String) -> bool:
	FileAccess.open("%s/%s-%s" % [sync_dir, role, name], FileAccess.WRITE).store_string("1")
	var other := "%s/%s-%s" % [sync_dir, "b" if role == "a" else "a", name]
	var deadline := Time.get_ticks_msec() + 600000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if FileAccess.file_exists(other):
			return true
	return fail("%s: partner never reached %s" % [role, name])

func enter_world(character: String) -> bool:
	var deadline := Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 and client.get_node_or_null("CharacterSelectUI") != null and not state.assets_starting:
			break
	var ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		return fail("No character select: " + str(client.account_state()))
	await click(ui.find_child("CharCard_0", true, false))
	if ui.find_child("CharSelectCharacterName", true, false).text != character:
		return fail("Card 0 is not " + character)
	await click(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			await wait_frames(120)
			return true
	return fail("Timed out entering the world: " + str(client.account_state()))

# --- input -----------------------------------------------------------------

func press(name: String) -> void:
	await click(control(name))

func click(target: Control) -> void:
	await click_at(target.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)

func click_at(point: Vector2, button: int) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(2)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func send_line(line: String) -> bool:
	await tap(KEY_ENTER)
	await type_text(line)
	await tap(KEY_ENTER)
	return true

func type_text(text: String) -> void:
	for character in text:
		var code := OS.find_keycode_from_string(character.to_upper())
		if character == "/":
			code = KEY_SLASH
		push_key(code, character.unicode_at(0), true)
		await process_frame
		push_key(code, 0, false)
		await process_frame

func tap(code: Key) -> void:
	push_key(code, 0, true)
	await process_frame
	push_key(code, 0, false)
	await wait_frames(2)

func push_key(code: Key, unicode: int, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.unicode = unicode
	event.pressed = pressed
	root.push_input(event, true)

func capture(name: String) -> void:
	await RenderingServer.frame_post_draw
	var path := "%s/%s-%s.png" % [shots, role, name]
	root.get_texture().get_image().save_png(path)
	print("FIXTURE SHOT ", path)

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> bool:
	push_error(message)
	quit(1)
	return false
