extends SceneTree

# Live: log in through the authored login UI (protocol check must pass: the host reports
# Connected only after ProtocolVerified), signal ready, then the harness kicks the account.
# Env: GODOT_TEST_SERVER, KICK_READY_FILE, KICK_DONE_FILE, KICK_REASON.

func _initialize() -> void:
	call_deferred("_run")

func _fail(message: String) -> void:
	push_error(message)
	quit(1)

func _run() -> void:
	root.size = Vector2i(1280, 720)
	var client = ClassDB.instantiate("GameClient")
	root.add_child(client)
	client.set_server(OS.get_environment("GODOT_TEST_SERVER"))
	var username: LineEdit = client.find_child("UsernameInput", true, false)
	var password: LineEdit = client.find_child("PasswordInput", true, false)
	username.text = "fb_kick"
	username.text_changed.emit(username.text)
	password.text = "fbtest"
	password.text_changed.emit(password.text)
	client.find_child("ConnectButton", true, false).pressed.emit()
	var deadline = Time.get_ticks_msec() + 30000
	while client.account_state().screen != "CharacterSelect":
		if Time.get_ticks_msec() > deadline:
			_fail("never reached CharacterSelect: %s" % client.account_state())
			return
		await process_frame
	print("LOGGED_IN %s" % client.account_state().screen)
	FileAccess.open(OS.get_environment("KICK_READY_FILE"), FileAccess.WRITE).store_string("ready")
	deadline = Time.get_ticks_msec() + 30000
	while not FileAccess.file_exists(OS.get_environment("KICK_DONE_FILE")):
		if Time.get_ticks_msec() > deadline:
			_fail("harness never kicked")
			return
		await process_frame
	var kicked_at = Time.get_ticks_msec()
	var expected = "You were kicked: %s" % OS.get_environment("KICK_REASON")
	while Time.get_ticks_msec() - kicked_at < 5000:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "Login" and state.status == expected:
			var status: Label = client.find_child("LoginStatus", true, false)
			print("PASS: kicked to Login in %d ms, status label: %s" % [Time.get_ticks_msec() - kicked_at, status.text if status else "<no label>"])
			quit(0)
			return
	_fail("not kicked within 5 s: %s" % client.account_state())
