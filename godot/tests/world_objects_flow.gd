extends "res://tests/world_units_flow.gd"

# Real-server world entry spawns every parsed tile's authored `_obj0` doodads and
# WMOs (Stormwind's city is ADT-placed WMOs), and reconnect removes them.
# GODOT_TEST_CARD selects the roster card (default 1); GODOT_CAPTURE_PATH
# optionally saves the settled in-world frame.

func select_second_character(client: Node) -> void:
	var ui = client.get_node("CharacterSelectUI")
	var card := OS.get_environment("GODOT_TEST_CARD")
	await click_control(ui.find_child("CharCard_" + (card if not card.is_empty() else "1"), true, false))
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
		await create_timer(3.0).timeout
		if not assert_scenery_distance(client, root_node):
			return
		print("settled fps=%.1f nodes=%d draws=%d objects=%d" % [Engine.get_frames_per_second(), root_node.get_child_count(), Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME), Performance.get_monitor(Performance.RENDER_TOTAL_OBJECTS_IN_FRAME)])
		var hold := OS.get_environment("GODOT_PROBE_HOLD_SECONDS")
		if not hold.is_empty():
			print("holding for profiler")
			await create_timer(float(hold)).timeout
		if OS.get_environment("GODOT_PROBE_OBJECT_COST") == "1":
			root_node.visible = false
			await create_timer(3.0).timeout
			print("hidden fps=%.1f" % Engine.get_frames_per_second())
			root_node.visible = true
			for player in root_node.find_children("M2Animation", "", true, false):
				player.process_mode = Node.PROCESS_MODE_DISABLED
			await create_timer(3.0).timeout
			print("no-anim fps=%.1f" % Engine.get_frames_per_second())
		await capture()
		var error = client.connect_account(server, account, password, false)
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

# Retail scenery distance: the smallest size class is drawn to 30 yd from its
# box center, larger classes farther, so a doodad whose origin is within 29 yd of
# the camera is drawn and a hidden one lies beyond that. Hidden doodads do not
# animate.
func assert_scenery_distance(client: Node, root_node: Node) -> bool:
	var camera: Camera3D = client.get_node("WorldCamera")
	var eye := camera.global_position
	var shown := 0
	var hidden := 0
	for child in root_node.get_children():
		if not child.name.begins_with("Doodad"):
			continue
		var distance: float = eye.distance_to(child.global_position)
		var animation = child.get_node_or_null("M2Animation")
		if child.visible:
			shown += 1
			if animation != null and not animation.is_processing():
				fail("Drawn doodad %s does not animate" % child.name)
				return false
		else:
			hidden += 1
			if distance <= 29.0:
				fail("Doodad %s hidden %.1f yd from the camera" % [child.name, distance])
				return false
			if animation != null and animation.is_processing():
				fail("Hidden doodad %s still animates" % child.name)
				return false
	print("scenery distance: shown=%d hidden=%d" % [shown, hidden])
	if shown == 0 or hidden == 0:
		fail("Scenery distance must draw near doodads and hide far ones: shown=%d hidden=%d" % [shown, hidden])
		return false
	return true

func capture() -> void:
	var output := OS.get_environment("GODOT_CAPTURE_PATH")
	if output.is_empty():
		return
	for frame in range(30):
		await process_frame
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(output)
