extends SceneTree

# The server dies while the login screen shows "Connecting...". The harness stops the server
# process (SIGSTOP) before this script runs so the connect stays pending; this script kills it.
# Env: GODOT_TEST_SERVER (host:port), GODOT_TEST_SERVER_PID (the stopped server process).

func _initialize() -> void:
	call_deferred("_run")

func _fail(message: String) -> void:
	push_error(message)
	quit(1)

func _run() -> void:
	root.size = Vector2i(1280, 720)
	var server = OS.get_environment("GODOT_TEST_SERVER")
	var server_pid = OS.get_environment("GODOT_TEST_SERVER_PID").to_int()
	if server.is_empty() or server_pid <= 0:
		_fail("Set GODOT_TEST_SERVER and GODOT_TEST_SERVER_PID to the stopped test server")
		return
	var client = ClassDB.instantiate("GameClient")
	root.add_child(client)
	client.set_server(server)
	var username: LineEdit = client.find_child("UsernameInput", true, false)
	var password: LineEdit = client.find_child("PasswordInput", true, false)
	username.text = "fb_loss"
	username.text_changed.emit(username.text)
	password.text = "fbtest"
	password.text_changed.emit(password.text)
	var button: Button = client.find_child("ConnectButton", true, false)
	var status: Label = client.find_child("LoginStatus", true, false)
	button.pressed.emit()
	await process_frame
	await process_frame
	if status.text != "Connecting..." or not button.disabled:
		_fail("Pressing connect must show Connecting... (status %s)" % status.text)
		return
	OS.kill(server_pid)
	# The dead server never answers; the transport reports the loss at the 60 s netcode
	# timeout (network/src/lib.rs `client_timeout_secs`).
	var deadline = Time.get_ticks_msec() + 75000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if status.text == "Connecting...":
			continue
		var state: Dictionary = client.account_state()
		if state.screen != "Login" or state.status.is_empty() or status.text != state.status:
			_fail("Login status must show the loss reason, got %s (session %s)" % [status.text, state.status])
			return
		if button.disabled:
			_fail("Connect must be enabled again after the loss")
			return
		client.queue_free()
		print("PASS: server loss while connecting replaces Connecting... with: %s" % status.text)
		quit(0)
		return
	var final_state: Dictionary = client.account_state()
	_fail("Login status still shows Connecting... 75 s after the server died (session status %s)" % final_state.status)
