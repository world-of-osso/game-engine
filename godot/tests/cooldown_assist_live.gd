extends SceneTree
# Private UDP5860 fixture; fb_* credentials supplied by the owned run environment.
# Real auth/roster/native input. No fake cooldown/aura/charge assignment.
var client: Node
var directory := OS.get_environment("COOLDOWN_ASSIST_RUN")
var failed := false

func _initialize() -> void:
	call_deferred("run_fixture")

func fail(message: String) -> void:
	failed = true
	push_error("Cooldown assist fixture: " + message)
	quit(1)

func control(name: String) -> Control:
	return client.find_child(name, true, false) as Control

func key(code: int) -> void:
	for down in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = down
		root.push_input(event, true)
		await process_frame
	await process_frame

func click(name: String) -> void:
	var node := control(name)
	if node == null or not node.is_visible_in_tree():
		fail("missing visible control " + name)
		return
	var point := node.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = down
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if down else 0
		root.push_input(event, true)
		await process_frame
	await process_frame

func write_json(name: String, value: Dictionary) -> void:
	var file := FileAccess.open(directory.path_join(name + ".json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(value, "\t"))
	file.close()

func capture(name: String) -> void:
	for frame in range(20):
		await process_frame
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(directory.path_join(name + ".png"))
	write_json(name, {"account": client.account_state(), "spells": client.spells_state(), "assisted": client.assisted_combat_state(), "viewer": client.cooldown_viewer_state()})

func wait_screen(screen: String) -> bool:
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().get("screen", "") != screen and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.account_state().get("screen", "") != screen:
		fail("screen timeout " + screen + ": " + str(client.account_state()))
		return false
	return true

func run_fixture() -> void:
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().get("assets_starting", true) and Time.get_ticks_msec() < deadline:
		await process_frame
	var error: String = client.connect_account("127.0.0.1:5860", "fb_cooldown_assist", "fbtest", false)
	if not error.is_empty():
		fail(error)
		return
	if not await wait_screen("CharacterSelect"):
		return
	await click("CharCard_0")
	await click("EnterWorld")
	if not await wait_screen("InWorld"):
		return
	var spec_error: String = client.set_specialization(70)
	if not spec_error.is_empty():
		fail(spec_error)
		return
	deadline = Time.get_ticks_msec() + 30000
	while client.spells_state().get("spec", 0) != 70 and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.spells_state().get("spec", 0) != 70:
		fail("Ret specialization was not accepted")
		return
	await configure_options()
	if failed:
		return
	deadline = Time.get_ticks_msec() + 30000
	while control("CooldownViewer19389") == null and Time.get_ticks_msec() < deadline:
		await process_frame
	if control("CooldownViewer19389") == null:
		fail("Essential Crusader Strike viewer not mounted")
		return
	await capture("viewer-ready")
	await prove_timer()
	if failed:
		return
	await key(KEY_TAB)
	deadline = Time.get_ticks_msec() + 10000
	while client.assisted_combat_state().get("next_spell", 0) == 0 and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.assisted_combat_state().get("next_spell", 0) == 0:
		write_json("assisted-blocker", {"assisted": client.assisted_combat_state(), "target": client.target_state()})
		fail("No authored next spell: " + str(client.assisted_combat_state()))
		return
	var expected: int = client.assisted_combat_state().next_spell
	var before: int = client.spells_state().sent.size()
	var cast_error: String = client.use_spell(1229376)
	if not cast_error.is_empty():
		fail(cast_error)
		return
	await create_timer(0.6).timeout
	var after: Dictionary = client.spells_state()
	if after.sent.size() != before + 1 or int(after.sent.back()) != expected:
		fail("Assistant did not send exactly its authored recommendation")
		return
	await capture("assisted-cast")
	print("PASS: cooldown timers and authored single-button private sample; numeric/all-spec acceptance remains separate")
	quit(0)

func scroll_to(name: String) -> bool:
	for step in range(30):
		var node := control(name)
		var pane := control("OptionsContentArea")
		if pane == null:
			fail("Options content pane absent")
			return false
		var bounds := pane.get_global_rect()
		if node != null and bounds.has_point(node.get_global_rect().get_center()):
			return true
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_WHEEL_DOWN
		event.pressed = true
		event.position = bounds.get_center()
		event.global_position = event.position
		root.push_input(event, true)
		await process_frame
		await process_frame
	fail("Could not scroll to " + name)
	return false

func configure_options() -> void:
	await key(KEY_ESCAPE)
	await click("MenuBtnOptions")
	await click("OptionsTabhud")
	var layout := "2" if OS.get_environment("COOLDOWN_ASSIST_SKIN") == "Forever" else "0"
	var choice := "Choiceui_layout" + layout + "Hit"
	if control(choice) != null:
		await click(choice)
	for option in ["cooldown_viewer_enabled", "assisted_highlight", "cooldown_assisted_highlight"]:
		var hit := "ToggleSwitch" + option + "RightHit"
		if not await scroll_to("ToggleSwitch" + option):
			return
		if control(hit) != null:
			await click(hit)
	await capture("configured-options")
	await click("OptionsDoneButton")
	await process_frame
	# Done returns to the menu; Escape closes its remaining owner.
	if control("MenuBtnResume") != null:
		await click("MenuBtnResume")

func viewer_remaining(cooldown_id: int) -> float:
	for icon in client.cooldown_viewer_state().get("icons", []):
		if int(icon.cooldown_id) == cooldown_id:
			return float(icon.remaining)
	return 0.0

func prove_timer() -> void:
	var error: String = client.use_spell(642)
	if not error.is_empty():
		fail(error)
		return
	var deadline := Time.get_ticks_msec() + 5000
	while viewer_remaining(19411) == 0 and Time.get_ticks_msec() < deadline:
		await process_frame
	var first: float = viewer_remaining(19411)
	if first <= 0:
		write_json("timer-blocker", client.spells_state())
		fail("Server did not start Divine Shield cooldown")
		return
	await capture("viewer-timer-start")
	await create_timer(2.0).timeout
	var second: float = viewer_remaining(19411)
	if second <= 0 or second >= first:
		fail("Server cooldown projection did not tick")
		return
	await capture("viewer-timer-tick")
