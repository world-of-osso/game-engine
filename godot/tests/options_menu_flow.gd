extends "res://tests/startup_game_menu.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var config_dir := OS.get_environment("XDG_CONFIG_HOME")
	if config_dir.is_empty():
		fail("Options fixture requires isolated XDG_CONFIG_HOME")
		return
	var canonical_path := config_dir.path_join("world-of-osso/options_settings.ron")
	var legacy_path := ProjectSettings.globalize_path("res://../data/ui/options_settings.ron")
	if not FileAccess.file_exists(canonical_path) or not FileAccess.file_exists(legacy_path):
		fail("Options fixture requires existing canonical and legacy options files")
		return
	if not FileAccess.get_file_as_string(canonical_path).contains("frameRateLimit: 144"):
		fail("Options fixture requires canonical FPS limit 144 before editing")
		return
	var legacy_bytes := FileAccess.get_file_as_bytes(legacy_path)
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
			await exercise_options(client, canonical_path, legacy_path, legacy_bytes)
			return
	fail("Options click did not open authored Options panel")

func exercise_options(client: Node, path: String, legacy_path: String, legacy_bytes: PackedByteArray) -> void:
	if not await exercise_modified_capture(client, path):
		return
	await click_option(client, "OptionsTabadvanced")
	var fps := client.get_node_or_null("FpsOverlay")
	var before: bool = fps.visible if fps != null else false
	await click_option(client, "ToggleSwitchshow_fps_overlayLeftHit" if before else "ToggleSwitchshow_fps_overlayRightHit")
	await process_frame
	if fps == null or fps.visible == before:
		fail("HUD toggle did not immediately update FPS overlay")
		return
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
	await click_option(client, "OptionsDefaultsButton")
	if not FileAccess.get_file_as_string(path).contains("frameRateLimit: 144"):
		fail("Graphics Defaults did not persist category defaults to canonical options")
		return
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
	await click_option(client, "OptionsTabkeybindings")
	if option_control(client, "KeybindingButtonmove_forward") == null:
		fail("Slider outside release swallowed subsequent keybinding tab click")
		return
	var previous_binding := binding_value(client)
	await click_option(client, "KeybindingButtonmove_forward")
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
	await click_option(client, "KeybindingButtonmove_forward")
	push_key(KEY_R, true)
	await process_frame
	push_key(KEY_R, false)
	await process_frame
	if not binding_value(client).contains("R"):
		fail("Captured key did not replace binding")
		return
	await click_option(client, "KeybindingButtonmove_forward")
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
	if not saved.contains("MoveForward: Some(\"mouse:Left\")") or not saved.contains("frameRateLimit: 240"):
		fail("Captured mouse binding or changed FPS not persisted to canonical options")
		return
	if FileAccess.get_file_as_bytes(legacy_path) != legacy_bytes:
		fail("Options edit changed legacy options file")
		return
	client.free()
	var reloaded: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(reloaded)
	await process_frame
	var reloaded_fps := reloaded.get_node_or_null("FpsOverlay")
	if reloaded_fps == null or reloaded_fps.visible == before:
		fail("Reinstantiated client did not reload saved FPS overlay")
		return
	if not await wait_for_startup_menu(reloaded):
		return
	await click_menu_action(reloaded, "MenuBtnOptions")
	await click_option(reloaded, "OptionsTabgraphics")
	var reloaded_slider := option_control(reloaded, "Sliderframe_rate_limit")
	var reloaded_fps_value := option_control(reloaded, "SliderValueframe_rate_limit") as Label
	if reloaded_slider == null or reloaded_fps_value == null or reloaded_fps_value.text != "240.0":
		fail("Reinstantiated client did not project saved canonical FPS limit")
		return
	if FileAccess.get_file_as_bytes(legacy_path) != legacy_bytes:
		fail("Options reload changed legacy options file")
		return
	print("PASS: canonical FPS reload, legacy unchanged, external preferences, defaults, Ctrl/Shift capture and mouse capture")
	quit(0)

func exercise_modified_capture(client: Node, path: String) -> bool:
	await click_option(client, "OptionsTabkeybindings")
	await click_option(client, "KeybindingButtonmove_forward")
	push_modified_key(KEY_CTRL, true, true, false)
	await process_frame
	if not binding_value(client).contains("Press a key"):
		fail("Modifier-only key ended capture")
		return false
	push_modified_key(KEY_R, true, true, true)
	await process_frame
	push_modified_key(KEY_R, false, true, true)
	push_modified_key(KEY_CTRL, false, false, false)
	await process_frame
	if not binding_value(client).contains("Ctrl") or not binding_value(client).contains("R"):
		fail("Ctrl+Shift+R did not capture with Ctrl precedence")
		return false
	if not FileAccess.get_file_as_string(path).contains("MoveForward: Some(\"ctrl+key:KeyR\")"):
		fail("Captured Ctrl+R not persisted to canonical options")
		return false
	await click_option(client, "KeybindingButtonmove_forward")
	push_modified_key(KEY_SHIFT, true, false, true)
	await process_frame
	if not binding_value(client).contains("Press a key"):
		fail("Shift-only key ended capture")
		return false
	push_modified_key(KEY_T, true, false, true)
	await process_frame
	push_modified_key(KEY_T, false, false, true)
	push_modified_key(KEY_SHIFT, false, false, false)
	await process_frame
	if not binding_value(client).contains("Shift") or not binding_value(client).contains("T"):
		fail("Shift+T did not capture")
		return false
	if not FileAccess.get_file_as_string(path).contains("MoveForward: Some(\"shift+key:KeyT\")"):
		fail("Captured Shift+T not persisted to canonical options")
		return false
	return true

func push_modified_key(code: Key, pressed: bool, ctrl: bool, shift: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	event.ctrl_pressed = ctrl
	event.shift_pressed = shift
	root.push_input(event, true)

func option_control(client: Node, name: String) -> Control:
	var menu := client.get_node_or_null("GameMenuUI")
	return menu.find_child(name, true, false) as Control if menu != null else null

func binding_value(client: Node) -> String:
	var label := option_control(client, "KeybindingButtonTextmove_forward") as Label
	return label.text if label != null else ""

func click_option(client: Node, name: String) -> void:
	var control := option_control(client, name)
	if control == null:
		fail("Authored option control missing: " + name)
		return
	await click(control)
