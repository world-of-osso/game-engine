extends SceneTree

## World-object census capture for one map pose. Unlike `water_coast_compare`, this does not
## need a liquid surface or a terrain height at the point: it waits for the preview's object
## queue to drain and reports how many objects spawned and how many failed, which is what the
## Forever receipt census has to move.
##
## WATER_CENSUS_MAP      map directory (required)
## WATER_CENSUS_POINT    looked-at point as "x,y,z" (required)
## WATER_CENSUS_OUTPUT   screenshot path (required)
## WATER_CENSUS_TIMEOUT  wait budget in milliseconds (default 180000)

const STABLE_FRAMES := 30

var client: Node3D

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var map := OS.get_environment("WATER_CENSUS_MAP")
	var point_text := OS.get_environment("WATER_CENSUS_POINT")
	var output := OS.get_environment("WATER_CENSUS_OUTPUT")
	if map == "" or point_text == "" or output == "":
		fail("WATER_CENSUS_MAP, WATER_CENSUS_POINT and WATER_CENSUS_OUTPUT are required")
		return
	var timeout := 180000
	if OS.get_environment("WATER_CENSUS_TIMEOUT") != "":
		timeout = int(OS.get_environment("WATER_CENSUS_TIMEOUT"))
	root.size = Vector2i(1280, 720)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	for frame in 120:
		await process_frame
	var point := parse_vector(point_text)
	var error: String = client.preview_world_map(map, point)
	if error != "":
		fail(error)
		return
	client.set_world_minutes(1440.0)
	var camera := Camera3D.new()
	camera.far = 4000.0
	root.add_child(camera)
	camera.current = true
	var eye := Vector3(90.0, 140.0, 90.0)
	if OS.get_environment("WATER_CENSUS_EYE") != "":
		eye = parse_vector(OS.get_environment("WATER_CENSUS_EYE"))
	camera.position = point + eye
	camera.look_at(point)
	var deadline := Time.get_ticks_msec() + timeout
	var stable := 0
	var state: Dictionary = {}
	var drained := false
	var lit := false
	while Time.get_ticks_msec() < deadline:
		await process_frame
		camera.current = true
		state = client.account_state()
		# The preview adopts the map's light only when this pass runs, and the call fails until
		# the WDT is loaded. Without it `scene_light` stays unset, so the terrain and the M2
		# shaders read an ambient of zero and the frame is a black silhouette.
		if not lit:
			var lighting_error: String = client.preview_world_lighting(point, camera.position)
			if lighting_error == "":
				lit = true
			continue
		var objects: Dictionary = state.world_objects
		if int(objects.pending) == 0 and int(objects.spawned) > 0:
			stable += 1
			if stable >= STABLE_FRAMES:
				drained = true
				break
		else:
			stable = 0
	# The root viewport keeps the frame it was last handed: reading it without hiding the UI
	# returns the interface frame, which is identical for every pose.
	hide_ui(client)
	for frame in 30:
		camera.current = true
		await process_frame
		await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	image.save_png(output)
	var objects: Dictionary = state.world_objects
	print("WORLD_CENSUS_PROOF map=%s point=%s drained=%s spawned=%d failures=%d pending=%d tiles=%d terrain_failures=%d screenshot=%s" % [
		map, point, drained, int(objects.spawned), int(objects.failures), int(objects.pending),
		state.terrain.parsed_tiles.size(), state.terrain.failures.size(), output])
	if not drained:
		print("WORLD_CENSUS_WARNING object queue did not drain within %d ms" % timeout)
	if state.terrain.parsed_tiles.size() == 0:
		print("WORLD_CENSUS_TERRAIN attached=%s terrain=%s" % [state.world_attached, state.terrain])
	quit(0)

func hide_ui(node: Node) -> void:
	for child in node.get_children():
		if child is CanvasLayer:
			child.visible = false
		elif child is Control:
			child.hide()
		else:
			hide_ui(child)

func parse_vector(text: String) -> Vector3:
	var parts := text.split(",")
	return Vector3(float(parts[0]), float(parts[1]), float(parts[2]))

func fail(message: String) -> void:
	push_error(message)
	quit(1)
