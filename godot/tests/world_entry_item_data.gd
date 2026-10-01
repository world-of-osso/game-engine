extends "res://tests/world_bags_flow.gd"

## World entry while the item catalog still loads: the inventory snapshot is applied
## without waiting for the load, its items wait for their data, and once the catalog is
## loaded the bags and the item tooltip show the catalog's names, quality and icons
## (Retail's GET_ITEM_INFO_RECEIVED model). Environment:
##   GODOT_TEST_SERVER                             server address (a private test server)
##   WORLD_ENTRY_ACCOUNT / WORLD_ENTRY_CHARACTER   account (password fbtest) and character
## whose bags hold Linen Cloth (2589).

const PASSWORD := "fbtest"
const LINEN := 2589
## `INV_Misc_QuestionMark`: the icon of an item whose data has not arrived.
const QUESTION_MARK_FDID := 134400
## Item.IconFileDataID of Linen Cloth (build 12.1.0.69933).
const LINEN_ICON_FDID := 132889
const CATALOG_WAIT_MS := 300000

## The longest frame between the snapshot's arrival and the catalog's load.
var longest_pending_ms := 0.0

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("WORLD_ENTRY_ACCOUNT")
	var character := OS.get_environment("WORLD_ENTRY_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, WORLD_ENTRY_ACCOUNT and WORLD_ENTRY_CHARACTER are required")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await select_and_enter(client, character):
		return
	# The first frame with the snapshot applied: the catalog must still be loading, or
	# this run did not exercise the early snapshot.
	var arrival := await wait_inventory(client)
	if arrival.is_empty():
		return
	if arrival.item_catalog_loaded:
		fail("The item catalog loaded before the inventory arrived; the early snapshot was not exercised")
		return
	for item in arrival.bags:
		if item.name != "" or item.icon_fdid != QUESTION_MARK_FDID:
			fail("A bag item showed data before the catalog loaded: " + str(item))
			return
	print("FIXTURE ITEM_DATA_PENDING bags=", arrival.bags)
	var loaded := await wait_catalog(client)
	if loaded.is_empty():
		return
	print("FIXTURE ITEM_DATA_LOADED longest_pending_frame_ms=%.1f bags=%s equipment=%s" % [longest_pending_ms, loaded.bags, loaded.equipment])
	if not check_resolved(loaded):
		return
	if not await check_linen_tooltip(client, loaded):
		return
	print("FIXTURE ITEM_DATA_DONE")
	client.free()
	quit(0)

func select_and_enter(client: Node, character: String) -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	var ui: Node = null
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		ui = client.get_node_or_null("CharacterSelectUI")
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 and ui != null:
			break
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await click(ui.find_child("CharCard_0", true, false))
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != character:
		fail("Card 0 is %s, not %s" % [selected.text, character])
		return false
	await click(ui.find_child("EnterWorld", true, false))
	return true

func wait_inventory(client: Node) -> Dictionary:
	var deadline := Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.merchant_state()
		if not state.bags.is_empty():
			return state
	fail("No inventory snapshot arrived: " + str(client.account_state()))
	return {}

func wait_catalog(client: Node) -> Dictionary:
	var deadline := Time.get_ticks_msec() + CATALOG_WAIT_MS
	var previous := Time.get_ticks_usec()
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var now := Time.get_ticks_usec()
		longest_pending_ms = maxf(longest_pending_ms, (now - previous) / 1000.0)
		previous = now
		var state: Dictionary = client.merchant_state()
		if state.item_catalog_loaded:
			# The bags take the data in the frame step after the load is seen.
			await process_frame
			return client.merchant_state()
	fail("The item catalog did not load within %d ms" % CATALOG_WAIT_MS)
	return {}

func check_resolved(state: Dictionary) -> bool:
	var linen_found := false
	for item in state.bags:
		if item.name == "" or item.icon_fdid == QUESTION_MARK_FDID:
			fail("Bag item without its data after the catalog loaded: " + str(item))
			return false
		if item.item_id == LINEN:
			linen_found = true
			if item.name != "Linen Cloth" or item.icon_fdid != LINEN_ICON_FDID or item.quality != 1:
				fail("Linen Cloth resolved wrong: " + str(item))
				return false
	for item in state.equipment:
		if item.name == "" or item.icon_fdid == QUESTION_MARK_FDID:
			fail("Equipped item without its data after the catalog loaded: " + str(item))
			return false
	if not linen_found:
		fail("The character's bags hold no Linen Cloth: " + str(state.bags))
		return false
	return true

func check_linen_tooltip(client: Node, state: Dictionary) -> bool:
	var linen_slot := -1
	for item in state.bags:
		if item.item_id == LINEN and item.bag == 0:
			linen_slot = item.slot
	if linen_slot < 0:
		fail("Linen Cloth is not in the backpack: " + str(state.bags))
		return false
	var backpack := await wait_bag_button(client, BACKPACK)
	if backpack == null:
		return false
	await click(backpack)
	if not await wait_container(client, 0, true):
		return false
	if not await hover_bag_slot(client, linen_slot):
		return false
	if not await wait_tooltip_title(client, "Linen Cloth"):
		return false
	var title := authored_control(client, "TooltipTitle") as Label
	print("FIXTURE ITEM_DATA_TOOLTIP title=", title.text, " color=", title.get_theme_color("font_color"))
	if not title.get_theme_color("font_color").is_equal_approx(Color.WHITE):
		fail("Linen Cloth tooltip title must be Common white")
		return false
	return true

func fail(message: String) -> void:
	push_error(message)
	print("FIXTURE FAIL ", message)
	quit(1)
