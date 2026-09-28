extends "res://tests/startup_game_menu.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	await click_menu_action(client, "MenuBtnOptions")
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var menu := client.get_node_or_null("GameMenuUI")
		var done = menu.find_child("OptionsDoneButton", true, false) if menu != null else null
		if done is Control and done.is_visible_in_tree():
			await exercise_options(client)
			return
	fail("Options click did not open authored Options panel")

func exercise_options(client: Node) -> void:
	await click_option(client, "OptionsTabadvanced")
	var fps := client.get_node_or_null("FpsOverlay")
	var before: bool = fps.visible if fps != null else false
	await click_option(client, "ToggleSwitchshow_fps_overlayLeftHit" if before else "ToggleSwitchshow_fps_overlayRightHit")
	await process_frame
	if fps == null or fps.visible == before:
		fail("HUD toggle did not immediately update FPS overlay")
		return
	var path := ProjectSettings.globalize_path("res://../data/ui/options_settings.ron")
	var external := FileAccess.get_file_as_string(path)
	var original_eula := "accepted_eula: true" if external.contains("accepted_eula: true") else "accepted_eula: false"
	var changed_eula := "accepted_eula: false" if original_eula.ends_with("true") else "accepted_eula: true"
	external = external.replace(original_eula, changed_eula)
	var original_realm := "preferredRealm: Prod" if external.contains("preferredRealm: Prod") else "preferredRealm: Dev"
	var changed_realm := "preferredRealm: Dev" if original_realm.ends_with("Prod") else "preferredRealm: Prod"
	external = external.replace(original_realm, changed_realm)
	var external_file := FileAccess.open(path, FileAccess.WRITE)
	external_file.store_string(external)
	external_file.close()
	await click_option(client, "OptionsTabgraphics")
	var slider := option_control(client, "Sliderframe_rate_limit")
	if slider == null:
		fail("Graphics FPS slider missing")
		return
	var from := slider.get_global_rect().get_center()
	var to := from + Vector2(250, 90)
	var down := InputEventMouseButton.new()
	down.button_index = MOUSE_BUTTON_LEFT
	down.position = from
	down.global_position = from
	down.pressed = true
	root.push_input(down, true)
	await process_frame
	var motion := InputEventMouseMotion.new()
	motion.position = to
	motion.global_position = to
	motion.relative = to - from
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	root.push_input(motion, true)
	await process_frame
	down.position = to
	down.global_position = to
	down.pressed = false
	root.push_input(down, true)
	await process_frame
	var slider_file := FileAccess.get_file_as_string(path)
	if not slider_file.contains(changed_eula) or not slider_file.contains(changed_realm):
		fail("Options save discarded externally changed EULA or realm")
		return
	if not slider_file.contains("frameRateLimit: 240"):
		fail("Outside slider drag/release did not persist clamped FPS value")
		return
	await click_option(client, "OptionsDefaultsButton")
	if not FileAccess.get_file_as_string(path).contains("frameRateLimit: 144"):
		fail("Graphics Defaults did not persist category defaults")
		return
	await click_option(client, "OptionsTabkeybindings")
	if option_control(client, "KeybindingRebindmove_forward") == null:
		fail("Slider outside release swallowed subsequent keybinding tab click")
		return
	var previous_binding := binding_value(client)
	await click_option(client, "KeybindingRebindmove_forward")
	if not binding_value(client).contains("Press a key"):
		fail("Keybinding did not arm on click/release")
		return
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	await process_frame
	if binding_value(client) != previous_binding or not option_control(client, "OptionsDoneButton"):
		fail("Escape did not cancel capture while preserving Options")
		return
	await click_option(client, "KeybindingRebindmove_forward")
	push_key(KEY_R, true)
	await process_frame
	push_key(KEY_R, false)
	await process_frame
	if not binding_value(client).contains("R"):
		fail("Captured key did not replace binding")
		return
	await click_option(client, "KeybindingRebindmove_forward")
	await click_option(client, "OptionsDoneButton")
	if not option_control(client, "OptionsDoneButton") or not binding_value(client).contains("Mouse"):
		fail("Captured mouse click fell through to Done or failed binding")
		return
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	await process_frame
	if not menu_authored(client):
		fail("Options Escape did not return to main menu")
		return
	await click_menu_action(client, "MenuBtnAddons")
	await process_frame
	if option_control(client, "OptionsDoneButton") == null:
		fail("AddOns did not route to authored Options panel")
		return
	await click_option(client, "OptionsDoneButton")
	if not await wait_menu_closed(client, null):
		return
	if not FileAccess.file_exists(path):
		fail("Edited options were not persisted")
		return
	var saved := FileAccess.get_file_as_string(path)
	if not saved.contains("MoveForward: Some(\"mouse:Left\")"):
		fail("Captured mouse binding not persisted to canonical options")
		return
	print("PASS: Options FPS live, slider outside release, Defaults, capture/cancel, Escape back, AddOns, Done, external preferences preserved")
	quit(0)

func option_control(client: Node, name: String) -> Control:
	var menu := client.get_node_or_null("GameMenuUI")
	return menu.find_child(name, true, false) as Control if menu != null else null

func binding_value(client: Node) -> String:
	var label := option_control(client, "KeybindingValuemove_forward") as Label
	return label.text if label != null else ""

func click_option(client: Node, name: String) -> void:
	var control := option_control(client, name)
	if control == null:
		fail("Authored option control missing: " + name)
		return
	await click(control)
