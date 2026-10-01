extends "res://tests/world_menu_flow.gd"

# The original first-login legal screen (`src/scenes/eula/mod.rs`) through real startup
# and viewport mouse input, under an isolated XDG_CONFIG_HOME:
#   GODOT_EULA_ACTION=accept  — the screen covers a hidden Login; a click on Accept writes
#                               `accepted_eula: true` to the canonical options file and
#                               reveals Login.
#   GODOT_EULA_ACTION=decline — a click on Decline quits the client without accepting.
#   GODOT_EULA_ACTION=absent  — startup goes straight to Login (gate satisfied or disabled).
# GODOT_EULA_SCREENSHOT saves the shown screen. Run with `-- --screen eula`, or with
# ENABLE_EULA=1 and no screen for the startup gate.
const EULA_WAIT_MS := 15000
const DECLINE_WAIT_MS := 3000

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var action := OS.get_environment("GODOT_EULA_ACTION")
	if action not in ["accept", "decline", "absent"]:
		fail("EULA fixture requires GODOT_EULA_ACTION=accept|decline|absent")
		return
	var config_dir := OS.get_environment("XDG_CONFIG_HOME")
	if config_dir.is_empty():
		fail("EULA fixture requires isolated XDG_CONFIG_HOME")
		return
	var options_path := config_dir.path_join("world-of-osso/options_settings.ron")
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if action == "absent":
		await expect_login_without_eula(client)
		return
	if accepted(options_path):
		fail("EULA fixture must start without accepted_eula: true")
		return
	var accept := await wait_eula(client)
	if accept == null:
		return
	if not await capture():
		return
	if action == "decline":
		var decline := eula_control(client, "EulaDeclineButton")
		if decline == null or not decline.is_visible_in_tree():
			fail("EULA Decline button missing")
			return
		await click(decline)
		print("FIXTURE EULA_DECLINE_CLICKED")
		var deadline := Time.get_ticks_msec() + DECLINE_WAIT_MS
		while Time.get_ticks_msec() < deadline:
			await process_frame
		fail("Decline did not quit the client")
		return
	await click(accept)
	for frame in range(5):
		await process_frame
	if client.get_node_or_null("Eula") != null:
		fail("Accept left the EULA screen open")
		return
	if not login_visible(client) or client.account_state().screen != "Login":
		fail("Accept did not proceed to Login: " + str(client.account_state()))
		return
	if not accepted(options_path):
		fail("Accept did not persist accepted_eula: true to " + options_path)
		return
	print("PASS: EULA accept persisted and proceeded to Login")
	quit(0)

func wait_eula(client: Node) -> Control:
	var deadline := Time.get_ticks_msec() + EULA_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var accept := eula_control(client, "EulaAcceptButton")
		if accept != null and accept.is_visible_in_tree():
			if login_visible(client):
				fail("Login UI is visible over the EULA screen")
				return null
			if client.account_state().reply_received:
				fail("EULA screen contacted a server")
				return null
			return accept
	fail("Startup did not show the EULA screen")
	return null

func expect_login_without_eula(client: Node) -> void:
	var deadline := Time.get_ticks_msec() + 2000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.get_node_or_null("Eula") != null:
			fail("EULA screen shown after acceptance")
			return
	if not login_visible(client):
		fail("Startup did not show Login")
		return
	print("PASS: startup skipped the accepted EULA")
	quit(0)

func eula_control(client: Node, name: String) -> Control:
	var eula := client.get_node_or_null("Eula")
	return eula.find_child(name, true, false) as Control if eula != null else null

func login_visible(client: Node) -> bool:
	var login := client.get_node_or_null("LoginUI") as CanvasLayer
	return login != null and login.visible

func accepted(path: String) -> bool:
	return FileAccess.file_exists(path) and FileAccess.get_file_as_string(path).contains("accepted_eula: true")

func capture() -> bool:
	var path := OS.get_environment("GODOT_EULA_SCREENSHOT")
	if path.is_empty():
		return true
	for frame in range(10):
		await process_frame
	await RenderingServer.frame_post_draw
	var saved := root.get_texture().get_image().save_png(path)
	if saved != OK:
		fail("Save EULA screenshot: " + error_string(saved))
		return false
	print("FIXTURE EULA_CAPTURED ", path)
	return true
