extends "res://tests/capture_world_view.gd"

## A Light's LightSkybox model drawn in-world (WebWowViewerCpp
## DayNightLightHolder::SkyBoxCollector, map.cpp skybox view): standing inside a Light whose
## LightParams has a skybox, the AuthoredSky<SKYBOX_FDID> node is shown and changes at
## least MIN_SKYBOX_PIXELS of the frame (shown vs hidden). Environment as
## capture_world_view.gd plus SKYBOX_FDID; VIEW_PLAN shots follow.

const MIN_SKYBOX_PIXELS := 5000

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("VIEW_ACCOUNT")
	character = OS.get_environment("VIEW_CHARACTER")
	shots = OS.get_environment("VIEW_SHOTS")
	var plan := OS.get_environment("VIEW_PLAN")
	var fdid := OS.get_environment("SKYBOX_FDID")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	for entry in plan.split(";", false):
		var fields := entry.split(",")
		client.set_world_minutes(float(fields[1]))
		client.set_camera_orbit(float(fields[2]), float(fields[3]), float(fields[4]))
		await wait_real(float(fields[5]))
		var skybox := client.get_node_or_null("WorldLighting/AuthoredSky" + fdid) as Node3D
		print("FIXTURE SKYBOX %s fdid=%s node=%s visible=%s at=%s" % [fields[0], fdid, skybox != null, skybox != null and skybox.visible, client.account_state().local_player_position])
		if skybox == null or not skybox.visible:
			await capture(fields[0] + ".png")
			fail("%s: skybox %s not drawn" % [fields[0], fdid])
			return
		if not await check_pixels(fields[0], skybox):
			return
	print("FIXTURE WORLD_SKYBOX_DONE")
	client.free()
	quit(0)

## Pixels differing by more than 0.1 in any channel between the frame with `skybox`
## shown and hidden (every second pixel, counted four times).
func check_pixels(shot: String, skybox: Node3D) -> bool:
	await RenderingServer.frame_post_draw
	var shown := root.get_texture().get_image()
	shown.save_png(shots.path_join(shot + "_shown.png"))
	skybox.visible = false
	await wait_frames(2)
	await RenderingServer.frame_post_draw
	var hidden := root.get_texture().get_image()
	hidden.save_png(shots.path_join(shot + "_hidden.png"))
	skybox.visible = true
	var changed := 0
	for y in range(0, shown.get_height(), 2):
		for x in range(0, shown.get_width(), 2):
			var a := shown.get_pixel(x, y)
			var b := hidden.get_pixel(x, y)
			if maxf(maxf(absf(a.r - b.r), absf(a.g - b.g)), absf(a.b - b.b)) > 0.1:
				changed += 4
	print("FIXTURE SKYBOX_PIXELS %s changed=%d" % [shot, changed])
	if changed < MIN_SKYBOX_PIXELS:
		fail("%s: skybox changes %d pixels (< %d)" % [shot, changed, MIN_SKYBOX_PIXELS])
		return false
	return true
