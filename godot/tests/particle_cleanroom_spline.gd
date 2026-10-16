extends "res://tests/m2_loader_pixels.gd"

# P4 native placement and raster: source type3 header, controlled cubic payloads.
var stage: Node3D
var camera3d: Camera3D
var probe: Object

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(FIXTURE + "/textures"))
	if not write_fixture(FIXTURE + "/textures/910001.blp", make_blp(Color.WHITE)):
		quit(1)
		return
	stage = Node3D.new()
	root.add_child(stage)
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color.BLACK
	env.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	var environment := WorldEnvironment.new()
	environment.environment = env
	stage.add_child(environment)
	camera3d = Camera3D.new()
	camera3d.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera3d.size = 4.5
	stage.add_child(camera3d)
	camera3d.look_at_from_position(Vector3(1.5, 0.0, 6.0), Vector3(1.5, 0.0, 0.0))
	camera3d.current = true
	probe = ClassDB.instantiate("WowParticleProbe")
	var passed := await check_curve(1.0)
	passed = await check_curve(-1.0) and passed
	stage.free()
	probe = null
	cleanup()
	print("PASS: P4 native bent-curve placement and pixels" if passed else "FAIL: P4 native bent-curve placement and pixels")
	quit(0 if passed else 1)

func check_curve(sign: float) -> bool:
	# Generator axes map WoW(x,y,z) -> Godot(-y,z,-x).
	var controls := PackedVector3Array([Vector3.ZERO, Vector3(0.0, -1.0, sign*2.0), Vector3(0.0, -2.0, sign*2.0), Vector3(0.0, -3.0, 0.0)])
	var result = probe.spline_node(stage, FIXTURE + "/textures", controls)
	if not result is Node3D:
		push_error("Spline native placement failed: " + str(result))
		return false
	var node: Node3D = result
	var pool := node.find_child("Particles6316199_0", true, false) as MultiMeshInstance3D
	if pool == null or pool.multimesh.visible_instance_count <= 0:
		push_error("Type3 did not create a drawing native pool")
		node.free()
		return false
	await process_frame
	await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var output := OS.get_environment("GODOT_PARTICLE_SCREENSHOTS")
	var label := "p4-bent-up" if sign > 0.0 else "p4-bent-down"
	var passed := true
	var bent := 0
	var pixels := 0
	for index in pool.multimesh.visible_instance_count:
		var position := pool.multimesh.get_instance_transform(index).origin
		var t := position.x / 3.0
		var expected_y := sign * 6.0*t*(1.0-t)
		if absf(position.y - expected_y) > 0.0001 or absf(position.z) > 0.0001:
			push_error("Native spline point off cubic: " + str(position))
			passed = false
		if absf(position.y) > 0.5:
			bent += 1
		var pixel := camera3d.unproject_position(position)
		if image.get_pixel(roundi(pixel.x), roundi(pixel.y)).r > 0.5:
			pixels += 1
	if bent == 0 or pixels != pool.multimesh.visible_instance_count:
		push_error("Curve was not bent/visible: " + str([bent, pixels, pool.multimesh.visible_instance_count]))
		passed = false
	if image.get_size() != Vector2i(1920, 1080) or output.is_empty() or image.save_png(output + "/" + label + ".png") != OK:
		push_error("Spline capture failed")
		passed = false
	print(label, " native_particles=", pool.multimesh.visible_instance_count, " bent=", bent, " visible_centers=", pixels, " passed=", passed)
	node.free()
	return passed
