extends SceneTree

var client: Node3D

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var retail := OS.get_environment("WATER_COAST_CASE") == "retail"
	var map := "KulTiras" if retail else "2991"
	var point := Vector3(1050, 0, 950) if retail else Vector3(3065, 756.5897, -1380)
	var output := OS.get_environment("WATER_COAST_OUTPUT")
	if output == "":
		fail("WATER_COAST_OUTPUT is required")
		return
	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	for frame in 120:
		await process_frame
	var error: String = client.preview_world_map(map, point)
	if error != "":
		fail(error)
		return
	client.set_world_minutes(1440.0)
	var camera := Camera3D.new()
	camera.far = 3000.0
	root.add_child(camera)
	camera.current = true
	camera.position = point + Vector3(110, 120, 110)
	camera.look_at(point)
	var deadline := Time.get_ticks_msec() + 240000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if client.terrain_height_at(point.x, point.z) == null or state.terrain.pending_count != 0:
			continue
		var surfaces := client.find_children("Water", "Node3D", true, false)
		if surfaces.is_empty():
			continue
		error = client.preview_world_lighting(point, camera.position)
		if error != "":
			fail(error)
			return
		hide_ui(client)
		for frame in 60:
			await process_frame
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		if image == null or image.is_empty() or image.save_png(output) != OK:
			fail("Cannot capture coast")
			return
		print("WATER_COAST_PROOF map=%s point=%s water_roots=%d terrain=%s objects=%s screenshot=%s lighting=production" % [map, point, surfaces.size(), state.terrain, state.world_objects, output])
		client.free()
		print("PASS: normal-lit coast capture")
		quit(0)
		return
	fail("Coast readiness deadline: " + str(client.account_state()))

func hide_ui(node: Node) -> void:
	for child in node.get_children():
		if child is CanvasLayer:
			child.visible = false
		elif child is Control:
			child.hide()
		else:
			hide_ui(child)

func fail(error: String) -> void:
	push_error(error)
	if is_instance_valid(client):
		client.free()
	quit(1)
