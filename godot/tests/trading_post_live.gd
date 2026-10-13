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
	root.size = Vector2i(1280, 720)
	var account := OS.get_environment("TP_ACCOUNT")
	if server != "127.0.0.1:5731" or not account.begins_with("fb_") or shots.is_empty() or receipts.is_empty() or not phase in ["browse", "relog"]:
		fail("Explicit private 127.0.0.1:5731, fb_* account, phase and evidence paths required")
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
	client.set_world_minutes(720.0)
	await wait_frames(120)
	if not await travelers_log(): return
	if not await open_vendor(): return
	if phase == "relog":
		await relog_checks()
	else:
		await browse()

func browse() -> void:
	var state: Dictionary = client.trading_post_state()
	if int(state.items) == 0: fail("Vendor has no sourced items"); return
	if not await wait_until(func(): return client.trading_post_state().preview_ready, "default selection preview", 120000): return
	await capture("vendor-all")
	await click("PerksProgramFilter")
	if not await wait_until(func(): return control("PerksProgramCategoryMenu") != null, "category dropdown"): return
	await capture("category-dropdown")
	for category in [[1, "transmog"], [8, "transmog-set"], [3, "pet"], [2, "mount"]]:
		if not await choose_category(category[0]): return
		if not await wait_until(func(): return client.trading_post_state().preview_ready, category[1] + " preview", 120000): return
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

func travelers_log() -> bool:
	await click("MinimapLauncherButton")
	if not await wait_until(func(): return control("LauncherEntryEJMicroButton") != null, "Adventure Guide launcher entry"): return false
	await click("LauncherEntryEJMicroButton")
	if not await wait_until(func(): return control("EncounterJournalTab4") != null, "Traveler's Log tab"): return false
	await click("EncounterJournalTab4")
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
	return await wait_until(func(): return client.trading_post_state().open and int(client.trading_post_state().get("items", 0)) > 0, "authoritative Trading Post snapshot", 60000)

func choose_category(id: int) -> bool:
	if control("PerksProgramCategoryMenu") == null:
		await click("PerksProgramFilter")
	await click("PerksProgramCategory%d" % id)
	return await wait_until(func(): return int(client.trading_post_state().category) == id, "category %d" % id)

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
	await click("PerksProgramLeaveButton")
	if not await wait_until(func(): return not client.trading_post_state().open and control("MinimapLauncherButton") != null, "Leave restores the HUD"): return
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
	root.get_texture().get_image().save_png(shots.path_join(stem + ".png"))
	var file := FileAccess.open(receipts.path_join(stem + ".json"), FileAccess.WRITE)
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
