extends SceneTree

# Real fountain pixels under a moving camera and unrelated transparent draws.
# The independent skin reader supplies WebWowViewerCpp's ascending priority-plane
# order. Zero-alpha pads contribute no pixels, but exercise the full transparent
# sort rather than the isolated model's accidentally stable equal-key order.
const Oracle := preload("res://tests/m2_wwv_oracle.gd")
const SIZE := Vector2i(480, 270)
const POSES := 120
const BASIN := Rect2i(165, 145, 150, 55)

var loader: Object
var batches: Array[MeshInstance3D] = []
var camera: Camera3D
var evidence := ""

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = SIZE
	root.msaa_3d = Viewport.MSAA_DISABLED
	root.screen_space_aa = Viewport.SCREEN_SPACE_AA_DISABLED
	root.use_taa = false
	evidence = OS.get_environment("FOUNTAIN_OUTPUT")
	if not evidence.is_empty():
		DirAccess.make_dir_recursive_absolute(evidence)
	configure_scene()
	loader = ClassDB.instantiate("WowAssetLoader")
	var result: Dictionary = loader.load_m2("res://../data/models/366804.m2")
	if result.has("error"):
		fail(str(result.error))
		return
	var model: Node3D = result.node
	root.add_child(model)
	model.process_mode = Node.PROCESS_MODE_DISABLED
	for child in model.get_children():
		if child is MeshInstance3D:
			batches.append(child)
			configure_material(child.material_override)
	add_transparent_draws()
	camera = Camera3D.new()
	camera.fov = 65.0
	root.add_child(camera)
	camera.current = true
	var actual := await capture_path("actual")
	apply_reference_priorities(model)
	var expected := await capture_path("reference")
	if count_visible_water_pixels(expected[30]) < 100:
		fail("Reference did not render the fountain's blue water")
		return
	var failures := 0
	var worst_pixels := 0
	for pose in range(POSES):
		var mismatches := count_different_basin_pixels(actual[pose], expected[pose])
		worst_pixels = maxi(worst_pixels, mismatches)
		if mismatches > 0:
			failures += 1
			print("FOUNTAIN_ORDER_MISMATCH pose=", pose, " basin_pixels=", mismatches)
	print("FOUNTAIN_ORDER_RESULT poses=", POSES, " failures=", failures, " worst_basin_pixels=", worst_pixels)
	model.free()
	loader = null
	if failures > 0:
		fail("Fountain composition differs from authored priority order during the camera sweep")
		return
	print("FOUNTAIN_ORDER_PASS")
	quit(0)

func configure_scene() -> void:
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color(0.3, 0.3, 0.3)
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_DISABLED
	environment.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	environment.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	root.add_child(world_environment)
	var sun := DirectionalLight3D.new()
	sun.shadow_enabled = false
	root.add_child(sun)
	var clock: Node = ClassDB.instantiate("WowMaterialClock")
	clock.name = "M2MaterialClock"
	clock.process_mode = Node.PROCESS_MODE_DISABLED
	root.add_child(clock)
	clock.call("advance_time_ms", 700.0)

func configure_material(material: ShaderMaterial) -> void:
	material.set_shader_parameter("ambient", Vector3(0.4, 0.38, 0.36))
	material.set_shader_parameter("horizon_ambient", Vector3(0.3, 0.32, 0.36))
	material.set_shader_parameter("ground_ambient", Vector3(0.18, 0.16, 0.14))
	material.set_shader_parameter("direct", Vector3(0.55, 0.52, 0.48))
	material.set_shader_parameter("sun_direction", Vector3(-0.35, -0.85, -0.4))

func add_transparent_draws() -> void:
	var shader := Shader.new()
	shader.code = "shader_type spatial; render_mode unshaded; void fragment() { ALBEDO = vec3(0.0); ALPHA = 0.0; }"
	var material := ShaderMaterial.new()
	material.shader = shader
	var mesh := QuadMesh.new()
	mesh.size = Vector2(0.1, 0.1)
	mesh.material = material
	for index in range(128):
		var instance := MeshInstance3D.new()
		instance.mesh = mesh
		instance.position = Vector3(sin(index * 1.7) * 8.0, float(index % 17), cos(index * 2.3) * 8.0)
		root.add_child(instance)

func apply_reference_priorities(model: Node3D) -> void:
	var oracle := Oracle.new(FileAccess.get_file_as_bytes("res://../data/models/366804.m2"), FileAccess.get_file_as_bytes("res://../data/models/36680400.skin"))
	var order: Array = []
	for index in oracle.batch_count():
		var batch := oracle.batch(index)
		order.append([batch.priority_plane, batch.material_layer, index])
	order.sort_custom(func(a, b): return a[0] < b[0] or (a[0] == b[0] and (a[1] < b[1] or (a[1] == b[1] and a[2] < b[2]))))
	for index in order.size():
		var batch := model.get_node("Batch%d" % index) as MeshInstance3D
		batch.material_override.render_priority = order[index][0]

func capture_path(label: String) -> Array[Image]:
	var images: Array[Image] = []
	for pose in range(POSES):
		var phase := float(pose % 60) / 59.0
		var yaw := PI + (phase - 0.5) * 1.5
		var pitch := -0.35 if pose < 60 else -0.6 + phase * 0.5
		var focus := Vector3(0.0, 6.0, 0.0)
		camera.position = focus + Vector3(sin(yaw) * 30.0, -sin(pitch) * 30.0, cos(yaw) * 30.0)
		camera.look_at(focus)
		await process_frame
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		image.convert(Image.FORMAT_RGB8)
		images.append(image)
		if not evidence.is_empty():
			image.save_png(evidence + "/%s-%03d.png" % [label, pose])
	return images

func count_different_basin_pixels(actual: Image, expected: Image) -> int:
	var count := 0
	for y in range(BASIN.position.y, BASIN.end.y):
		for x in range(BASIN.position.x, BASIN.end.x):
			var a := actual.get_pixel(x, y)
			var b := expected.get_pixel(x, y)
			if maxf(absf(a.r - b.r), maxf(absf(a.g - b.g), absf(a.b - b.b))) > 2.0 / 255.0:
				count += 1
	return count

func count_visible_water_pixels(image: Image) -> int:
	var count := 0
	for y in range(BASIN.position.y, BASIN.end.y):
		for x in range(BASIN.position.x, BASIN.end.x):
			var pixel := image.get_pixel(x, y)
			if pixel.b > 0.35 and pixel.b - pixel.r > 0.1:
				count += 1
	return count

func fail(message: String) -> void:
	push_error(message)
	quit(1)
