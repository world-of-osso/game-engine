extends "res://tests/capture_world_view.gd"

## Diagnostic: shots as capture_world_view.gd, plus every terrain water surface's shader
## inputs and the screen colour at VIEW_PROBE_PIXELS ("x:y" joined by ";").

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
	for entry in OS.get_environment("VIEW_PLAN").split(";", false):
		var fields := entry.split(",")
		client.set_world_minutes(float(fields[1]))
		client.set_camera_orbit(float(fields[2]), float(fields[3]), float(fields[4]))
		await wait_real(float(fields[5]))
		dump_water(fields[0])
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		image.save_png(shots.path_join(fields[0] + ".png"))
		probe(fields[0] + "/all", image)
		# Attribution: the same frame without the specular term, then also without foam.
		set_ocean("specular_color", Vector3.ZERO)
		await wait_frames(3)
		await RenderingServer.frame_post_draw
		image = root.get_texture().get_image()
		image.save_png(shots.path_join(fields[0] + "_nospec.png"))
		probe(fields[0] + "/nospec", image)
		set_ocean("floats_4", null)
		await wait_frames(3)
		await RenderingServer.frame_post_draw
		image = root.get_texture().get_image()
		image.save_png(shots.path_join(fields[0] + "_nospec_nofoam.png"))
		probe(fields[0] + "/nospec_nofoam", image)
	print("FIXTURE OCEAN_PROBE_DONE")
	client.free()
	quit(0)

func dump_water(shot: String) -> void:
	var seen := {}
	for water in client.find_children("Water", "Node3D", true, false):
		for child in water.get_children():
			var mesh := child as MeshInstance3D
			if mesh == null:
				continue
			var material := mesh.get_surface_override_material(0) as ShaderMaterial
			if material == null or seen.has(material.get_instance_id()):
				continue
			seen[material.get_instance_id()] = true
			var line := "FIXTURE WATER %s %s/%s" % [shot, water.get_parent().name, mesh.name]
			for name in ["color_source", "floats_8", "ocean_close", "ocean_far", "river_close", "river_far", "ambient", "direct", "specular_color", "sun_direction", "underwater_fog", "underwater_fog_color"]:
				line += " %s=%s" % [name, material.get_shader_parameter(name)]
			print(line)

## Water and terrain only: start once the terrain is in, without every object.
func wait_objects(_deadline: int) -> bool:
	return true

func probe(label: String, image: Image) -> void:
	for spec in OS.get_environment("VIEW_PROBE_PIXELS").split(";", false):
		var xy := spec.split(":")
		var c := image.get_pixel(int(xy[0]), int(xy[1]))
		print("FIXTURE PIXEL %s %s %d %d %d" % [label, spec, c.r8, c.g8, c.b8])

## Sets `name` on every ocean surface; floats_4 null zeroes only f7 (foam).
func set_ocean(name: String, value) -> void:
	for water in client.find_children("Water", "Node3D", true, false):
		for child in water.get_children():
			var mesh := child as MeshInstance3D
			var material := mesh.get_surface_override_material(0) as ShaderMaterial if mesh else null
			if material == null or int(material.get_shader_parameter("color_source")) != 0:
				continue
			if value == null:
				var f: Vector4 = material.get_shader_parameter(name)
				material.set_shader_parameter(name, Vector4(f.x, f.y, f.z, 0.0))
			else:
				material.set_shader_parameter(name, value)
