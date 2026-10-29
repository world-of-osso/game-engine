extends SceneTree
# Real private session, native pointer/key input. Never mutates server list data.
var client: Node
var output := OS.get_environment("SELLIST_OUTPUT")
var stage := OS.get_environment("SELLIST_STAGE")
var skin := OS.get_environment("SELLIST_SKIN")
var failed := false
var pointer_notified := false

func _initialize() -> void:
	Engine.max_fps = 20
	call_deferred("run_fixture")

func fail(message: String) -> void:
	failed = true
	push_error("SELECTION_LIST_FAIL: " + message)
	quit(1)

func wait_until(predicate: Callable, description: String, seconds := 120.0) -> bool:
	var deadline := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call(): return true
	fail("Timeout: " + description)
	return false

func control(name: String) -> Control:
	return root.find_child(name, true, false) as Control

func settle(count := 6) -> void:
	for index in range(count): await process_frame

func motion(point: Vector2) -> void:
	if not pointer_notified:
		root.notify_mouse_entered()
		pointer_notified = true
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	root.push_input(event, true)
	await settle()

func pointer(name: String, button := MOUSE_BUTTON_LEFT) -> void:
	var target := control(name)
	if target == null or not target.is_visible_in_tree():
		fail("Missing visible control: " + name)
		return
	await pointer_at(target.get_global_rect().get_center(), button)

