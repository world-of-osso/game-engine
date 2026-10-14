extends SceneTree
# Trading Post live proof: native input and read-only observations against an owned
# private server only. TP_PHASE=browse captures the Log, categories, previews, freeze
# and purchase; TP_PHASE=relog proves the frozen slot and purchase survived.
var client: Node
var shots := OS.get_environment("TP_SHOTS")
var receipts := OS.get_environment("TP_RECEIPTS")
var skin := OS.get_environment("TP_SKIN")
var phase := OS.get_environment("TP_PHASE")
var server := OS.get_environment("TP_SERVER")

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_fixture")

func run_fixture() -> void:
	root.size = Vector2i(1920, 1080)
	if DisplayServer.screen_get_size() != Vector2i(1920, 1080):
		fail("Private compositor must expose 1920x1080 output")
		return
	var account := OS.get_environment("TP_ACCOUNT")
	if server != "127.0.0.1:5791" or not account.begins_with("fb_") or shots.is_empty() or receipts.is_empty() or not phase in ["layout", "layout-proof", "browse", "relog", "purchase", "refund", "cart", "cart-lifetime"]:
		fail("Explicit private 127.0.0.1:5791, fb_* account, phase and evidence paths required")
		return
	for directory in [shots, receipts]:
		if DirAccess.make_dir_recursive_absolute(directory) != OK:
			push_error("Cannot create Trading Post evidence directory: " + directory)
			quit(1)
			return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, "asset startup", 180000): return
	var error: String = client.connect_account(server, account, OS.get_environment("TP_PASSWORD"), false)
	if error != "": fail(error); return
	if not await wait_until(func(): return client.get_node_or_null("CharacterSelectUI") != null, "character select", 120000): return
	await click("CharCard_0")
	await click("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and control("MinimapLauncherButton") != null, "world", 180000): return
	var expected := "Modern" if skin == "modern" else "Forever"
	if client.account_state().ui_skin != expected:
		fail("Active skin %s, expected %s" % [client.account_state().ui_skin, expected]); return
	if not await wait_until(func(): return float(client.account_state().local_player_health) > 0.0, "private character alive", 120000): return
	client.set_world_minutes(720.0)
	await wait_frames(120)
	if not phase in ["layout", "layout-proof", "cart", "cart-lifetime"] and not await travelers_log(): return
	if not await open_vendor(): return
	if phase in ["cart", "cart-lifetime"]:
		# Collection UI owners may replace a hidden CanvasLayer while this vendor is open.
		var old_layer := CanvasLayer.new()
		old_layer.name = "TradingPostLayerLifetimeFixture"
		client.add_child(old_layer)
		await wait_frames(3)
		old_layer.free()
		var new_layer := CanvasLayer.new()
		new_layer.name = "TradingPostLayerLifetimeFixture"
		client.add_child(new_layer)
		await wait_frames(3)
		if new_layer.visible: fail("Replacement UI owner stayed visible over vendor"); return
	match phase:
		"layout":
			await capture("vendor-all")
			await click("PerksProgramFilter")
			if not await wait_until(func(): return control("PerksProgramFilterMenu") != null, "filter description"): return
			await capture("category-dropdown")
			await choose_category(1)
			await capture("vendor-transmog")
			finish("layout")
		"layout-proof": await layout_proof()
		"relog": await relog_checks()
		"purchase": await purchase_each_collection()
		"cart": await checkout_cart()
		"cart-lifetime": await finish("layer-lifetime")
		"refund": await refund_after_relog()
		_: await browse()

func layout_proof() -> void:
	print("TP_LAYOUT_INITIAL footer=", control("PerksProgramFooterFrame").get_global_rect(), " purchase=", control("PerksProgramPurchaseButton").get_global_rect(), " filter=", control("PerksProgramFilter").get_global_rect())
	if control("PerksProgramFilterBackground").mouse_filter != Control.MOUSE_FILTER_IGNORE:
		fail("Decorative WowStyle2 Button intercepted owner input")
		return
	if not assert_vendor_rects(): return
	if not await assert_header_hover(): return
	await capture("vendor-all")
	await click("PerksProgramPriceSortButton")
	if not await wait_until(func(): return client.trading_post_state().ascending, "ascending price header"): return
	await capture("price-ascending")
	await click("PerksProgramNameSortButton")
	if not await wait_until(func(): return client.trading_post_state().sort == "name", "name header"): return
	await click("PerksProgramFilter")
	if not await wait_until(func(): return control("PerksProgramFilterMenu") != null, "filter description"): return
	await capture("filter-description")
	if not await assert_filter_interactions(): return
	if not await choose_category(2): return
	if not await select_first_available(int(client.trading_post_state().tender)): return
	if not assert_masked_icon("PerksProgramIcon%d" % int(client.trading_post_state().selected)): return
	await capture("vendor-mount")
	var first := int(client.trading_post_state().selected)
	if not await drag_to_frozen("PerksProgramProduct%d" % first): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) == first and not client.trading_post_state().pending, "native freeze", 180000): return
	if not assert_masked_icon("PerksProgramFrozenIcon"): return
	await capture("frozen")
	if not await choose_category(1): return
	if not await select_first_available(int(client.trading_post_state().tender)): return
	var replacement := int(client.trading_post_state().selected)
	if replacement == first: fail("replacement did not change vendor identity"); return
	if not await drag_to_frozen("PerksProgramProduct%d" % replacement): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) == replacement and not client.trading_post_state().pending, "replace frozen item", 180000): return
	await capture("frozen-replaced")
	if not await drag_off_frozen(): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) < 0 and not client.trading_post_state().pending, "native drag-off unfreeze", 180000): return
	await capture("unfrozen")
	var tender := int(client.trading_post_state().tender)
	var price := int(client.trading_post_state().price)
	var footer := control("PerksProgramFooterFrame")
	print("TP_LAYOUT_FOOTER ", footer.get_global_rect() if footer != null else "no projected control", " button=", control("PerksProgramPurchaseButton").get_global_rect(), " model=", control("PerksProgramMainModelScene").get_global_rect(), " enabled=", control_enabled("PerksProgramPurchaseButton"))
	await click("PerksProgramPurchaseButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "source popup"): return
	if not await wait_until(func(): return texture_ready("PerksProgramConfirmItem") and texture_ready("PerksProgramConfirmButtonNormalArt") and texture_ready("PerksProgramConfirmBorder"), "decoded StaticPopup source art"): return
	if not assert_dialog_rects_and_art(): return
	await capture("purchase-confirm")
	await tap(KEY_ESCAPE)
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") == null and client.trading_post_state().open, "Escape cancels popup without leaving vendor"): return
	if int(client.trading_post_state().tender) != tender: fail("cancel changed Tender"); return
	await click("PerksProgramPurchaseButton")
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return client.trading_post_state().purchased and int(client.trading_post_state().tender) == tender-price, "authoritative purchase"): return
	await capture("purchased")
	await click("PerksProgramRefundButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "refund popup"): return
	await capture("refund-confirm")
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return not client.trading_post_state().purchased and int(client.trading_post_state().tender) == tender, "authoritative refund"): return
	await capture("refunded")
	finish("layout-proof")

