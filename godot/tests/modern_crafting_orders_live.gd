extends "res://tests/modern_crafting_knowledge_live.gd"

## Private fixture guild/material grants are not acquisition or world-placement evidence.
func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if endpoint != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_"):
		fail("Orders proof requires owned accounts on UDP5951")
		return
	role = OS.get_environment("MODERN_CRAFTING_SKIN") + "-orders-" + OS.get_environment("CRAFTING_PHASE")
	shots = OS.get_environment("BANK_SHOTS")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await process_frame
	# The CASC startup join exceeded the shared 60s UI wait under host contention.
	# Give startup its own bounded budget; interactive response deadlines stay unchanged.
	var startup_deadline := Time.get_ticks_msec() + 240000
	while client.account_state().assets_starting and Time.get_ticks_msec() < startup_deadline:
		await process_frame
	if client.account_state().assets_starting:
		fail("Asset startup exceeded 240s before orders authentication")
		return
	var error = client.connect_account(endpoint, OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != OS.get_environment("MODERN_CRAFTING_SKIN"):
		fail("Observed skin differs from orders fixture")
		return
	var complete := false
	match OS.get_environment("CRAFTING_PHASE"):
		"customer": complete = await place_order_types()
		"crafter": complete = await fulfill_order_types()
		"customer-mail", "crafter-mail": complete = await collect_order_deliveries()
		_: fail("Unknown orders phase")
	if not complete:
		return
	print("FIXTURE MODERN_CRAFTING_ORDERS_DONE ", role)
	client.free()
	quit(0)

func place_order_types() -> bool:
	if not await click_pickable(func(node): return str(node.name) == "Clerk Gretal", "unit_server_id"):
		return false
	if not await wait_until(func(): return shown("AuctionGossipOption0"), "authored customer broker gossip"):
		return false
	await press("AuctionGossipOption0")
	if not await wait_until(func(): return shown("ProfessionsCustomerOrdersFrame"), "authoritative customer orders service"):
		return false
	var kinds := ["Public", "Guild", "Personal"]
	var first_kind := OS.get_environment("ORDERS_CUSTOMER_START").to_int()
	if first_kind < 0 or first_kind >= kinds.size():
		return fail("Invalid explicit customer resume index")
	if first_kind > 0:
		if not await wait_until(func(): return shown("CraftingOrderRecipe28208"), "initial owned customer catalog"):
			return false
		if not await observe_owned_order("Public"):
			return false
	for index in range(first_kind, kinds.size()):
		var kind: String = kinds[index]
		await press("CraftingOrdersBrowseTab")
		if not await wait_until(func(): return shown("CraftingOrderRecipe28208"), "source fixed-output order recipe"):
			return false
		await press("CraftingOrderRecipe28208")
		if not await wait_until(func(): return shown("CraftingOrderPlace"), "customer order form"):
			return false
		await press("CraftingOrderRecipientDropdown")
		if not await wait_until(func(): return shown("CraftingOrderMenu" + kind), "recipient type menu"):
			return false
		await press("CraftingOrderMenu" + kind)
		if kind == "Personal":
			await replace_field("CraftingOrderRecipient", "Craftworker")
		await replace_field("CraftingOrderGold", "1")
		await replace_field("CraftingOrderNote", role + " " + kind)
		for index in range(4):
			await press("CraftingOrderSupply%d" % index)
		var before := money()
		await capture(kind + "-1-form")
		await press("CraftingOrderPlace")
		if not await wait_until(func(): return money() == before - 10000 and text("CraftingOrdersStatus") == "Order placed", kind + " authoritative placement and commission escrow"):
			return false
		if not await observe_owned_order(kind):
			return false
		print("ORDERS CUSTOMER kind=", kind, " before=", before, " after=", money(), " recipient=", text("CraftingOrderRecipientDisplay"))
	return true

func observe_owned_order(kind: String) -> bool:
	await press("CraftingOrdersMyOrdersTab")
	if not await wait_until(func(): return not client.find_children("CraftingOrderRow*", "Control", true, false).is_empty(), "owned order snapshot"):
		return false
	await click(client.find_children("CraftingOrderRow*", "Control", true, false)[0])
	if not await wait_until(func(): return text("CraftingOrderState") == "Open", kind + " authoritative Open order"):
		return false
	await capture(kind + "-2-escrow")
	return true

func fulfill_order_types() -> bool:
	if not await open_recipe_book():
		return false
	await press("ProfessionsOrdersTab")
	var output_before := bag_count(22658)
	for kind in ["public", "guild", "personal"]:
		await press("CraftingOrderType" + kind)
		if not await wait_until(func(): return not client.find_children("CraftingOrderRow*", "Control", true, false).is_empty(), kind + " eligible order browse"):
			return false
		await click(client.find_children("CraftingOrderRow*", "Control", true, false)[0])
		if not await wait_until(func(): return shown("CraftingOrderClaim"), "eligible order detail"):
			return false
		await press("CraftingOrderClaim")
		if not await wait_until(func(): return text("CraftingOrderState") == "Claimed", kind + " authoritative claim"):
			return false
		await capture(kind + "-1-claimed")
		await press("CraftingOrderFulfill")
		if not await wait_until(func(): return text("CraftingOrdersStatus") == "Order completed" and not shown("CraftingOrderState"), kind + " cast settlement and open-row removal"):
			return false
		if bag_count(22658) != output_before:
			return fail("Crafter received a synthetic bag output instead of customer mail")
		print("ORDERS FULFILLED kind=", kind, " status=", text("CraftingOrdersStatus"))
		await capture(kind + "-2-completed")
	return true

func collect_order_deliveries() -> bool:
	if not await click_pickable(func(node): return node.has_meta("game_object_entry") and int(node.get_meta("game_object_entry")) == 202578, "game_object_server_id"):
		return false
	if not await wait_until(func(): return shown("MailFrame") and client.mail_state().has("mails") and not client.mail_state().mails.is_empty(), "order delivery inbox"):
		return false
	var before_items := bag_count(22658)
	var before_money := money()
	await capture("1-delivery-inbox")
	await press("OpenAllMail")
	var customer := OS.get_environment("CRAFTING_PHASE") == "customer-mail"
	if not await wait_until(func(): return bag_count(22658) == before_items + 3 if customer else money() == before_money + 30000, "three committed order deliveries"):
		return false
	print("ORDERS MAIL customer=", customer, " item_before=", before_items, " item_after=", bag_count(22658), " money_before=", before_money, " money_after=", money())
	await capture("2-deliveries-received")
	return true
