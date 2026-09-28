extends SceneTree

# Real-server campsite switching to scenes whose authored tiles contain MCNK chunks
# without MCLY texture layers: the session must stay on character select and those
# chunks must render with the original untextured material instead of failing the tile.
# GODOT_CAPTURE_DIR receives campsite-<id>.png for each scene.

const SCENES := [5, 25]
const TERRAIN_TIMEOUT_MS := 60000

func _initialize() -> void:
	call_deferred("run")

func fail(message: String, client: Node) -> void:
	push_error(message)
	client.queue_free()
	quit(1)

func settle() -> void:
	for frame in range(3):
		await process_frame

func click(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await settle()
	for pressed in [true, false]:
		var button := InputEventMouseButton.new()
		button.position = point
		button.global_position = point
		button.button_index = MOUSE_BUTTON_LEFT
		button.pressed = pressed
		root.push_input(button, true)
		await process_frame
	await settle()

func still_selecting(client: Node) -> String:
	var state = client.account_state()
	if state.screen != "CharacterSelect" or state.status != "":
		return "screen=%s status=%s" % [state.screen, state.status]
	return ""

# Chunk counts of attached tiles, and how many use the untextured (zero-layer) material.
func terrain_summary(client: Node) -> Dictionary:
	var summary := {"tiles": 0, "chunks": 0, "untextured": 0}
	var terrain = client.get_node_or_null("CharacterSelectScene/WorldTerrain")
	if terrain == null:
		return summary
	for tile in terrain.get_children():
		summary.tiles += 1
		for chunk in tile.get_children():
			if not chunk is MeshInstance3D:
				continue
			summary.chunks += 1
			var material = chunk.get_surface_override_material(0)
			var ground = material.get_shader_parameter("ground_0")
			if material.get_shader_parameter("config").x == 0.0 and ground is Texture2D and ground.get_width() == 1:
				summary.untextured += 1
	return summary

func capture(path: String) -> bool:
	for frame in range(3):
		await process_frame
		await RenderingServer.frame_post_draw
	var image = root.get_texture().get_image()
	return image != null and not image.is_empty() and image.save_png(path) == OK

func switch_campsite(client: Node, scene_id: int) -> String:
	var ui = client.get_node("CharacterSelectUI")
	await click(ui.find_child("CharSelectCampsitesTab", true, false))
	# The panel pages four campsites at a time and keeps its page; rewind, then page forward.
	var pages := int(ui.frame_text("CampsitePageText").get_slice("/", 1))
	for page in range(pages - 1):
		await click(ui.find_child("CampsitePrevPage", true, false))
	var card = ui.find_child("CampsiteScene_%d" % scene_id, true, false)
	for page in range(pages - 1):
		if card != null:
			break
		await click(ui.find_child("CampsiteNextPage", true, false))
		card = ui.find_child("CampsiteScene_%d" % scene_id, true, false)
	if card == null or not card.is_visible_in_tree():
		return "Campsite panel must list authored scene %d" % scene_id
	var scene_before = client.find_child("CharacterSelectScene", true, false)
	await click(card)
	var deadline = Time.get_ticks_msec() + TERRAIN_TIMEOUT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var problem = still_selecting(client)
		if problem != "":
			return "Scene %d ended character select: %s" % [scene_id, problem]
		var scene_after = client.find_child("CharacterSelectScene", true, false)
		var summary = terrain_summary(client)
		if scene_after != scene_before and summary.untextured > 0:
			return ""
	return "Scene %d terrain never attached untextured chunks: %s" % [scene_id, terrain_summary(client)]

func run() -> void:
	root.size = Vector2i(1280, 720)
	var server = OS.get_environment("GODOT_TEST_SERVER")
	var output_dir = OS.get_environment("GODOT_CAPTURE_DIR")
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if server.is_empty() or output_dir.is_empty():
		fail("Set GODOT_TEST_SERVER and GODOT_CAPTURE_DIR", client)
		return
	# GODOT_TEST_USER/GODOT_TEST_PASSWORD select a per-agent dev account.
	var user = OS.get_environment("GODOT_TEST_USER") if OS.has_environment("GODOT_TEST_USER") else "admin"
	var password = OS.get_environment("GODOT_TEST_PASSWORD") if OS.has_environment("GODOT_TEST_PASSWORD") else "admin"
	var error = client.connect_account(server, user, password, false)
	if error != "":
		fail(error, client)
		return
	var deadline = Time.get_ticks_msec() + 15000
	while client.account_state().screen != "CharacterSelect":
		if Time.get_ticks_msec() > deadline:
			fail("Timed out waiting for character selection", client)
			return
		await process_frame
	await settle()
	for scene_id in SCENES:
		var problem = await switch_campsite(client, scene_id)
		if problem != "":
			fail(problem, client)
			return
		# Let the remaining tiles, objects and sky settle, then prove the session survived.
		for frame in range(60):
			await process_frame
		problem = still_selecting(client)
		if problem != "":
			fail("Scene %d ended character select after attaching: %s" % [scene_id, problem], client)
			return
		var path = "%s/campsite-%d.png" % [output_dir, scene_id]
		if not await capture(path):
			fail("Campsite capture requires a rendering display", client)
			return
		print("Scene %d terrain %s -> %s" % [scene_id, terrain_summary(client), path])
	print("PASS: campsites %s render untextured MCLY-less chunks and keep the session" % [SCENES])
	client.queue_free()
	quit(0)
