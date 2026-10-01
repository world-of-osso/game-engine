extends "res://tests/world_merchant_flow.gd"

# Live private-server MerchantFrame run (docs/specs/merchant-frame.md). Every action is a
# physical pointer event; every expectation is the server's reply (Gold, InventoryDelta,
# DurabilityStateUpdate, BuybackList). The server logs each decoded vendor request
# ("merchant request ..."), which the runner compares with the markers printed here.
#
# MERCHANT_LIVE_STAGE=godric (Godric Rothgar 1213, repairer, 8 items): Repair An Item on
#   the equipped main hand, Repair All, Sell All Junk, buy, sell, buy back, sounds.
# MERCHANT_LIVE_STAGE=tharynn (Tharynn Bouden 66, more than 10 items): page buttons,
#   mouse wheel, Buy / UnableBuy cursor.
# Setup (offline, game-server-admin): see the stage functions.

const LIVE_ACCOUNT := "fb_merchant"
const LIVE_CHARACTER := "Fbmerchant"
const LIVE_WAIT_MS := 10000
const PELT := 4865
const SHOT_DIR_ENV := "MERCHANT_LIVE_SHOTS"
var shots := ""

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	shots = OS.get_environment(SHOT_DIR_ENV)
	if server.is_empty() or server == "127.0.0.1:5000" or shots.is_empty():
		fail("Live merchant needs a private GODOT_TEST_SERVER and " + SHOT_DIR_ENV)
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, LIVE_ACCOUNT, PASSWORD, false)
	if error != "":
		fail("Live connection: " + error)
		return
	if not await enter_live_world():
		return
	var stage := OS.get_environment("MERCHANT_LIVE_STAGE")
	var passed := false
	match stage:
		"godric":
			passed = await godric_stage()
		"tharynn":
			passed = await tharynn_stage()
		_:
			fail("Unknown MERCHANT_LIVE_STAGE " + stage)
			return
	if passed:
		print("LIVE MERCHANT DONE stage=", stage)
		client.free()
		quit(0)

