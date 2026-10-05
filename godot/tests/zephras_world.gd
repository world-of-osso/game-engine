extends SceneTree

# Offline integration of the production map reader, stream, meshes and object loaders.
# Asset failures remain visible: the local Forever installation is not a complete archive.
# WDT inventory labels slot 26×64+29 as (26,29); native filename coordinates are (29,26).
const CENTER := Vector3(2933.3333, 0.0, -1333.3333)
const OUTPUT := "res://../data/diagnostics/zephras-world-production-lighting.png"
const COAST_OUTPUT := "res://../data/diagnostics/zephras-liquid-coast.png"
# Authored lake shore on tile29_26; LiquidType1251 surface is Y=756.58972.
const COAST := Vector3(3065.0, 756.58972, -1380.0)
var client: Node3D

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not client.has_method("preview_world_map"):
		fail("GameClient has no offline world-map preview")
		return
	# Let startup settle before the fixture requests a map.
	for frame in 120:
		await process_frame
	var error: String = client.preview_world_map("2991", CENTER)
	if error != "":
		fail(error)
		return
	if not client.has_method("preview_world_lighting"):
		fail("GameClient has no production world-lighting preview")
		return
	client.set_world_minutes(1440.0)
	var camera := Camera3D.new()
	camera.far = 3000.0
	root.add_child(camera)
	camera.current = true
	var deadline := Time.get_ticks_msec() + 240000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		var tile := client.get_node_or_null("WorldTerrain/Tile29_26")
		var doodads := client.find_children("Doodad*", "Node3D", true, false)
		var wmos := client.find_children("Wmo*", "Node3D", true, false).filter(func(node): return node.has_meta("wmo_model"))
		if tile == null or doodads.is_empty() or wmos.is_empty() or state.terrain.pending_count != 0:
			continue
		var water := tile.get_node_or_null("Water")
		if water == null or water.get_child_count() == 0:
			fail("Authored tile29_26 MH2O water has no rendered surfaces")
			return
		var height = client.terrain_height_at(CENTER.x, CENTER.z)
		if height == null:
			fail("Loaded terrain lacks tile-center height")
			return
		var surface := Vector3(CENTER.x, float(height), CENTER.z)
		camera.position = surface + Vector3(350.0, 400.0, 350.0)
		camera.look_at(surface)
		var lighting_error: String = client.preview_world_lighting(surface, camera.position)
		if lighting_error != "":
			fail(lighting_error)
			return
		hide_ui(client)
		for frame in 30:
			await process_frame
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		if image == null or image.is_empty() or image.save_png(OUTPUT) != OK:
			fail("Cannot capture rendered map")
			return
		var meshes := tile.find_children("*", "MeshInstance3D", true, false)
		print("ZEPHRAS_PROOF chunks=%d doodad_nodes=%d wmo_nodes=%d height=%s terrain=%s objects=%s screenshot=%s lighting=production-forever" % [meshes.size(), doodads.size(), wmos.size(), height, state.terrain, state.world_objects, OUTPUT])
		if meshes.size() < 256:
			fail("Expected 256 terrain meshes")
			return
		camera.position = COAST + Vector3(110.0, 120.0, 110.0)
		camera.look_at(COAST)
		lighting_error = client.preview_world_lighting(COAST, camera.position)
		if lighting_error != "":
			fail(lighting_error)
			return
		for frame in 30:
			await process_frame
		await RenderingServer.frame_post_draw
		image = root.get_texture().get_image()
		if image == null or image.is_empty() or image.save_png(COAST_OUTPUT) != OK:
			fail("Cannot capture authored lake shore")
			return
		print("ZEPHRAS_LIQUID_PROOF water_surfaces=%d coast=%s screenshot=%s" % [water.get_child_count(), COAST, COAST_OUTPUT])
		client.free()
		print("PASS: Zephras terrain, doodads, WMO nodes and authored water rendered through native stream")
		quit(0)
		return
	fail("Timed out waiting for authored terrain/doodad/WMO nodes: " + str(client.account_state()))

func hide_ui(node: Node) -> void:
	for child in node.get_children():
		if child is CanvasLayer:
			child.visible = false
		elif child is Control:
			child.hide()
		else:
			hide_ui(child)

func fail(message: String) -> void:
	push_error("ZEPHRAS_FAIL " + message)
	if is_instance_valid(client):
		client.free()
	quit(1)
