extends "res://tests/m2_loader_pixels.gd"

# Synthetic only: asymmetric 3-D arrow geometry, never a replacement billboard.
func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	var probe = ClassDB.instantiate("WowParticleProbe")
	if not probe.has_method("model_recursion_node"):
		push_error("P5 missing native geometry/recursive emitter consumer")
		quit(1)
		return
	var stage := Node3D.new()
	root.add_child(stage)
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color.BLACK
	env.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	var environment := WorldEnvironment.new()
	environment.environment = env
	stage.add_child(environment)
	var camera3d := Camera3D.new()
	camera3d.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera3d.size = 8.0
	stage.add_child(camera3d)
	camera3d.look_at_from_position(Vector3(0, 0, 8), Vector3.ZERO)
	camera3d.current = true
	var passed := true
	for count in [1, 2]:
		var result = probe.model_recursion_node(stage, count, 0.05)
		if not result is Node3D:
			push_error("P5 native fixture failed: " + str(result))
			passed = false
			continue
		var node: Node3D = result
		var geometry := node.find_child("GeometryParticles", true, false) as MultiMeshInstance3D
		var children := node.find_child("RecursiveParticles", true, false) as MultiMeshInstance3D
		if geometry == null or children == null:
			push_error("P5 geometry and recursive outputs must be separate")
			passed = false
		else:
			passed = geometry.multimesh.visible_instance_count == count and passed
			passed = children.multimesh.visible_instance_count > count and passed
			var mesh := geometry.multimesh.mesh
			var vertices: PackedVector3Array = mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
			var depth := false
			for vertex in vertices:
				depth = depth or absf(vertex.z) > 0.01
			passed = depth and vertices.size() > 4 and passed
			passed = absf(float(node.get_meta("tagged_child_age")) - 0.55) < 0.00001 and passed
			passed = bool(node.get_meta("lifetime_state_pass")) and passed
			passed = bool(node.get_meta("retirement_and_drain_pass")) and passed
			passed = bool(node.get_meta("zero_duration_pass")) and passed
			await process_frame
			await process_frame
			await RenderingServer.frame_post_draw
			var image := root.get_texture().get_image()
			for index in geometry.multimesh.visible_instance_count:
				var origin := geometry.multimesh.get_instance_transform(index).origin
				var pixel := camera3d.unproject_position(origin)
				var color := image.get_pixel(roundi(pixel.x), roundi(pixel.y))
				passed = color.r > 0.1 and passed
			var output := OS.get_environment("GODOT_PARTICLE_SCREENSHOTS")
			passed = not output.is_empty() and image.save_png(output + "/p5-models-" + str(count) + ".png") == OK and passed
			print("P5 synthetic parents=", count, " geometry=", geometry.multimesh.visible_instance_count, " children=", children.multimesh.visible_instance_count, " passed=", passed)
		node.free()
	stage.free()
	probe = null
	cleanup()
	print("PASS: P5 synthetic geometry and recursive lifecycle" if passed else "FAIL: P5 synthetic geometry and recursive lifecycle")
	quit(0 if passed else 1)
