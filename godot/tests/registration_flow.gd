extends SceneTree

var client: Node
var output: String

func _initialize() -> void:
	call_deferred("run")

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func click(name: String) -> void:
	var control := client.find_child(name, true, false) as Control
	assert(control != null and control.is_visible_in_tree(), "Missing visible control: " + name)
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

func type_text(value: String) -> void:
	for character in value:
		var event := InputEventKey.new()
		event.unicode = character.unicode_at(0)
		event.pressed = true
		root.push_input(event)
		await process_frame
		event.pressed = false
		root.push_input(event)

func capture(name: String) -> void:
	await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	assert(image != null and not image.is_empty(), "Rendered viewport required")
	assert(image.save_png(output.path_join(name + ".png")) == OK)
	var file := FileAccess.open(output.path_join(name + ".json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(client.account_state()))

func run() -> void:
	output = OS.get_environment("REGISTRATION_ARTIFACTS")
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("GODOT_TEST_ACCOUNT")
	if output.is_empty() or server != "127.0.0.1:5300" or account != "fb_regtest1":
		fail("Private registration fixture requires owned artifacts and fb_regtest1 on UDP 5300")
		return
	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	client.set_server(server)
	var deadline := Time.get_ticks_msec() + 150000
	while client.find_child("CreateAccountButton", true, false) == null and Time.get_ticks_msec() < deadline:
		await process_frame
	await create_timer(1.0).timeout
	await click("CreateAccountButton")
	var submit := client.find_child("ConnectButton", true, false) as Button
	assert(submit.text == "Register")
	await click("ConnectButton")
	var status := client.find_child("LoginStatus", true, false) as Label
	assert(status.text == "Please fill in all fields")
	await click("UsernameInput")
	await type_text(account)
	await click("PasswordInput")
	await type_text(OS.get_environment("GODOT_TEST_PASSWORD"))
	await click("ConnectButton")
	deadline = Time.get_ticks_msec() + 20000
	while not client.account_state().reply_received and Time.get_ticks_msec() < deadline:
		await process_frame
	assert(client.account_state().reply_received, "Registration reply timed out")
	assert(client.account_state().screen == "Login")
	assert("approval" in status.text.to_lower(), "Pending approval feedback missing")
	assert(not submit.disabled)
	await capture("pending")
	print("PASS: UI validation, real-input registration, private server pending approval")
	FileAccess.open(output.path_join("pending-ready"), FileAccess.WRITE).store_string("pending")
	deadline = Time.get_ticks_msec() + 180000
	while not FileAccess.file_exists(output.path_join("approved")) and Time.get_ticks_msec() < deadline:
		await process_frame
	assert(FileAccess.file_exists(output.path_join("approved")), "Private approval marker timed out")
	await click("CreateAccountButton")
	assert(submit.text == "Login")
	await click("ConnectButton")
	deadline = Time.get_ticks_msec() + 20000
	while client.account_state().screen != "CharacterSelect" and Time.get_ticks_msec() < deadline:
		await process_frame
	assert(client.account_state().screen == "CharacterSelect", "Approved account login did not reach character select")
	assert(client.get_node("CharacterSelectUI").visible)
	assert(not client.get_node("LoginUI").visible)
	await capture("character-select")
	print("PASS: private approval followed by real-input password login and rendered character select")
	FileAccess.open(output.path_join("complete"), FileAccess.WRITE).store_string("pass")
	client.queue_free()
	await process_frame
	quit(0)