func pointer_at(point: Vector2, button: MouseButton) -> void:
	await motion(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.button_mask = (1 << (button - 1)) if pressed else 0
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await settle()

func tap(key: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = key
		event.physical_keycode = key
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await settle()

func open_hashom() -> bool:
	# IPC opens the real server-backed service; list interactions below use native input.
	# NPC-world picking is not part of the selection-row acceptance boundary.
	var binary := ProjectSettings.globalize_path("res://../target/debug/game-engine-cli")
	var agent_run := ProjectSettings.globalize_path("res://../scripts/agent/agent-run")
	var socket := "/tmp/game-engine-%d.sock" % OS.get_process_id()
	var pid := OS.create_process(agent_run, ["selection-list-ui", binary, "--socket", socket, "quest", "interact", "--npc", "Warpweaver Hashom"])
	if pid < 0: fail("Could not start owned service IPC client"); return false
	return true

func selected_label_has_ink(name: String) -> bool:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var rect := Rect2i(control(name).get_global_rect())
	var ink := 0
	for y in range(rect.position.y, rect.end.y):
		for x in range(rect.position.x, rect.end.x):
			var pixel := image.get_pixel(x, y)
			if pixel.r > 0.7 and pixel.g > 0.4 and pixel.b < 0.3: ink += 1
	if ink < 8:
		fail("Selected label artwork covers rendered text: " + name + " ink=" + str(ink))
		return false
	return true

func capture(suffix: String) -> void:
	await settle(30)
	await RenderingServer.frame_post_draw
	var path := output.path_join(skin + "-" + stage + "-" + suffix + ".png")
	var image := root.get_texture().get_image()
	if image.get_width() > 1280:
		image.resize(1280, int(image.get_height() * 1280.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
	if image.save_png(path) != OK: fail("Capture: " + path)

## A pointer press leaves no keyboard focus (Retail); the row only activates from Enter
## once it holds keyboard focus, which code gives it (no key enters a list from no focus).
func keyboard_activate_next(row: String) -> bool:
	if root.gui_get_focus_owner() != null:
		fail("Pointer selection left keyboard focus on " + str(root.gui_get_focus_owner()))
		return false
	control(row).grab_focus()
	await tap(KEY_ENTER)
	return true

func run_fixture() -> void:
	if OS.get_environment("SELLIST_ENDPOINT") != "127.0.0.1:5933" or not OS.get_environment("SELLIST_ACCOUNT").begins_with("fb_sellist_"):
		fail("Owned UDP5933 and fb_sellist_* required")
		return
	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, "asset startup", 180.0): return
	var error: String = client.connect_account("127.0.0.1:5933", OS.get_environment("SELLIST_ACCOUNT"), "fbtest", false)
	if error != "": fail(error); return
	if not await wait_until(func(): return control("EnterWorld") != null, "character select"): return
	await pointer("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld", "world", 180.0): return
	if client.account_state().ui_skin.to_lower() != skin: fail("Wrong skin: " + str(client.account_state())); return
	if not await wait_until(func(): return control("MinimapLauncherButton") != null, "inworld HUD"): return
	await pointer("MinimapLauncherButton")
	await pointer("LauncherEntryLFDMicroButton")
	if not await wait_until(func(): return control("HonorFrameSpecificBattlegroundButton120") != null and not (control("HonorFrameSpecificBattlegroundButton120") as Button).disabled, "queue list ready"): return
	await pointer("HonorFrameSpecificBattlegroundButton120")
	if not await wait_until(func(): return control("HonorFrameDescription").text.contains("The Battle for Gilneas"), "pointer selection"): return
	if stage == "after" and not await selected_label_has_ink("HonorFrameSpecificBattlegroundButton120Name"): return
	await motion(Vector2(650, 480))
	await capture("queue-selected")
	await motion(control("HonorFrameSpecificBattlegroundButton1014").get_global_rect().get_center())
	await capture("queue-hover")
	if stage == "after":
		await pointer("HonorFrameSpecificBattlegroundButton1014")
		if not await wait_until(func(): return control("HonorFrameDescription").text.contains("Warsong Gulch"), "selection changed"): return
		if not await keyboard_activate_next("HonorFrameSpecificBattlegroundButton1018"): return
		if not await wait_until(func(): return control("HonorFrameDescription").text.contains("Arathi Basin"), "keyboard selection activates next row"): return
		await pointer("HonorFrameSpecificScrollBox", MOUSE_BUTTON_WHEEL_DOWN)
		await pointer("HonorFrameSpecificScrollBox", MOUSE_BUTTON_WHEEL_UP)
		if not control("HonorFrameDescription").text.contains("Arathi Basin"): fail("Scroll lost selection"); return
		await capture("queue-keyboard-scroll")
	await pointer("PVPUIFrameCloseButton")
	await tap(KEY_J)
	if not await wait_until(func(): return client.adventure_guide_state().loaded, "guide catalog"): return
	await pointer("EncounterJournalTierDropdown")
	await pointer("EncounterJournalTier68")
	await pointer("EncounterJournalRaidTab")
	await pointer("JournalInstance741")
	await pointer("JournalBoss1519")
	if stage == "after":
		if not await selected_label_has_ink("JournalBoss1519Name"): return
		for label in root.find_children("JournalBoss*Name", "Label", true, false):
			if label.get_line_count() != 1: fail("Boss list label spills into adjacent row: " + str(label.name)); return
	await motion(Vector2(950, 500))
	await capture("guide-boss-selected")
	await pointer("EncounterJournalCloseButton")
	await pointer("MinimapLauncherButton")
	await pointer("LauncherEntryGuildMicroButton")
	if not await wait_until(func(): return control("GuildMember0") != null, "guild roster"): return
	await motion(Vector2(1100, 950))
	await capture("guild-roster")
	root.size = Vector2i(1600, 1000)
	await settle(12)
	await pointer("GuildControlButton")
	if not await wait_until(func(): return control("GuildRankRow1") != null, "guild rank list"): return
	await pointer("GuildRankRow1")
	if stage == "after":
		var expected_rank: String = control("GuildRankRow2Name").text
		if not await keyboard_activate_next("GuildRankRow2"): return
		if not await wait_until(func(): return control("GuildRankName").text == expected_rank, "keyboard guild rank selection"): return
		await pointer("GuildRankRow1")
	if stage == "after" and not await selected_label_has_ink("GuildRankRow1Name"): return
	await motion(Vector2(1500, 950))
	await capture("guild-rank-selected")
	await pointer("GuildControlUICloseButton")
	await pointer("CommunitiesFrameCloseButton")
	root.size = Vector2i(1280, 720)
	await settle(12)
	if not await open_hashom(): return
	if not await wait_until(func(): return control("AuctionGossipOption0") != null, "source Hashom gossip"): return
	await capture("service-choices")
	await pointer("AuctionGossipOption0")
	if not await wait_until(func(): return control("TransmogOutfitEntry3Button") != null and not (control("TransmogOutfitEntry3Button") as Button).disabled, "source transmog outfits"): return
	await pointer("TransmogOutfitEntry3Button")
	if not await wait_until(func(): return client.transmog_state().viewed_outfit == 3, "viewed outfit"): return
	if stage == "after":
		await pointer("TransmogOutfitEntry2Button")
		if not await keyboard_activate_next("TransmogOutfitEntry3Button"): return
		if not await wait_until(func(): return client.transmog_state().viewed_outfit == 3, "keyboard outfit selection"): return
	await motion(Vector2(1150, 680))
	await capture("outfit-selected")
	print("SELECTION_LIST_PASS stage=", stage, " skin=", skin)
	client.free()
	quit(0)
