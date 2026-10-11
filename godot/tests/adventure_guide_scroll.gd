extends SceneTree
# Rendered regression: real private session, model -> Classic raids, wheel -> Molten Core.
var client: Node
var failed := false
func _initialize() -> void:
	call_deferred("run")
func stop(reason: String) -> void:
	failed = true
	push_error("ADVENTURE_GUIDE_SCROLL_FAIL: " + reason)
	quit(1)
func settle() -> void:
	for frame in range(6): await process_frame
func click(name: String) -> void:
	var control := client.find_child(name, true, false) as Control
	if control == null:
		stop("Missing control " + name)
		return
	await pointer(control.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
func pointer(point: Vector2, button: int) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await settle()
func wait_screen(screen: String) -> bool:
	var deadline := Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.account_state().screen == screen: return true
	stop("Screen not ready: " + screen)
	return false
func run() -> void:
	var server := OS.get_environment("ADVENTURE_GUIDE_SERVER")
	var account := OS.get_environment("ADVENTURE_GUIDE_ACCOUNT")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":5000") or not account.begins_with("fb_"):
		stop("Explicit private endpoint and fb_* account required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await settle()
	var error: String = client.connect_account(server, account, OS.get_environment("ADVENTURE_GUIDE_PASSWORD"), false)
	if not error.is_empty():
		stop(error)
		return
	if not await wait_screen("CharacterSelect"): return
	await click("EnterWorld")
	if not await wait_screen("InWorld"): return
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_J
		event.physical_keycode = KEY_J
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await settle()
	if not client.adventure_guide_state().loaded:
		await click("EncounterJournalCloseButton")
		if client.adventure_guide_state().visible:
			stop("Pending guide ignored its close button")
			return
		for pressed in [true, false]:
			var reopen := InputEventKey.new()
			reopen.keycode = KEY_J
			reopen.physical_keycode = KEY_J
			reopen.pressed = pressed
			root.push_input(reopen, true)
			await process_frame
		await settle()
	var open_deadline := Time.get_ticks_msec() + 60000
	while not client.adventure_guide_state().loaded and Time.get_ticks_msec() < open_deadline:
		await process_frame
	if not client.adventure_guide_state().loaded or not client.adventure_guide_state().visible:
		stop("First J press did not finish loading the guide")
		return
	await click("JournalInstance67")
	await click("JournalBoss112")
	await click("EncounterJournalPage3")
	var deadline := Time.get_ticks_msec() + 60000
	while client.adventure_guide_state().model_display != 36475 and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.adventure_guide_state().model_display != 36475:
		stop("Ozruk model did not attach")
		return
	await click("EncounterJournalTab1")
	await click("EncounterJournalTierDropdown")
	await click("EncounterJournalTier68")
	var list := client.find_child("EncounterJournalContent", true, false) as Control
	var molten_core := client.find_child("JournalInstance741", true, false) as Control
	if list == null or molten_core == null:
		stop("Classic raid controls missing")
		return
	var before := molten_core.get_global_rect().position.y
	for tick in range(5): await pointer(list.get_global_rect().get_center(), MOUSE_BUTTON_WHEEL_DOWN)
	var after := molten_core.get_global_rect().position.y
	if not after < before or not list.get_global_rect().has_point(molten_core.get_global_rect().get_center()):
		stop("Wheel did not reveal Molten Core: before=%s after=%s" % [before, after])
		return
	await click("JournalInstance741")
	await click("JournalBoss1519")
	var state = client.adventure_guide_state()
	if state.instance != 741 or state.encounter != 1519 or state.difficulty != 9:
		stop("Scroll did not select authored Molten Core/Lucifron: " + str(state))
		return
	print("ADVENTURE_GUIDE_SCROLL_PASS skin=%s before=%s after=%s" % [client.account_state().ui_skin, before, after])
	quit(0)
