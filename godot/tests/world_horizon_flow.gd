extends "res://tests/capture_world_view.gd"

## The map's WDL horizon in the real client (solarityclient terrain/low_detail, client
## 7D5150/7D5E70): beyond the streamed tiles the distant terrain stands in the scene fog's
## colour against the sky. Environment as capture_world_view.gd; each VIEW_PLAN shot
## requires WDL tiles shown and the Horizon node to change at least MIN_HORIZON_PIXELS of
## the frame (shown vs hidden).

const MIN_HORIZON_PIXELS := 2000

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("VIEW_ACCOUNT")
	character = OS.get_environment("VIEW_CHARACTER")
	shots = OS.get_environment("VIEW_SHOTS")
	var plan := OS.get_environment("VIEW_PLAN")
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
		var horizon := client.get_node_or_null("Horizon") as Node3D
		var tiles = client.account_state().get("horizon_tiles")
		print("FIXTURE HORIZON %s tiles=%s" % [fields[0], tiles])
		if horizon == null or tiles == null or int(tiles) == 0:
			await capture(fields[0] + ".png")
			fail("%s: no horizon drawn (tiles %s)" % [fields[0], tiles])
			return
		if not await check_pixels(fields[0], horizon):
			return
	print("FIXTURE WORLD_HORIZON_DONE")
	client.free()
	quit(0)

## Pixels differing by more than 0.05 in any channel between the frame with `horizon`
## shown and hidden (every second pixel, counted four times).
func check_pixels(shot: String, horizon: Node3D) -> bool:
	await RenderingServer.frame_post_draw
	var shown := root.get_texture().get_image()
	shown.save_png(shots.path_join(shot + "_shown.png"))
	horizon.visible = false
	await wait_frames(2)
	await RenderingServer.frame_post_draw
	var hidden := root.get_texture().get_image()
	hidden.save_png(shots.path_join(shot + "_hidden.png"))
	horizon.visible = true
	var changed := 0
	for y in range(0, shown.get_height(), 2):
		for x in range(0, shown.get_width(), 2):
			var a := shown.get_pixel(x, y)
			var b := hidden.get_pixel(x, y)
			if maxf(maxf(absf(a.r - b.r), absf(a.g - b.g)), absf(a.b - b.b)) > 0.05:
				changed += 4
	print("FIXTURE HORIZON_PIXELS %s changed=%d" % [shot, changed])
	if changed < MIN_HORIZON_PIXELS:
		fail("%s: horizon changes %d pixels (< %d)" % [shot, changed, MIN_HORIZON_PIXELS])
		return false
	return true
