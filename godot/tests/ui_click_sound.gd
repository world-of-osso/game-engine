extends SceneTree

var failures := 0

func require(ok: bool, message: String) -> void:
	if not ok:
		push_error(message)
		failures += 1

func pointer(position: Vector2, button: MouseButton, down: bool) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = button
	event.position = position
	event.pressed = down
	root.push_input(event, true)

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var ui: Node = ClassDB.instantiate("RegistryUi")
	root.add_child(ui)
	require(ui.show_login() == "", "login projection failed")
	var sound: Node = ClassDB.instantiate("NativeSound")
	root.add_child(sound)
	var effects := sound.get_node("Effects") as AudioStreamPlayer
	var connect := ui.find_child("ConnectButton", true, false) as Button
	var username := ui.find_child("UsernameInput", true, false) as LineEdit
	if connect == null or username == null or effects == null:
		require(false, "native click fixture nodes missing")
		quit(1)
		return
	var pos := connect.get_global_rect().get_center()
	pointer(pos, MOUSE_BUTTON_LEFT, true)
	await process_frame
	ui.sync_input()
	require(ui.pop_ui_clicks() == 1, "eligible left down must emit exactly one click")
	require(ui.pop_ui_clicks() == 0, "click count must drain")
	pointer(pos, MOUSE_BUTTON_LEFT, false)
	await process_frame
	ui.sync_input()
	require(ui.pop_ui_clicks() == 0, "release must not click")
	pointer(pos, MOUSE_BUTTON_RIGHT, true)
	await process_frame
	ui.sync_input()
	require(ui.pop_ui_clicks() == 0, "right down must not click")
	pointer(pos, MOUSE_BUTTON_RIGHT, false)
	await process_frame
	connect.focus_mode = Control.FOCUS_ALL
	connect.grab_focus()
	var enter := InputEventKey.new()
	enter.keycode = KEY_ENTER
	enter.pressed = true
	root.push_input(enter, true)
	await process_frame
	ui.sync_input()
	require(ui.pop_ui_clicks() == 0, "keyboard submit must not click")
	connect.pressed.emit()
	ui.sync_input()
	require(ui.pop_ui_clicks() == 0, "programmatic action must not click")
	pointer(username.get_global_rect().get_center(), MOUSE_BUTTON_LEFT, true)
	await process_frame
	ui.sync_input()
	require(ui.pop_ui_clicks() == 0, "nonactionable edit box must not click")
	pointer(username.get_global_rect().get_center(), MOUSE_BUTTON_LEFT, false)
	await process_frame
	require(ui.set_connecting(true) == "", "disable button failed")
	pointer(pos, MOUSE_BUTTON_LEFT, true)
	await process_frame
	ui.sync_input()
	require(ui.pop_ui_clicks() == 0, "disabled button must not click")
	pointer(pos, MOUSE_BUTTON_LEFT, false)
	await process_frame
	require(sound.play_ui_click(0.8, 0.5, false), "click playback failed")
	require(effects.stream is AudioStreamWAV and effects.stream.mix_rate == 44100 and effects.stream.data.size() == 3528 and effects.stream.data.decode_s16(0) == 0, "PCM WAV format/content")
	require(absf(effects.volume_linear - 0.22) < 0.001 and effects.is_playing(), "master*effects*.55 independent of music")
	require(sound.play_ui_click(0.8, 0.5, true), "muted click failed")
	require(effects.volume_linear == 0.0, "mute must silence click")
	ui.queue_free()
	sound.queue_free()
	await process_frame
	var roster_ui: Node = ClassDB.instantiate("RegistryUi")
	root.add_child(roster_ui)
	require(roster_ui.show_character_select() == "", "character selection projection failed")
	var child := roster_ui.find_child("CharSelectEmptyCardBackdrop", true, false) as Control
	if child != null:
		pointer(child.get_global_rect().get_center(), MOUSE_BUTTON_LEFT, true)
		await process_frame
		roster_ui.sync_input()
		require(roster_ui.pop_ui_clicks() == 1, "nonactionable child of onclick frame must click once")
		pointer(child.get_global_rect().get_center(), MOUSE_BUTTON_LEFT, false)
		await process_frame
	else:
		require(false, "clickable ancestor fixture child missing")
	roster_ui.queue_free()
	await process_frame
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await process_frame
	var client_ui := client.get_node_or_null("LoginUI")
	var client_sound := client.get_node_or_null("NativeSound")
	if client_ui == null or client_sound == null:
		require(false, "GameClient did not own login UI and NativeSound")
	else:
		var login_button := client_ui.find_child("ConnectButton", true, false) as Button
		var live_effects := client_sound.get_node("Effects") as AudioStreamPlayer
		if login_button == null or live_effects == null:
			require(false, "GameClient click integration nodes missing")
		else:
			pointer(login_button.get_global_rect().get_center(), MOUSE_BUTTON_LEFT, true)
			await process_frame
			require(live_effects.is_playing(), "GameClient left down must play owned effect before action release")
			pointer(login_button.get_global_rect().get_center(), MOUSE_BUTTON_LEFT, false)
			await process_frame
	client.queue_free()
	await process_frame
	if failures == 0:
		print("PASS: native pointer click eligibility, PCM WAV, volume and mute")
	quit(1 if failures else 0)
