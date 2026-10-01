extends "res://tests/startup_game_menu.gd"

# `--screen optionsmenu` (original `StartupGameMenuView(Options)`): startup opens the
# standalone game menu directly on its Options panel. Under an isolated XDG_CONFIG_HOME:
#   GODOT_OPTIONS_PHASE=change — real clicks on the Advanced tab and the FPS overlay
#                                toggle show the overlay and persist `show_fps_overlay:
#                                true`; Done closes the menu.
#   GODOT_OPTIONS_PHASE=reload — a new process starts with the overlay shown and the
#                                toggle on.
# GODOT_OPTIONS_SCREENSHOT saves the startup panel.

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var phase := OS.get_environment("GODOT_OPTIONS_PHASE")
	if phase not in ["change", "reload"]:
		fail("Options startup fixture requires GODOT_OPTIONS_PHASE=change|reload")
		return
	var config_dir := OS.get_environment("XDG_CONFIG_HOME")
	if config_dir.is_empty():
		fail("Options startup fixture requires isolated XDG_CONFIG_HOME")
		return
	var path := config_dir.path_join("world-of-osso/options_settings.ron")
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	await process_frame
	var done := option_control(client, "OptionsDoneButton")
	if done == null or not done.is_visible_in_tree():
		fail("--screen optionsmenu did not open the Options panel")
		return
	var resume := option_control(client, "MenuBtnResume")
	if resume != null and resume.is_visible_in_tree():
		fail("--screen optionsmenu left the main menu buttons visible")
		return
	var fps := client.get_node_or_null("FpsOverlay") as CanvasLayer
	if fps == null:
		fail("FpsOverlay missing")
		return
	if phase == "reload":
		if not fps.visible:
			fail("Persisted FPS overlay option was not applied at startup")
			return
		print("PASS: optionsmenu reload applied persisted show_fps_overlay")
		quit(0)
		return
	if not await capture():
		return
	if fps.visible or saved_fps_overlay(path):
		fail("Change phase requires the FPS overlay initially off")
		return
	if not await click_named(client, "OptionsTabadvanced"):
		return
	if not await click_named(client, "ToggleSwitchshow_fps_overlayRightHit"):
		return
	await process_frame
	if not fps.visible:
		fail("FPS overlay toggle did not show the overlay")
		return
	if not saved_fps_overlay(path):
		fail("FPS overlay toggle did not persist to " + path)
		return
	if not await click_named(client, "OptionsDoneButton"):
		return
	if not await wait_menu_closed(client, null):
		return
	print("PASS: optionsmenu opened Options, toggle persisted, Done closed")
	quit(0)

func option_control(client: Node, name: String) -> Control:
	var menu := client.get_node_or_null("GameMenuUI")
	return menu.find_child(name, true, false) as Control if menu != null else null

func click_named(client: Node, name: String) -> bool:
	var control := option_control(client, name)
	if control == null or not control.is_visible_in_tree():
		fail("Authored Options control missing: " + name)
		return false
	await click(control)
	return true

func saved_fps_overlay(path: String) -> bool:
	return FileAccess.file_exists(path) and FileAccess.get_file_as_string(path).contains("show_fps_overlay: true")

func capture() -> bool:
	var path := OS.get_environment("GODOT_OPTIONS_SCREENSHOT")
	if path.is_empty():
		return true
	for frame in range(10):
		await process_frame
	await RenderingServer.frame_post_draw
	var saved := root.get_texture().get_image().save_png(path)
	if saved != OK:
		fail("Save Options screenshot: " + error_string(saved))
		return false
	print("FIXTURE OPTIONS_CAPTURED ", path)
	return true
