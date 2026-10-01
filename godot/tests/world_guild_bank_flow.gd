extends "res://tests/world_sound_flow.gd"

# Owned authenticated peer, no direct UI setters or developer service.
# Opening probe isolates missing consumer; normal mode requires the actual vault pick.
func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if not endpoint.begins_with("127.0.0.1:") or endpoint.ends_with(":0"):
		fail("Guild bank requires parent-owned loopback endpoint")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_screen(client, "Loading", 15000):
		return
	print("FIXTURE GUILD_BANK_LOADING")
	if not await wait_world(client):
		return
	print("FIXTURE GUILD_BANK_READY")
	if OS.get_environment("GUILD_BANK_OPENING_PROBE") != "1":
		var vault := await find_vault(client)
		if vault.is_empty():
			return
		await click_point(vault.point, MOUSE_BUTTON_RIGHT)
	# AuthoritySent on the peer distinguishes setup/transport from native UI absence.
	var deadline := Time.get_ticks_msec() + 10000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var host := client.get_node_or_null("GuildBankUI")
		var frame := host.find_child("GuildBankFrame", true, false) as Control if host != null else null
		if frame == null or not frame.is_visible_in_tree():
			continue
		var title := host.find_child("GuildBankFrameTitleText", true, false) as Label
		var bag := host.find_child("ContainerFrame0", true, false) as Control
		if title == null or title.text != "Fixture Guild" or bag == null or not bag.is_visible_in_tree():
			fail("GuildBanker did not show authoritative guild title/backpack")
			return
		print("FIXTURE GUILD_BANK_OPEN_DONE")
		client.free()
		quit(0)
		return
	fail("Native GuildBankUI absent after authenticated authoritative GuildBanker open")

func find_vault(client: Node) -> Dictionary:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var camera := root.get_viewport().get_camera_3d()
		if camera == null:
			continue
		for node in client.find_children("*", "Node3D", true, false):
			if not node.has_meta("game_object_name") or str(node.get_meta("game_object_name")) != "Fixture Guild Vault":
				continue
			var area := node.find_child("UnitPick", true, false) as Area3D
			if area == null or area.get_child_count() == 0:
				continue
			var world_point := (area.get_child(0) as Node3D).global_position
			if not camera.is_position_in_frustum(world_point):
				continue
			var point := camera.unproject_position(world_point)
			var id: int = node.get_meta("game_object_server_id")
			if UnitPicker.pick(camera, point) == id:
				return {"point": point, "id": id}
	fail("SETUP: replicated usable vault lacks real model/pick; not native consumer RED")
	return {}

func click_point(point: Vector2, button: MouseButton) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = down
		root.push_input(event, true)
		await process_frame
