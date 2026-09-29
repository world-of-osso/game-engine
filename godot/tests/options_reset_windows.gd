extends "res://tests/world_menu_flow.gd"

const LAYOUT_PATH := "world-of-osso/ui_layout.ron"
const OPTIONS_PATH := "world-of-osso/options_settings.ron"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if config.is_empty():
		fail("Reset fixture requires isolated config")
		return
	var path := config.path_join(LAYOUT_PATH)
	var options := config.path_join(OPTIONS_PATH)
	var initial_options := FileAccess.get_file_as_bytes(options)
	if OS.get_environment("GODOT_TEST_RESET_VERIFY") == "1":
		if not FileAccess.get_file_as_string(options).contains("modal_offset:Some((80.0,-32.0))"):
			fail("Relaunch lost nondefault Options modal offset")
			return
		if check_persistence(path, options, initial_options):
			print("PASS: fresh process read reset layout and retained modal offset")
			quit(0)
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 15000):
		return
	print("FIXTURE RESET_LOADING")
	if not await wait_screen(client, "InWorld", WORLD_WAIT_MS):
		return
	var roster: Dictionary = client.account_state()
	if roster.selected_character_id != 17:
		fail("Expected selected server character ID 17: " + str(roster))
		return
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client):
		return
	await click_menu_action(client, "MenuBtnOptions")
	await process_frame
	var menu := client.get_node_or_null("GameMenuUI")
	var interface_tab := menu.find_child("OptionsTabinterface", true, false) as Control
	if interface_tab == null:
		fail("Authored Interface tab missing")
		return
	await click(interface_tab)
	var reset := menu.find_child("ActionButtonreset_window_positions", true, false) as Control
	if reset == null or not reset.is_visible_in_tree():
		fail("Authored Reset Window Positions button missing")
		return
	await click(reset)
	await process_frame
	if client.get_node_or_null("GameMenuUI") == null:
		fail("Reset closed Options")
		return
	if not check_persistence(path, options, initial_options):
		return
	client.free()
	var reloaded: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(reloaded)
	await process_frame
	if not check_persistence(path, options, initial_options):
		return
	print("PASS: authenticated authored reset cleared only ID 17, retained ID 18/edit layout/modal on reload")
	quit(0)

func check_persistence(path: String, options: String, initial_options: PackedByteArray) -> bool:
	var layout := FileAccess.get_file_as_string(path)
	if layout.contains("\"17\"") or not layout.contains("\"18\"") or not layout.contains("CharacterFrame") or not layout.contains("75.0") or not layout.contains("80.0") or not layout.contains("PlayerFrame") or not layout.contains("12.0") or not layout.contains("24.0") or not layout.contains("active_layout") or not layout.contains("Layout 1"):
		fail("Reset changed other character or edit-mode layout: " + layout)
		return false
	if FileAccess.get_file_as_bytes(options) != initial_options:
		fail("Reset changed saved Options modal position")
		return false
	return true
