extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var server = OS.get_environment("GODOT_TEST_SERVER")
	if server.is_empty():
		push_error("Set GODOT_TEST_SERVER to the existing local test server")
		quit(1)
		return
	var client = ClassDB.instantiate("GameClient")
	root.add_child(client)
	if not client.has_method("connect_account"):
		push_error("Native client must route account requests through the transport")
		quit(1)
		return
	var error: String = client.connect_account(server, "godot-parity-nonexistent-account", "invalid-test-password", false)
	if not error.is_empty():
		push_error(error)
		quit(1)
		return
	var deadline = Time.get_ticks_msec() + 10000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.get("reply_received", false):
			if state.screen != "Login" or state.status.is_empty() or state.character_count != 0:
				push_error("Rejected login must remain unauthenticated and expose server feedback")
				quit(1)
				return
			client.queue_free()
			print("PASS: actual server login rejection reaches native account state")
			quit(0)
			return
	push_error("No actual account response from test server")
	quit(1)
