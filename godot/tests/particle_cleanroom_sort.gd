extends "res://tests/m2_loader_pixels.gd"

# Actual EmitterSim draw order -> ParticlePool -> native alpha/additive raster.
# Orthogonal camera makes near/far heads overlap at identical screen coordinates.
var stage: Node3D
var camera3d: Camera3D
var probe: Object
var particle: Node3D
var output := ""

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	output = OS.get_environment("GODOT_PARTICLE_SCREENSHOTS")
	if output.is_empty() or not DirAccess.dir_exists_absolute(output):
		push_error("Existing GODOT_PARTICLE_SCREENSHOTS directory required")
		quit(1)
		return
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(FIXTURE + "/textures"))
	if not write_fixture(FIXTURE + "/textures/910001.blp", make_blp(Color.WHITE)):
		quit(1)
		return
	probe = ClassDB.instantiate("WowParticleProbe")
	stage = Node3D.new()
	root.add_child(stage)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color.BLACK
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_DISABLED
	environment.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	environment.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	var world := WorldEnvironment.new()
	world.environment = environment
	stage.add_child(world)
	camera3d = Camera3D.new()
	camera3d.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera3d.size = 4.0
	stage.add_child(camera3d)
	camera3d.current = true
	var passed := true
	for blend in [2, 4]:
		for sorted in [false, true]:
			for reverse in [false, true]:
				passed = await check_case(blend, sorted, reverse) and passed
	stage.free()
	probe = null
	cleanup()
	print("PASS: P6 native overlap controls" if passed else "FAIL: P6 native overlap controls")
	quit(0 if passed else 1)

func check_case(blend: int, sorted: bool, reverse: bool) -> bool:
	if particle != null and is_instance_valid(particle):
		particle.free()
	camera3d.look_at_from_position(Vector3(0.0, 0.0, -5.0 if reverse else 5.0), Vector3.ZERO)
	var result = probe.overlap_node(FIXTURE + "/textures", sorted, blend, reverse)
	if not result is Node3D:
		push_error("Overlap node: " + str(result))
		return false
	particle = result
	stage.add_child(particle)
	await process_frame
	await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var label := "p6-blend%d-sort%d-reverse%d" % [blend, int(sorted), int(reverse)]
	if image.get_size() != Vector2i(1920, 1080) or image.save_png(output + "/" + label + ".png") != OK:
		push_error(label + " capture failed")
		return false
	# Two alpha=.5 heads over black: last contributes .5, first .25. Particles blend
	# authored (gamma) values (particle_composite_effect.gd), so these are the
	# displayed values.
	var blue_last := sorted and not reverse
	var expected := Color(0.25, 0.0, 0.5) if blue_last else Color(0.5, 0.0, 0.25)
	if blend == 4:
		expected = Color(0.5, 0.0, 0.5)
	var actual := image.get_pixel(960, 540)
	var passed := absf(actual.r-expected.r) <= TOLERANCE and absf(actual.g-expected.g) <= TOLERANCE and absf(actual.b-expected.b) <= TOLERANCE
	print(label, " expected=", expected, " actual=", actual, " passed=", passed)
	if not passed:
		push_error(label + " incorrect overlap color")
	return passed
