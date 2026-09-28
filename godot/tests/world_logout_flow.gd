extends "res://tests/world_menu_flow.gd"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":0"):
		fail("Logout fixture requires its owned loopback endpoint")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 15000):
		return
	print("FIXTURE LOGOUT_LOADING")
	if not await wait_world(client):
		return
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client):
		return
	await click_menu_action(client, "MenuBtnLogout")
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline and client.get_node_or_null("LogoutOverlay") == null:
		await process_frame
	var overlay := client.get_node_or_null("LogoutOverlay")
	if overlay == null:
		fail("Authored Logout did not show countdown overlay")
		return
	if not await wait_menu_closed(client, null):
		return
	if not await wait_screen(client, "Login", 25000):
		return
	print("FIXTURE LOGOUT_DONE")
	quit(0)
