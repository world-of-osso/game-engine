extends "res://tests/world_planets_flow.gd"

## The LightSkybox flag 0x4 fog cone (WebWowViewerCpp map.cpp skyMesh0x4) in the real
## client: inside a Light whose skybox has flag 0x4 the WorldLighting/SkyFogCone node is
## shown and changes at least MIN_CONE_PIXELS of the frame (shown vs hidden, stable across
## two shown frames). Environment as capture_world_view.gd, plus
##   CONE_CASES   "name,minutes,yaw,pitch,distance" joined by ";"
## Twilight Highlands (-5138.5, -5567.8, 35; skybox 451101 flags 6): "noon,1440,2.6,0.05,15".

const MIN_CONE_PIXELS := 500

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	character = OS.get_environment("VIEW_CHARACTER")
	shots = OS.get_environment("VIEW_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("VIEW_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	for entry in OS.get_environment("CONE_CASES").split(";", false):
		var fields := entry.split(",")
		client.set_world_minutes(float(fields[1]))
		client.set_camera_orbit(float(fields[2]), float(fields[3]), float(fields[4]))
		await wait_real(2.0)
		var cone := client.get_node_or_null("WorldLighting/SkyFogCone") as MeshInstance3D
		if cone == null or not cone.visible:
			await capture(fields[0] + ".png")
			fail("%s: fog cone not shown (%s)" % [fields[0], cone])
			return
		var changed := await changed_pixels(fields[0], cone)
		print("FIXTURE CONE_PIXELS %s changed=%d" % [fields[0], changed])
		if changed < MIN_CONE_PIXELS:
			fail("%s: fog cone changes %d pixels (< %d)" % [fields[0], changed, MIN_CONE_PIXELS])
			return
	print("FIXTURE WORLD_FOG_CONE_DONE")
	client.free()
	quit(0)

## Pixels differing by more than 0.05 between `node` shown and hidden and by less than 0.02
## between two shown frames.
func changed_pixels(shot: String, node: GeometryInstance3D) -> int:
	await RenderingServer.frame_post_draw
	var shown := root.get_texture().get_image()
	shown.save_png(shots.path_join(shot + "_shown.png"))
	node.visible = false
	await wait_frames(2)
	await RenderingServer.frame_post_draw
	var hidden := root.get_texture().get_image()
	hidden.save_png(shots.path_join(shot + "_hidden.png"))
	node.visible = true
	await wait_frames(2)
	await RenderingServer.frame_post_draw
	var again := root.get_texture().get_image()
	var changed := 0
	for y in shown.get_height():
		for x in shown.get_width():
			var a := shown.get_pixel(x, y)
			if channel_difference(a, hidden.get_pixel(x, y)) > 0.05 and channel_difference(a, again.get_pixel(x, y)) < 0.02:
				changed += 1
	return changed
