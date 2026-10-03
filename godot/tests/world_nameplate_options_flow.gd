extends "res://tests/world_npc_visual_flow.gd"

# Run only with native_npc_visual_fixture nameplates: its private loopback server
# replicates the player and the selectable enemy NPC; no dev server is used.
func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":0") or not prepare_assets():
		fail("Nameplate fixture needs its isolated assets and loopback server")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error: String = client.connect_account(server, "fixture", "fixture", false)
	if error != "" or not await wait_screen(client, "CharacterSelect", STARTUP_WAIT_MS):
		fail("Fixture authentication: " + error)
		return
	var ui = client.get_node("CharacterSelectUI")
	await click_control(ui.find_child("CharCard_0", true, false))
	await click_control(ui.find_child("EnterWorld", true, false))
	var world_deadline := Time.get_ticks_msec() + 40000
	while Time.get_ticks_msec() < world_deadline and client.account_state().screen != "InWorld":
		await process_frame
	if client.account_state().screen != "InWorld":
		fail("Native world readiness: " + str(client.account_state()))
		return
	var npc := client.get_node_or_null("WorldUnits/Fixture Creature") as Node3D
	if npc == null:
		fail("Replicated NPC absent")
		return
	var id: int = -1
	for area in npc.find_children("UnitPick", "Area3D", true, false):
		id = area.get_meta("unit_server_id")
	if id < 0:
		fail("Replicated NPC has no pick ID")
		return
	# Retail's default Status Text is None (hover only); the authored values below are
	# read from the bars' Numeric Value text.
	await open_options(client, "interface")
	await click_option(client, "Choicestatus_text_display1Hit")
	await click_option(client, "OptionsDoneButton")
	for attempt in range(8):
		if client.target_state().target == id:
			break
		await tap(KEY_TAB)
	if client.target_state().target != id:
		fail("Tab never targeted replicated enemy")
		return
	if not await turn_until_plate(client, id):
		return
	if not await expect_plate(client, id, true, Color.WHITE):
		return
	if not await expect_target_frame(client, id, true):
		return
	if not await expect_player_frame(client, true, "10 / 40"):
		return
	var fill_color: Color = client.nameplate_state()[id].color
	await open_options(client, "hud")
	await click_option(client, "ToggleSwitchshow_health_barsLeftHit")
	await click_option(client, "OptionsDoneButton")
	await tap(KEY_TAB)
	if not await expect_target_frame(client, id, false):
		return
	if not await expect_player_frame(client, false, "10 / 40"):
		return
	if not await expect_plate(client, id, false, Color.WHITE):
		return
	await open_options(client, "accessibility")
	await click_option(client, "ToggleSwitchcolorblind_modeRightHit")
	await click_option(client, "OptionsDoneButton")
	await tap(KEY_TAB)
	if not await expect_plate(client, id, false, Color(1.0, 0.92, 0.35)):
		return
	if not client.nameplate_state()[id].color.is_equal_approx(fill_color):
		fail("Colorblind label mode changed health fill tint")
		return
	await open_options(client, "accessibility")
	await click_option(client, "ToggleSwitchcolorblind_modeLeftHit")
	await click_option(client, "OptionsDoneButton")
	await tap(KEY_TAB)
	if not await expect_plate(client, id, false, Color.WHITE):
		return
	await open_options(client, "hud")
	await click_option(client, "ToggleSwitchshow_health_barsRightHit")
	await click_option(client, "OptionsDoneButton")
	await tap(KEY_TAB)
	if not await expect_target_frame(client, id, true):
		return
	if not await expect_player_frame(client, true, "10 / 40"):
		return
	if not await expect_plate(client, id, true, Color.WHITE):
		return
	print("FIXTURE NAMEPLATE_MOVE")
	var deadline := Time.get_ticks_msec() + 10000
	while Time.get_ticks_msec() < deadline and npc.global_position.x < -8925.0:
		await process_frame
	if npc.global_position.x < -8925.0:
		fail("NPC did not move via replication")
		return
	await open_options(client, "hud")
	await set_distance(client, 80.0)
	await click_option(client, "OptionsDoneButton")
	await tap(KEY_TAB)
	if not await turn_until_plate(client, id):
		return
	if not await expect_distance(client, id, 80.0):
		return
	await open_options(client, "hud")
	await set_distance(client, 40.0)
	await click_option(client, "OptionsDoneButton")
	await tap(KEY_TAB)
	if not await expect_distance(client, id, 40.0):
		return
	await open_options(client, "hud")
	await set_distance(client, 20.0)
	await click_option(client, "OptionsDoneButton")
	await tap(KEY_TAB)
	await process_frame
	if client.nameplate_state().has(id) or not client.nameplate_rules(id).shown:
		fail("Camera fade boundary changed CVar eligibility or retained plate")
		return
	print("FIXTURE PLAYER_HEALTH_UPDATE")
	if not await wait_player_health(client, "27 / 40"):
		return
	print("FIXTURE PLAYER_REMOVE")
	if not await wait_player_hidden(client):
		return
	var reconnect_error: String = client.connect_account(server, "fixture", "fixture", false)
	if reconnect_error != "":
		fail("World reset failed: " + reconnect_error)
		return
	if client.account_state().unit_count != 0 or client.account_state().local_player_position != null:
		fail("World reset retained local player data: " + str(client.account_state()))
		return
	if not await wait_screen(client, "CharacterSelect"):
		return
	if client.account_state().local_player_position != null:
		fail("Reconnect retained local player position: " + str(client.account_state()))
		return
	print("FIXTURE NAMEPLATE_OPTIONS_DONE")
	client.free()
	quit(0)

