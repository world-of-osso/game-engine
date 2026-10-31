extends SceneTree
# Native input and read-only observations against owned private UDP5901 only.
const MOUNT_ID := 18
const ITEM_ID := 5655
const SPELL_ID := 6648
var client: Node
var shots := OS.get_environment("MOUNT_SHOTS")
var skin := OS.get_environment("MOUNT_SKIN")
var relog := OS.get_environment("MOUNT_RELOG") == "1"
var receipts := OS.get_environment("MOUNT_RECEIPTS")

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_fixture")

func run_fixture() -> void:
	root.size = Vector2i(1280, 720)
	var account := OS.get_environment("MOUNT_ACCOUNT")
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5901" or not account.begins_with("fb_mount_") or shots.is_empty() or receipts.is_empty():
		fail("Explicit private5901, fb_mount_* and evidence paths required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, "asset startup", 180000): return
	var error: String = client.connect_account("127.0.0.1:5901", account, "fbtest", false)
	if error != "": fail(error); return
	if not await wait_until(func(): return client.get_node_or_null("CharacterSelectUI") != null, "character select", 120000): return
	await click("CharCard_" + OS.get_environment("MOUNT_CHARACTER_INDEX"))
	await click("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and int(client.mount_journal_state().catalog_count) > 0 and control("MinimapLauncherButton") != null, "world/catalog", 180000): return
	var ready := FileAccess.open(OS.get_environment("MOUNT_READY_FILE"), FileAccess.WRITE)
	ready.store_string(str(client.account_state()))
	ready.close()
	if not await wait_until(func(): return FileAccess.file_exists(OS.get_environment("MOUNT_GO_FILE")), "online riding setup", 120000): return
	if client.account_state().ui_skin != "Forever": fail("Wrong active skin for proof"); return
	client.set_world_minutes(720.0)
	client.set_camera_orbit(PI, -0.3, 12.0)
	await wait_frames(90)
	if OS.get_environment("MOUNT_VENDOR") == "1" and not relog:
		if not await buy_mare(): return
	await open_mounts()
	if not await wait_until(func(): return control("MountJournalSearchBox") != null, "Mounts tab"): return
	await click("MountJournalSearchBox")
	await type_text("Chestnut Mare")
	if not await wait_until(func(): return int(client.mount_journal_state().selected) == MOUNT_ID and client.mount_journal_state().preview_ready, "authentic mount preview", 120000): return
	await capture("relog-collected" if relog else "uncollected")
	if relog:
		if learned().is_empty() or not learned().favourite: fail("Account mount/favourite did not survive character change: " + str(client.mount_journal_state())); return
	else:
		if not learned().is_empty(): fail("Fresh account already owns mare"); return
		await tap(KEY_ESCAPE)
		await click("MainMenuBarBackpackButton")
		if not await wait_until(func(): return control("ContainerFrame0") != null, "backpack"): return
		var bag_name := ""
		for entry in client.merchant_state().bags:
			if int(entry.item_id) == ITEM_ID: bag_name = "ContainerFrame%dSlot%d" % [entry.bag, entry.slot]
		if bag_name.is_empty(): fail("Authentic mount item missing from owned backpack"); return
		await capture("bag-before-learn")
		print("MOUNT_ITEM_INPUT ", bag_name, " rect=", control(bag_name).get_global_rect(), " bags=", client.merchant_state().bags)
		await click(bag_name, MOUSE_BUTTON_RIGHT)
		if not await wait_until(func(): return not learned().is_empty(), "UseItem learns Mount18"): return
		for entry in client.merchant_state().bags:
			if int(entry.item_id) == ITEM_ID: fail("Learning did not consume item5655"); return
		await tap(KEY_ESCAPE)
		await open_mounts()
		if not await wait_until(func(): return control("MountJournalRow18") != null, "learned mount row"): return
		await capture("learned")
		if OS.get_environment("MOUNT_VENDOR") == "1":
			await tap(KEY_ESCAPE)
			if not await open_vendor(): return
			await motion(control("MerchantItem2").get_global_rect().get_center(), false)
			if not await wait_until(func(): return "Already known|" in client.tooltip_state().lines, "vendor known tooltip"): return
			if "Already known" not in control("MerchantItem2Name").text: fail("Vendor name lacks known feedback"); return
			await capture("vendor-already-known")
			await tap(KEY_ESCAPE)
			await open_mounts()
		await click("MountJournalRow18", MOUSE_BUTTON_RIGHT)
		if not await wait_until(func(): return control("MountJournalFavoriteToggle") != null, "Retail right-click favourite menu"): return
		if not await menu_text_visible(): fail("Favourite popup labels are covered by their background"); return
		await capture("favourite-menu")
		await click("MountJournalFavoriteToggle")
		if not await wait_until(func(): return learned().favourite, "authoritative account favourite"): return
		await capture("favourite")
	if relog:
		await click("MountJournalRow18", MOUSE_BUTTON_RIGHT)
		if not await wait_until(func(): return control("MountJournalFavoriteToggle") != null, "relog favourite menu"): return
		if not await menu_text_visible(): fail("Favourite popup labels are covered by their background"); return
		await capture("favourite-menu")
		await click("MountJournalContextCancel")
	var model := control("MountJournalModelScene")
	var start := model.get_global_rect().get_center()
	await motion(start, false)
	await button_event(start, MOUSE_BUTTON_LEFT, true)
	await motion(start + Vector2(90, 0), true, Vector2(90, 0))
	await button_event(start + Vector2(90, 0), MOUSE_BUTTON_LEFT, false)
	if abs(float(client.mount_journal_state().yaw)) < 0.5: fail("Native model rotation did not turn preview"); return
	await capture("rotated")
	await click("MountJournalMountButton")
	if not await wait_until(func(): return mounted(), "normal spell-mounted rider", 60000): return
	await capture("summoned")
	await click("MountJournalMountButton")
	if not await wait_until(func(): return dismounted(), "real aura dismount"): return
	await capture("dismounted")
	await click("MountJournalSummonRandomFavorite")
	if not await wait_until(func(): return mounted(), "random favourite normal mount spell", 60000): return
	await capture("random-favourite")
	await click("MountJournalMountButton")
	if not await wait_until(func(): return dismounted(), "final dismount"): return
	print("MOUNT_JOURNAL_LIVE PASS skin=", skin, " relog=", relog, " state=", client.mount_journal_state())
	client.free()
	quit(0)

func buy_mare() -> bool:
	if not await open_vendor(): return false
	var before := int(client.merchant_state().money)
	await motion(control("MerchantItem2").get_global_rect().get_center(), false)
	if not await wait_until(func(): return client.tooltip_state().title == "Chestnut Mare Bridle", "unknown vendor mount tooltip"): return false
	if "Already known|" in client.tooltip_state().lines: fail("Unknown account showed known mount"); return false
	await capture("vendor-before-buy")
	await click("MerchantItem2", MOUSE_BUTTON_RIGHT)
	if not await wait_until(func(): return int(client.merchant_state().money) == before - 10000 and has_item(ITEM_ID), "authoritative one-gold purchase and bag delivery"): return false
	await capture("vendor-purchased")
	await tap(KEY_ESCAPE)
	return true

func has_item(item_id: int) -> bool:
	for item in client.merchant_state().bags:
		if int(item.item_id) == item_id: return true
	return false

func open_vendor() -> bool:
	# Setup sends the existing NPC interaction request; purchase/use remains native input.
	var reply := await ipc(["quest", "interact", "--npc", "Katie Hunter"])
	if reply.is_empty(): return false
	# TDB1210 Katie Hunter uses quest greeting menu4004 option0 (vendor).
	if not await wait_until(func(): return control("GossipOption0Text") != null, "Katie Hunter authored vendor greeting"): return false
	await click("GossipOption0Text")
	return await wait_until(func(): return client.merchant_state().open and control("MerchantItem2") != null, "projected Katie Hunter mount inventory")

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
	print("MOUNT_SETUP_IPC ", arguments, " code=", result.code, " text=", result.text)
	if result.code != 0: fail("Bounded NPC setup failed"); return ""
	return result.text

func open_mounts() -> void:
	await click("MinimapLauncherButton")
	if not await wait_until(func(): return control("LauncherEntryCollectionsMicroButton") != null, "Collections launcher entry"): return
	await click("LauncherEntryCollectionsMicroButton")

func menu_text_visible() -> bool:
	await wait_frames(10)
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var rect := control("MountJournalFavoriteToggle").get_global_rect().grow(-3.0)
	var foreground := 0
	for y in range(int(rect.position.y), int(rect.end.y)):
		for x in range(int(rect.position.x), int(rect.end.x)):
			var color := image.get_pixel(x, y)
			if color.r > 0.7 and color.g > 0.5: foreground += 1
	return foreground > 10

func learned() -> Dictionary:
	for mount in client.mount_journal_state().collected:
		if int(mount.mount_id) == MOUNT_ID: return mount
	return {}

func mounted() -> bool:
	if learned().is_empty() or not learned().active: return false
	for aura in client.aura_state().buffs:
		if int(aura.spell_id) == SPELL_ID: return true
	return false

func dismounted() -> bool:
	if not learned().is_empty() and learned().active: return false
	for aura in client.aura_state().buffs:
		if int(aura.spell_id) == SPELL_ID: return false
	return true

func control(name: String) -> Control:
	for node in client.find_children(name, "Control", true, false):
		if node.is_visible_in_tree(): return node
	return null

func wait_until(predicate: Callable, label: String, timeout: int = 30000) -> bool:
	var deadline := Time.get_ticks_msec() + timeout
	while Time.get_ticks_msec() < deadline:
		if predicate.call(): return true
		await process_frame
	fail("timeout: " + label + " account=" + str(client.account_state()) + " mounts=" + str(client.mount_journal_state()))
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

func type_text(value: String) -> void:
	for character in value:
		var event := InputEventKey.new()
		event.unicode = character.unicode_at(0)
		event.keycode = OS.find_keycode_from_string(character.to_upper())
		event.physical_keycode = event.keycode
		event.pressed = true
		root.push_input(event, true)
		await process_frame
		event.pressed = false
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
	var stem := skin + "-" + ("alt-" if relog else "") + label
	root.get_texture().get_image().save_png(shots.path_join(stem + ".png"))
	var file := FileAccess.open(receipts.path_join(stem + ".json"), FileAccess.WRITE)
	file.store_string(JSON.stringify({"mounts": client.mount_journal_state(), "account": client.account_state(), "auras": client.aura_state()}, "\t"))
	file.close()
	print("MOUNT_CAPTURE ", stem)

func wait_frames(count: int) -> void:
	for frame in range(count): await process_frame

func fail(message: String) -> void:
	if client != null and not receipts.is_empty():
		var file := FileAccess.open(receipts.path_join(skin + "-failure.json"), FileAccess.WRITE)
		file.store_string(JSON.stringify({"failure": message, "mounts": client.mount_journal_state(), "merchant": client.merchant_state()}, "\t"))
		file.close()
	push_error("Mount Journal live: " + message)
	quit(1)
