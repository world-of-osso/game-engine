extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	root.size = Vector2i(1280, 720)
	var server = OS.get_environment("GODOT_TEST_SERVER")
	if server.is_empty():
		push_error("Set GODOT_TEST_SERVER to the local test server")
		quit(1)
		return
	var client = ClassDB.instantiate("GameClient")
	root.add_child(client)
	if not client.has_method("set_server"):
		push_error("Native login host must accept the selected server")
		quit(1)
		return
	client.set_server(server)
	var username: LineEdit = client.find_child("UsernameInput", true, false)
	var password: LineEdit = client.find_child("PasswordInput", true, false)
	username.text = "admin"
	username.text_changed.emit(username.text)
	password.text = "invalid-test-password"
	password.text_changed.emit(password.text)
	var button: Button = client.find_child("ConnectButton", true, false)
	var ui = client.get_node("LoginUI")
	if ui.set_connecting(true) != "" or not button.disabled:
		push_error("Pending login must disable the authored connect button")
		quit(1)
		return
	if ui.set_connecting(false) != "" or button.disabled:
		push_error("Finished login must restore the authored connect button")
		quit(1)
		return
	button.pressed.emit()
	var deadline = Time.get_ticks_msec() + 10000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received:
			var status: Label = client.find_child("LoginStatus", true, false)
			if state.screen != "Login" or status.text.is_empty() or status.text != state.status:
				push_error("Login button must project the real server rejection into authored status")
				quit(1)
				return
			client.queue_free()
			print("PASS: authored login button routes credentials and projects actual server rejection")
			quit(0)
			return
	push_error("Authored login button did not receive a server reply")
	quit(1)
