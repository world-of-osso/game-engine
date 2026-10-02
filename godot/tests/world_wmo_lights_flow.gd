extends "res://tests/world_fog_cone_flow.gd"

## WMO group point lights (MOLP per active MLSP doodad set; WebWowViewerCpp CPointLight)
## in the real client: the placed WMOs carry `Group<g>_Light<i>` OmniLight3D nodes, and
## turning them off changes at least MIN_LIGHT_PIXELS of the frame (stable across two lit
## frames). Environment as capture_world_view.gd, plus
##   LIGHT_CASES   "name,minutes,yaw,pitch,distance" joined by ";"
##   LIGHT_MIN     the light node floor (12, one Stockade group's torches)
## The Stockade (map 34, WoW 103, 76, -34.5): "hall,1440,0,-0.2,8".

const MIN_LIGHT_PIXELS := 2000

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
	var floor := int(OS.get_environment("LIGHT_MIN")) if OS.get_environment("LIGHT_MIN") != "" else 12
	for entry in OS.get_environment("LIGHT_CASES").split(";", false):
		var fields := entry.split(",")
		client.set_world_minutes(float(fields[1]))
		client.set_camera_orbit(float(fields[2]), float(fields[3]), float(fields[4]))
		await wait_real(5.0)
		var lights := client.find_children("Group*_Light*", "OmniLight3D", true, false)
		print("FIXTURE WMO_LIGHTS %s count=%d" % [fields[0], lights.size()])
		if lights.size() < floor:
			await capture(fields[0] + ".png")
			fail("%s: %d WMO point lights (< %d)" % [fields[0], lights.size(), floor])
			return
		var changed := await lights_changed_pixels(fields[0], lights)
		print("FIXTURE WMO_LIGHT_PIXELS %s changed=%d" % [fields[0], changed])
		if changed < MIN_LIGHT_PIXELS:
			fail("%s: WMO lights change %d pixels (< %d)" % [fields[0], changed, MIN_LIGHT_PIXELS])
			return
	print("FIXTURE WORLD_WMO_LIGHTS_DONE")
	client.free()
	quit(0)

func lights_changed_pixels(shot: String, lights: Array) -> int:
	await RenderingServer.frame_post_draw
	var lit := root.get_texture().get_image()
	lit.save_png(shots.path_join(shot + "_lit.png"))
	for light in lights:
		light.visible = false
	await wait_frames(2)
	await RenderingServer.frame_post_draw
	var dark := root.get_texture().get_image()
	dark.save_png(shots.path_join(shot + "_dark.png"))
	for light in lights:
		light.visible = true
	await wait_frames(2)
	await RenderingServer.frame_post_draw
	var again := root.get_texture().get_image()
	var changed := 0
	for y in lit.get_height():
		for x in lit.get_width():
			var a := lit.get_pixel(x, y)
			if channel_difference(a, dark.get_pixel(x, y)) > 0.05 and channel_difference(a, again.get_pixel(x, y)) < 0.02:
				changed += 1
	return changed
