extends "res://tests/world_npc_visual_flow.gd"

const AILEE := "Ailee Farheart"
const VENTAARI := "Ventaari Brightwish"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var blank := Image.create(1024, 1024, false, Image.FORMAT_RGBA8)
	blank.fill(Color(0.12, 0.14, 0.17))
	if capture_has_visible_pixels(blank):
		fail("NPC capture guard accepted a blank frame")
		return
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":0"):
		fail("Skyborne fixture requires owned loopback endpoint")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, "fixture", "fixture", false)
	if error != "":
		fail("Skyborne connection: " + error)
		return
	if not await wait_screen(client, "CharacterSelect", STARTUP_WAIT_MS):
		return
	var ui = client.get_node("CharacterSelectUI")
	await click_control(ui.find_child("CharCard_0", true, false))
	await click_control(ui.find_child("EnterWorld", true, false))
	var deadline := Time.get_ticks_msec() + WORLD_LOAD_WAIT_MS
	var visual: Node3D
	while Time.get_ticks_msec() < deadline:
		visual = client.get_node_or_null("WorldUnits/" + VENTAARI + "/NpcVisualRoot")
		if visual != null and not visual.find_children("Batch*", "MeshInstance3D", true, false).is_empty():
			break
		await process_frame
	if visual == null or visual.find_children("Batch*", "MeshInstance3D", true, false).is_empty():
		fail("Ventaari authored NPC visual was not allocated")
		return
	var ailee = client.get_node_or_null("WorldUnits/" + AILEE)
	if ailee == null:
		fail("Ailee did not replicate")
		return
	if ailee.get_node_or_null("NpcVisualRoot") != null:
		fail("Ailee unexpectedly rendered despite withheld required profile")
		return
	print("FIXTURE AILEE_BLOCKED display=136968 resource=1102747; no substituted body")
	print("FIXTURE VENTAARI_READY display=139694 body=7478494")
	var meshes := visual.find_children("*", "MeshInstance3D", true, false)
	print("FIXTURE VENTAARI_MESHES count=", meshes.size())
	for mesh in meshes:
		print("FIXTURE MESH ", visual.get_path_to(mesh), " visible=", mesh.is_visible_in_tree())
	if not await capture_authored_visual(visual):
		return
	print("FIXTURE SKYBORNE_DONE")
	quit(0)

func capture_authored_visual(visual: Node3D) -> bool:
	var output := OS.get_environment("SKYBORNE_NPC_SCREENSHOTS")
	if output.is_empty() or not output.is_absolute_path():
		fail("SKYBORNE_NPC_SCREENSHOTS must be an absolute output directory")
		return false
	DirAccess.make_dir_recursive_absolute(output)
	viewport = SubViewport.new()
	viewport.size = Vector2i(1024, 1024)
	viewport.world_3d = visual.get_world_3d()
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(viewport)
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 3.4
	viewport.add_child(camera)
	camera.make_current()
	var origin := visual.global_position
	print("FIXTURE CAPTURE_ORIGIN ", origin)
	for view in ["front", "back"]:
		var direction := 1.0 if view == "front" else -1.0
		camera.position = origin + Vector3(4.0 * direction, 1.35, 5.0 * direction)
		camera.look_at(origin + Vector3(0.0, 1.0, 0.0))
		for frame in 8:
			await process_frame
		await RenderingServer.frame_post_draw
		var path := output.path_join("ventaari-139694-" + view + ".png")
		var image := viewport.get_texture().get_image()
		if not capture_has_visible_pixels(image):
			fail("NPC capture has no visible body: " + path)
			return false
		var result := image.save_png(path)
		if result != OK:
			fail("Cannot save " + path + ": " + str(result))
			return false
		print("FIXTURE SCREENSHOT ", path)
	return true

func capture_has_visible_pixels(image: Image) -> bool:
	var background := image.get_pixel(0, 0)
	var changed_samples := 0
	for y in range(0, image.get_height(), 16):
		for x in range(0, image.get_width(), 16):
			if image.get_pixel(x, y) != background:
				changed_samples += 1
	print("FIXTURE CAPTURE_CHANGED_SAMPLES ", changed_samples)
	return changed_samples >= 64