# Setup: set-durability Fbmerchant 50, set-gold Fbmerchant 10000, grant-item Fbmerchant 4865 2,
# teleport next to Godric Rothgar.
func godric_stage() -> bool:
	var vendor := await find_named_vendor("Godric Rothgar")
	if vendor.is_empty() or not await open_named_vendor(vendor, "Godric Rothgar"):
		return false
	if not await expect_sound(839, "MerchantFrame OnShow"):
		return false
	await capture_live("01-godric-open.png")
	var state: Dictionary = client.merchant_state()
	if state.repair_cost <= 0:
		fail("set-durability 50 left nothing to repair: " + str(state))
		return false
	if not await expect_repair_all_tooltip(state.repair_cost):
		return false
	if not await repair_main_hand():
		return false
	if not await repair_all():
		return false
	if not await sell_junk():
		return false
	if not await buy_sell_buy_back():
		return false
	var closes := sound_count(840)
	await click_control(merchant_control("MerchantFrameCloseButton"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(s): return not s.open and sound_count(840) == closes + 1, "close + OnHide sound"):
		return false
	print("LIVE MERCHANT CLOSED sound840")
	return true

func expect_repair_all_tooltip(cost: int) -> bool:
	await hover_point(merchant_control("MerchantRepairAllButton").get_global_rect().get_center())
	await frames(3)
	var tooltip: Dictionary = client.tooltip_state()
	if not tooltip.visible or tooltip.title != "Repair All Items":
		fail("Repair All tooltip: " + str(tooltip))
		return false
	print("LIVE MERCHANT TOOLTIP Repair All Items cost=", cost, " lines=", tooltip.lines)
	await capture_live("02-repair-all-tooltip.png")
	await hover_point(merchant_control("MerchantRepairItemButton").get_global_rect().get_center())
	await frames(3)
	tooltip = client.tooltip_state()
	if not tooltip.visible or tooltip.title != "Repair an Item":
		fail("Repair an Item tooltip: " + str(tooltip))
		return false
	await hover_point(merchant_control("MerchantSellAllJunkButton").get_global_rect().get_center())
	await frames(3)
	tooltip = client.tooltip_state()
	if not tooltip.visible or tooltip.title != "Sell All Junk Items":
		fail("Sell All Junk tooltip: " + str(tooltip))
		return false
	print("LIVE MERCHANT TOOLTIP service buttons ok")
	return true

# MerchantRepairItemButton, then the repair cursor on CharacterMainHandSlot.
func repair_main_hand() -> bool:
	await click_control(merchant_control("MerchantRepairItemButton"), MOUSE_BUTTON_LEFT)
	await frames(2)
	if not client.merchant_state().repair_mode or client.merchant_state().cursor != "Repair":
		fail("Repair An Item did not show the repair cursor: " + str(client.merchant_state()))
		return false
	await click_control(named_control("MicroMenuUI", "CharacterMicroButton"), MOUSE_BUTTON_LEFT)
	var deadline := Time.get_ticks_msec() + LIVE_WAIT_MS
	while not client.character_frame_state().open and Time.get_ticks_msec() < deadline:
		await process_frame
	await frames(5)
	var slot := named_control("CharacterFrameUI", "CharacterMainHandSlot")
	if slot == null or not slot.is_visible_in_tree():
		fail("CharacterMainHandSlot is not shown")
		return false
	await capture_live("03-repair-cursor.png")
	var before: Dictionary = client.merchant_state()
	var weapon := equipped(before, "MainHand")
	if weapon.is_empty():
		fail("No equipped main hand: " + str(before.equipment))
		return false
	print("LIVE MERCHANT REQUEST RepairItem item_guid=Some(", weapon.item_guid, ")")
	await click_control(slot, MOUSE_BUTTON_LEFT)
	if not await wait_for(func(s): return s.repair_cost < before.repair_cost and s.money == before.money - (before.repair_cost - s.repair_cost), "main-hand repair"):
		return false
	var after: Dictionary = client.merchant_state()
	print("LIVE MERCHANT REPAIRED main hand: cost ", before.repair_cost, " -> ", after.repair_cost, " money ", before.money, " -> ", after.money)
	if not after.repair_mode:
		fail("The repair cursor hid after one repair")
		return false
	# Close the CharacterFrame first: it may cover the MerchantFrame's buttons.
	await click_control(named_control("MicroMenuUI", "CharacterMicroButton"), MOUSE_BUTTON_LEFT)
	await frames(5)
	await click_control(merchant_control("MerchantRepairItemButton"), MOUSE_BUTTON_LEFT)
	await frames(2)
	if client.merchant_state().repair_mode or client.merchant_state().cursor == "Repair":
		fail("A second Repair An Item click kept the repair cursor")
		return false
	return true

func repair_all() -> bool:
	var before: Dictionary = client.merchant_state()
	var repairs := sound_count(7994)
	print("LIVE MERCHANT REQUEST RepairItem item_guid=None")
	await click_control(merchant_control("MerchantRepairAllButton"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(s): return s.repair_cost == 0 and s.money == before.money - before.repair_cost, "Repair All"):
		return false
	if sound_count(7994) != repairs + 1:
		fail("Repair All played ITEM_REPAIR %d times" % (sound_count(7994) - repairs))
		return false
	print("LIVE MERCHANT REPAIRED all: money ", before.money, " -> ", client.merchant_state().money, " sound7994")
	await capture_live("04-repaired.png")
	return true

func sell_junk() -> bool:
	var before: Dictionary = client.merchant_state()
	if bag_count(before, PELT) != 2:
		fail("Setup needs 2 Ruined Pelt: " + str(before.bags))
		return false
	print("LIVE MERCHANT REQUEST SellAllJunkItems")
	await click_control(merchant_control("MerchantSellAllJunkButton"), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(s): return bag_count(s, PELT) == 0 and s.buyback.has("Ruined Pelt") and s.money > before.money, "Sell All Junk"):
		return false
	print("LIVE MERCHANT SOLD junk: money ", before.money, " -> ", client.merchant_state().money, " buyback ", client.merchant_state().buyback)
	await capture_live("05-junk-sold.png")
	return true

func buy_sell_buy_back() -> bool:
	var before: Dictionary = client.merchant_state()
	var name: String = before.items[0]
	print("LIVE MERCHANT REQUEST BuyItem ", name)
	await click_control(merchant_control("MerchantItem1"), MOUSE_BUTTON_RIGHT)
	if not await wait_for(func(s): return named_count(s, name) == named_count(before, name) + 1 and s.money < before.money, "buy " + name):
		return false
	var bought: Dictionary = client.merchant_state()
	print("LIVE MERCHANT BOUGHT ", name, ": money ", before.money, " -> ", bought.money)
	var slot := named_slot(bought, name)
	print("LIVE MERCHANT REQUEST SellItem ", name)
	await click_control(merchant_control("ContainerFrame%dSlot%d" % [slot.bag, slot.slot]), MOUSE_BUTTON_RIGHT)
	if not await wait_for(func(s): return named_count(s, name) == named_count(before, name) and s.buyback.has(name) and s.money > bought.money, "sell " + name):
		return false
	var sold: Dictionary = client.merchant_state()
	print("LIVE MERCHANT SOLD ", name, ": money ", bought.money, " -> ", sold.money)
	await click_control(merchant_control("MerchantFrameTab2"), MOUSE_BUTTON_LEFT)
	await frames(3)
	var index: int = sold.buyback.find(name)
	await capture_live("06-buyback-tab.png")
	print("LIVE MERCHANT REQUEST BuybackItemRequest ", name)
	await click_control(merchant_control("MerchantItem%d" % (index + 1)), MOUSE_BUTTON_LEFT)
	if not await wait_for(func(s): return named_count(s, name) == named_count(before, name) + 1 and not s.buyback.has(name) and s.money < sold.money, "buy back " + name):
		return false
	print("LIVE MERCHANT BOUGHT BACK ", name, ": money ", sold.money, " -> ", client.merchant_state().money)
	await click_control(merchant_control("MerchantFrameTab1"), MOUSE_BUTTON_LEFT)
	await frames(3)
	return true

# Setup: set-gold Fbmerchant 0, teleport next to Tharynn Bouden.
func tharynn_stage() -> bool:
	var vendor := await find_named_vendor("Tharynn Bouden")
	if vendor.is_empty() or not await open_named_vendor(vendor, "Tharynn Bouden"):
		return false
	var count: int = client.merchant_state().items.size()
	var pages := int(ceil(count / 10.0))
	if pages < 2 or merchant_text("MerchantPageText") != "Page 1 of %d" % pages:
		fail("Paging text '%s' for %d items" % [merchant_text("MerchantPageText"), count])
		return false
	await capture_live("11-tharynn-page1.png")
	var pages_sound := sound_count(856)
	await click_control(merchant_control("MerchantNextPageButton"), MOUSE_BUTTON_LEFT)
	await frames(3)
	if merchant_text("MerchantPageText") != "Page 2 of %d" % pages or sound_count(856) != pages_sound + 1:
		fail("Next page: '%s' sounds %d" % [merchant_text("MerchantPageText"), sound_count(856) - pages_sound])
		return false
	await capture_live("12-tharynn-page2.png")
	var center := merchant_control("MerchantFrameInset").get_global_rect().get_center() if merchant_control("MerchantFrameInset") != null else merchant_control("MerchantFrame").get_global_rect().get_center()
	await wheel(center, MOUSE_BUTTON_WHEEL_UP)
	if merchant_text("MerchantPageText") != "Page 1 of %d" % pages:
		fail("Wheel up did not page back: " + merchant_text("MerchantPageText"))
		return false
	await wheel(center, MOUSE_BUTTON_WHEEL_UP)
	if merchant_text("MerchantPageText") != "Page 1 of %d" % pages:
		fail("Wheel up on page 1 changed the page")
		return false
	await wheel(center, MOUSE_BUTTON_WHEEL_DOWN)
	if merchant_text("MerchantPageText") != "Page 2 of %d" % pages:
		fail("Wheel down did not page forward: " + merchant_text("MerchantPageText"))
		return false
	print("LIVE MERCHANT PAGING buttons+wheel ok pages=", pages, " sounds856=", sound_count(856) - pages_sound)
	await click_control(merchant_control("MerchantPrevPageButton"), MOUSE_BUTTON_LEFT)
	await hover_point(merchant_control("MerchantItem1").get_global_rect().get_center())
	await frames(3)
	var cursor: String = client.merchant_state().cursor
	if cursor != "UnableBuy":
		fail("Gold0 over a priced item shows cursor '%s'" % cursor)
		return false
	print("LIVE MERCHANT CURSOR UnableBuy at Gold0")
	await capture_live("13-unable-buy.png")
	await tap(KEY_ESCAPE)
	return await wait_for(func(s): return not s.open, "Escape close")

func wheel(point: Vector2, button: MouseButton) -> void:
	await hover_point(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await frames(3)

func sound_count(kit: int) -> int:
	return client.merchant_state().ui_sounds.count(kit)

func expect_sound(kit: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + LIVE_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		if sound_count(kit) >= 1:
			print("LIVE MERCHANT SOUND ", kit, " ", what)
			return true
		await process_frame
	fail("%s did not play sound kit %d: %s" % [what, kit, client.merchant_state().ui_sounds])
	return false

func equipped(state: Dictionary, slot: String) -> Dictionary:
	for item in state.equipment:
		if item.slot == slot:
			return item
	return {}

func named_count(state: Dictionary, name: String) -> int:
	var count := 0
	for item in state.bags:
		if item.name == name:
			count += item.count
	return count

func named_slot(state: Dictionary, name: String) -> Dictionary:
	for item in state.bags:
		if item.name == name:
			return item
	return {}

func named_control(ui_name: String, name: String) -> Control:
	var ui = client.get_node_or_null(ui_name)
	return ui.find_child(name, true, false) if ui != null else null

func capture_live(file: String) -> void:
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(shots.path_join(file))

func open_named_vendor(npc: Dictionary, vendor: String) -> bool:
	await click(npc.point, MOUSE_BUTTON_RIGHT)
	if not await wait_for(func(s): return s.open and s.vendor_name == vendor, vendor + " frame"):
		return false
	await frames(3)
	print("LIVE MERCHANT OPEN ", vendor, " items ", client.merchant_state().items, " money ", client.merchant_state().money)
	return merchant_visible("MerchantFrame")

func find_named_vendor(vendor: String) -> Dictionary:
	var deadline := Time.get_ticks_msec() + NPC_WAIT_MS
	var turned := 0
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var units := client.get_node_or_null("WorldUnits")
		if units == null or camera() == null:
			continue
		for unit in units.get_children():
			if str(unit.name) != vendor:
				continue
			var area := unit.find_child("UnitPick", true, false) as Area3D
			if area == null:
				continue
			var world_point := (area.get_child(0) as Node3D).global_position
			if not camera().is_position_in_frustum(world_point):
				continue
			var point := camera().unproject_position(world_point)
			var id = area.get_meta("unit_server_id")
			if UnitPicker.pick(camera(), point) == id:
				return {"id": id, "name": vendor, "point": point}
		if turned < 60:
			push_key(KEY_RIGHT, true)
			await frames(3)
			push_key(KEY_RIGHT, false)
			turned += 1
	fail("%s is not visible and unoccluded" % vendor)
	return {}

func enter_live_world() -> bool:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1:
			break
	var ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	var card: Control = null
	var index := 0
	while card == null:
		var candidate = ui.find_child("CharCard_%d" % index, true, false)
		if not candidate is Control:
			break
		for label in candidate.find_children("*", "Label", true, false):
			if label.text == LIVE_CHARACTER:
				card = candidate
		index += 1
	if card == null:
		fail("Roster has no " + LIVE_CHARACTER)
		return false
	await click_control(card, MOUSE_BUTTON_LEFT)
	await click_control(ui.find_child("EnterWorld", true, false), MOUSE_BUTTON_LEFT)
	deadline = Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null:
			await frames(60)
			print("LIVE MERCHANT IN_WORLD ", state.selected_character_name, " at ", state.local_player_position)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false
