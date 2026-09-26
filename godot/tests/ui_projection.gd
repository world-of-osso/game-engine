extends SceneTree

var failures := 0

func _initialize() -> void:
	call_deferred("run_test")

func require(condition: bool, message: String) -> void:
	if not condition:
		push_error(message)
		failures += 1

func run_test() -> void:
	require(ClassDB.class_exists("RegistryUi"), "native registry UI host missing")
	if failures > 0:
		quit(1)
		return
	var host = ClassDB.instantiate("RegistryUi")
	root.add_child(host)
	var error = host.show_login()
	require(error == "", "login projection: " + error)
	if failures > 0:
		quit(1)
		return
	var form = host.find_child("LoginInputContainer", true, false)
	var username = host.find_child("UsernameInput", true, false)
	var password = host.find_child("PasswordInput", true, false)
	var connect = host.find_child("ConnectButton", true, false)
	require(form is Control and username is LineEdit and password is LineEdit and connect is Button, "login native controls missing")
	if failures > 0:
		quit(1)
		return
	require(form.position.is_equal_approx(Vector2(480, 193)), "authored centered form bounds: " + str(form.position))
	require(form.size.is_equal_approx(Vector2(320, 200)), "authored form size")
	require(username.position.is_equal_approx(Vector2.ZERO) and username.size.is_equal_approx(Vector2(320, 42)), "username bounds")
	require(password.position.is_equal_approx(Vector2(0, 72)) and password.secret, "password geometry and masking")
	require(connect.position.is_equal_approx(Vector2(35, 134)) and connect.text == "Login", "authored login button: " + str(connect.position) + " text=" + connect.text)
	var background = host.find_child("LoginBackground", true, false)
	var logo = host.find_child("LoginGameLogo", true, false)
	var border = username.get_node_or_null("NinePart0")
	require(background is TextureRect and background.texture != null and background.texture.get_width() >= 1280, "authored login backdrop texture missing")
	require(logo is TextureRect and logo.texture != null and logo.texture.get_width() > 100, "authored logo texture missing")
	require(logo.size.is_equal_approx(Vector2(384, 256)), "authored logo bounds: " + str(logo.size))
	require(border is TextureRect and border.texture != null, "authored input border missing")
	username.grab_focus()
	var key := InputEventKey.new()
	key.keycode = KEY_A
	key.unicode = 97
	key.pressed = true
	Input.parse_input_event(key)
	await process_frame
	require(username.has_focus() and username.text == "a", "native keyboard event and focus: " + username.text)
	username.insert_text_at_caret("dminé")
	username.text_changed.emit(username.text)
	require(username.text == "adminé", "native unicode text insertion")
	host.sync_input()
	require(host.frame_text("UsernameInput") == "adminé", "native edits must update registry")
	connect.pressed.emit()
	require(host.pop_action() == "connect", "named-frame button action")
	host.set_status("Retry")
	require(host.find_child("LoginStatus", true, false).text == "Retry", "registry update must project")
	host.remove_login()
	await process_frame
	require(host.find_child("UsernameInput", true, false) == null, "removed registry subtree must disappear")
	host.queue_free()
	if failures > 0:
		quit(1)
	else:
		print("PASS: login native layout, editing, focus, actions, updates, removal")
		quit(0)
