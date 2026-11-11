extends SceneTree

# Process acceptance: set GAME_ENGINE_ACCOUNT_TOKEN privately and --server after --.
# No connect_account call or credentials: only production startup may authenticate.
func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var expected := OS.get_environment("HANDOFF_EXPECTED_SCREEN")
	if expected not in ["CharacterSelect", "Login"]:
		fail("HANDOFF_EXPECTED_SCREEN must be CharacterSelect or Login")
		return
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if not state.reply_received:
			continue
		if state.screen != expected:
			fail("Handoff reached the wrong account screen")
			return
		var login = client.get_node_or_null("LoginUI")
		if expected == "CharacterSelect":
			if client.get_node_or_null("CharacterSelectUI") == null or (login != null and login.visible):
				fail("Authenticated handoff did not replace the login form")
				return
		else:
			if login == null or not login.visible or str(state.status).is_empty():
				fail("Rejected handoff did not show the login form with feedback")
				return
		print("PASS launcher handoff: ", expected, "; no client credentials; roster=", state.character_count)
		client.free()
		quit(0)
		return
	fail("Handoff did not receive an authentication response")

func fail(message: String) -> void:
	printerr(message)
	quit(1)
