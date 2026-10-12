extends SceneTree
# Real private session, native pointer/key input. Never mutates server list data.
var client: Node
var output := OS.get_environment("SELLIST_OUTPUT")
var stage := OS.get_environment("SELLIST_STAGE")
var skin := OS.get_environment("SELLIST_SKIN")
var failed := false

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
	root.notify_mouse_entered()
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
	var point := target.get_global_rect().get_center()
	await motion(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
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

func capture(suffix: String) -> void:
	await settle(30)
	await RenderingServer.frame_post_draw
	var path := output.path_join(skin + "-" + stage + "-" + suffix + ".png")
	if root.get_texture().get_image().save_png(path) != OK: fail("Capture: " + path)

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
	if not await wait_until(func(): return control("LFDMicroButton") != null, "inworld HUD"): return
	await pointer("LFDMicroButton")
	if not await wait_until(func(): return control("HonorFrameSpecificBattlegroundButton699") != null, "queue list"): return
	await pointer("HonorFrameSpecificBattlegroundButton699")
	await motion(Vector2(650, 480))
	await capture("queue-selected")
	await motion(control("HonorFrameSpecificBattlegroundButton1014").get_global_rect().get_center())
	await capture("queue-hover")
	if stage == "after":
		await pointer("HonorFrameSpecificBattlegroundButton1014")
		await tap(KEY_DOWN)
		await tap(KEY_ENTER)
		if not control("HonorFrameDescription").text.contains("Arathi Basin"): fail("Keyboard selection did not activate next row"); return
		await pointer("HonorFrameSpecificScrollBox", MOUSE_BUTTON_WHEEL_DOWN)
		await pointer("HonorFrameSpecificScrollBox", MOUSE_BUTTON_WHEEL_UP)
		if not control("HonorFrameDescription").text.contains("Arathi Basin"): fail("Scroll lost selection"); return
		await capture("queue-keyboard-scroll")
	await pointer("PVPUIFrameCloseButton")
	await tap(KEY_J)
	if not await wait_until(func(): return client.adventure_guide_state().loaded, "guide catalog"): return
	await pointer("EncounterJournalTierDropdown")
	await pointer("EncounterJournalTier68")
	await pointer("EncounterJournalTab1")
	await pointer("JournalInstance741")
	await pointer("JournalBoss1519")
	await motion(Vector2(950, 500))
	await capture("guide-boss-selected")
	print("SELECTION_LIST_PASS stage=", stage, " skin=", skin)
	client.free()
	quit(0)
