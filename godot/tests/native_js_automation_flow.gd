extends SceneTree

# Observation only. Production startup reads debug/login.js and owns every input/action.
# No credential setters, emitted input signals, connect_account, or automation hooks.
var client: Node
var artifacts: String
var username: LineEdit
var password: LineEdit
var connect_button: Button
var saw_credentials := false
var saw_authored_click := false

func _initialize() -> void:
	call_deferred("run")

func mark(name: String) -> bool:
	var file := FileAccess.open(artifacts.path_join(name), FileAccess.WRITE)
	if file == null:
		push_error("SETUP: cannot write observation marker " + name)
		quit(1)
		return false
	file.store_string("observed\n")
	file.close()
	return true

func fail(message: String, feature: bool = false) -> void:
	if feature:
		mark("feature-failure")
	# Never print account feedback, control contents, or credential values.
	push_error(message)
	if client != null and is_instance_valid(client):
		client.queue_free()
	quit(1)

func observe_credentials(_text: String = "") -> void:
	if username == null or password == null:
		return
	if username.text == OS.get_environment("LOGIN_USER") and password.text == OS.get_environment("LOGIN_PASS"):
		saw_credentials = true

func observe_authored_click() -> void:
	observe_credentials()
	saw_authored_click = saw_authored_click or saw_credentials

func observe_node(node: Node) -> void:
	# Install observers during mounting, before the startup consumer can use controls.
	if node.name == "UsernameInput" and node is LineEdit:
		username = node
		username.text_changed.connect(observe_credentials)
	elif node.name == "PasswordInput" and node is LineEdit:
		password = node
		password.text_changed.connect(observe_credentials)
	elif node.name == "ConnectButton" and node is Button:
		connect_button = node
		connect_button.pressed.connect(observe_authored_click)

func run() -> void:
	artifacts = OS.get_environment("NATIVE_JS_ARTIFACTS")
	if artifacts.is_empty() or OS.get_environment("LOGIN_USER").is_empty() or OS.get_environment("LOGIN_PASS").is_empty():
		fail("SETUP: owned artifacts and synthetic credential environment required")
		return
	if not ClassDB.class_exists("GameClient"):
		fail("SETUP: native GameClient extension unavailable")
		return
	root.size = Vector2i(1280, 720)
	var scene: PackedScene = load("res://scenes/client.tscn")
	if scene == null:
		fail("SETUP: production client scene unavailable")
		return
	client = scene.instantiate()
	node_added.connect(observe_node)
	root.add_child(client)
	var setup_deadline := Time.get_ticks_msec() + 150000
	while Time.get_ticks_msec() < setup_deadline:
		username = client.find_child("UsernameInput", true, false) as LineEdit
		password = client.find_child("PasswordInput", true, false) as LineEdit
		connect_button = client.find_child("ConnectButton", true, false) as Button
		if username != null and password != null and connect_button != null:
			break
		await process_frame
	if username == null or password == null or connect_button == null:
		fail("SETUP: authored Login never mounted; cache/initialization failure is not JS RED")
		return
	var login = client.get_node_or_null("LoginUI")
	if login == null or not login.visible or not password.secret:
		fail("SETUP: visible authored Login and secret PasswordInput required")
		return
	# Signal observers only retain evidence; they never mutate UI/account state.
	observe_credentials()
	if not mark("login-ready"):
		return
	print("OBSERVE: authored Login ready; password=***")
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		observe_credentials()
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect":
			var selection = client.get_node_or_null("CharacterSelectUI")
			if selection == null or not selection.visible or login.visible or state.character_count != 1:
				fail("FEATURE: successful Account auth did not project one-character authored CharSelect", true)
				return
			var selected: Label = selection.find_child("CharSelectCharacterName", true, false) as Label
			if selected == null or selected.text != OS.get_environment("NATIVE_JS_CHARACTER"):
				fail("FEATURE: authored CharSelect does not show authoritative fixture roster name", true)
				return
			if not saw_credentials or not saw_authored_click:
				fail("FEATURE: Account success without observed exact credential entry and authored Connect click", true)
				return
			if not mark("observed-charselect"):
				return
			print("OBSERVE: exact credential entry, authored Connect click, real Account reply and CharSelect roster=1; password=***")
			# Let the script's final dump action run. Parent verifies stdout, not this observer.
			var dump_deadline := Time.get_ticks_msec() + 2000
			while Time.get_ticks_msec() < dump_deadline:
				await process_frame
			client.queue_free()
			await process_frame
			quit(0)
			return
		await process_frame
	if not saw_credentials and not saw_authored_click:
		fail("FEATURE RED: authored Login ready but unchanged startup JS did not enter credentials/click Connect; no full parity proof", true)
	else:
		fail("FEATURE RED: script input observed but real Account-to-CharSelect flow did not finish; inspect protocol evidence", true)