func assert_filter_interactions() -> bool:
	await click("PerksProgramFilterCollected")
	if not await wait_until(func(): return not client.trading_post_state().filter_collected, "Collected checkbox"): return false
	await click("PerksProgramFilterUncollected")
	if not await wait_until(func(): return not client.trading_post_state().filter_uncollected and client.trading_post_state().rows.is_empty(), "both collection flags exclude all rows"): return false
	await capture("filter-empty")
	await click("PerksProgramFilterResetButton")
	if not await wait_until(func(): return client.trading_post_state().filter_collected and client.trading_post_state().filter_uncollected and not client.trading_post_state().rows.is_empty(), "Reset restores source filter flags"): return false
	if control("PerksProgramFilterMenu") == null: await click("PerksProgramFilter")
	await click("PerksProgramFilterUsable")
	if not await wait_until(func(): return client.trading_post_state().filter_usable, "Useable Only checkbox"): return false
	await motion(control("PerksProgramFilterSort").get_global_rect().get_center(), false)
	if not await wait_until(func(): return control("PerksProgramSortMenu") != null, "Sort By hover submenu"): return false
	if control_enabled("PerksProgramFilterSortTime"):
		fail("Unsourced per-item remaining time became enabled"); return false
	await capture("filter-sort")
	await click("PerksProgramFilterSortPrice")
	if not await wait_until(func(): return client.trading_post_state().sort == "price" and not client.trading_post_state().ascending, "Price radio"): return false
	await click("PerksProgramFilterAscending")
	if not await wait_until(func(): return client.trading_post_state().ascending, "Ascending checkbox"): return false
	await click("PerksProgramFilterResetButton")
	if not await wait_until(func(): return not client.trading_post_state().filter_usable and client.trading_post_state().sort == "price" and client.trading_post_state().ascending, "Reset preserves independent sort state"): return false
	if control("PerksProgramFilterMenu") == null: await click("PerksProgramFilter")
	await motion(control("PerksProgramFilterSort").get_global_rect().get_center(), false)
	if not await wait_until(func(): return control("PerksProgramSortMenu") != null, "Sort By after Reset"): return false
	await click("PerksProgramFilterSortName")
	if not await wait_until(func(): return client.trading_post_state().sort == "name", "Name radio"): return false
	await click("PerksProgramFilter")
	return true

