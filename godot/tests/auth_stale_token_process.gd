extends SceneTree

const EXPIRED_MESSAGE = "Saved session expired. Please log in again."

var client: Node

func _initialize() -> void:
	call_deferred("run_fixture")

func run_fixture() -> void:
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var phase := OS.get_environment("AUTH_STALE_PHASE")
	var account := OS.get_environment("AUTH_STALE_ACCOUNT")
	if server.is_empty() or account.is_empty() or not ["seed", "stale"].has(phase):
		fail("Set GODOT_TEST_SERVER, AUTH_STALE_ACCOUNT, and AUTH_STALE_PHASE=seed|stale")
		return

	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	client.set_server(server)
	var deadline := Time.get_ticks_msec() + 120000
	while client.find_child("LoginStatus", true, false) == null and Time.get_ticks_msec() < deadline:
		await process_frame
	if not require(client.find_child("LoginStatus", true, false) != null, "Login form did not initialize"):
		return

	if phase == "seed":
		await seed_saved_session(server, account)
	else:
		await reject_stale_session_then_log_in(server, account)

func seed_saved_session(server: String, account: String) -> void:
	var error: String = client.connect_account(server, account, "fbtest", false)
	if not require(error.is_empty(), "Password login could not start: " + error):
		return
	var deadline := Time.get_ticks_msec() + 30000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "CharacterSelect" and state.reply_received:
			var token_path := saved_token_path(server)
			if not require(FileAccess.file_exists(token_path), "Accepted login did not persist its token"):
				return
			print("PASS AUTH_STALE_TOKEN_SEED reply_ms=%d" % (Time.get_ticks_msec() - (deadline - 30000)))
			quit(0)
			return
	fail("Password login did not reach character select: " + str(client.account_state()))

func reject_stale_session_then_log_in(server: String, account: String) -> void:
	var token_path := saved_token_path(server)
	if not require(FileAccess.file_exists(token_path), "Seeded saved token is missing"):
		return
	var started := Time.get_ticks_msec()
	var error: String = client.connect_account(server, "", "", false)
	if not require(error.is_empty(), "Saved-token login could not start: " + error):
		return
	var status := client.find_child("LoginStatus", true, false) as Label
	var login_root := client.find_child("LoginRoot", true, false) as Control
	var deadline := started + 1000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "Login" and state.status == EXPIRED_MESSAGE:
			break
	var rejected: Dictionary = client.account_state()
	var reject_ms := Time.get_ticks_msec() - started
	if not require(reject_ms < 1000, "Stale-token response exceeded 1 s: %d ms" % reject_ms):
		return
	if not require(rejected.reply_received and rejected.screen == "Login", "Reject did not return to login form"):
		return
	if not require(status.is_visible_in_tree() and status.text == EXPIRED_MESSAGE, "Login form did not show the stale-session message: " + status.text):
		return
	if not require(login_root != null and login_root.is_visible_in_tree(), "Login form is not visible after stale-token rejection"):
		return
	if not require(not FileAccess.file_exists(token_path), "Rejected token remains saved on disk"):
		return

	var username := client.find_child("UsernameInput", true, false) as LineEdit
	var password := client.find_child("PasswordInput", true, false) as LineEdit
	var button := client.find_child("ConnectButton", true, false) as Button
	if not require(username != null and password != null and button != null, "Login controls are missing"):
		return
	username.text = account
	username.text_changed.emit(username.text)
	password.text = "fbtest"
	password.text_changed.emit(password.text)
	button.pressed.emit()
	var login_deadline := Time.get_ticks_msec() + 30000
	while Time.get_ticks_msec() < login_deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect":
			var elapsed := Time.get_ticks_msec() - started
			if not require(FileAccess.file_exists(token_path), "Valid login did not save a replacement token"):
				return
			print("PASS AUTH_STALE_TOKEN_PROCESS reject_ms=%d total_ms=%d screen=%s" % [reject_ms, elapsed, state.screen])
			quit(0)
			return
	fail("Valid password login after stale-token reject failed: " + str(client.account_state()))

func saved_token_path(server: String) -> String:
	var endpoint := server.replace(":", "_").replace("/", "_")
	return ProjectSettings.globalize_path("res://../data/auth_token." + endpoint)

func require(condition: bool, message: String) -> bool:
	if condition:
		return true
	fail(message)
	return false

func fail(message: String) -> void:
	push_error("AUTH_STALE_TOKEN: " + message)
	quit(1)
