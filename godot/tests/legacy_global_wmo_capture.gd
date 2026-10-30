extends SceneTree

# Offline native render proof: production WDT loader, groups, doodads and lighting.
# No server/instance gameplay or no-install certification is claimed.
const MAPS := [
	[1213, "alliance_garrison_alchemy", 951907],
	[1214, "alliance_garrison_enchanting", 948623],
	[1258, "garrison_alliance_tailoring1", 951994],
]
var client: Node3D
var camera: Camera3D
var output := ""
var evidence := ""

func _initialize() -> void:
	Engine.max_fps = 30
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	output = OS.get_environment("GLOBAL_WMO_SHOTS")
	evidence = OS.get_environment("GLOBAL_WMO_EVIDENCE")
	if output.is_empty() or evidence.is_empty():
		fail("GLOBAL_WMO_SHOTS and GLOBAL_WMO_EVIDENCE required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	camera = Camera3D.new()
	camera.far = 1000.0
	camera.fov = 55.0
	root.add_child(camera)
	camera.current = true
	for frame in 120:
		await process_frame
	# One map per client avoids unrelated asynchronous carryover between previews.
	var map_id := int(OS.get_environment("GLOBAL_WMO_MAP"))
	if not MAPS.any(func(spec): return spec[0] == map_id):
		fail("GLOBAL_WMO_MAP must name one of the three recovered maps")
		return
	for spec in MAPS:
		if spec[0] != map_id:
			continue
		var error: String = client.preview_world_map(spec[1], Vector3.ZERO)
		if not error.is_empty():
			fail(error)
			return
		var placed = await wait_map(spec[2])
		if placed == null:
			return
		var bounds := mesh_bounds(placed)
		if bounds.size.length() == 0.0:
			fail("Empty authored WMO geometry")
			return
		var center := bounds.get_center()
		var distance := maxf(bounds.size.length() * 1.05, 20.0)
		camera.position = center + Vector3(0.72, 0.50, 0.85).normalized() * distance
		camera.look_at(center)
		client.set_world_minutes(1440.0)
		error = client.preview_world_lighting(center, camera.position)
		if not error.is_empty():
			fail(error)
			return
		for forever in [true]:
			var skin := "Forever" if forever else "Modern"
			error = client.preview_world_skin(forever)
			if not error.is_empty():
				fail(error)
				return
			hide_ui(client)
			for frame in 90:
				await process_frame
			var state: Dictionary = client.account_state()
			if state.ui_skin != skin or state.terrain.global_wmo_fdid != spec[2]:
				fail("Resolved FDID/skin mismatch: " + str(state))
				return
			var name := "%s-map%d" % [skin.to_lower(), spec[0]]
			await RenderingServer.frame_post_draw
			var image := root.get_texture().get_image()
			if image == null or image.is_empty() or image.save_png(output.path_join(name + ".png")) != OK:
				fail("Cannot capture " + name)
				return
			var file := FileAccess.open(evidence.path_join(name + ".json"), FileAccess.WRITE)
			file.store_string(JSON.stringify({"state": state, "map": spec[0], "skin": skin, "bounds": str(bounds), "camera": str(camera.global_transform), "meshes": placed.find_children("*", "MeshInstance3D", true, false).size()}, "\t"))
			file.close()
			print("GLOBAL_WMO_CAPTURE ", name, " fdid=", spec[2], " bounds=", bounds)
	client.free()
	quit(0)

func wait_map(fdid: int) -> Node3D:
	var deadline := Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if not state.terrain.failures.is_empty():
			fail("Map resource failure: " + str(state.terrain))
			return null
		if state.terrain.global_wmo_fdid == fdid:
			var error: String = client.preview_world_lighting(Vector3.ZERO, camera.position)
			if not error.is_empty():
				fail(error)
				return null
		var placed := client.get_node_or_null("WorldWmos/GlobalWmo%d" % fdid) as Node3D
		if placed != null and not placed.find_children("Group*_Batch*", "MeshInstance3D", true, false).is_empty() and state.world_objects.pending == 0:
			return placed
	fail("Timed out waiting for global WMO: " + str(client.account_state()))
	return null

func mesh_bounds(placed: Node3D) -> AABB:
	var result := AABB()
	var first := true
	for mesh in placed.find_children("*", "MeshInstance3D", true, false):
		if mesh.mesh == null:
			continue
		var box: AABB = mesh.global_transform * mesh.get_aabb()
		result = box if first else result.merge(box)
		first = false
	return result

func hide_ui(node: Node) -> void:
	for child in node.get_children():
		if child is CanvasLayer:
			child.visible = false
		elif child is Control:
			child.hide()
		else:
			hide_ui(child)

func fail(message: String) -> void:
	push_error("GLOBAL_WMO_FAIL " + message)
	if is_instance_valid(client):
		client.free()
	quit(1)
