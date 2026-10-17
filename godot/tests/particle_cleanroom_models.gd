extends "res://tests/m2_loader_pixels.gd"

# Synthetic only: asymmetric 3-D arrow geometry, never a replacement billboard.
func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	var probe = ClassDB.instantiate("WowParticleProbe")
	if not probe.has_method("model_recursion_node"):
		push_error("P5 missing native geometry/recursive emitter consumer")
		quit(1)
		return
	if not prepare_fixture(7, 0, 1) or not write_fixture(FIXTURE + "/textures/910001.blp", make_blp(Color.WHITE)):
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
		var result = probe.model_recursion_node(stage, FIXTURE + "/models/quad.m2", count, 0.05)
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
			passed = absf(float(node.get_meta("tagged_child_age_delta")) - 0.05) < 0.00001 and passed
			var after := geometry.multimesh.get_instance_transform(0)
			var before_origin: Vector3 = node.get_meta("before_origin")
			var before_axis: Vector3 = node.get_meta("before_axis_x")
			var before_color: Color = node.get_meta("before_color")
			var after_color := geometry.multimesh.get_instance_color(0)
			passed = after.origin.distance_to(before_origin) > 0.05 and passed
			passed = after.basis.x.length() > before_axis.length() and passed
			passed = after.basis.x.normalized().distance_to(before_axis.normalized()) > 0.05 and passed
			passed = after_color.r < before_color.r and after_color.b > before_color.b and after_color.a < before_color.a and passed
			passed = int(node.get_meta("drained_parent_count")) == 0 and int(node.get_meta("drained_child_count")) == 0 and passed
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
		var geometry_id := geometry.get_instance_id() if geometry != null else 0
		var child_id := children.get_instance_id() if children != null else 0
		node.free()
		passed = not is_instance_id_valid(geometry_id) and not is_instance_id_valid(child_id) and passed
	stage.free()
	probe = null
	cleanup()
	print("PASS: P5 synthetic geometry and recursive lifecycle" if passed else "FAIL: P5 synthetic geometry and recursive lifecycle")
	quit(0 if passed else 1)
