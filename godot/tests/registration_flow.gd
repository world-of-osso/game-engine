extends SceneTree

var completion = preload("res://tests/completion.gd").new(self, 2)

var client: Node
var output: String

func _initialize() -> void:
	call_deferred("run_completion")

func fail(message: String) -> void:
	push_error(message)
	completion.fail("registration_flow: fail failed")

func click(name: String) -> bool:
	var control := client.find_child(name, true, false) as Control
	if not completion.require(control != null and control.is_visible_in_tree(), "Missing visible control: " + name):
		return false
	var position := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = position
	root.push_input(motion)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = position
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event)
		await process_frame

	return true

func type_text(value: String) -> bool:
	for character in value:
		var event := InputEventKey.new()
		event.unicode = character.unicode_at(0)
		event.pressed = true
		root.push_input(event)
		await process_frame
		event.pressed = false
		root.push_input(event)

	return true

func wait_for_text(control: Control, expected: String) -> bool:
	# Authored buttons render their text in a child Label, not Button.text.
	var deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		var label := control as Label
		if control is Button:
			label = control.find_child("Text", true, false) as Label
		if label != null and label.text == expected:
			return true
		await process_frame
	if not completion.require(false, "Rendered text did not reach: " + expected):
		return false

	return true

func capture(name: String) -> bool:
	await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if not completion.require(image != null and not image.is_empty(), "Rendered viewport required"):
		return false
	if not completion.require(image.save_png(output.path_join(name + ".png")) == OK):
		return false
	var file := FileAccess.open(output.path_join(name + ".json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(client.account_state()))

	return true

func run() -> bool:
	output = OS.get_environment("REGISTRATION_ARTIFACTS")
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("GODOT_TEST_ACCOUNT")
	var port := OS.get_environment("REGISTRATION_FLOW_PORT")
	if port.is_empty():
		port = "5300"
	if not port.is_valid_int() or int(port) < 1 or int(port) > 65535:
		fail("REGISTRATION_FLOW_PORT must be an integer in 1..65535")
		return false
	if output.is_empty() or server != "127.0.0.1:" + str(int(port)) or account != "fb_regtest1":
		fail("Private registration fixture requires owned artifacts and fb_regtest1 on loopback UDP " + str(int(port)))
		return false
	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	client.set_server(server)
	var deadline := Time.get_ticks_msec() + 150000
	while client.find_child("CreateAccountButton", true, false) == null and Time.get_ticks_msec() < deadline:
		await process_frame
	await create_timer(1.0).timeout
	if not completion.step(await click("CreateAccountButton"), "click"):
		return false
	var submit := client.find_child("ConnectButton", true, false) as Button
	if not completion.step(await wait_for_text(submit, "Register"), "wait_for_text"):
		return false
	if not completion.step(await click("ConnectButton"), "click"):
		return false
	var status := client.find_child("LoginStatus", true, false) as Label
	if not completion.step(await wait_for_text(status, "Please fill in all fields"), "wait_for_text"):
		return false
	if not completion.require(not client.account_state().connected and not client.account_state().reply_received, "Invalid form started account transport"):
		return false
	if not completion.step(await click("UsernameInput"), "click"):
		return false
	if not completion.step(await type_text(account), "type_text"):
		return false
	if not completion.step(await click("PasswordInput"), "click"):
		return false
	if not completion.step(await type_text(OS.get_environment("GODOT_TEST_PASSWORD")), "type_text"):
		return false
	if not completion.step(await click("ConnectButton"), "click"):
		return false
	deadline = Time.get_ticks_msec() + 20000
	while not client.account_state().reply_received and Time.get_ticks_msec() < deadline:
		await process_frame
	if not completion.require(client.account_state().reply_received, "Registration reply timed out"):
		return false
	if not completion.require(client.account_state().screen == "Login"):
		return false
	if not completion.require("approval" in status.text.to_lower(), "Pending approval feedback missing"):
		return false
	if not completion.require(not submit.disabled):
		return false
	if not completion.step(await capture("pending"), "capture"):
		return false
	if not completion.case_done():
		return false
	print("PASS: UI validation, real-input registration, private server pending approval")
	FileAccess.open(output.path_join("pending-ready"), FileAccess.WRITE).store_string("pending")
	deadline = Time.get_ticks_msec() + 180000
	while not FileAccess.file_exists(output.path_join("approved")) and Time.get_ticks_msec() < deadline:
		await process_frame
	if not completion.require(FileAccess.file_exists(output.path_join("approved")), "Private approval marker timed out"):
		return false
	if not completion.step(await click("CreateAccountButton"), "click"):
		return false
	if not completion.step(await wait_for_text(submit, "Login"), "wait_for_text"):
		return false
	if not completion.step(await click("ConnectButton"), "click"):
		return false
	deadline = Time.get_ticks_msec() + 20000
	while client.account_state().screen != "CharacterSelect" and Time.get_ticks_msec() < deadline:
		await process_frame
	if not completion.require(client.account_state().screen == "CharacterSelect", "Approved account login did not reach character select"):
		return false
	if not completion.require(client.get_node("CharacterSelectUI").visible):
		return false
	if not completion.require(not client.get_node("LoginUI").visible):
		return false
	if not completion.step(await capture("character-select"), "capture"):
		return false
	if not completion.case_done():
		return false
	print("PASS: private approval followed by real-input password login and rendered character select")
	FileAccess.open(output.path_join("complete"), FileAccess.WRITE).store_string("pass")
	client.queue_free()
	await process_frame
	return true

func run_completion() -> void:
	completion.finish(await run())
