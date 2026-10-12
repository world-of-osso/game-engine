extends SceneTree
# Owned private endpoint/account from the run environment; never shared UDP5000.
# Real auth/roster/native input. No fake cooldown/aura/charge assignment.
var client: Node
var directory := OS.get_environment("COOLDOWN_ASSIST_RUN")
var failed := false
var endpoint := OS.get_environment("COOLDOWN_ASSIST_SERVER")
var account := OS.get_environment("COOLDOWN_ASSIST_ACCOUNT")

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
	write_json(name, {"account": client.account_state(), "spells": client.spells_state(), "assisted": client.assisted_combat_state(), "viewer": client.cooldown_viewer_state(), "target": client.target_state()})

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
	var error: String = client.connect_account(endpoint, account, "fbtest", false)
	if not error.is_empty():
		fail(error)
		return
	if not await wait_screen("CharacterSelect"):
		return
	await click("CharCard_0")
	await click("EnterWorld")
	if not await wait_screen("InWorld"):
		return
	if not prove_grounded():
		return
	await capture("world-online")
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
	await prove_edit_mode()
	if failed:
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
	var attack_error: String = client.use_spell(6603)
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
		await click("ActionButton6")
		if client.spells_state().sent.size() != before_blocked:
			fail("Unresolved rotation sent an invented spell")
			return
		print("PARTIAL: native manager/settings/timers and assistant button; recommendation unavailable: ", client.assisted_combat_state(), "; target: ", client.target_state())
		quit(2)
		return
	var expected: int = client.assisted_combat_state().next_spell
	if not await prove_highlights(expected):
		return
	expected = client.assisted_combat_state().next_spell
	var before: int = client.spells_state().sent.size()
	var successes_before: int = client.spells_state().cast_success.size()
	await click("ActionButton6")
	await create_timer(0.6).timeout
	var after: Dictionary = client.spells_state()
	if after.sent.size() != before + 1 or int(after.sent[after.sent.size() - 1]) != expected:
		fail("Assistant did not send exactly its authored recommendation")
		return
	if after.cast_success.size() != successes_before + 1 or int(after.cast_success[after.cast_success.size() - 1]) != expected:
		fail("Assistant request did not resolve as the exact server CastSuccess")
		return
	await capture("assisted-cast")
	print("PASS: real manager clocks, authored single-button cast and matching animated action/viewer highlights; unresolved rule boundaries remain separate")
	quit(0)

func prove_highlights(expected: int) -> bool:
	var viewer_name := ""
	for icon in client.cooldown_viewer_state().icons:
		if int(icon.spell_id) == expected and bool(icon.highlight):
			viewer_name = "CooldownViewer%dAssistedHighlightFlipbook" % int(icon.cooldown_id)
			break
	var action_name := ""
	var slots: PackedInt64Array = client.spells_state().bar
	for slot in range(12):
		if int(slots[slot]) == expected:
			var name := "ActionButton%dAssistedHighlightFlipbook" % (slot + 1)
			var node := control(name)
			if node != null and node.is_visible_in_tree():
				action_name = name
				break
	if viewer_name.is_empty() or action_name.is_empty():
		fail("Matching recommendation highlights absent: spell%d, viewer=%s, action=%s" % [expected, viewer_name, action_name])
		return false
	var crops := {}
	for name in [viewer_name, action_name]:
		var node := control(name)
		if node == null or not node.is_visible_in_tree():
			fail("Highlight not rendered: " + name)
			return false
		var rect := node.get_node("Parts/Part0") as TextureRect
		var atlas := rect.texture as AtlasTexture
		if atlas == null:
			fail("Retail highlight art not loaded: " + name)
			return false
		crops[name] = atlas.region
	await capture("assisted-highlight-frame1")
	await create_timer(0.12).timeout
	for name in crops:
		var rect := control(name).get_node("Parts/Part0") as TextureRect
		var atlas := rect.texture as AtlasTexture
		if atlas.region == crops[name]:
			fail("Retail ants flipbook did not advance: " + name)
			return false
	await capture("assisted-highlight-frame2")
	write_json("highlight-proof", {"spell": expected, "viewer": viewer_name, "action": action_name, "in_combat": client.assisted_combat_state().in_combat})
	return true

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
	var icons: Array = client.cooldown_viewer_state().icons
	if icons.is_empty() or int(icons[0].cooldown_id) != 19389:
		fail("Settings Up did not move Crusader Strike before Judgment")
		return
	await click("CooldownSettingsCategory2")
	await click("CooldownSettingsSelect92839")
	for frame in range(3):
		await process_frame
	await click("CooldownSettingsCategory3")
	await click("CooldownSettingsSelect68661")
	await capture("settings-tracked-bars")
	await click("CooldownSettingsClose")
	await click("OptionsDoneButton")
	await process_frame
	# Done returns to the menu; Escape closes its remaining owner.
	if control("MenuBtnResume") != null:
		await click("MenuBtnResume")

