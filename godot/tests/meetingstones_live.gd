extends "res://tests/unitrank_live.gd"

# Two real clients against an explicitly private endpoint. Commands are owned JSON
# files; object/StaticPopup clicks use viewport input, never a preview injection.
# MS_ACCOUNT, MS_CHARACTER, MS_RUN, MS_INDEX, GODOT_TEST_SERVER are required.
var run_dir := ""
var client_index := ""
var last_command := -1

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	run_dir = OS.get_environment("MS_RUN")
	client_index = OS.get_environment("MS_INDEX")
	character = OS.get_environment("MS_CHARACTER")
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("MS_ACCOUNT")
	if endpoint != "127.0.0.1:5310" or not account.begins_with("fb_") or run_dir.is_empty():
		fail("Explicit private :5310 endpoint, fb_* account and owned run directory required")
		return
	shots = run_dir + "/client" + client_index + "/"
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(endpoint, account, PASSWORD, false)
	if error != "":
		fail("Meeting-stone live login: " + error)
		return
	if not await enter_world():
		return
	await capture("00-inworld.png")
	write_state("ready.json")
	while true:
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

func write_state(filename: String) -> void:
	var popups := []
	for slot in range(1, 4):
		var text := control("StaticPopupUI", "StaticPopup%dText" % slot)
		if text != null and text.is_visible_in_tree():
			popups.append(str(text.text))
	var file := FileAccess.open(shots + filename, FileAccess.WRITE)
	file.store_string(JSON.stringify({"account": client.account_state(), "group": client.group_state(), "target": client.target_state(), "spells": client.spells_state(), "popups": popups}))

func execute_command(command: Dictionary) -> String:
	match str(command.get("op", "")):
		"capture":
			await capture(str(command.file))
			write_state(str(command.file) + ".json")
			return "captured"
		"click_ui":
			var button := control("StaticPopupUI", str(command.frame))
			if button == null or not button.is_visible_in_tree():
				return "missing popup button " + str(command.frame)
			await click(button)
			write_state("after-click.json")
			return "clicked " + str(command.frame)
		"click_object":
			return await click_object(int(command.entry))
		"orbit":
			client.set_camera_orbit(float(command.yaw), -0.35, float(command.get("distance", 12)))
			return "orbited"
		"quit":
			client.free()
			quit(0)
			return "quit"
	return "unknown operation"

func click_object(entry: int) -> String:
	var target := client.target_state()
	for turn in range(24):
		await wait_frames(3)
		for node in client.find_children("Mailbox_*", "Node3D", true, false):
			if node.get_meta("game_object_entry", 0) != entry:
				continue
			var area := node.find_child("UnitPick", true, false) as Area3D
			if area == null:
				continue
			var shape := area.get_child(0) as CollisionShape3D
			var world_point := shape.global_position
			if not camera().is_position_in_frustum(world_point):
				continue
			var point := camera().unproject_position(world_point)
			if UnitPicker.pick(camera(), point) != node.get_meta("game_object_server_id"):
				continue
			await capture("before-object-%d.png" % entry)
			await click_point(point, MOUSE_BUTTON_RIGHT)
			if client.target_state().target != target.target:
				return "FAIL object click changed target"
			write_state("after-object-%d.json" % entry)
			print("MEETINGSTONES CLICK entry=", entry, " target retained=", target.target, " point=", point)
			return "clicked object %d; target retained" % entry
		client.set_camera_orbit(float(turn) * TAU / 24.0, -0.35, 12.0)
	return "no pickable object entry %d" % entry
