extends "res://tests/world_menu_flow.gd"

const LOGOUT_WAIT_MS := 25000

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
	var world := client.get_node_or_null("WorldUnits")
	var camera := client.get_node_or_null("WorldCamera")
	if world == null or camera == null:
		fail("Native world and camera missing")
		return

	print("FIXTURE LOGOUT_COMBAT_TRUE")
	await wait_frames(70)
	if not await open_logout_menu(client):
		return
	await click_menu_action(client, "MenuBtnLogout")
	await wait_frames(8)
	if not menu_authored(client) or countdown(client) != "" or client.account_state().screen != "InWorld":
		fail("Combat logout did not leave menu open without countdown")
		return
	print("FIXTURE LOGOUT_COMBAT_BLOCKED")
	print("FIXTURE LOGOUT_COMBAT_FALSE")
	await wait_frames(70)
	await click_menu_action(client, "MenuBtnResume")
	if not await wait_menu_closed(client, null):
		return

	print("FIXTURE LOGOUT_REST_TRUE")
	await wait_frames(40)
	print("FIXTURE LOGOUT_REST_NONE")
	await wait_frames(40)
	if not await open_logout_menu(client):
		return
	await click_menu_action(client, "MenuBtnLogout")
	if not await wait_countdown(client):
		return
	if not await wait_menu_closed(client, null):
		return
	if not countdown_layout(client):
		fail("Countdown layout/text did not match authored original")
		return
	if OS.get_environment("GODOT_TEST_VISUAL") == "1":
		await RenderingServer.frame_post_draw
		var path := ProjectSettings.globalize_path("res://../data/diagnostics/godot-conversion/logout-countdown.png")
		if root.get_texture().get_image().save_png(path) != OK:
			fail("Could not capture logout countdown")
			return
	print("FIXTURE LOGOUT_COUNTDOWN")
	push_key(KEY_W, true)
	await wait_frames(12)
	push_key(KEY_W, false)
	if not await wait_countdown_hidden(client):
		return
	print("FIXTURE LOGOUT_CANCELLED")
	await wait_frames(20)
	if not await open_logout_menu(client):
		return
	await click_menu_action(client, "MenuBtnLogout")
	if not await wait_countdown(client):
		return
	var original := countdown_seconds(client)
	await wait_frames(150)
	if not await open_logout_menu(client):
		return
	await click_menu_action(client, "MenuBtnLogout")
	await wait_frames(5)
	if countdown_seconds(client) >= original or countdown_seconds(client) <= 0:
		fail("Repeated logout reset or lost original countdown: %s -> %s" % [original, countdown(client)])
		return
	print("FIXTURE LOGOUT_REPEATED")
	if not await wait_screen(client, "CharacterSelect", LOGOUT_WAIT_MS):
		return
	if client.account_state().world_attached or countdown(client) != "":
		fail("Countdown logout retained world or left overlay active")
		return
	print("FIXTURE LOGOUT_EXPIRED")
	var ui := client.get_node_or_null("CharacterSelectUI")
	# This fixture's roster puts the unequipped character first; select Input Fixture.
	var card = ui.find_child("CharCard_1", true, false) if ui != null else null
	if not card is Control:
		fail("Relogin roster missing Input Fixture card")
		return
	await click(card)
	var enter = ui.find_child("EnterWorld", true, false) if ui != null else null
	if not enter is Button:
		fail("Relogin roster missing Enter World")
		return
	await click(enter)
	if not await wait_screen(client, "Loading", 15000):
		return
	print("FIXTURE LOGOUT_RELOADING")
	if not await wait_world(client):
		return
	print("FIXTURE LOGOUT_REST_TRUE_FINAL")
	await wait_frames(50)
	if not await open_logout_menu(client):
		return
	await click_menu_action(client, "MenuBtnLogout")
	if not await wait_screen(client, "CharacterSelect", MENU_WAIT_MS) or countdown(client) != "" or client.account_state().world_attached:
		fail("Rest-area logout did not return immediately to character select")
		return
	print("FIXTURE LOGOUT_DONE")
	quit(0)

func open_logout_menu(client: Node) -> bool:
	push_key(KEY_ESCAPE, true)
	await process_frame
	push_key(KEY_ESCAPE, false)
	if not await wait_menu(client) or not menu_authored(client):
		fail("InWorld Escape did not open authored game menu")
		return false
	return true

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func countdown(client: Node) -> String:
	var overlay := client.get_node_or_null("LogoutOverlay") as CanvasLayer
	if overlay == null or not overlay.visible:
		return ""
	var label := overlay.get_node_or_null("Panel/Countdown") as Label
	return label.text if label != null else ""

func countdown_seconds(client: Node) -> int:
	var text := countdown(client)
	return int(text.trim_prefix("Logging out in ").get_slice("s", 0)) if text.begins_with("Logging out in ") else -1

func wait_countdown(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if countdown_seconds(client) > 0:
			return true
	fail("Authored logout countdown missing")
	return false

func wait_countdown_hidden(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if countdown(client) == "":
			return true
	fail("Movement did not cancel logout countdown")
	return false

func countdown_layout(client: Node) -> bool:
	var overlay := client.get_node_or_null("LogoutOverlay") as CanvasLayer
	var panel := overlay.get_node_or_null("Panel") as Control
	var label := overlay.get_node_or_null("Panel/Countdown") as Label
	return panel != null and label != null and absf(panel.size.x - 340.0) < 1.0 and absf(panel.position.x - 470.0) < 1.0 and absf(panel.position.y + panel.size.y - 600.0) < 1.0 and label.get_theme_font_size("font_size") == 24 and label.get_theme_color("font_color") == Color(1, 0.82, 0.52) and countdown(client).ends_with("s\nMove to cancel")