func drag_off_frozen() -> bool:
	var slot := control("PerksProgramFrozenSlot")
	var model := control("PerksProgramMainModelScene")
	if slot == null or model == null: fail("Missing frozen slot/model canvas"); return false
	var from := slot.get_global_rect().get_center()
	var to := model.get_global_rect().get_center()
	await motion(from, false)
	await button_event(from, MOUSE_BUTTON_LEFT, true)
	for step in range(1, 11):
		await motion(from.lerp(to, step / 10.0), true, (to-from)/10.0)
	await button_event(to, MOUSE_BUTTON_LEFT, false)
	return true

func browse() -> void:
	var state: Dictionary = client.trading_post_state()
	if int(state.items) == 0: fail("Vendor has no sourced items"); return
	if not await wait_until(func(): return client.trading_post_state().preview_ready, "default selection preview", 120000): return
	await capture("vendor-all")
	await click("PerksProgramFilter")
	if not await wait_until(func(): return control("PerksProgramFilterMenu") != null, "filter description"): return
	await capture("filter-description")
	for category in [[1, "transmog", "player"], [8, "transmog-set", "player"], [3, "pet", "creature"], [2, "mount", "creature"]]:
		if not await choose_category(category[0]): return
		var first: PackedStringArray = str(client.trading_post_state().rows[0]).split("|")
		if first[1] == "0":
			# Unreadable source rows keep their gap text and must not show a stale model.
			await click("PerksProgramProduct" + first[0])
			if not await wait_until(func(): return str(client.trading_post_state().selected) == first[0] and client.trading_post_state().preview_kind == "none", "unavailable row clears preview"): return
			await capture("vendor-" + category[1] + "-unavailable-row")
		if not await select_first_available(): return
		var kind: String = category[2]
		if not await wait_until(func(): return client.trading_post_state().preview_ready and client.trading_post_state().preview_kind == kind, category[1] + " " + kind + " preview", 120000): return
		await capture("vendor-" + category[1])
	if not await rotate_model(): return
	if not await choose_category(5): return
	if control("PerksProgramToyIcon") == null or control("PerksProgramToyDescription") == null:
		fail("ToyOverlay icon/details missing"); return
	await capture("vendor-toy")
	if not await choose_category(2): return
	var frozen := int(client.trading_post_state().selected)
	if not await drag_to_frozen("PerksProgramProduct%d" % frozen): return
	if not await wait_until(func(): return int(client.trading_post_state().frozen) == frozen and not client.trading_post_state().pending, "authoritative freeze"): return
	await capture("frozen")
	await purchase_affordable()

func purchase_affordable() -> void:
	var tender := int(client.trading_post_state().tender)
	print("TP_TENDER ", tender, " selected=", client.trading_post_state())
	if tender < int(client.trading_post_state().price):
		await click("PerksProgramPurchaseButton")
		if control("PerksProgramConfirmDialog") != null: fail("Unaffordable purchase opened confirmation"); return
		await capture("purchase-unaffordable")
		finish("browse-unaffordable")
		return
	await click("PerksProgramPurchaseButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "purchase confirmation"): return
	if int(client.trading_post_state().tender) != tender: fail("Confirmation debited locally"); return
	await capture("purchase-confirm")
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return client.trading_post_state().purchased and int(client.trading_post_state().tender) == tender - int(client.trading_post_state().price), "authoritative purchase and debit"): return
	await capture("purchased")
	finish("browse-purchased")

func relog_checks() -> void:
	var state: Dictionary = client.trading_post_state()
	if int(state.frozen) < 0: fail("Frozen slot did not survive relog"); return
	await capture("relog-vendor")
	finish("relog")

# Vendor category, label and the client collection receipt each purchase must change.
const PURCHASES := [[1, "transmog", "appearance"], [2, "mount", "mounts"], [3, "pet", "pets"], [5, "toy", "toys"]]

