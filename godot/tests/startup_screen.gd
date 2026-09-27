extends SceneTree

var client: Node

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var expected := OS.get_environment("GODOT_EXPECT_SCREEN")
	if expected not in ["Login", "Loading", "CharacterCreate"]:
		fail("Startup fixture requires an explicit offline screen expectation")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen != expected:
			continue
		var error := inspect_screen(expected, state)
		if error != "":
			fail(error)
			return
		for frame in 4:
			await RenderingServer.frame_post_draw
		var screenshot := OS.get_environment("GODOT_STARTUP_SCREENSHOT")
		if screenshot.is_empty():
			fail("Startup fixture requires an owned screenshot path")
			return
		var saved := root.get_texture().get_image().save_png(screenshot)
		if saved != OK:
			fail("Save startup screenshot: " + error_string(saved))
			return
		print("PASS: CLI startup ", expected, " customize=", OS.get_environment("GODOT_EXPECT_CUSTOMIZE"), " screenshot=", screenshot)
		client.free()
		quit(0)
		return
	fail("CLI did not reach " + expected + ": " + str(client.account_state()))

func inspect_screen(expected: String, state: Dictionary) -> String:
	if state.reply_received or state.character_count != 0 or state.unit_count != 0:
		return "Offline screen startup unexpectedly authenticated or fabricated world state"
	var ui_name := "LoginUI" if expected == "Login" else "LoadingUI"
	if expected == "CharacterCreate":
		ui_name = "CharacterCreateUI"
	var ui := client.get_node_or_null(ui_name) as Control
	if ui == null or not ui.is_visible_in_tree():
		return "Requested startup UI is absent or hidden: " + ui_name
	if expected == "CharacterCreate":
		var name_input := ui.find_child("CharCreateNameInput", true, false) as Control
		var customize := OS.get_environment("GODOT_EXPECT_CUSTOMIZE") == "1"
		var name_shown := name_input != null and name_input.is_visible_in_tree()
		if name_shown != customize:
			return "Creation startup selected the wrong Select/Customize mode"
	return ""

func fail(message: String) -> void:
	push_error(message)
	if is_instance_valid(client):
		client.free()
	quit(1)
