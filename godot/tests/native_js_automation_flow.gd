extends SceneTree

# Observation only. Production startup reads chosen JS and owns every input/action.
# Default remains debug/login.js; offline modes never click Connect.
# No credential setters, emitted input signals, connect_account, or automation hooks.
var client: Node
var artifacts: String
var mode: String
var username: LineEdit
var password: LineEdit
var connect_button: Button
var saw_credentials := false
var saw_authored_click := false
var saw_connect_press := false
var saw_offline_typed := false
var saw_offline_deleted := false
var saw_negative_editor_change := false

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

func observe_offline_text(text: String) -> void:
	if mode != "offline-actions":
		return
	if text == "abcd":
		saw_offline_typed = true
	elif text == "abc" and saw_offline_typed:
		saw_offline_deleted = true

func observe_negative_editor_change(_text: String) -> void:
	if mode == "noneditable-type":
		saw_negative_editor_change = true

func observe_authored_click() -> void:
	saw_connect_press = true
	observe_credentials()
	saw_authored_click = saw_authored_click or saw_credentials

func observe_node(node: Node) -> void:
	# Install observers during mounting, before the startup consumer can use controls.
	if node.name == "UsernameInput" and node is LineEdit:
		username = node
		username.text_changed.connect(observe_credentials)
		username.text_changed.connect(observe_offline_text)
		username.text_changed.connect(observe_negative_editor_change)
	elif node.name == "PasswordInput" and node is LineEdit:
		password = node
		password.text_changed.connect(observe_credentials)
		password.text_changed.connect(observe_negative_editor_change)
	elif node.name == "ConnectButton" and node is Button:
		connect_button = node
		connect_button.pressed.connect(observe_authored_click)

func observe_timeout_login(login: CanvasLayer) -> void:
	# Give production's frame-driven deadline and queued successor time to run.
	# Parent alone checks the native timeout diagnostic and live stdout dump.
	var deadline := Time.get_ticks_msec() + 2000
	while Time.get_ticks_msec() < deadline:
		await process_frame
	var state: Dictionary = client.account_state()
	if not login.visible or not username.is_visible_in_tree() or not connect_button.is_visible_in_tree() or state.screen != "Login" or state.reply_received:
		fail("FEATURE: timeout-continuation did not retain actual visible Login without auth", true)
		return
	if saw_credentials or saw_authored_click:
		fail("FEATURE: timeout-continuation unexpectedly entered credentials or clicked Connect", true)
		return
	if not mark("observed-timeout-login"):
		return
	print("OBSERVE: timeout-continuation retained actual visible Login without credential entry/Connect click; parent owns deadline/dump assertions")
	client.queue_free()
	await process_frame
	quit(0)

func observe_offline_actions(login: CanvasLayer) -> void:
	# Observe real changes only; parent checks production hierarchy/UI/deadline output.
	var deadline := Time.get_ticks_msec() + 2000
	while Time.get_ticks_msec() < deadline:
		await process_frame
	var state: Dictionary = client.account_state()
	if not login.visible or not username.is_visible_in_tree() or not connect_button.is_visible_in_tree() or state.screen != "Login" or state.reply_received:
		fail("FEATURE: offline-actions did not retain actual visible Login without auth", true)
		return
	if not saw_offline_typed or not saw_offline_deleted or username.text != "abc":
		fail("FEATURE: offline-actions lacked actual typed text then Backspace deletion outcome", true)
		return
	if saw_connect_press or saw_credentials or not password.text.is_empty():
		fail("FEATURE: offline-actions entered credentials or clicked Connect", true)
		return
	if not mark("observed-offline-actions"):
		return
	print("OBSERVE: offline-actions actual LineEdit typing/deletion and visible Login without Connect; parent owns production output assertions")
	client.queue_free()
	await process_frame
	quit(0)

func negative_login_is_mounted(login: CanvasLayer) -> bool:
	var state: Dictionary = client.account_state()
	var editors_visible := username.is_visible_in_tree() and password.is_visible_in_tree()
	var login_visible := login.is_inside_tree() and login.visible and connect_button.is_visible_in_tree()
	var awaiting_login: bool = state.screen == "Login" and not state.reply_received
	return editors_visible and login_visible and awaiting_login

func observe_noneditable_type(login: CanvasLayer) -> void:
	var label := client.find_child("BlizzardThanks", true, false) as Label
	if label == null:
		fail("SETUP: noneditable-type requires actual authored BlizzardThanks label")
		return
	var label_has_area := label.size.x > 0 and label.size.y > 0
	var label_is_clickable: bool = label.is_visible_in_tree() and label_has_area
	if not label_is_clickable:
		fail("SETUP: noneditable-type requires actual visible authored BlizzardThanks label with click area")
		return
	if root.gui_get_focus_owner() != null:
		fail("SETUP: noneditable-type requires actual absent focus; no artificial focus mutation")
		return
	var editors_empty := username.text.is_empty() and password.text.is_empty()
	if not editors_empty:
		fail("SETUP: noneditable-type requires empty authored editors")
		return
	var initial_username := username.text
	var initial_password := password.text
	var deadline := Time.get_ticks_msec() + 900
	while true:
		if not negative_login_is_mounted(login):
			fail("FEATURE: noneditable-type did not retain actual mounted visible Login stable900ms", true)
			return
		if root.gui_get_focus_owner() != null:
			fail("SETUP: noneditable-type absent-focus seam changed; not queue-stop RED")
			return
		var editors_unchanged := username.text == initial_username and password.text == initial_password
		if not editors_unchanged or saw_negative_editor_change:
			fail("FEATURE: noneditable-type changed actual editor values", true)
			return
		if saw_connect_press or saw_credentials:
			fail("FEATURE: noneditable-type clicked Connect or entered credentials", true)
			return
		if Time.get_ticks_msec() >= deadline:
			break
		await process_frame
	if not mark("observed-noneditable-type"):
		return
	print("OBSERVE: noneditable-type actual absent focus, unchanged editors and mounted visible Login stable900ms without Connect; parent owns exact terminal diagnostic/no successor dump/Auth0 assertions")
	# Production terminal errors stop only the runtime, not Godot. Observer owns exit0.
	client.queue_free()
	await process_frame
	quit(0)

func run() -> void:
	artifacts = OS.get_environment("NATIVE_JS_ARTIFACTS")
	mode = OS.get_environment("NATIVE_JS_MODE")
	var known_modes := ["", "login", "timeout-continuation", "offline-actions", "noneditable-type"]
	if mode not in known_modes:
		fail("SETUP: unknown native JS observer mode")
		return
	if artifacts.is_empty() or ((mode == "" or mode == "login") and (OS.get_environment("LOGIN_USER").is_empty() or OS.get_environment("LOGIN_PASS").is_empty())):
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
	if mode == "timeout-continuation":
		await observe_timeout_login(login)
		return
	if mode == "offline-actions":
		await observe_offline_actions(login)
		return
	if mode == "noneditable-type":
		await observe_noneditable_type(login)
		return
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
