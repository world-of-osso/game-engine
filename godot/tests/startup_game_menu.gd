extends "res://tests/world_menu_flow.gd"

const STARTUP_WAIT_MS := 15000

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var action := OS.get_environment("GODOT_MENU_ACTION")
	if action not in ["escape", "resume", "exit"]:
		fail("Startup menu fixture requires GODOT_MENU_ACTION=escape|resume|exit")
		return
	var screenshot := OS.get_environment("GODOT_STARTUP_SCREENSHOT")
	if screenshot.is_empty():
		fail("Startup menu fixture requires an owned screenshot path")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	var error := inspect_standalone_menu(client)
	if error != "":
		fail(error)
		return
	# Capture the initial authored overlay before any input dismisses it.
	var capture_after := Time.get_ticks_msec() + 1000
	while Time.get_ticks_msec() < capture_after:
		await process_frame
	await RenderingServer.frame_post_draw
	error = inspect_standalone_menu(client)
	if error != "":
		fail(error)
		return
	var saved := root.get_texture().get_image().save_png(screenshot)
	if saved != OK:
		fail("Save standalone menu screenshot: " + error_string(saved))
		return
	print("FIXTURE STARTUP_GAME_MENU_CAPTURED ", screenshot)
	match action:
		"escape":
			push_key(KEY_ESCAPE, true)
			await process_frame
			push_key(KEY_ESCAPE, false)
		"resume":
			await click_menu_action(client, "MenuBtnResume")
		"exit":
			await click_menu_action(client, "MenuBtnExit")
			var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
			while Time.get_ticks_msec() < deadline:
				await process_frame
			fail("Exit Game did not terminate native client")
			return
	if not await wait_menu_closed(client, null):
		return
	for frame in range(8):
		await process_frame
	error = inspect_offline_state(client)
	if error != "":
		fail(error)
		return
	if client.get_node_or_null("GameMenuUI") != null:
		fail("Standalone game menu remained open after " + action)
		return
	print("PASS: standalone GameMenu ", action, " retains GameMenu without Login screenshot=", screenshot)
	quit(0)

func wait_for_startup_menu(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + STARTUP_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.account_state().screen == "GameMenu" and client.get_node_or_null("GameMenuUI") != null:
			return true
	fail("CLI did not open standalone GameMenu: " + str(client.account_state()))
	return false

func inspect_standalone_menu(client: Node) -> String:
	var error := inspect_offline_state(client)
	if error != "":
		return error
	var menu := client.get_node_or_null("GameMenuUI") as CanvasLayer
	if menu == null or not menu.visible:
		return "Standalone GameMenuUI absent or hidden"
	for name in ["GameMenuRoot", "MenuBtnResume", "MenuBtnLogout", "MenuBtnExit"]:
		var control := menu.find_child(name, true, false) as Control
		if control == null or not control.is_visible_in_tree():
			return "Standalone authored menu control absent or hidden: " + name
	return ""

func inspect_offline_state(client: Node) -> String:
	var state: Dictionary = client.account_state()
	if state.screen != "GameMenu":
		return "Standalone menu changed account screen: " + str(state)
	if state.reply_received or state.character_count != 0 or state.unit_count != 0 or state.world_attached:
		return "Standalone menu authenticated or populated characters/world: " + str(state)
	if client.get_node_or_null("WorldUnits") != null:
		return "Standalone menu spawned world units"
	for name in ["LoginUI", "LoadingUI", "CharacterSelectUI", "CharacterCreateUI"]:
		var ui := client.get_node_or_null(name) as CanvasLayer
		if ui != null and ui.visible:
			return "Standalone menu exposed account UI: " + name
	return ""