func tap(key: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = key
		event.physical_keycode = key
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame

func turn_until_plate(client: Node, id: int) -> bool:
	for step in range(40):
		if client.nameplate_state().has(id):
			return true
		var event := InputEventKey.new()
		event.keycode = KEY_RIGHT
		event.physical_keycode = KEY_RIGHT
		event.pressed = true
		root.push_input(event, true)
		for frame in range(6):
			await process_frame
		event.pressed = false
		root.push_input(event, true)
		await process_frame
	fail("No eligible on-screen NPC plate: " + str(client.nameplate_rules(id)))
	return false

func expect_plate(client: Node, id: int, bars: bool, expected_color: Color) -> bool:
	await process_frame
	var state: Dictionary = client.nameplate_state()
	if not state.has(id) or state[id].name != "Fixture Creature":
		fail("Replicated name lost: " + str(state))
		return false
	var layer := client.get_node_or_null("Nameplates")
	var plate: Control = layer.get_child(0) if layer != null and layer.get_child_count() == 1 else null
	if plate == null or plate.get_child_count() != 4:
		fail("Expected one live plate with fill, frame, label and cast bar")
		return false
	if (plate.get_child(3) as Control).visible:
		fail("Cast bar shown without a cast")
		return false
	var fill := plate.get_child(0) as TextureRect
	var frame := plate.get_child(1) as TextureRect
	var name := plate.get_child(2) as Label
	if name == null or name.text != "Fixture Creature" or not name.is_visible_in_tree():
		fail("Name label absent when bars hidden")
		return false
	if fill.visible != bars or frame.visible != bars:
		fail("Health fill/frame visibility did not follow authored HUD switch")
		return false
	if not name.get_theme_color("font_color").is_equal_approx(expected_color):
		fail("Name label color %s, expected %s" % [name.get_theme_color("font_color"), expected_color])
		return false
	return true

func expect_player_frame(client: Node, shown: bool, expected_health: String) -> bool:
	await process_frame
	var ui := client.get_node_or_null("UnitFramesUI")
	var frame := ui.find_child("PlayerFrame", true, false) as Control if ui != null else null
	var name := ui.find_child("PlayerName", true, false) as Label if ui != null else null
	var level := ui.find_child("PlayerLevelText", true, false) as Label if ui != null else null
	var health := ui.find_child("PlayerHealthBar", true, false) as Control if ui != null else null
	var text := ui.find_child("PlayerHealthBarText", true, false) as Label if ui != null else null
	if frame == null or name == null or level == null or health == null or text == null:
		fail("Player frame cluster missing")
		return false
	var power := ui.find_child("PlayerManaBarText", true, false) as Label
	var power_text := power.text if power != null else "missing"
	if [name.text, level.text, text.text, power_text] != ["Fixture Player", "12", expected_health, "19 / 60"]:
		fail("Player authored values: %s %s %s power=%s" % [name.text, level.text, text.text, power_text])
		return false
	if frame.is_visible_in_tree() != shown or name.is_visible_in_tree() != shown or health.is_visible_in_tree() != shown:
		fail("Player cluster did not follow HUD visibility %s" % shown)
		return false
	return true

func wait_player_health(client: Node, expected: String) -> bool:
	var deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var ui := client.get_node_or_null("UnitFramesUI")
		var text := ui.find_child("PlayerHealthBarText", true, false) as Label if ui != null else null
		if text != null and text.text == expected:
			return await expect_player_frame(client, true, expected)
	fail("Replicated player health did not update to " + expected)
	return false

func wait_player_hidden(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var ui := client.get_node_or_null("UnitFramesUI")
		var frame := ui.find_child("PlayerFrame", true, false) as Control if ui != null else null
		var name := ui.find_child("PlayerName", true, false) as Label if ui != null else null
		if frame != null and not frame.is_visible_in_tree() and name != null and name.text == "":
			return true
	fail("Removed local player left stale frame")
	return false

func expect_target_frame(client: Node, id: int, shown: bool) -> bool:
	await process_frame
	var target: Dictionary = client.target_state()
	if target.target != id or target.target_name != "Fixture Creature" or target.sent != id:
		fail("HUD toggle changed selected replicated target: " + str(target))
		return false
	var ui := client.get_node_or_null("UnitFramesUI")
	var frame := ui.find_child("TargetFrame", true, false) as Control if ui != null else null
	var name := ui.find_child("TargetName", true, false) as Label if ui != null else null
	var health := ui.find_child("TargetHealthBar", true, false) as Control if ui != null else null
	if frame == null or name == null or health == null or name.text != "Fixture Creature":
		fail("Selected target frame cluster missing its name/health content")
		return false
	if frame.is_visible_in_tree() != shown or name.is_visible_in_tree() != shown or health.is_visible_in_tree() != shown:
		fail("Selected target frame cluster visibility did not follow authored HUD switch")
		return false
	return true

func expect_distance(client: Node, id: int, limit: float) -> bool:
	await process_frame
	var state: Dictionary = client.nameplate_state()
	if not state.has(id):
		fail("Distance %s plate absent despite CVar eligibility %s" % [limit, client.nameplate_rules(id)])
		return false
	var npc := client.get_node("WorldUnits/Fixture Creature") as Node3D
	var body: Vector3 = npc.global_transform * Vector3(0.0, 2.5, 0.0)
	var camera := root.get_viewport().get_camera_3d()
	var distance := camera.global_position.distance_to(body)
	var near := maxf(limit * 0.5, 1.0)
	var fade := 1.0 if distance <= near else (0.0 if distance >= limit else 1.0 - (distance - near) / (limit - near))
	var occlusion := 0.4 if state[id].occluded else 1.0
	if distance <= 20.0 or distance >= 40.0 or not is_equal_approx(state[id].alpha, fade * occlusion):
		fail("Distance %s alpha %s vs camera-body distance %s fade %s, CVar %s" % [limit, state[id].alpha, distance, fade, occlusion])
		return false
	return true

func open_options(client: Node, category: String) -> void:
	await tap(KEY_ESCAPE)
	await tap(KEY_ESCAPE)
	await process_frame
	await click_option(client, "MenuBtnOptions")
	await click_option(client, "OptionsTab" + category)

func click_option(client: Node, name: String) -> void:
	var menu := client.get_node_or_null("GameMenuUI")
	var control = menu.find_child(name, true, false) if menu != null else null
	if not control is Control:
		fail("Authored Options control absent: " + name)
		return
	await click_control(control)

func set_distance(client: Node, value: float) -> void:
	var menu := client.get_node("GameMenuUI")
	var slider := menu.find_child("Slidernameplate_distance", true, false) as Control
	if slider == null:
		fail("Authored HUD nameplate distance slider absent")
		return
	var rect := slider.get_global_rect()
	var start := Vector2(rect.position.x + 2.0, rect.get_center().y)
	var point := Vector2(lerpf(rect.position.x, rect.end.x - 1.0, (value - 20.0) / 60.0), rect.get_center().y)
	var press := InputEventMouseButton.new()
	press.position = start
	press.global_position = start
	press.button_index = MOUSE_BUTTON_LEFT
	press.button_mask = MOUSE_BUTTON_MASK_LEFT
	press.pressed = true
	root.push_input(press, true)
	await process_frame
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	root.push_input(motion, true)
	await process_frame
	var release := InputEventMouseButton.new()
	release.position = point
	release.global_position = point
	release.button_index = MOUSE_BUTTON_LEFT
	release.pressed = false
	root.push_input(release, true)
	await process_frame
