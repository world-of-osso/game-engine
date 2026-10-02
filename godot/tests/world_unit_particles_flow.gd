extends "res://tests/capture_world_view.gd"

## Creature and item model particle emitters in the real client (WebWowViewerCpp
## animationManager.cpp calcParticleEmitters runs for every M2 object, units included):
## after the world's units attach, the creatures whose display models author emitters
## draw them, and so do the item models units hold (VIEW_MIN_ITEM_EMITTERS: at least that
## many item emitters). Environment as capture_world_view.gd; VIEW_PLAN shots follow. A shot whose
## yaw is "aim" turns the camera onto unit VIEW_AIM_UNIT and requires its particles to
## change at least MIN_PARTICLE_PIXELS pixels around it (frame with the unit particle
## pools shown vs hidden).

const MIN_PARTICLE_PIXELS := 200
const AIM_STEPS := 64
const AIM_OFFSET := Vector2(-280, 0)

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
	var deadline := Time.get_ticks_msec() + 120000
	while client.account_state().unit_visuals_pending > 0 and Time.get_ticks_msec() < deadline:
		await wait_frames(10)
	for entry in plan.split(";", false):
		var fields := entry.split(",")
		client.set_world_minutes(float(fields[1]))
		var target: Node3D = null
		if fields[2] == "aim":
			target = aim_unit(OS.get_environment("VIEW_AIM_UNIT"))
			if target == null:
				fail("%s: no unit named %s" % [fields[0], OS.get_environment("VIEW_AIM_UNIT")])
				return
			var yaw := await aim_yaw(target, float(fields[3]), float(fields[4]))
			client.set_camera_orbit(yaw, float(fields[3]), float(fields[4]))
		else:
			client.set_camera_orbit(float(fields[2]), float(fields[3]), float(fields[4]))
		await wait_real(float(fields[5]))
		var state: Dictionary = client.account_state()
		var units = state.get("unit_particles")
		var pools := client.get_node("WorldUnits").find_children("Particles*", "MultiMeshInstance3D", true, false) if client.has_node("WorldUnits") else []
		var names := []
		for pool in pools:
			names.append("%s:%d" % [pool.name, (pool as MultiMeshInstance3D).multimesh.visible_instance_count])
		print("FIXTURE UNIT_PARTICLES %s %s pools=%s units=%s" % [fields[0], units, names, unit_names()])
		await capture(fields[0] + ".png")
		if units == null or int(units.units) == 0 or int(units.emitters) == 0:
			fail("%s: no creature carries particle emitters: %s" % [fields[0], units])
			return
		var min_items := int(OS.get_environment("VIEW_MIN_ITEM_EMITTERS"))
		if int(units.get("item_emitters", 0)) < min_items:
			fail("%s: item models carry %s emitters (< %d)" % [fields[0], units.get("item_emitters"), min_items])
			return
		if target != null and not await check_particle_pixels(fields[0], target, pools):
			return
	print("FIXTURE WORLD_UNIT_PARTICLES_DONE")
	client.free()
	quit(0)

func unit_names() -> Array:
	var names := []
	if client.has_node("WorldUnits"):
		for unit in client.get_node("WorldUnits").get_children():
			names.append(str(unit.get_meta("unit_name", unit.name)))
	return names

func aim_unit(unit_name: String) -> Node3D:
	if not client.has_node("WorldUnits"):
		return null
	for unit in client.get_node("WorldUnits").get_children():
		if str(unit.get_meta("unit_name", "")) == unit_name:
			return unit
	return null

## The orbit yaw that puts `target`'s chest nearest AIM_OFFSET from the screen centre,
## beside the local player the orbit camera centres.
func aim_yaw(target: Node3D, pitch: float, distance: float) -> float:
	var best_yaw := 0.0
	var best_score := INF
	var centre := Vector2(root.size) / 2.0 + AIM_OFFSET
	for step in range(AIM_STEPS):
		var yaw := TAU * step / AIM_STEPS
		client.set_camera_orbit(yaw, pitch, distance)
		await wait_frames(2)
		var camera := client.get_node("WorldCamera") as Camera3D
		var point := target.global_position + Vector3.UP * 1.5
		if camera.is_position_behind(point):
			continue
		var score := camera.unproject_position(point).distance_to(centre)
		if score < best_score:
			best_score = score
			best_yaw = yaw
	return best_yaw

## Pixels inside `target`'s projected model bounds (grown by 24 px) that differ by more
## than 0.1 in any channel between the frame with the unit particle pools drawn and with
## them hidden.
func check_particle_pixels(shot: String, target: Node3D, pools: Array) -> bool:
	await RenderingServer.frame_post_draw
	var shown := root.get_texture().get_image()
	shown.save_png(shots.path_join(shot + "_shown.png"))
	for pool in pools:
		pool.visible = false
	await wait_frames(2)
	await RenderingServer.frame_post_draw
	var hidden := root.get_texture().get_image()
	hidden.save_png(shots.path_join(shot + "_hidden.png"))
	for pool in pools:
		pool.visible = true
	var region := screen_bounds(target).grow(24).intersection(Rect2(Vector2.ZERO, Vector2(shown.get_size())))
	var changed := 0
	for y in range(int(region.position.y), int(region.end.y)):
		for x in range(int(region.position.x), int(region.end.x)):
			var a := shown.get_pixel(x, y)
			var b := hidden.get_pixel(x, y)
			if maxf(maxf(absf(a.r - b.r), absf(a.g - b.g)), absf(a.b - b.b)) > 0.1:
				changed += 1
	print("FIXTURE PARTICLE_PIXELS %s unit=%s region=%s changed=%d" % [shot, target.get_meta("unit_name", ""), region, changed])
	if changed < MIN_PARTICLE_PIXELS:
		fail("%s: %s particles change %d pixels (< %d)" % [shot, target.get_meta("unit_name", ""), changed, MIN_PARTICLE_PIXELS])
		return false
	return true

## Screen rectangle of the corners of `target`'s mesh bounds.
func screen_bounds(target: Node3D) -> Rect2:
	var camera := client.get_node("WorldCamera") as Camera3D
	var region := Rect2()
	var first := true
	for mesh in target.find_children("*", "MeshInstance3D", true, false):
		var box: AABB = mesh.global_transform * mesh.get_aabb()
		for corner in range(8):
			var point := camera.unproject_position(box.get_endpoint(corner))
			region = Rect2(point, Vector2.ZERO) if first else region.expand(point)
			first = false
	return region
