extends "res://tests/unitrank_live.gd"

# Real private5811, fb_* account. Opens via ordinary minimap launcher clicks.
var run_dir := ""
var last_command := -1

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	Engine.max_fps = 30
	run_dir = OS.get_environment("MENTOR_RUN")
	shots = run_dir.trim_suffix("/") + "/"
	character = OS.get_environment("MENTOR_CHARACTER")
	var account := OS.get_environment("MENTOR_ACCOUNT")
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if endpoint != "127.0.0.1:5811" or not account.begins_with("fb_") or run_dir.is_empty():
		fail("Explicit private5811, fb_* account and output directory required")
		return
	DirAccess.make_dir_recursive_absolute(run_dir)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(endpoint, account, PASSWORD, false)
	if error != "":
		fail("Mentor login: " + error)
		return
	if not await enter_world():
		return
	await wait_frames(30)
	write_state("ready.json")
	var deadline := Time.get_ticks_msec() + 900000
	while Time.get_ticks_msec() < deadline:
		await wait_frames(10)
		write_state("state.json")
		var path := shots + "command.json"
		if not FileAccess.file_exists(path):
			continue
		var command = JSON.parse_string(FileAccess.get_file_as_string(path))
		if not command is Dictionary or int(command.get("id", -1)) <= last_command:
			continue
		last_command = int(command.id)
		var result := await execute_command(command)
		var file := FileAccess.open(shots + "result.json", FileAccess.WRITE)
		file.store_string(JSON.stringify({"id": last_command, "result": result}))
		file.close()
	fail("Mentor fixture command deadline")

func write_state(filename: String) -> void:
	var file := FileAccess.open(shots + filename, FileAccess.WRITE)
	file.store_string(JSON.stringify({"account": client.account_state(), "keys": JSON.parse_string(client.party_keystone_state()), "panel": control("MythicMentorUI", "MythicMentorPanel") != null}))
	file.close()

func execute_command(command: Dictionary) -> String:
	match str(command.get("op", "")):
		"open":
			var launcher := control("MinimapUI", "MinimapLauncherButton")
			if launcher == null:
				return "missing minimap launcher"
			await click(launcher)
			await wait_frames(5)
			var button := control("LauncherUI", "LauncherEntryMythicMentor")
			if button == null:
				return "missing Mythic Mentor launcher entry"
			await click(button)
			await wait_frames(5)
			return "opened" if control("MythicMentorUI", "MythicMentorPanel") != null else "missing Keys panel"
		"close":
			var button := control("MythicMentorUI", "MythicMentorCloseButton")
			if button == null:
				return "missing close button"
			await click(button)
			await wait_frames(5)
			return "closed" if client.get_node_or_null("MythicMentorUI") == null else "still open"
		"escape":
			push_key(KEY_ESCAPE, true)
			await process_frame
			push_key(KEY_ESCAPE, false)
			await wait_frames(5)
			return "closed" if client.get_node_or_null("MythicMentorUI") == null else "still open"
		"drag":
			var handle := control("MythicMentorUI", "MentorDragHandle")
			if handle == null:
				return "missing drag handle"
			var point := handle.get_global_rect().get_center()
			await move_mouse(point)
			var event := InputEventMouseButton.new()
			event.button_index = MOUSE_BUTTON_LEFT
			event.position = point
			event.pressed = true
			root.push_input(event, true)
			await process_frame
			await move_mouse(point + Vector2(110, 70))
			event.position = point + Vector2(110, 70)
			event.pressed = false
			root.push_input(event, true)
			await wait_frames(5)
			return "dragged"
		"capture":
			await capture(str(command.file))
			write_state(str(command.file) + ".json")
			return "captured"
		"quit":
			client.free()
			quit(0)
			return "quit"
	return "unknown operation"
