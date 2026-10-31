extends "res://tests/mount_journal_live.gd"
# Real mount teaching item, two accounts, native AH/payment/mail/UseItem authority.
const AUCTION_ITEM := 44413 # Mekgineer's Chopper, Mount275 / spell60424.
const AUCTION_MOUNT := 275
var mode := OS.get_environment("MOUNT_AUCTION_MODE")
var ids_file := OS.get_environment("MOUNT_AUCTION_IDS_FILE")

func run_fixture() -> void:
	root.size = Vector2i(1280, 720)
	var account := OS.get_environment("MOUNT_ACCOUNT")
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5901" or not account.begins_with("fb_mount_") or mode not in ["seller", "buyer"] or ids_file.is_empty():
		fail("Explicit private5901, fb_mount_*, AH mode and IDs path required"); return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, "asset startup", 180000): return
	var error: String = client.connect_account("127.0.0.1:5901", account, "fbtest", false)
	if error != "": fail(error); return
	if not await wait_until(func(): return client.get_node_or_null("CharacterSelectUI") != null, "character select", 120000): return
	await click("CharCard_" + OS.get_environment("MOUNT_CHARACTER_INDEX"))
	await click("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and int(client.mount_journal_state().catalog_count) > 0, "world/catalog", 180000): return
	FileAccess.open(OS.get_environment("MOUNT_READY_FILE"), FileAccess.WRITE).store_string(str(client.account_state()))
	if not await wait_until(func(): return FileAccess.file_exists(OS.get_environment("MOUNT_GO_FILE")), "online setup", 120000): return
	if client.account_state().ui_skin != "Forever": fail("Wrong AH proof skin"); return
	if not await open_auction(): return
	if mode == "seller":
		if not await post_mounts(): return
	else:
		if not await buy_and_learn(): return
	print("MOUNT_AUCTION_LIVE PASS skin=", skin, " mode=", mode, " mounts=", client.mount_journal_state(), " auctions=", client.auction_state())
	client.free()
	quit(0)

func open_auction() -> bool:
	if (await ipc(["quest", "interact", "--npc", "Auctioneer Fitch"])).is_empty(): return false
	return await wait_until(func(): return client.auction_state().open and control("AuctionHouseFrameTab1") != null, "authentic auctioneer and rendered Retail AH")

func action(token: String) -> bool:
	var ui := client.get_node_or_null("AuctionUI")
	if ui == null: fail("AH host missing"); return false
	var target := ui.control_for_action(token) as Control
	if target == null or not target.is_visible_in_tree(): fail("AH action unavailable: " + token); return false
	await motion(target.get_global_rect().get_center(), false)
	await button_event(target.get_global_rect().get_center(), MOUSE_BUTTON_LEFT, true)
	await button_event(target.get_global_rect().get_center(), MOUSE_BUTTON_LEFT, false)
	await wait_frames(5)
	return true

func fill(name: String, value: String) -> void:
	await click(name)
	# Actual Ctrl+A and typing; no model or registry state injection.
	for down in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_A
		event.physical_keycode = KEY_A
		event.ctrl_pressed = true
		event.pressed = down
		root.push_input(event, true)
		await process_frame
	await type_text(value)

func post_mounts() -> bool:
	var ids := []
	for price in [10, 11]:
		await click("AuctionHouseFrameTab2")
		if not await wait_until(func(): return client.auction_state().inventory.any(func(row): return int(row.item_id) == AUCTION_ITEM), "unbound authentic mount sell inventory"): return false
		var item: Dictionary = {}
		for row in client.auction_state().inventory:
			if int(row.item_id) == AUCTION_ITEM: item = row; break
		if not await action("auction_sell_item:%s" % item.guid): return false
		await fill("AuctionHouseFrameItemSellFrameQuantityInputBox", "1")
		await fill("AuctionHouseFrameItemSellFramePriceInputGold", str(price))
		await capture("ah-mount-listing-%d" % price)
		await click("AuctionHouseFrameItemSellFramePostButton")
		if not await wait_until(func(): return client.auction_state().owned.any(func(row): return row.auction_id not in ids), "persisted mount auction"): return false
		for row in client.auction_state().owned:
			if row.auction_id not in ids: ids.append(row.auction_id)
		await wait_frames(30)
	FileAccess.open(ids_file, FileAccess.WRITE).store_string(JSON.stringify(ids))
	if not await browse_mounts(): return false
	await capture("ah-mounts-category-posted")
	return true

func browse_mounts() -> bool:
	await click("AuctionHouseFrameTab1")
	# Blizzard_AuctionData: Miscellaneous class15 -> Mount subclass5.
	if not await action("auction_category:13"): return false
	if not await action("auction_category:13/4"): return false
	await fill("AuctionHouseFrameSearchBox", "Mekgineer")
	var revision: int = client.auction_state().search_revision
	if not await action("auction_search"): return false
	return await wait_until(func(): return client.auction_state().search_revision > revision and client.auction_state().groups.any(func(row): return int(row.item_id) == AUCTION_ITEM), "Retail Miscellaneous Mount results")

func buy_and_learn() -> bool:
	var ids: Array = JSON.parse_string(FileAccess.get_file_as_string(ids_file))
	if ids.size() != 2: fail("Two native seller auctions required"); return false
	if not await browse_mounts(): return false
	await capture("ah-mounts-category-before-buy")
	var revision: int = client.auction_state().search_revision
	if not await action("auction_browse_item:%d" % AUCTION_ITEM): return false
	if not await wait_until(func(): return client.auction_state().search_revision > revision and ids.all(func(id): return client.auction_state().search.any(func(row): return int(row.auction_id) == int(id))), "both authentic seller teaching item listings"): return false
	var before: int = client.auction_state().money
	if not await action("auction_select:%s" % int(ids[0])) or not await action("auction_buyout"): return false
	if not await wait_until(func(): return control("StaticPopup1Button1") != null, "buyout confirmation"): return false
	await capture("ah-buyout-confirm")
	await click("StaticPopup1Button1")
	if not await wait_until(func(): return int(client.auction_state().money) == before - 100000 and client.auction_state().search.all(func(row): return int(row.auction_id) != int(ids[0])), "paid second-account mount buyout"): return false
	await capture("ah-paid-buyout")
	await tap(KEY_ESCAPE)
	FileAccess.open(OS.get_environment("MOUNT_MAIL_READY_FILE"), FileAccess.WRITE).store_string("paid")
	if not await wait_until(func(): return FileAccess.file_exists(OS.get_environment("MOUNT_MAIL_GO_FILE")), "private mailbox positioning", 120000): return false
	if (await ipc(["quest", "interact", "--npc", "Mailbox"])).is_empty(): return false
	if not await wait_until(func(): return client.mail_state().open and control("MailItem1Button") != null, "real auction-won mailbox"): return false
	await click("MailItem1Button")
	if not await wait_until(func(): return control("OpenMailAttachmentButton1") != null, "auction-won mount attachment"): return false
	await capture("ah-auction-won-mount-mail")
	await click("OpenMailAttachmentButton1")
	if not await wait_until(func(): return has_item(AUCTION_ITEM), "mount attachment bag delivery"): return false
	await tap(KEY_ESCAPE)
	await click("MainMenuBarBackpackButton")
	if not await wait_until(func(): return control("ContainerFrame0") != null, "buyer backpack"): return false
	var bag_name := ""
	for entry in client.merchant_state().bags:
		if int(entry.item_id) == AUCTION_ITEM: bag_name = "ContainerFrame%dSlot%d" % [entry.bag, entry.slot]
	await click(bag_name, MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return client.mount_journal_state().collected.any(func(mount): return int(mount.mount_id) == AUCTION_MOUNT) and not has_item(AUCTION_ITEM), "AH item consumed and account Mount275 learned"): return false
	await tap(KEY_ESCAPE)
	await open_mounts()
	if not await wait_until(func(): return control("MountJournalSearchBox") != null, "Mount Journal after AH learning"): return false
	await click("MountJournalSearchBox")
	await type_text("Mekgineer")
	if not await wait_until(func(): return int(client.mount_journal_state().selected) == AUCTION_MOUNT, "learned authored mount journal details"): return false
	await capture("ah-learned-journal")
	await tap(KEY_ESCAPE)
	FileAccess.open(OS.get_environment("MOUNT_AH_RETURN_READY_FILE"), FileAccess.WRITE).store_string("learned")
	if not await wait_until(func(): return FileAccess.file_exists(OS.get_environment("MOUNT_AH_RETURN_GO_FILE")), "private auctioneer repositioning", 120000): return false
	if not await open_auction() or not await browse_mounts(): return false
	var row := control("AuctionHouseFrameBrowseResultsRow1")
	await motion(row.get_global_rect().get_center(), false)
	if not await wait_until(func(): return "Already known|" in client.tooltip_state().lines, "AH already-known teaching item tooltip"): return false
	await capture("ah-already-known")
	return true
