extends "res://tests/unitrank_live.gd"

# Production UDP5800 + source-populated Azure Vault; no preview/synthetic snapshots.
# MPLUS_ACCOUNT, MPLUS_CHARACTER, MPLUS_RUN and GODOT_TEST_SERVER are required.
var run_dir := ""
var last_command := -1

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	Engine.max_fps = 30
	run_dir = OS.get_environment("MPLUS_RUN")
	shots = run_dir.trim_suffix("/") + "/"
	character = OS.get_environment("MPLUS_CHARACTER")
	var account := OS.get_environment("MPLUS_ACCOUNT")
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5800" or not account.begins_with("fb_") or run_dir.is_empty():
		fail("Explicit private5800, fb_* account and owned output directory required")
		return
	DirAccess.make_dir_recursive_absolute(run_dir)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account("127.0.0.1:5800", account, PASSWORD, false)
	if error != "":
		fail("Mythic login: " + error)
		return
	if not await enter_world():
		return
	error = client.choose_mythic_difficulty()
	if error != "":
		fail(error)
		return
	await wait_frames(20)
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
	fail("Mythic fixture command deadline")

func write_state(filename: String) -> void:
	var fonts := []
	for node in client.find_children("Mailbox_*", "Node3D", true, false):
		if int(node.get_meta("game_object_entry", 0)) == 246779:
			fonts.append({"id": node.get_meta("game_object_server_id"), "position": str(node.global_position)})
	var file := FileAccess.open(shots + filename, FileAccess.WRITE)
	file.store_string(JSON.stringify({"account": client.account_state(), "challenge": JSON.parse_string(client.challenge_state()), "bags": client.merchant_state().bags, "fonts": fonts}))
	file.close()

func execute_command(command: Dictionary) -> String:
	match str(command.get("op", "")):
		"capture":
			await capture(str(command.file))
			write_state(str(command.file) + ".json")
			return "captured"
		"click_ui":
			var button := control(str(command.get("host", "ChallengeUI")), str(command.frame))
			if button == null or not button.is_visible_in_tree():
				return "missing button " + str(command.frame)
			await click(button)
			return "clicked " + str(command.frame)
		"pickup_key":
			push_key(KEY_B, true)
			await process_frame
			push_key(KEY_B, false)
			await wait_frames(10)
			for item in client.merchant_state().bags:
				if int(item.item_id) != 180653:
					continue
				var button := control("BagsUI", "ContainerFrame%dSlot%d" % [item.bag, item.slot])
				if button == null or not button.is_visible_in_tree():
					return "missing keystone bag button"
				await click(button)
				return "picked up owned keystone"
			return "no keystone in bags"
		"click_font":
			return await click_font()
		"request":
			return client.challenge_request(JSON.stringify(command.request))
		"quit":
			client.free()
			quit(0)
			return "quit"
	return "unknown operation"

func click_font() -> String:
	for turn in range(24):
		await wait_frames(3)
		for node in client.find_children("Mailbox_*", "Node3D", true, false):
			if int(node.get_meta("game_object_entry", 0)) != 246779:
				continue
			var area := node.find_child("UnitPick", true, false) as Area3D
			if area == null:
				continue
			var point := camera().unproject_position((area.get_child(0) as Node3D).global_position)
			if UnitPicker.pick(camera(), point) != node.get_meta("game_object_server_id"):
				continue
			await click_point(point, MOUSE_BUTTON_RIGHT)
			return "clicked source Font"
		client.set_camera_orbit(float(turn) * TAU / 24.0, -0.35, 12.0)
	return "no pickable source Font"
