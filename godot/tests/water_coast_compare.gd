extends SceneTree

## Captures one liquid view for comparisons against retail. The capture must not depend on
## how far the preview's streaming got: the earlier version saved as soon as one Water node
## existed, so the same pose produced 1 to 9 roots and background where tiles were missing.
##
## WATER_COAST_OUTPUT  screenshot path (required)
## WATER_COAST_CASE    "retail" selects KulTiras and point (1050, 0, 950), else map 2991
## WATER_COAST_MAP     overrides the map
## WATER_COAST_POINT   overrides the looked-at point as "x,y,z"
## WATER_COAST_EYE     camera offset from the point as "dx,dy,dz" (default 110,120,110)
## WATER_COAST_HIDE_LIQUIDS "1" hides the terrain liquid surfaces, so the same pose shows the
##                     scene without them and the difference measures what the liquid covers

const STABLE_FRAMES := 60

var client: Node3D

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var retail := OS.get_environment("WATER_COAST_CASE") == "retail"
	var map := OS.get_environment("WATER_COAST_MAP")
	if map == "":
		map = "KulTiras" if retail else "2991"
	var point := Vector3(1050, 0, 950) if retail else Vector3(3065, 756.5897, -1380)
	if OS.get_environment("WATER_COAST_POINT") != "":
		point = parse_vector(OS.get_environment("WATER_COAST_POINT"))
	var offset := Vector3(110, 120, 110)
	if OS.get_environment("WATER_COAST_EYE") != "":
		offset = parse_vector(OS.get_environment("WATER_COAST_EYE"))
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
	camera.position = point + offset
	camera.look_at(point)
	var deadline := Time.get_ticks_msec() + 240000
	var lit := false
	while Time.get_ticks_msec() < deadline:
		await process_frame
		camera.current = true
		var state: Dictionary = client.account_state()
		if client.terrain_height_at(point.x, point.z) == null or state.terrain.pending_count != 0:
			continue
		# The preview does not spawn its world objects until the lighting pass adopts the global
		# WMO, so run that pass once the WDT is loaded. Without it the object queue stays idle
		# while the readiness flags already read ready, and the capture saves either the empty
		# world or the populated one depending on how many objects finish in the draw frames:
		# the same pose then differs by 89% of its pixels (9 spawned objects against 10001).
		if not lit:
			error = client.preview_world_lighting(point, camera.position)
			if error != "":
				fail(error)
				return
			lit = true
			continue
		if state.world_objects.pending != 0 or state.world_objects.spawned == 0:
			continue
		var surfaces := await wait_surfaces(camera, deadline)
		if surfaces < 0:
			break
		var ground = client.terrain_height_at(camera.position.x, camera.position.z)
		if ground == null:
			fail("No terrain under the camera at " + str(camera.position))
			return
		if camera.position.y <= float(ground) + 0.5:
			fail("Camera is at or below the terrain surface: eye y=%.2f ground=%.2f. The near ground is culled and the shot shows background instead of the world." % [camera.position.y, float(ground)])
			return
		var liquid_root_count := client.find_children("Water", "Node3D", true, false).size()
		if OS.get_environment("WATER_COAST_HIDE_LIQUIDS") == "1":
			for node in client.find_children("Water", "Node3D", true, false):
				node.visible = false
		hide_ui(client)
		# The root viewport keeps the frame it was last handed: reading it in the same frame as
		# the hide returns the UI frame, which is identical for every pose. Draw with the
		# capture camera active first. The water animation is time-driven, so align the
		# material clock on a 30 s boundary and freeze it: without this, two runs sample
		# different authored frames and the comparison measures the animation, not the code.
		for frame in 30:
			camera.current = true
			await process_frame
		var clock := client.get_node_or_null("/root/M2MaterialClock")
		if clock != null:
			var delta := 30000.0 - fmod(float(clock.call("elapsed_time_ms")), 30000.0)
			clock.call("advance_time_ms", delta)
			for frame in 3:
				await process_frame
			clock.set_process(false)
		client.set_process(false)
		RenderingServer.global_shader_parameter_set("liquid_animation_time_ms", 1000.0)
		# Lighting is a one-shot sync that skips itself when its sample did not change, so calling it
		# while the preview is still building materials leaves the materials made afterwards unlit:
		# the same pose then saves either a lit or a dark frame depending on that race. Apply it once
		# here, after every tile and material exists.
		error = client.preview_world_lighting(point, camera.position)
		if error != "":
			fail(error)
			return
		for frame in 3:
			camera.current = true
			await process_frame
			await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		if image == null or image.is_empty() or image.save_png(output) != OK:
			fail("Cannot capture coast")
			return
		state = client.account_state()
		var view_camera := root.get_viewport().get_camera_3d()
		print("WATER_COAST_CAMERA current=%s transform=%s" % [view_camera.name if view_camera != null else "none", view_camera.global_transform if view_camera != null else Transform3D()])
		print("WATER_COAST_PROOF map=%s point=%s water_roots=%d water_hidden=%d eye=%s ground=%.2f above_ground=%.2f terrain=%s objects=%s screenshot=%s lighting=production" % [map, point, surfaces, liquid_root_count if OS.get_environment("WATER_COAST_HIDE_LIQUIDS") == "1" else 0, camera.position, float(ground), camera.position.y - float(ground), state.terrain, state.world_objects, output])
		client.free()
		print("PASS: normal-lit coast capture")
		quit(0)
		return
	fail("Coast readiness deadline: " + str(client.account_state()))

## Waits until the Water root count and the rendered mesh count both hold for
## STABLE_FRAMES, so the capture cannot race the preview's per-tile streaming. The
## client also re-activates its own camera while tiles stream, so the capture camera
## takes the view again on every frame of the wait.
func wait_surfaces(camera: Camera3D, deadline: int) -> int:
	var last_roots := -1
	var last_meshes := -1
	var stable := 0
	while stable < STABLE_FRAMES and Time.get_ticks_msec() < deadline:
		await process_frame
		camera.current = true
		var roots := client.find_children("Water", "Node3D", true, false).size()
		var meshes := client.find_children("*", "MeshInstance3D", true, false).size()
		if roots > 0 and roots == last_roots and meshes == last_meshes:
			stable += 1
		else:
			last_roots = roots
			last_meshes = meshes
			stable = 0
	if stable < STABLE_FRAMES:
		fail("Water nodes never stabilized: roots=%d meshes=%d" % [last_roots, last_meshes])
		return -1
	return last_roots

func parse_vector(text: String) -> Vector3:
	var fields := text.split(",")
	if fields.size() != 3:
		fail("Expected x,y,z but got " + text)
		return Vector3.ZERO
	return Vector3(float(fields[0]), float(fields[1]), float(fields[2]))

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
