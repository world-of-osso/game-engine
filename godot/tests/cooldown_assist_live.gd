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
	for frame in range(3):
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	image.resize(1280, 720, Image.INTERPOLATE_LANCZOS)
	image.save_png(directory.path_join(name + ".png"))
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
	if OS.get_environment("COOLDOWN_ASSIST_ADMIN") == "1":
		var ready := FileAccess.open(directory.path_join("online.ready"), FileAccess.WRITE)
		ready.store_string("private fb_cooldown_assist online")
		ready.close()
		deadline = Time.get_ticks_msec() + 120000
		while not FileAccess.file_exists(directory.path_join("server-setup.done")) and Time.get_ticks_msec() < deadline:
			await process_frame
		if not FileAccess.file_exists(directory.path_join("server-setup.done")):
			fail("private admin setup timeout")
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
	await place_assistant()
	if failed:
		return
	if OS.get_environment("COOLDOWN_ASSIST_ADMIN") == "1":
		var ready := FileAccess.open(directory.path_join("combat.ready"), FileAccess.WRITE)
		ready.store_string("native options, settings and assistant placement complete")
		ready.close()
		deadline = Time.get_ticks_msec() + 120000
		while not FileAccess.file_exists(directory.path_join("combat-setup.done")) and Time.get_ticks_msec() < deadline:
			await process_frame
		if not FileAccess.file_exists(directory.path_join("combat-setup.done")):
			fail("combat setup timeout")
			return
		while float(client.account_state().get("local_player_health", 0.0)) <= 0.0 and Time.get_ticks_msec() < deadline:
			await process_frame
	await capture("viewer-ready")
	await prove_timer()
	if failed:
		return
	await key(KEY_TAB)
	var attack_error: String = client.use_spell(20271)
	if not attack_error.is_empty():
		fail(attack_error)
		return
	deadline = Time.get_ticks_msec() + 10000
	while client.assisted_combat_state().get("next_spell", 0) == 0 and Time.get_ticks_msec() < deadline:
		await process_frame
	if client.assisted_combat_state().get("next_spell", 0) == 0:
		write_json("assisted-blocker", {"assisted": client.assisted_combat_state(), "target": client.target_state()})
		await capture("assistant-unresolved")
		var before_blocked: int = client.spells_state().sent.size()
		await click("ActionButton12")
		if client.spells_state().sent.size() != before_blocked:
			fail("Unresolved rotation sent an invented spell")
			return
		print("PARTIAL: native manager/settings/timers and assistant button; exact rotation blocked: ", client.assisted_combat_state().failure_reason)
		quit(2)
		return
	var expected: int = client.assisted_combat_state().next_spell
	var before: int = client.spells_state().sent.size()
	await click("ActionButton12")
	await create_timer(0.6).timeout
	var after: Dictionary = client.spells_state()
	if after.sent.size() != before + 1 or int(after.sent[after.sent.size() - 1]) != expected:
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
		var hit: String = "ToggleSwitch" + option + "RightHit"
		if not await scroll_to("ToggleSwitch" + option):
			return
		if control(hit) != null:
			await click(hit)
	await capture("configured-options")
	if not await scroll_to("ActionRowcooldown_settings"):
		return
	await click("ActionButtoncooldown_settings")
	await click("CooldownSettingsUp19389")
	await capture("settings-essential-reordered")
	await click("CooldownSettingsCategory2")
	await click("CooldownSettingsSelect92839")
	await click("CooldownSettingsCategory3")
	await click("CooldownSettingsSelect68661")
	await capture("settings-tracked-bars")
	await click("CooldownSettingsClose")
	await click("OptionsDoneButton")
	await process_frame
	# Done returns to the menu; Escape closes its remaining owner.
	if control("MenuBtnResume") != null:
		await click("MenuBtnResume")

func mouse(at: Vector2, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = at
	event.global_position = at
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = pressed
	root.push_input(event, true)
	await process_frame

func place_assistant() -> void:
	await key(KEY_P)
	for category in range(1, 6):
		var tab := control("SpellBookCategoryTab%d" % category)
		if tab == null:
			break
		await click(str(tab.name))
		for page in range(16):
			var source := control("SpellBookItem1229376Button")
			if source != null and source.is_visible_in_tree():
				var destination := control("ActionButton12")
				if destination == null:
					fail("ActionButton12 absent")
					return
				await capture("assistant-spellbook")
				await mouse(source.get_global_rect().get_center(), true)
				var event := InputEventMouseMotion.new()
				event.position = destination.get_global_rect().get_center()
				event.global_position = event.position
				root.push_input(event, true)
				await process_frame
				await mouse(event.position, false)
				var deadline := Time.get_ticks_msec() + 5000
				while client.spells_state().bar[11] != 1229376 and Time.get_ticks_msec() < deadline:
					await process_frame
				if client.spells_state().bar[11] != 1229376:
					fail("Assistant spellbook drag did not persist to action button12")
					return
				await key(KEY_P)
				return
			if control("SpellBookNextPageButton") == null:
				break
			await click("SpellBookNextPageButton")
	fail("Learned Single-Button Assistant absent from spellbook")

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
	if viewer_remaining(92839) <= 0:
		fail("Tracked Divine Shield aura icon did not use real aura time")
		return
	await create_timer(1.6).timeout
	var aura_error: String = client.use_spell(31884)
	if not aura_error.is_empty():
		fail(aura_error)
		return
	deadline = Time.get_ticks_msec() + 5000
	while viewer_remaining(68661) <= 0 and Time.get_ticks_msec() < deadline:
		await process_frame
	if viewer_remaining(68661) <= 0:
		fail("Tracked Avenging Wrath bar did not use real aura time")
		return
	await capture("viewer-buff-bar")