func prove_edit_mode() -> void:
	var before := control("EssentialCooldownViewer").get_global_rect()
	await key(KEY_F10)
	for name in ["essential_cooldowns", "utility_cooldowns", "tracked_buffs", "tracked_buff_bars"]:
		if control("EditModeSelection_" + name + "Label") == null:
			fail("Viewer Edit Mode mover absent: " + name)
			return
	await capture("viewer-edit-mode")
	var start := before.position + Vector2(20, 20)
	var motion := InputEventMouseMotion.new()
	motion.position = start
	motion.global_position = start
	root.push_input(motion, true)
	await process_frame
	await mouse(start, true)
	for step in range(1, 9):
		var event := InputEventMouseMotion.new()
		event.position = start + Vector2(-120, -80) * float(step) / 8.0
		event.global_position = event.position
		event.relative = Vector2(-120, -80) / 8.0
		event.button_mask = MOUSE_BUTTON_MASK_LEFT
		root.push_input(event, true)
		await process_frame
	await mouse(start + Vector2(-120, -80), false)
	var moved := control("EssentialCooldownViewer").get_global_rect()
	if moved.position.distance_to(before.position) < 100:
		fail("Viewer Edit Mode drag did not move Essential root")
		return
	await key(KEY_ESCAPE)
	for frame in range(3):
		await process_frame
	if control("EssentialCooldownViewer").get_global_rect().position.distance_to(moved.position) > 1:
		fail("Viewer Edit Mode exit lost placement")
		return
	var account_directory := "world-of-osso/accounts/" + endpoint.to_utf8_buffer().hex_encode() + "/" + account.to_utf8_buffer().hex_encode()
	var path := OS.get_environment("XDG_CONFIG_HOME").path_join(account_directory).path_join("ui_layout.ron")
	if not FileAccess.file_exists(path):
		fail("Viewer Edit Mode did not persist account layout")
		return
	var file := FileAccess.open(directory.path_join("saved-layout.ron"), FileAccess.WRITE)
	file.store_string(FileAccess.get_file_as_string(path))
	file.close()
	write_json("viewer-placement", {"before": str(before), "saved": str(moved)})
	await capture("viewer-layout-saved")

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
				var destination := control("ActionButton6")
				if destination == null:
					fail("ActionButton6 absent")
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
				while client.spells_state().bar[5] != 1229376 and Time.get_ticks_msec() < deadline:
					await process_frame
				if client.spells_state().bar[5] != 1229376:
					fail("Assistant spellbook drag did not persist to action button6")
					return
				await key(KEY_P)
				return
			if control("SpellBookNextPageButton") == null:
				break
			await click("SpellBookNextPageButton")
	fail("Learned Single-Button Assistant absent from spellbook")

func prove_grounded() -> bool:
	var state: Dictionary = client.account_state()
	var surface: Variant = client.terrain_height_at(-8985.0, 179.0)
	if surface == null:
		fail("Retail destination surface not loaded")
		return false
	var predicted := coordinates(state.local_player_position)
	var authority := coordinates(state.local_server_position)
	if absf(predicted.y - float(surface)) > 0.02 or absf(authority.y - float(surface)) > 0.02:
		write_json("ground-blocker", {"surface": surface, "account": state})
		fail("Private destination must use place-private-character.py before login, not legacy NPC Z")
		return false
	return true

func coordinates(value: String) -> Vector3:
	var parts := value.trim_prefix("(").trim_suffix(")").split(",")
	return Vector3(float(parts[0]), float(parts[1]), float(parts[2]))

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
		write_json("bar-blocker", {"account": client.account_state(), "spells": client.spells_state(), "viewer": client.cooldown_viewer_state(), "auras": client.aura_state()})
		fail("Tracked Avenging Wrath bar did not use real aura time")
		return
	await capture("viewer-buff-bar")
