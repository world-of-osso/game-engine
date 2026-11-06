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
	if not await enter_world(client):
		return
	if not OS.get_environment("SKYBORNE_DORNOGAL").is_empty():
		if not await capture_dornogal(client):
			return
	else:
		if not await capture_skyborne_profiles(client):
			return
	print("FIXTURE SKYBORNE_DONE")
	quit(0)

func enter_world(client) -> bool:
	var ui = client.get_node("CharacterSelectUI")
	# EnterWorld appears only after the selected character's preview model loads.
	var deadline := Time.get_ticks_msec() + WORLD_LOAD_WAIT_MS
	var card: Control
	var enter: Control
	while Time.get_ticks_msec() < deadline:
		card = ui.find_child("CharCard_0", true, false)
		enter = ui.find_child("EnterWorld", true, false)
		if card != null and card.is_visible_in_tree() and enter != null and enter.is_visible_in_tree():
			break
		await process_frame
	if card == null or enter == null or not enter.is_visible_in_tree():
		fail("Character preview did not reveal EnterWorld")
		return false
	await click_control(card)
	await ready_deadline_check(ui)
	await click_control(enter)
	# The screen label stays Loading until terrain finishes; the capture waits for the
	# replicated NPC visuals instead.
	await ready_deadline_check(client)
	return true

func capture_skyborne_profiles(client) -> bool:
	var deadline := Time.get_ticks_msec() + WORLD_LOAD_WAIT_MS
	var visual: Node3D
	while Time.get_ticks_msec() < deadline:
		visual = client.get_node_or_null("WorldUnits/" + VENTAARI + "/NpcVisualRoot")
		if visual != null and not visual.find_children("Batch*", "MeshInstance3D", true, false).is_empty():
			break
		await process_frame
	if visual == null or visual.find_children("Batch*", "MeshInstance3D", true, false).is_empty():
		fail("Ventaari authored NPC visual was not allocated")
		return false
	var ailee = client.get_node_or_null("WorldUnits/" + AILEE)
	if ailee == null:
		fail("Ailee did not replicate")
		return false
	if ailee.get_node_or_null("NpcVisualRoot") != null:
		fail("Ailee unexpectedly rendered despite withheld required profile")
		return false
	print("FIXTURE AILEE_BLOCKED display=136968 resource=1102747; no substituted body")
	print("FIXTURE VENTAARI_READY display=139694 body=7478494")
	var meshes := visual.find_children("*", "MeshInstance3D", true, false)
	print("FIXTURE VENTAARI_MESHES count=", meshes.size())
	for mesh in meshes:
		print("FIXTURE MESH ", visual.get_path_to(mesh), " visible=", mesh.is_visible_in_tree())
	return await capture_authored_visual(visual, "ventaari-139694")

func capture_dornogal(client) -> bool:
	var extras := []
	for entry in [
		["Dornogal Earthen Female", "earthen-117276"],
		["Dornogal Earthen Male", "earthen-117271"],
		["Dornogal Blood Elf", "bloodelf-121978"],
		["Dornogal Control Guard", "control-2989"],
	]:
		var visual: Node3D = await await_npc_visual(client, entry[0])
		if visual == null:
			return false
		extras.append([visual, entry[1]])
	# Grass cards and scattered doodads sit between the camera and a ground-level
	# portrait; hide them for the equipment captures and restore them afterwards.
	var ground: Node = client.find_child("GroundDetail", true, false)
	var objects: Node = client.find_child("WorldObjects", true, false)
	if ground != null:
		ground.visible = false
	if objects != null:
		objects.visible = false
	for current in extras:
		for other in extras:
			other[0].visible = other == current
		freeze_animation(current[0])
		var meshes: Array = current[0].find_children("Batch*", "MeshInstance3D", true, false)
		print("FIXTURE ", current[1], "_MESHES count=", meshes.size())
		if not await capture_authored_visual(current[0], current[1]):
			return false
	for extra in extras:
		extra[0].visible = true
	if ground != null:
		ground.visible = true
	if objects != null:
		objects.visible = true
	return true

func await_npc_visual(client, npc_name: String) -> Node3D:
	var deadline := Time.get_ticks_msec() + WORLD_LOAD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		var visual := client.get_node_or_null("WorldUnits/" + npc_name + "/NpcVisualRoot") as Node3D
		if visual != null and not visual.find_children("Batch*", "MeshInstance3D", true, false).is_empty():
			return visual
		await process_frame
	fail(npc_name + " authored NPC visual was not allocated")
	return null

func freeze_animation(visual: Node3D) -> void:
	# The model root is NpcVisualRoot/NpcModel, so the player is not a direct child.
	for animation in visual.find_children("*", "WowAnimationPlayer", true, false):
		animation.set_process(false)
		if animation.play_animation(0, true):
			animation.advance_time_ms(1000.0)
			animation.set_paused(true)

func ready_deadline_check(ui: Node) -> void:
	var deadline := Time.get_ticks_msec() + 2000
	while Time.get_ticks_msec() < deadline:
		await process_frame

func capture_authored_visual(visual: Node3D, prefix: String) -> bool:
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
	camera.size = 2.4
	viewport.add_child(camera)
	camera.make_current()
	var origin := visual.global_position
	print("FIXTURE CAPTURE_ORIGIN ", origin)
	for view in ["front", "back"]:
		var direction := 1.0 if view == "front" else -1.0
		# A raised three-quarter view keeps the ground-grass cards out of the portrait.
		camera.position = origin + Vector3(2.6 * direction, 2.0, 3.6 * direction)
		camera.look_at(origin + Vector3(0.0, 0.9, 0.0))
		for frame in 8:
			await process_frame
		await RenderingServer.frame_post_draw
		var path := output.path_join(prefix + "-" + view + ".png")
		var image := viewport.get_texture().get_image()
		if not capture_has_visible_pixels(image):
			fail("NPC capture has no visible body: " + path)
			return false
		var result := image.save_png(path)
		if result != OK:
			fail("Cannot save " + path + ": " + str(result))
			return false
		print("FIXTURE SCREENSHOT ", path)
	viewport.queue_free()
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
