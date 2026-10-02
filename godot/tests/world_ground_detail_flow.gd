extends "res://tests/capture_world_view.gd"

## Terrain detail doodads (ground clutter) in the real client: the chunks around the
## camera scatter their GroundEffectTexture grass and flowers (solarityclient
## terrain/detail_doodad, client 7D3390). Environment as capture_world_view.gd; each
## VIEW_PLAN shot waits for the detail to finish loading, then requires the GroundDetail
## node to change at least MIN_DETAIL_PIXELS pixels of the frame (shown vs hidden).

const MIN_DETAIL_PIXELS := 5000

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
		if not await wait_detail(fields[0]):
			return
		if not await check_detail_pixels(fields[0]):
			return
	print("FIXTURE WORLD_GROUND_DETAIL_DONE")
	client.free()
	quit(0)

## As capture_world_view.gd, printing the object and detail progress every 10 s.
func wait_objects(deadline: int) -> bool:
	var next_report := 0
	while Time.get_ticks_msec() < deadline:
		await wait_frames(30)
		var state: Dictionary = client.account_state()
		if Time.get_ticks_msec() >= next_report:
			next_report = Time.get_ticks_msec() + 10000
			print("FIXTURE PROGRESS fps=%d objects=%s detail=%s" % [Engine.get_frames_per_second(), state.world_objects.spawned, state.get("ground_detail")])
		if state.world_objects.pending == 0 and state.world_objects.spawned > 0:
			print("FIXTURE OBJECTS ", state.world_objects)
			return true
	fail("Timed out spawning world objects: " + str(client.account_state().world_objects))
	return false

## Waits until no detail model is loading and some chunk drew detail.
func wait_detail(shot: String) -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		var detail = client.account_state().get("ground_detail")
		if detail != null and int(detail.loading_models) == 0 and int(detail.chunks) > 0:
			await wait_frames(10)
			print("FIXTURE GROUND_DETAIL %s %s" % [shot, client.account_state().ground_detail])
			return true
		await wait_frames(10)
	fail("%s: no ground detail drawn: %s" % [shot, client.account_state().get("ground_detail")])
	return false

## Pixels differing by more than 0.1 in any channel between the frame with the
## GroundDetail node shown and hidden.
func check_detail_pixels(shot: String) -> bool:
	var detail := client.get_node_or_null("GroundDetail") as Node3D
	if detail == null:
		fail("%s: no GroundDetail node" % shot)
		return false
	await RenderingServer.frame_post_draw
	var shown := root.get_texture().get_image()
	shown.save_png(shots.path_join(shot + "_shown.png"))
	detail.visible = false
	await wait_frames(2)
	await RenderingServer.frame_post_draw
	var hidden := root.get_texture().get_image()
	hidden.save_png(shots.path_join(shot + "_hidden.png"))
	detail.visible = true
	var changed := 0
	for y in range(0, shown.get_height(), 2):
		for x in range(0, shown.get_width(), 2):
			var a := shown.get_pixel(x, y)
			var b := hidden.get_pixel(x, y)
			if maxf(maxf(absf(a.r - b.r), absf(a.g - b.g)), absf(a.b - b.b)) > 0.1:
				changed += 4
	print("FIXTURE DETAIL_PIXELS %s changed=%d" % [shot, changed])
	if changed < MIN_DETAIL_PIXELS:
		fail("%s: ground detail changes %d pixels (< %d)" % [shot, changed, MIN_DETAIL_PIXELS])
		return false
	return true
