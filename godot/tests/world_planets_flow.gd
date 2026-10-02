extends "res://tests/capture_world_view.gd"

## The sun and moon discs (WebWowViewerCpp planetShader, DayNightLightHolder
## updatePlanetsAndStars) in the real client on a private server. Environment as
## capture_world_view.gd, plus
##   PLANET_CASES   "name,minutes,fdid,yaw,pitch,distance" joined by ";"
## Each case sets the time and camera, requires disc `fdid` shown and in view, and requires
## it to change at least MIN_DISC_PIXELS pixels of a box around its projected centre
## (shown vs hidden, stable across two shown frames). Northshire (-8949, -132): noon sun
## "noon,1440,186220,0,1.3,15", midnight moon "night,0,4629581,0.8,0.9,15".

const MIN_DISC_PIXELS := 40
const BOX := 60

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
	for entry in OS.get_environment("PLANET_CASES").split(";", false):
		var fields := entry.split(",")
		client.set_world_minutes(float(fields[1]))
		client.set_camera_orbit(float(fields[3]), float(fields[4]), float(fields[5]))
		await wait_real(2.0)
		var disc := client.get_node_or_null("WorldLighting/Planet" + fields[2]) as MeshInstance3D
		if disc == null or not disc.visible:
			await capture(fields[0] + ".png")
			fail("%s: disc %s not shown (%s)" % [fields[0], fields[2], disc])
			return
		var camera := client.get_node("WorldCamera") as Camera3D
		var centre := camera.unproject_position(disc.global_position)
		var in_view := not camera.is_position_behind(disc.global_position) and Rect2(Vector2.ZERO, Vector2(root.size)).has_point(centre)
		print("FIXTURE PLANET %s fdid=%s centre=%s in_view=%s direction=%s" % [fields[0], fields[2], centre, in_view, (disc.global_position - camera.global_position).normalized()])
		if not in_view:
			await capture(fields[0] + ".png")
			fail("%s: disc %s is out of view at %s" % [fields[0], fields[2], centre])
			return
		if not await check_disc(fields[0], disc, Vector2i(centre)):
			return
	print("FIXTURE WORLD_PLANETS_DONE")
	client.free()
	quit(0)

func check_disc(shot: String, disc: MeshInstance3D, centre: Vector2i) -> bool:
	await RenderingServer.frame_post_draw
	var shown := root.get_texture().get_image()
	shown.save_png(shots.path_join(shot + "_shown.png"))
	disc.visible = false
	await wait_frames(2)
	await RenderingServer.frame_post_draw
	var hidden := root.get_texture().get_image()
	hidden.save_png(shots.path_join(shot + "_hidden.png"))
	disc.visible = true
	await wait_frames(2)
	await RenderingServer.frame_post_draw
	var again := root.get_texture().get_image()
	var changed := 0
	for y in range(maxi(0, centre.y - BOX), mini(shown.get_height(), centre.y + BOX)):
		for x in range(maxi(0, centre.x - BOX), mini(shown.get_width(), centre.x + BOX)):
			var a := shown.get_pixel(x, y)
			if channel_difference(a, hidden.get_pixel(x, y)) > 0.05 and channel_difference(a, again.get_pixel(x, y)) < 0.02:
				changed += 1
	print("FIXTURE PLANET_PIXELS %s changed=%d" % [shot, changed])
	if changed < MIN_DISC_PIXELS:
		fail("%s: disc changes %d pixels (< %d)" % [shot, changed, MIN_DISC_PIXELS])
		return false
	return true

func channel_difference(a: Color, b: Color) -> float:
	return maxf(maxf(absf(a.r - b.r), absf(a.g - b.g)), absf(a.b - b.b))

## The discs stand behind the world: start once the terrain is in.
func wait_objects(_deadline: int) -> bool:
	return true
