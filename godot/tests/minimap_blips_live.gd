extends SceneTree

# Private-server proof driver. Two accounts run separately; no synthetic group state.
var client: Node
var output := OS.get_environment("MINIMAP_BLIPS_OUTPUT")
var account := OS.get_environment("MINIMAP_BLIPS_ACCOUNT")
var label := OS.get_environment("MINIMAP_BLIPS_LABEL")
var endpoint := OS.get_environment("MINIMAP_BLIPS_SERVER")
var command_path := output.path_join(label + "-command.json")

func _initialize() -> void:
	call_deferred("run_proof")

func write_json(name: String, value: Variant) -> void:
	var file := FileAccess.open(output.path_join(label + "-" + name + ".json"), FileAccess.WRITE)
	assert(file != null, "cannot write live evidence")
	file.store_string(JSON.stringify(value, "\t"))
	file.close()

func find_control(name: String) -> Control:
	return client.find_child(name, true, false) as Control

func click(control: Control) -> void:
	assert(control != null and control.is_visible_in_tree(), "missing live control")
	var point := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame

func capture(name: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	assert(image.save_png(output.path_join(label + "-" + name + ".png")) == OK)
	var rects := {}
	var parts := {}
	var target_name := "MinimapTarget" + str(client.target_state().get("target", ""))
	for blip_name in ["MinimapMember57", "MinimapMember58", "MinimapDisplay", "MinimapCluster", target_name]:
		var control := find_control(blip_name)
		if control != null:
			rects[blip_name] = str(control.get_global_rect())
			var part := control.find_child("Part0", true, false) as Control
			if part != null:
				parts[blip_name] = {"rotation": part.rotation, "colour": str(part.modulate), "rect": str(part.get_global_rect())}
	write_json(name, {"account": client.account_state(), "group": client.group_state(),
		"target": client.target_state(), "minimap": client.minimap_state(), "rects": rects, "parts": parts})
	print("MINIMAP BLIPS CAPTURE ", label, " ", name)

func run_proof() -> void:
	assert(not output.is_empty() and not account.is_empty() and not endpoint.is_empty())
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + 180000
	while client.account_state().get("assets_starting", true) and Time.get_ticks_msec() < deadline:
		await process_frame
	assert(not client.account_state().get("assets_starting", true), "assets did not start")
	assert(client.connect_account(endpoint, account, "fbtest", false).is_empty())
	deadline = Time.get_ticks_msec() + 90000
	while client.account_state().screen != "CharacterSelect" and Time.get_ticks_msec() < deadline:
		await process_frame
	assert(client.account_state().screen == "CharacterSelect", "private login failed")
	await click(find_control("CharCard_0"))
	await click(find_control("EnterWorld"))
	deadline = Time.get_ticks_msec() + 180000
	while client.account_state().screen != "InWorld" and Time.get_ticks_msec() < deadline:
		await process_frame
	assert(client.account_state().screen == "InWorld", "private world failed")
	deadline = Time.get_ticks_msec() + 180000
	while not client.minimap_state().get("open", false) and Time.get_ticks_msec() < deadline:
		await process_frame
	assert(client.minimap_state().get("open", false), "minimap did not mount")
	await capture("ready")
	while true:
		# Accept the real server invite through the native popup.
		if not client.group_state().get("pending_invite", "").is_empty():
			var accept := find_control("StaticPopup1Button1")
			if accept != null and accept.is_visible_in_tree():
				await click(accept)
		if FileAccess.file_exists(command_path):
			var file := FileAccess.open(command_path, FileAccess.READ)
			var command: Dictionary = JSON.parse_string(file.get_as_text())
			file.close()
			DirAccess.remove_absolute(command_path)
			await capture(command["capture"])
		await process_frame
