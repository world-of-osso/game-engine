extends SceneTree
## Same geometry/settings on Dozen and Lavapipe, with authored and standard lighting.

const FLAT := Rect2i(1100, 550, 100, 100)
const CONTACT := Rect2i(300, 440, 320, 80)
var meshes: Array[MeshInstance3D] = []

func _initialize() -> void:
	call_deferred("run_probe")

func run_probe() -> void:
	root.size = Vector2i(1280, 720)
	var directory := OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	if not directory.contains("/data/diagnostics/"):
		push_error("Probe requires owned diagnostic captures")
		quit(1)
		return
	DirAccess.make_dir_recursive_absolute(directory)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color(0.12, 0.12, 0.12)
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color.WHITE
	environment.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	environment.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	print("SSAO_DEFAULTS radius=", environment.ssao_radius, " intensity=", environment.ssao_intensity, " power=", environment.ssao_power, " light=", environment.ssao_light_affect, " ao=", environment.ssao_ao_channel_affect)
	var world := WorldEnvironment.new()
	world.environment = environment
	root.add_child(world)
	var camera := Camera3D.new()
	camera.fov = 60.0
	root.add_child(camera)
	camera.current = true
	root.add_child(DirectionalLight3D.new())
	var floor_mesh := PlaneMesh.new()
	floor_mesh.size = Vector2(60, 60)
	add_mesh(floor_mesh, Vector3(0, -2, -20))
	var box := BoxMesh.new()
	box.size = Vector3(4, 2, 4)
	add_mesh(box, Vector3(-3, -1, -12))
	for authored in [false, true]:
		var material: Material
		if authored:
			var terrain := ShaderMaterial.new()
			terrain.shader = load("res://shaders/terrain.gdshader")
			var image := Image.create(1, 1, false, Image.FORMAT_RGBA8)
			image.fill(Color(0.6, 0.6, 0.6, 1.0))
			terrain.set_shader_parameter("ground_0", ImageTexture.create_from_image(image))
			for parameter in ["ambient", "horizon_ambient", "ground_ambient"]:
				terrain.set_shader_parameter(parameter, Vector3.ONE * 0.6)
			material = terrain
		else:
			var standard := StandardMaterial3D.new()
			standard.albedo_color = Color(0.6, 0.6, 0.6)
			material = standard
		for mesh in meshes:
			mesh.material_override = material
		var prefix := "authored" if authored else "standard"
		environment.ssao_enabled = false
		if OS.get_environment("SSAO_PROBE_PREPASS") == "true":
			ProjectSettings.set_setting("rendering/driver/depth_prepass/enable", false)
		var off := await capture(directory, prefix + "-off")
		for affect in [Vector2(0, 0), Vector2(1, 0), Vector2(1, 1)]:
			if OS.get_environment("SSAO_PROBE_PREPASS") == "true":
				ProjectSettings.set_setting("rendering/driver/depth_prepass/enable", true)
			environment.ssao_enabled = true
			environment.ssao_light_affect = affect.x
			environment.ssao_ao_channel_affect = affect.y
			var suffix := "%s-on-light%d-ao%d" % [prefix, affect.x, affect.y]
			var on := await capture(directory, suffix)
			print(suffix, " flat_darkening=", darkening(off, on, FLAT), " contact_darkening=", darkening(off, on, CONTACT))
	quit(0)

func add_mesh(mesh: Mesh, position: Vector3) -> void:
	var node := MeshInstance3D.new()
	node.mesh = mesh
	node.position = position
	root.add_child(node)
	meshes.append(node)

func capture(directory: String, name: String) -> Image:
	for frame in range(20):
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(directory.path_join(name + ".png"))
	if error != OK:
		push_error("Probe capture failed: " + error_string(error))
		quit(1)
	return image

func darkening(before: Image, after: Image, region: Rect2i) -> float:
	var total := 0.0
	for y in range(region.position.y, region.end.y):
		for x in range(region.position.x, region.end.x):
			total += before.get_pixel(x, y).r - after.get_pixel(x, y).r
	return total / (region.size.x * region.size.y)
