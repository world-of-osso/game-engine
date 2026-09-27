extends "res://tests/world_units_flow.gd"

# Real-server world entry spawns every parsed tile's authored `_obj0` doodads and
# WMOs (Stormwind's city is ADT-placed WMOs), and reconnect removes them.
# GODOT_CAPTURE_PATH optionally saves the settled in-world frame.

func select_second_character(client: Node) -> void:
	var ui = client.get_node("CharacterSelectUI")
	await click_control(ui.find_child("CharCard_1", true, false))
	await click_control(ui.find_child("EnterWorld", true, false))
	var deadline := Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		var terrain: Dictionary = state.terrain
		if state.screen != "InWorld" or terrain.pending_count != 0 or terrain.parsed_tiles.is_empty():
			continue
		var objects: Dictionary = state.world_objects
		if objects.pending != 0:
			continue
		var authored_wmos := 0
		var authored_doodads := 0
		for tile in terrain.parsed_tiles:
			authored_wmos += tile.wmo_count
			authored_doodads += tile.doodad_count
		if authored_wmos == 0 or authored_doodads == 0:
			fail("Fixture character must enter tiles with authored objects: " + str(terrain.parsed_tiles))
			return
		var root_node := client.get_node_or_null("WorldObjects")
		if root_node == null:
			fail("In-world objects root missing: " + str(objects))
			return
		var wmos := 0
		var doodads := 0
		for child in root_node.get_children():
			if child.name.begins_with("Wmo"):
				wmos += 1
			elif child.name.begins_with("Doodad"):
				doodads += 1
		print("world objects: wmos=%d/%d doodads=%d/%d failures=%d" % [wmos, authored_wmos, doodads, authored_doodads, objects.failures])
		# WMOs shared by adjacent tiles are listed per tile but spawned once.
		if wmos == 0 or doodads < authored_doodads / 2 or wmos + doodads != objects.spawned:
			fail("Authored tile objects were not spawned: " + str(objects))
			return
		await capture()
		var error = client.connect_account("127.0.0.1:5000", "admin", "admin", false)
		if error != "":
			fail("Reconnect failed: " + error)
			return
		await process_frame
		if client.get_node_or_null("WorldObjects") != null or client.account_state().world_objects.spawned != 0:
			fail("Reconnect retained previous map objects")
			return
		print("PASS: in-world ADT doodads and WMOs spawn from parsed tiles and reconnect clears them")
		client.free()
		quit(0)
		return
	fail("Timed out waiting for world objects: " + str(client.account_state().world_objects))

func capture() -> void:
	var output := OS.get_environment("GODOT_CAPTURE_PATH")
	if output.is_empty():
		return
	for frame in range(30):
		await process_frame
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(output)
