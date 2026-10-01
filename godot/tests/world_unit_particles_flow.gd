extends "res://tests/capture_world_view.gd"

## Creature model particle emitters in the real client (WebWowViewerCpp
## animationManager.cpp calcParticleEmitters runs for every M2 object, units included):
## after the world's units attach, the creatures whose display models author emitters
## draw them. Environment as capture_world_view.gd; VIEW_PLAN shots follow.

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
	print("FIXTURE WORLD_UNIT_PARTICLES_DONE")
	client.free()
	quit(0)

func unit_names() -> Array:
	var names := []
	if client.has_node("WorldUnits"):
		for unit in client.get_node("WorldUnits").get_children():
			names.append(str(unit.get_meta("unit_name", unit.name)))
	return names