func cart_ensemble_preview() -> bool:
	if not await choose_category(8): return false
	if not await select_first_available(): return false
	var state: Dictionary = client.trading_post_state()
	var vendor := int(state.selected)
	var members: PackedInt64Array = state.preview_item_ids
	if members.size() < 2: fail("Authored ensemble needs multiple member rows"); return false
	await click("PerksProgramAddToCartButton")
	if not await wait_until(func(): return client.trading_post_state().cart.size() == 1, "ensemble add"): return false
	await click("PerksProgramViewCartButton")
	if not await wait_until(func(): return control("PerksProgramCartSet%d" % vendor) != null and client.trading_post_state().preview_ready, "source ensemble header and composite preview"): return false
	await capture("cart-ensemble-preview")
	var item := int(members[0])
	await click("PerksProgramCartSubItem%d_%d" % [vendor,item])
	if not await wait_until(func(): return not item in client.trading_post_state().cart_preview_items and client.trading_post_state().preview_ready, "ensemble member deselection"): return false
	await capture("cart-ensemble-member-deselected")
	await click("PerksProgramCartSubItem%d_%d" % [vendor,item])
	if not await wait_until(func(): return item in client.trading_post_state().cart_preview_items, "ensemble member restored"): return false
	await click("PerksProgramCartClearButton")
	if not await wait_until(func(): return control("PerksProgramConfirmButton") != null, "ensemble clear confirmation"): return false
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return client.trading_post_state().cart.size() == 0 and not client.trading_post_state().cart_visible, "confirmed clear"): return false
	return true

func checkout_cart() -> void:
	var tender := int(client.trading_post_state().tender)
	if tender < 2000: fail("Admin cart Tender setup missing"); return
	if not await cart_ensemble_preview(): return
	var bought := []
	var total := 0
	for entry in PURCHASES:
		if not await choose_category(entry[0]): return
		if not await select_first_available(tender - total): return
		var state: Dictionary = client.trading_post_state()
		if state.purchased or state.collected: fail("Cart fixture needs unowned authored rows: " + str(state)); return
		if entry[0] == 1:
			if not await wait_until(func(): return client.trading_post_state().preview_ready, "dressed preview"): return
			await capture("cart-outfit-default")
			var default_hide: bool = state.hide_armor
			await click("PerksProgramToggleHideArmor")
			if not await wait_until(func(): return client.trading_post_state().hide_armor != default_hide and client.trading_post_state().preview_ready, "Hide Armor override"): return
			await capture("cart-outfit-hidden")
			await click("PerksProgramToggleHideArmor")
			if not await wait_until(func(): return client.trading_post_state().hide_armor == default_hide, "restore authored outfit"): return
		var before := await collection_count(entry[2], int(state.item_id))
		await click("PerksProgramAddToCartButton")
		total += int(state.price)
		bought.append({"category":entry[0],"kind":entry[2],"vendor":int(state.selected),"item_id":int(state.item_id),"price":int(state.price),"count":before + 1})
		if not await wait_until(func(): return client.trading_post_state().cart.size() == bought.size(), "cart add"): return
	await click("PerksProgramViewCartButton")
	if not await wait_until(func(): return client.trading_post_state().cart_visible and control("PerksProgramCartPurchaseButton") != null and client.trading_post_state().preview_ready and client.trading_post_state().preview_kind == "player", "cart composite preview"): return
	if int(client.trading_post_state().cart_price) != total: fail("Cart total differs from authored prices"); return
	await capture("cart-four-products")
	var wearable = bought[0]
	var mount = bought[1]
	await click("PerksProgramCartItem%d" % int(mount.vendor))
	if not await wait_until(func(): return client.trading_post_state().preview_ready and client.trading_post_state().preview_kind == "creature", "OTHER cart preview"): return
	var dimmed := control("PerksProgramCartItem%d" % int(wearable.vendor))
	if absf(dimmed.modulate.a - 0.7) > 0.01: fail("OTHER preview did not dim wearable to source alpha0.7"); return
	await capture("cart-other-preview")
	await click("PerksProgramCartItem%d" % int(wearable.vendor))
	if not await wait_until(func(): return client.trading_post_state().preview_ready and client.trading_post_state().preview_kind == "player" and int(wearable.item_id) in client.trading_post_state().cart_preview_items, "disabled wearable clears OTHER without deselection"): return
	await capture("cart-other-cleared")
	await click("PerksProgramCartClearButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "clear confirmation"): return
	await capture("cart-clear-confirm")
	await click("PerksProgramConfirmCancelButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") == null, "cancel clear"): return
	if client.trading_post_state().cart.size() != 4: fail("Cancel clear removed products"); return
	await click("PerksProgramCartPurchaseButton")
	if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "cart checkout confirmation"): return
	if control("PerksProgramConfirmItemFrame") != null: fail("Cart incorrectly has single-item frame"); return
	if int(client.trading_post_state().tender) != tender: fail("Cart confirmation debited locally"); return
	await capture("cart-checkout-confirm")
	await click("PerksProgramConfirmButton")
	if not await wait_until(func(): return not client.trading_post_state().pending and int(client.trading_post_state().tender) == tender-total, "atomic checkout reply"): return
	if client.trading_post_state().cart.size() != 0: fail("Accepted cart not cleared"); return
	for row in bought:
		if not await choose_category(int(row.category)): return
		await click("PerksProgramProduct%d" % int(row.vendor))
		if not await wait_until(func(): return int(client.trading_post_state().selected) == int(row.vendor) and client.trading_post_state().purchased and client.trading_post_state().collected, "checkout affected row"): return
		if not await wait_until_count(row.kind,int(row.item_id),int(row.count),"checkout collection"): return
		if control("PerksProgramHistoryRefundTime") == null: fail("Selected purchased item missing source refund history"); return
		await capture("cart-owned-" + str(row.kind))
	var file := FileAccess.open(receipts.path_join(skin + "-purchases.json"),FileAccess.WRITE)
	file.store_string(JSON.stringify({"tender":tender-total,"bought":bought},"\t"))
	file.close()
	finish("cart-checkout")

