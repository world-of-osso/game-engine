extends SceneTree

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server = OS.get_environment("GODOT_TEST_SERVER")
	if server.is_empty():
		fail("GODOT_TEST_SERVER must select a local server")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, "admin", "admin", false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	var deadline = Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if not state.reply_received:
			continue
		if state.screen != "CharacterSelect" or state.character_count < 1:
			fail("Fixture needs successful auth and at least one character: " + str(state))
			return
		var ui = client.get_node_or_null("CharacterSelectUI")
		if ui == null:
			fail("Authenticated character selection UI missing")
			return
		var selected = ui.find_child("CharCard_0Selected", true, false)
		if not selected is TextureRect or selected.texture == null:
			fail("Authored selected card texture missing")
			return
		var tint := Color(0.82, 0.74, 0.46, 0.9)
		if not selected.self_modulate.is_equal_approx(tint):
			fail("Selected card lost authored vertex tint: " + str(selected.self_modulate))
			return
		if not selected.modulate.is_equal_approx(Color.WHITE):
			fail("Vertex tint must not also multiply frame alpha: " + str(selected.modulate))
			return
		var parent = selected.get_parent()
		if not parent.modulate.is_equal_approx(Color.WHITE):
			fail("Selected card tint leaked into parent: " + str(parent.modulate))
			return
		client.free()
		print("PASS: selected character card preserves authored vertex tint once on TextureRect")
		quit(0)
		return
	fail("Timed out waiting for selected character card")

func fail(message: String) -> void:
	push_error(message)
	quit(1)
