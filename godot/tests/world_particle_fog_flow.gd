extends "res://tests/capture_world_view.gd"

## Doodad particles take the scene fog in the real client (WebWowViewerCpp
## m2ParticleShader.frag.slang makeFog2): every pooled emitter material carries the same
## retail fog uniforms as the terrain, then VIEW_PLAN shots are captured
## (capture_world_view.gd environment).

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("VIEW_ACCOUNT")
	character = OS.get_environment("VIEW_CHARACTER")
	shots = OS.get_environment("VIEW_SHOTS")
	var plan := OS.get_environment("VIEW_PLAN")
	if server == "" or account == "" or character == "" or shots == "":
		fail("GODOT_TEST_SERVER, VIEW_ACCOUNT, VIEW_CHARACTER and VIEW_SHOTS are required")
		return
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
		if not check_particle_fog(fields[0]):
			return
		await capture(fields[0] + ".png")
	print("FIXTURE WORLD_PARTICLE_FOG_DONE")
	client.free()
	quit(0)

func check_particle_fog(name: String) -> bool:
	var terrain_material: ShaderMaterial = null
	for mesh in client.get_node("WorldTerrain").find_children("*", "MeshInstance3D", true, false):
		var candidate := (mesh as MeshInstance3D).get_surface_override_material(0) as ShaderMaterial
		if candidate != null and candidate.get_shader_parameter("fog_mode") == 1:
			terrain_material = candidate
			break
	if terrain_material == null:
		fail("%s: no fogged terrain material" % name)
		return false
	var want: Vector2 = terrain_material.get_shader_parameter("fog_range")
	var want_color: Vector3 = terrain_material.get_shader_parameter("fog_color")
	var pools := client.find_children("Particles*", "MultiMeshInstance3D", true, false)
	if pools.is_empty():
		fail("%s: no particle pools in the world" % name)
		return false
	for pool in pools:
		var material := (pool as GeometryInstance3D).material_override as ShaderMaterial
		var mode = material.get_shader_parameter("fog_mode")
		var range = material.get_shader_parameter("fog_range")
		var color = material.get_shader_parameter("fog_color")
		if mode != 1 or range != want or color != want_color:
			fail("%s: pool %s fog mode %s range %s colour %s, scene fog %s %s" % [name, pool.name, mode, range, color, want, want_color])
			return false
	print("FIXTURE PARTICLE_FOG %s pools=%d fog_range=%s fog_color=%s" % [name, pools.size(), want, want_color])
	return true