func purchase_each_collection() -> void:
	var tender := int(client.trading_post_state().tender)
	if tender < 2000: fail("Admin Tender setup missing: %d" % tender); return
	var bought := []
	for entry in PURCHASES:
		if not await choose_category(entry[0]): return
		if not await select_first_available(tender): return
		var state: Dictionary = client.trading_post_state()
		if state.purchased or state.collected: fail("Fresh account already owns " + str(state)); return
		var before := await collection_count(entry[2], int(state.item_id))
		await click("PerksProgramPurchaseButton")
		if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, entry[1] + " purchase confirmation"): return
		if int(client.trading_post_state().tender) != tender: fail("Confirmation debited locally"); return
		await capture("confirm-" + entry[1])
		await click("PerksProgramConfirmButton")
		var price := int(state.price)
		if not await wait_until(func(): return client.trading_post_state().purchased and client.trading_post_state().collected and int(client.trading_post_state().tender) == tender - price, entry[1] + " authoritative purchase"): return
		if not await wait_until_count(entry[2], int(state.item_id), before + 1, entry[1] + " client collection receipt"): return
		if int(client.trading_post_state().refund_seconds) <= 0 or not control_enabled("PerksProgramRefundButton"): fail("Admin refund window not offered"); return
		tender -= price
		await capture("purchased-" + entry[1])
		bought.append({"category": entry[0], "kind": entry[2], "vendor": int(state.selected), "item_id": int(state.item_id), "price": price, "count": before + 1})
	var file := FileAccess.open(receipts.path_join(skin + "-purchases.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify({"tender": tender, "bought": bought}, "\t"))
	file.close()
	finish("purchased")

func refund_after_relog() -> void:
	var saved: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(receipts.path_join(skin + "-purchases.json")))
	var tender := int(client.trading_post_state().tender)
	if tender != int(saved.tender): fail("Tender changed across relog: %d != %d" % [tender, int(saved.tender)]); return
	for row in saved.bought:
		if not await choose_category(int(row.category)): return
		await click("PerksProgramProduct%d" % int(row.vendor))
		if not await wait_until(func(): return int(client.trading_post_state().selected) == int(row.vendor), "select purchased row"): return
		var state: Dictionary = client.trading_post_state()
		if not state.purchased or not state.collected: fail("Purchase/collection lost across relog: " + str(state)); return
		if not await wait_until_count(row.kind, int(row.item_id), int(row.count), str(row.kind) + " collection after relog"): return
		await capture("relog-owned-" + str(row.kind))
		await click("PerksProgramRefundButton")
		if not await wait_until(func(): return control("PerksProgramConfirmDialog") != null, "refund confirmation"): return
		await capture("refund-confirm-" + str(row.kind))
		await click("PerksProgramConfirmButton")
		tender += int(row.price)
		if not await wait_until(func(): return not client.trading_post_state().purchased and not client.trading_post_state().collected and int(client.trading_post_state().tender) == tender, str(row.kind) + " authoritative refund"): return
		if not await wait_until_count(row.kind, int(row.item_id), int(row.count) - 1, str(row.kind) + " collection revoked"): return
		await capture("refunded-" + str(row.kind))
	finish("refunded")

func collection_count(kind: String, item_id: int) -> int:
	match kind:
		"mounts": return client.mount_journal_state().collected.size()
		"toys":
			for toy in client.toybox_state().learned:
				if int(toy.item_id) == item_id: return 1
			return 0
		"pets":
			var text := await ipc(["collection", "pets"])
			return 0 if text.strip_edges().is_empty() else text.strip_edges().split("\n").size()
	# Appearances: the server computes `collected` from the account appearance store.
	return 1 if client.trading_post_state().collected else 0

func wait_until_count(kind: String, item_id: int, expected: int, label: String) -> bool:
	var deadline := Time.get_ticks_msec() + 30000
	while Time.get_ticks_msec() < deadline:
		if await collection_count(kind, item_id) == expected: return true
		await wait_frames(30)
	fail("timeout: %s (expected %d, have %d)" % [label, expected, await collection_count(kind, item_id)])
	return false

func control_enabled(name: String) -> bool:
	var node := control(name)
	return node != null and not (node is BaseButton and node.disabled)

func travelers_log() -> bool:
	await click("MinimapLauncherButton")
	if not await wait_until(func(): return control("LauncherEntryEJMicroButton") != null, "Adventure Guide launcher entry"): return false
	await click("LauncherEntryEJMicroButton")
	if not await wait_until(func(): return control("EncounterJournalMonthlyActivitiesTab") != null, "Traveler's Log tab"): return false
	await click("EncounterJournalMonthlyActivitiesTab")
	if not await wait_until(func(): return control("JournalMonthlyPoints") != null and "Tender earned" in control("JournalMonthlyPoints").text, "authoritative Log Tender line", 60000): return false
	await capture("travelers-log")
	await tap(KEY_ESCAPE)
	return true

func open_vendor() -> bool:
	# Setup sends the existing NPC interaction request; vendor actions remain native input.
	var reply := await ipc(["quest", "interact", "--npc", "Tawny Seabraid"])
	if reply.is_empty(): return false
	if not await wait_until(func(): return client.trading_post_state().open or control("GossipOption0Text") != null, "Tawny greeting or vendor"): return false
	if not client.trading_post_state().open:
		await click("GossipOption0Text")
	return await wait_until(func(): return client.trading_post_state().open and int(client.trading_post_state().get("items", 0)) > 0, "authoritative Trading Post snapshot", 180000)

func choose_category(id: int) -> bool:
	if control("PerksProgramFilterMenu") == null:
		await click("PerksProgramFilter")
	await motion(control("PerksProgramFilterTypes").get_global_rect().get_center(), false)
	if not await wait_until(func(): return control("PerksProgramTypeMenu") != null, "source Type submenu on hover"): return false
	for row in client.trading_post_state().types:
		var parts: PackedStringArray = str(row).split("|")
		var checked := parts[1] == "1"
		if checked != (int(parts[0]) == id):
			await click("PerksProgramFilterType" + parts[0])
	await capture("filter-type-%d" % id)
	await click("PerksProgramFilter")
	return await wait_until(func(): return int(client.trading_post_state().category) == id and control("PerksProgramFilterMenu") == null, "only source type %d checked" % id)

# First listed row with a bound grant (and, given `budget`, a price within it).
func select_first_available(budget: int = -1) -> bool:
	for row in client.trading_post_state().rows:
		var parts: PackedStringArray = row.split("|")
		if parts[1] != "1": continue
		await click("PerksProgramProduct" + parts[0])
		if not await wait_until(func(): return str(client.trading_post_state().selected) == parts[0], "select available row " + parts[0]): return false
		var state: Dictionary = client.trading_post_state()
		if budget < 0: return true
		if int(state.price) <= budget and not state.purchased and not state.collected: return true
	fail("No available row within %d Tender in category %d" % [budget, int(client.trading_post_state().category)])
	return false

func rotate_model() -> bool:
	var start := control("PerksProgramMainModelScene").get_global_rect().get_center()
	var before := await pixels("PerksProgramMainModelScene")
	await motion(start, false)
	await button_event(start, MOUSE_BUTTON_LEFT, true)
	for step in range(6):
		await motion(start + Vector2(20 * (step + 1), 0), true, Vector2(20, 0))
	await button_event(start + Vector2(120, 0), MOUSE_BUTTON_LEFT, false)
	await wait_frames(20)
	var after := await pixels("PerksProgramMainModelScene")
	if before == after: fail("Model drag did not change the rendered preview"); return false
	await capture("rotated")
	return true

func drag_to_frozen(name: String) -> bool:
	var source := control(name)
	var target := control("PerksProgramFrozenSlot")
	if source == null or target == null: fail("Missing drag source or frozen slot"); return false
	var from := source.get_global_rect().get_center()
	var to := target.get_global_rect().get_center()
	await motion(from, false)
	await button_event(from, MOUSE_BUTTON_LEFT, true)
	for step in range(1, 11):
		await motion(from.lerp(to, step / 10.0), true, (to - from) / 10.0)
	await button_event(to, MOUSE_BUTTON_LEFT, false)
	return true

func assert_native_rect(name: String, expected: Rect2) -> bool:
	var node := control(name)
	if node == null: fail("Missing projected control " + name); return false
	var actual := node.get_global_rect()
	for delta in [actual.position.x-expected.position.x, actual.position.y-expected.position.y, actual.size.x-expected.size.x, actual.size.y-expected.size.y]:
		if absf(delta) > 0.51:
			fail("Source projected rect %s: %s != %s" % [name,actual,expected]); return false
	return true

func assert_vendor_rects() -> bool:
	for row in [
		["PerksProgramProductsScrollBoxContainer", Rect2(46,142.5,527,751)],
		["PerksProgramScrollBox", Rect2(46,200.5,503,573)],
		["PerksProgramHoldFrame", Rect2(46,773.5,527,120)],
		["PerksProgramFrozenSlot", Rect2(46,791.5,527,84)],
		["PerksProgramFilter", Rect2(55,95.5,145,25)],
		["PerksProgramFooterFrame", Rect2(0,980,1920,100)],
		["PerksProgramLeaveButton", Rect2(46,1014,180,46)],
		["PerksProgramPurchaseButton", Rect2(1694,1014,180,46)],
		["PerksProgramLeftGradient", Rect2(0,0,950,1080)],
		["PerksProgramRightGradient", Rect2(1320,0,600,1080)],
	]:
		if not assert_native_rect(row[0],row[1]): return false
	var price := control("PerksProgramPriceSortButton").get_global_rect()
	var time := control("PerksProgramTimeSortButton").get_global_rect()
	if absf(price.get_center().y-time.get_center().y) > 0.51 or absf(price.end.x+48-time.position.x) > 0.51:
		fail("Price RIGHT-to-Time.LEFT centering diverged"); return false
	return true

func assert_header_hover() -> bool:
	await motion(control("PerksProgramMainModelScene").get_global_rect().get_center(),false)
	await wait_frames(5)
	if not assert_header_color(Color(1,0.82,0,1), "normal"): return false
	var before := await pixels("PerksProgramNameSortLabel")
	await motion(control("PerksProgramNameSortButton").get_global_rect().get_center(),false)
	await wait_frames(5)
	if not assert_header_color(Color.WHITE, "hover"): return false
	if before == await pixels("PerksProgramNameSortLabel"):
		fail("Header hover did not change rendered pixels"); return false
	await capture("headers-hover")
	await motion(control("PerksProgramMainModelScene").get_global_rect().get_center(),false)
	await wait_frames(5)
	if not assert_header_color(Color(1,0.82,0,1), "leave"): return false
	return true

func assert_header_color(expected: Color, boundary: String) -> bool:
	# Async snapshots can rebuild the projection while the fixture yields.
	var label := control("PerksProgramNameSortLabel")
	if label == null:
		fail("Missing projected header after " + boundary); return false
	if not label.get_theme_color("font_color").is_equal_approx(expected):
		fail("Header color diverged at " + boundary); return false
	return true

func texture_ready(name: String) -> bool:
	var node := control(name)
	if node == null: return false
	var part := node.get_node_or_null("Parts/Part0") as TextureRect
	return part != null and part.texture != null

func assert_texture_pixels(name: String) -> bool:
	var node := control(name)
	var part := node.get_node_or_null("Parts/Part0") as TextureRect if node != null else null
	if part == null or part.texture == null:
		fail("Source art is absent on " + name); return false
	var image := part.texture.get_image()
	if image == null or image.is_empty(): fail("Source image is empty on " + name); return false
	var covered := 0
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			if image.get_pixel(x,y).a > 0.1: covered += 1
	if covered == 0: fail("Source image has no visible pixels on " + name); return false
	return true

func assert_masked_icon(name: String) -> bool:
	if not assert_texture_pixels(name): return false
	var image: Image = control(name).get_node("Parts/Part0").texture.get_image()
	if image.get_pixel(0,0).a > 0.01 or image.get_pixel(floori(image.get_width()*0.5),floori(image.get_height()*0.5)).a < 0.75:
		fail("Authored mask did not cut corners/preserve icon on " + name); return false
	# Exact48:32 sampling is gated by native RGBA comparisons against each
	# decoded source icon; mount icons can have authored transparent borders.
	return true

func assert_dialog_rects_and_art() -> bool:
	var item := control("PerksProgramConfirmItem").get_global_rect()
	var text := control("PerksProgramConfirmItemText").get_global_rect()
	var button := control("PerksProgramConfirmButton").get_global_rect()
	if item.size != Vector2(37,37) or text.size != Vector2(103,38) or button.size != Vector2(128,21):
		fail("StaticPopup ItemFrame/button dimensions differ from source"); return false
	if absf(item.end.x+10-text.position.x)>0.51:
		fail("StaticPopup item is not RIGHT-to-text.LEFT-10"); return false
	if not assert_native_rect("PerksProgramConfirmCover", Rect2(0,0,1920,1080)): return false
	for name in ["PerksProgramConfirmItem", "PerksProgramConfirmItemNameFrame", "PerksProgramConfirmButtonNormalArt", "PerksProgramConfirmBorder"]:
		if not assert_texture_pixels(name): return false
	return true

func pixels(name: String) -> PackedByteArray:
	await RenderingServer.frame_post_draw
	var rect := Rect2i(control(name).get_global_rect())
	return root.get_texture().get_image().get_region(rect).get_data()

func ipc(arguments: Array) -> String:
	var worker := Thread.new()
	var argv := ["--signal=KILL", "15", OS.get_environment("GAME_ENGINE_CLI"), "--socket", "/tmp/game-engine-%d.sock" % OS.get_process_id()] + arguments
	var error := worker.start(func():
		var output := []
		var code := OS.execute("/usr/bin/timeout", argv, output, true)
		return {"code": code, "text": "\n".join(output)})
	if error != OK: fail("Cannot start IPC setup worker"); return ""
	while worker.is_alive(): await process_frame
	var result: Dictionary = worker.wait_to_finish()
	print("TP_SETUP_IPC ", arguments, " code=", result.code, " text=", result.text)
	if result.code != 0: fail("Bounded NPC setup failed"); return ""
	return result.text

func finish(label: String) -> void:
	if int(client.trading_post_state().hidden_layers) == 0: fail("HUD stayed visible under the full-screen vendor"); return
	var chat: CanvasLayer = client.get_node_or_null("ChatFrameUI")
	if chat == null or chat.visible: fail("Chat stayed visible under the full-screen vendor"); return
	await click("PerksProgramLeaveButton")
	if not await wait_until(func(): return not client.trading_post_state().open and control("MinimapLauncherButton") != null and chat.visible, "Leave restores the HUD and chat"): return
	print("TRADING_POST_LIVE PASS skin=", skin, " phase=", label, " state=", client.trading_post_state())
	client.free()
	quit(0)

func control(name: String) -> Control:
	for node in client.find_children(name, "Control", true, false):
		if node.is_visible_in_tree(): return node
	return null

func wait_until(predicate: Callable, label: String, timeout: int = 30000) -> bool:
	var deadline := Time.get_ticks_msec() + timeout
	while Time.get_ticks_msec() < deadline:
		if predicate.call(): return true
		await process_frame
	fail("timeout: " + label)
	return false

func click(name: String, button: int = MOUSE_BUTTON_LEFT) -> void:
	var node := control(name)
	if node == null: fail("Missing visible control " + name); return
	var point := node.get_global_rect().get_center()
	await motion(point, false)
	await button_event(point, button, true)
	await button_event(point, button, false)
	await wait_frames(5)

func motion(point: Vector2, down: bool, relative: Vector2 = Vector2.ZERO) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	event.relative = relative
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
	root.push_input(event, true)
	await process_frame

func button_event(point: Vector2, button: int, down: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = button
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if down and button == MOUSE_BUTTON_LEFT else 0
	event.pressed = down
	root.push_input(event, true)
	await process_frame

func tap(code: Key) -> void:
	for down in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = down
		root.push_input(event, true)
		await process_frame
	await wait_frames(5)

func capture(label: String) -> void:
	await wait_frames(30)
	await RenderingServer.frame_post_draw
	var stem := skin + "-" + phase + "-" + label
	var image_path := shots.path_join(stem + ".png")
	if root.get_texture().get_image().save_png(image_path) != OK:
		fail("Cannot save Trading Post evidence PNG: " + image_path)
		return
	var file := FileAccess.open(receipts.path_join(stem + ".json"), FileAccess.WRITE)
	if file == null:
		fail("Cannot save Trading Post evidence receipt: " + stem)
		return
	file.store_string(JSON.stringify({"trading_post": client.trading_post_state(), "journal": client.adventure_guide_state(), "account": client.account_state()}, "\t"))
	file.close()
	print("TP_CAPTURE ", stem)

func wait_frames(count: int) -> void:
	for frame in range(count): await process_frame

func fail(message: String) -> void:
	if client != null and not receipts.is_empty():
		var file := FileAccess.open(receipts.path_join(skin + "-" + phase + "-failure.json"), FileAccess.WRITE)
		file.store_string(JSON.stringify({"failure": message, "trading_post": client.trading_post_state(), "journal": client.adventure_guide_state(), "account": client.account_state()}, "\t"))
		file.close()
	push_error("Trading Post live: " + message)
	quit(1)
