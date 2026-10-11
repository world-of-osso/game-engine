extends "res://tests/bank_live.gd"
# Real private realm, authentic vendor purchase and native paperdoll input.
const ENDPOINT := "127.0.0.1:5821"
const VENDOR_NAME := "Arred"
const PURCHASE := "Mood Ring"
const PURCHASE_ID := 7338
var character := ""
var receipt: Dictionary = {}

func run_test() -> void:
	if OS.get_environment("GODOT_TEST_SERVER") != ENDPOINT:
		fail("equipment proof requires private UDP 5821")
		return
	root.size = Vector2i(1280, 720)
	character = OS.get_environment("BANK_CHARACTER")
	role = OS.get_environment("EQUIP_SKIN")
	shots = OS.get_environment("BANK_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	while client.account_state().assets_starting:
		await process_frame
	var error: String = client.connect_account(ENDPOINT, OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(character): return
	if client.account_state().ui_skin != role:
		fail("wrong skin: " + str(client.account_state().ui_skin))
		return
	if OS.get_environment("EQUIP_RELOG") == "1":
		if not await wait_until(func(): return client.merchant_state().equipment.size() >= 5, "persisted equipment"): return
		await tap(KEY_C)
		await capture("relog-paperdoll")
		receipt = {"equipment": client.merchant_state().equipment, "stats": sheet_stats(), "relog": true}
		if not await prove_paired_slot_input(): return
	else:
		await tap(KEY_C)
		if not await wait_until(func(): return shown("CharacterFrame") and not sheet_stats().is_empty(), "baseline stats"): return
		var baseline := sheet_stats()
		await capture("baseline-paperdoll")
		await tap(KEY_ESCAPE)
		if not await click_pickable(func(node): return str(node.name) == VENDOR_NAME, "unit_server_id"): return
		if not await wait_until(func(): return client.merchant_state().open or shown("GossipOption0Text"), "vendor greeting"): return
		if not client.merchant_state().open:
			await press("GossipOption0Text")
		if not await wait_until(func(): return client.merchant_state().open, "authentic vendor"): return
		var index: int = Array(client.merchant_state().items).find(PURCHASE)
		if index < 0:
			fail("authentic trinket missing from vendor")
			return
		for page in range(index / 10): await press("MerchantNextPageButton")
		await click_at(control("MerchantItem%d" % (index % 10 + 1)).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
		if not await wait_until(func(): return bag_count(PURCHASE_ID) == 1, "paid vendor purchase"): return
		await capture("authentic-vendor-purchase")
		await tap(KEY_ESCAPE)
		await press("MainMenuBarBackpackButton")
		for item_id in [4112, PURCHASE_ID, 1319, 1490, 2820]:
			var bag_button := ""
			for item in client.merchant_state().bags:
				if int(item.item_id) == item_id:
					bag_button = "ContainerFrame%dSlot%d" % [item.bag, item.slot]
					break
			if bag_button.is_empty():
				fail("authentic item missing " + str(item_id))
				return
			await click_at(control(bag_button).get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
			if not await wait_until(func(): return bag_count(item_id) == 0, "authoritative equip " + str(item_id)): return
		await tap(KEY_ESCAPE)
		await tap(KEY_C)
		if not await wait_until(func(): return client.merchant_state().equipment.size() >= 5, "all nonvisual slots"): return
		var equipped := sheet_stats()
		if int(equipped.get("Stamina:", "0").replace(",", "")) <= int(baseline.get("Stamina:", "0").replace(",", "")):
			fail("equipped stamina did not increase: " + str([baseline, equipped]))
			return
		await capture("filled-paperdoll-stats")
		receipt = {"baseline": baseline, "equipped": equipped, "purchase_id": PURCHASE_ID}
		if not await prove_paired_slot_input(): return
	var file := FileAccess.open(OS.get_environment("EQUIP_RECEIPT"), FileAccess.WRITE)
	file.store_string(JSON.stringify(receipt, "\t"))
	file.close()
	print("PASS equipment live ", role, " ", JSON.stringify(receipt))
	client.free()
	quit(0)

func prove_paired_slot_input() -> bool:
	var source_index := -1
	for item in client.merchant_state().equipment:
		if int(item.item_id) == PURCHASE_ID:
			source_index = 0 if item.slot == "Finger1" else 1
	if source_index < 0: return fail("purchased ring absent from paperdoll")
	var target_index := 1 - source_index
	var source := control("CharacterFinger%dSlot" % source_index).get_global_rect().get_center()
	await hover_point(source)
	if not await wait_until(func(): return client.tooltip_state().visible and client.tooltip_state().title == PURCHASE, "ring tooltip"): return false
	await capture("ring-tooltip")
	receipt["before_swap"] = client.merchant_state().equipment.duplicate(true)
	await drag_slot(source, control("CharacterFinger%dSlot" % target_index).get_global_rect().get_center())
	var target_slot := "Finger1" if target_index == 0 else "Finger2"
	if not await wait_until(func():
		for item in client.merchant_state().equipment:
			if item.slot == target_slot and int(item.item_id) == PURCHASE_ID: return true
		return false, "native finger drag swap"): return false
	await capture("finger-drag-swap")
	receipt["equipment"] = client.merchant_state().equipment
	return true

func sheet_stats() -> Dictionary:
	var values := {}
	for index in range(20):
		var label := text("CharacterStatsPaneStat%dLabel" % index)
		if not label.is_empty(): values[label] = text("CharacterStatsPaneStat%dValue" % index)
	return values

func hover_point(point: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	root.push_input(event, true)
	Input.warp_mouse(point)
	await wait_frames(10)

func drag_slot(source: Vector2, destination: Vector2) -> void:
	await hover_point(source)
	var event := InputEventMouseButton.new()
	event.position = source
	event.global_position = source
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = true
	root.push_input(event, true)
	await wait_frames(10)
	await hover_point(source + Vector2(12, 0))
	await hover_point(destination)
	event = InputEventMouseButton.new()
	event.position = destination
	event.global_position = destination
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = false
	root.push_input(event, true)
	await wait_frames(10)
