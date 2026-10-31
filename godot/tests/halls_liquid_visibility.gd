extends "res://tests/capture_world_view.gd"

## Halls of Infusion group 22 water at the reported channel, with the camera in its
## split child. Requires GODOT_TEST_SERVER, VIEW_ACCOUNT, VIEW_CHARACTER and
## HALLS_ADMIN_SOCKET for a disposable level-80 character on the private server.
## VIEW_SHOTS optionally saves the visible and hidden-control frames.

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	character = OS.get_environment("VIEW_CHARACTER")
	shots = OS.get_environment("VIEW_SHOTS")
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("VIEW_ACCOUNT")
	var socket := OS.get_environment("HALLS_ADMIN_SOCKET")
	var admin := OS.get_environment("HALLS_ADMIN")
	if server == "" or account == "" or character == "" or socket == "" or admin == "":
		fail("Private server, account, character, admin and socket are required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "": fail("Fixture connection: " + error); return
	if not await enter_world(): return
	var output := []
	var rc := OS.execute("/usr/bin/env", ["GAME_SERVER_ADMIN_SOCKET=" + socket, admin, "teleport", character, "2527", "-51", "-152.9", "-39"], output, true)
	if rc != 0: fail("Halls teleport failed: " + str(output)); return
	if not await wait_halls(): return
	client.set_world_minutes(1440.0)
	client.set_camera_orbit(0.0, -0.35, 10.0)
	await wait_real(3.0)
	await wait_frames(3)
	var state: Dictionary = client.account_state()
	if state.ui_skin != "Forever": fail("Expected Forever skin, got " + str(state.ui_skin)); return
	var liquid := client.find_child("Group22_Liquid", true, false) as MeshInstance3D
	if liquid == null or not liquid.is_visible_in_tree(): fail("Halls group 22 liquid is not visible"); return
	await RenderingServer.frame_post_draw
	var visible := root.get_texture().get_image()
	liquid.visible = false
	await wait_frames(3)
	await RenderingServer.frame_post_draw
	var hidden := root.get_texture().get_image()
	var changed := changed_pixels(visible, hidden)
	if shots != "":
		DirAccess.make_dir_recursive_absolute(shots)
		visible.save_png(shots.path_join("halls-liquid-visible.png"))
		hidden.save_png(shots.path_join("halls-liquid-hidden-control.png"))
	if changed < 1000: fail("Halls liquid changed only %d pixels" % changed); return
	print("HALLS_LIQUID_VISIBLE pixels=", changed, " player=", state.local_player_position, " skin=", state.ui_skin)
	client.free()
	quit(0)

func wait_halls() -> bool:
	var deadline := Time.get_ticks_msec() + 600000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.world_map_id != 2527 or state.local_player_position == null: continue
		if Vector3(state.local_player_position).distance_to(Vector3(-51, -39.8, 152.9)) >= 3.0: continue
		if state.terrain.pending_count == 0 and state.world_objects.pending == 0 and not wmo_liquids().is_empty(): return true
	fail("Halls world did not settle: " + str(client.account_state()))
	return false

func changed_pixels(first: Image, second: Image) -> int:
	var changed := 0
	for y in range(180, 650, 2):
		for x in range(160, 1120, 2):
			var a := first.get_pixel(x, y)
			var b := second.get_pixel(x, y)
			if absf(a.r - b.r) + absf(a.g - b.g) + absf(a.b - b.b) > 0.04: changed += 1
	return changed
