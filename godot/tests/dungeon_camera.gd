extends "res://tests/frame_benchmark.gd"

## Private-server camera acceptance. Stage the account's offline character at the chosen
## dungeon/outdoor floor before running. Capture the unmodified Forever view, then check
## that a clear 15-yard orbit is not replaced by the height of distant overhead terrain.
## The geometric orbit assertion is enabled by default (CAMERA_EXPECT_CLEAR=0 captures only).
func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	var character := OS.get_environment("BENCH_CHARACTER")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), OS.get_environment("BENCH_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail("Connection: " + error)
		return
	if not await enter_world(character):
		return
	if client.account_state().get("ui_skin") != "Forever":
		fail("Camera evidence requires Forever")
		return
	player = client.get_node_or_null("WorldUnits/" + character) as Node3D
	if player == null:
		fail("Missing player")
		return
	client.set_world_minutes(1440.0)
	var yaw := float(OS.get_environment("CAMERA_YAW"))
	var pitch := float(OS.get_environment("CAMERA_PITCH")) if OS.get_environment("CAMERA_PITCH") != "" else -0.35
	client.set_camera_orbit(yaw, pitch, 15.0)
	await create_timer(5.0).timeout
	await RenderingServer.frame_post_draw
	var camera := root.get_camera_3d()
	var feet := player.global_position
	var offset := camera.global_position - feet
	var eye := feet + Vector3.UP * 1.8
	var intended := eye + Basis.from_euler(Vector3(pitch, yaw, 0.0), EULER_ORDER_YXZ) * Vector3.BACK * 15.0
	var query := PhysicsRayQueryParameters3D.create(eye, intended, 7)
	var hit := root.world_3d.direct_space_state.intersect_ray(query)
	var expected_distance := maxf(0.5, eye.distance_to(hit.position) - 0.3) if not hit.is_empty() else 15.0
	var report := {
		"label": OS.get_environment("CAMERA_LABEL"), "skin": client.account_state().ui_skin,
		"player": [feet.x, feet.y, feet.z],
		"camera": [camera.global_position.x, camera.global_position.y, camera.global_position.z],
		"orbit_yaw": yaw, "orbit_pitch": pitch, "requested_distance": 15.0,
		"distance_from_eye": camera.global_position.distance_to(eye),
		"mesh_expected_distance": expected_distance,
		"height_above_feet": offset.y,
		"terrain_at_player": client.terrain_height_at(feet.x, feet.z),
		"terrain_at_camera": client.terrain_height_at(camera.global_position.x, camera.global_position.z),
		"map": client.account_state().terrain.map,
		"terrain_pending": client.account_state().terrain.pending_count,
		"objects_pending": client.account_state().world_objects.pending,
		"collision_pending": client.account_state().world_objects.collision_pending,
	}
	var image := root.get_texture().get_image()
	if image.get_size() != Vector2i(1920, 1080):
		fail("Unexpected screenshot resolution: " + str(image.get_size()))
		return
	if image.save_png(OS.get_environment("CAMERA_SCREENSHOT")) != OK:
		fail("Cannot save screenshot")
		return
	var receipt := FileAccess.open(OS.get_environment("CAMERA_RECEIPT"), FileAccess.WRITE)
	receipt.store_string(JSON.stringify(report, "\t"))
	receipt.close()
	print("CAMERA_RECEIPT ", JSON.stringify(report))
	if OS.get_environment("CAMERA_EXPECT_CLEAR") != "0":
		if absf(report.distance_from_eye - expected_distance) > 0.1 or offset.y > 8.0:
			fail("Clear dungeon orbit changed by overhead terrain: " + JSON.stringify(report))
			return
	print("FIXTURE DUNGEON_CAMERA_DONE")
	client.free()
	quit(0)
