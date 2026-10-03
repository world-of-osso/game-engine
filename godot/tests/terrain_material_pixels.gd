extends SceneTree

const SHADER_PATH := "res://shaders/terrain.gdshader"
const SIZE := 64
const TOLERANCE := 0.015

var viewport: SubViewport
var material: ShaderMaterial
var terrain_instance: MeshInstance3D

func _initialize() -> void:
	call_deferred("_run")

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func solid_texture(color: Color) -> ImageTexture:
	var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	image.fill(color)
	return ImageTexture.create_from_image(image)

func striped_texture() -> ImageTexture:
	var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	for row in 2:
		image.set_pixel(0, row, Color(0.2, 0.2, 0.2))
		image.set_pixel(1, row, Color(0.8, 0.8, 0.8))
	return ImageTexture.create_from_image(image)

func constant_float_cubemap(color: Color) -> Cubemap:
	var faces: Array[Image] = []
	for face in 6:
		var image := Image.create(2, 2, false, Image.FORMAT_RGBAH)
		image.fill(color)
		faces.append(image)
	var cube := Cubemap.new()
	if cube.create_from_images(faces) != OK:
		fail("Could not create linear float cubemap")
	return cube

func set_vertex_color(color: Color) -> void:
	var arrays := terrain_instance.mesh.surface_get_arrays(0)
	var colors := PackedColorArray()
	for index in arrays[Mesh.ARRAY_VERTEX].size():
		colors.append(color)
	arrays[Mesh.ARRAY_COLOR] = colors
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	terrain_instance.mesh = mesh

func fixture_scene() -> void:
	viewport = SubViewport.new()
	viewport.size = Vector2i(SIZE, SIZE)
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	viewport.own_world_3d = true
	root.add_child(viewport)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color.BLACK
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_DISABLED
	environment.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	environment.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	viewport.add_child(world_environment)
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 2.0
	camera.position = Vector3(0.0, 2.0, 0.0)
	camera.look_at_from_position(camera.position, Vector3.ZERO, Vector3(0.0, 0.0, -1.0))
	viewport.add_child(camera)
	camera.current = true
	var sun := DirectionalLight3D.new()
	sun.rotation.x = -PI / 2.0
	sun.light_energy = 1.0
	viewport.add_child(sun)
	var plane := PlaneMesh.new()
	plane.size = Vector2(2.0, 2.0)
	var arrays := plane.surface_get_arrays(0)
	var colors := PackedColorArray()
	for index in arrays[Mesh.ARRAY_VERTEX].size():
		colors.append(Color(0.5, 0.5, 0.5, 1.0))
	arrays[Mesh.ARRAY_COLOR] = colors
	var uvs := PackedVector2Array()
	for index in colors.size():
		uvs.append(Vector2(0.25, 0.5))
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var instance := MeshInstance3D.new()
	instance.mesh = mesh
	instance.material_override = material
	viewport.add_child(instance)
	terrain_instance = instance

func set_base_inputs() -> void:
	material.set_shader_parameter("config", Vector4(4.0, 0.0, 1.0, 0.0))
	material.set_shader_parameter("animation_time", 0.0)
	material.set_shader_parameter("layer_params_0", Vector4(1.0, 0.0, 0.0, 1.0))
	material.set_shader_parameter("layer_params_1", Vector4(1.0, 0.0, 0.0, 1.0))
	material.set_shader_parameter("layer_params_2", Vector4(1.0, 0.0, 0.0, 1.0))
	material.set_shader_parameter("layer_params_3", Vector4(1.0, 0.0, 0.0, 1.0))
	for index in 4:
		material.set_shader_parameter("animation_params_%d" % index, Vector4.ZERO)
		material.set_shader_parameter("height_%d" % index, solid_texture(Color(0.0, 0.0, 0.0, 0.5)))
	var ground := [Color(0.2, 0.1, 0.05, 0.0), Color(0.6, 0.2, 0.1, 0.0), Color(0.1, 0.7, 0.3, 0.0), Color(0.9, 0.8, 0.2, 0.0)]
	for index in 4:
		material.set_shader_parameter("ground_%d" % index, solid_texture(ground[index]))
	material.set_shader_parameter("alpha_packed", solid_texture(Color(0.25, 0.5, 0.125, 1.0)))
	material.set_shader_parameter("ambient", Vector3.ONE)
	material.set_shader_parameter("horizon_ambient", Vector3.ONE)
	material.set_shader_parameter("ground_ambient", Vector3.ONE)
	material.set_shader_parameter("direct", Vector3.ZERO)
	material.set_shader_parameter("sun_direction", Vector3.DOWN)
	RenderingServer.global_shader_parameter_set("fog_color", Vector3.ZERO)
	RenderingServer.global_shader_parameter_set("fog_range", Vector2.ZERO)
	material.set_shader_parameter("fog_opacity", 0.0)

func sample_pixel() -> Color:
	await process_frame
	await RenderingServer.frame_post_draw
	await process_frame
	await RenderingServer.frame_post_draw
	var image := viewport.get_texture().get_image()
	if image == null or image.is_empty():
		fail("GPU pixel readback unavailable")
		return Color.BLACK
	return image.get_pixel(SIZE / 2, SIZE / 2)

func assert_pixel(label: String, expected: Color) -> bool:
	var actual := await sample_pixel()
	if absf(actual.r - expected.r) > TOLERANCE or absf(actual.g - expected.g) > TOLERANCE or absf(actual.b - expected.b) > TOLERANCE:
		fail("%s: expected %s, got %s" % [label, expected, actual])
		return false
	print("PASS: ", label, " ", actual)
	return true

func _run() -> void:
	if not ResourceLoader.exists(SHADER_PATH):
		fail("Terrain shader missing: " + SHADER_PATH)
		return
	material = ShaderMaterial.new()
	material.shader = load(SHADER_PATH)
	fixture_scene()
	set_base_inputs()
	# MCCV input 0.5 is decoded as byte / 127 (approximately 1.004).
	var tint := (0.5 * 255.0 / 127.0)
	var ground: Array[Color] = [Color(0.2, 0.1, 0.05), Color(0.6, 0.2, 0.1), Color(0.1, 0.7, 0.3), Color(0.9, 0.8, 0.2)]
	var layered: Color = ground[0] * (0.75 * 0.5 * 0.875) + ground[1] * (0.25 * 0.5 * 0.875) + ground[2] * (0.5 * 0.875) + ground[3] * 0.125
	if not await assert_pixel("layered MCAL and MCCV", layered * tint * 1.1):
		return
	material.set_shader_parameter("config", Vector4(4.0, 1.0, 1.0, 0.0))
	var weighted: Color = ground[0] * 0.125 + ground[1] * 0.25 + ground[2] * 0.5 + ground[3] * 0.125
	if not await assert_pixel("weighted MCAL", weighted * tint * 1.1):
		return
	material.set_shader_parameter("config", Vector4(4.0, 2.0, 1.0, 0.0))
	material.set_shader_parameter("height_0", solid_texture(Color(0.0, 0.0, 0.0, 0.8)))
	material.set_shader_parameter("height_1", solid_texture(Color(0.0, 0.0, 0.0, 0.2)))
	material.set_shader_parameter("height_2", solid_texture(Color(0.0, 0.0, 0.0, 0.4)))
	material.set_shader_parameter("height_3", solid_texture(Color(0.0, 0.0, 0.0, 0.9)))
	var height_weights := Vector4(0.125 * 0.8, 0.25 * 0.2, 0.5 * 0.4, 0.125 * 0.9)
	var max_height := maxf(maxf(height_weights.x, height_weights.y), maxf(height_weights.z, height_weights.w))
	var kept := Vector4(
		height_weights.x * (1.0 - clampf(max_height - height_weights.x, 0.0, 1.0)),
		height_weights.y * (1.0 - clampf(max_height - height_weights.y, 0.0, 1.0)),
		height_weights.z * (1.0 - clampf(max_height - height_weights.z, 0.0, 1.0)),
		height_weights.w * (1.0 - clampf(max_height - height_weights.w, 0.0, 1.0))
	)
	var normalized: Color = (ground[0] * kept.x + ground[1] * kept.y + ground[2] * kept.z + ground[3] * kept.w) / (kept.x + kept.y + kept.z + kept.w)
	if not await assert_pixel("height-weighted MCAL", normalized * tint * 1.1):
		return
	# Nonlinear gamma conversion must be applied to the complete authored ambient + direct + specular sum.
	material.set_shader_parameter("config", Vector4(1.0, 0.0, 1.0, 0.0))
	material.set_shader_parameter("ground_0", solid_texture(Color(0.25, 0.25, 0.25, 0.5)))
	material.set_shader_parameter("ambient", Vector3(0.25, 0.25, 0.25))
	material.set_shader_parameter("horizon_ambient", Vector3(0.25, 0.25, 0.25))
	material.set_shader_parameter("ground_ambient", Vector3(0.25, 0.25, 0.25))
	material.set_shader_parameter("direct", Vector3(0.25, 0.25, 0.25))
	var authored := 0.25 * tint * 0.5 * 1.1 + 0.5 * 0.25
	if not await assert_pixel("authored-space lighting/spec then gamma", Color(authored, authored, authored)):
		return
	material.set_shader_parameter("direct", Vector3.ZERO)
	material.set_shader_parameter("ambient", Vector3.ONE)
	material.set_shader_parameter("horizon_ambient", Vector3.ONE)
	material.set_shader_parameter("ground_ambient", Vector3.ONE)
	material.set_shader_parameter("layer_params_0", Vector4(1.0, 0.0, 0.0, 1.5))
	if not await assert_pixel("layer overbright", Color(0.25 * tint * 1.5 * 1.1, 0.25 * tint * 1.5 * 1.1, 0.25 * tint * 1.5 * 1.1)):
		return
	material.set_shader_parameter("layer_params_0", Vector4(1.0, 0.0, 0.0, 1.0))
	set_vertex_color(Color(1.0, 1.0, 1.0))
	var full_mccv := 0.25 * 255.0 / 127.0 * 1.1
	if not await assert_pixel("MCCV byte255 retains multiplier above one", Color(full_mccv, full_mccv, full_mccv)):
		return
	set_vertex_color(Color(0.5, 0.5, 0.5))
	material.set_shader_parameter("fog_mode", 1)
	# Half fog at the fixture distance of 2: exp(-(2 - 1) * ln 2).
	RenderingServer.global_shader_parameter_set("fog_range", Vector2(1.0, 100.0))
	RenderingServer.global_shader_parameter_set("fog_density", log(2.0))
	material.set_shader_parameter("fog_opacity", 1.0)
	RenderingServer.global_shader_parameter_set("fog_color", Vector3(0.5, 0.0, 0.0))
	var fog_red := Color(0.5, 0.0, 0.0).linear_to_srgb().r
	var fogged := Color((0.25 * tint * 1.1 + fog_red) * 0.5, 0.25 * tint * 1.1 * 0.5, 0.25 * tint * 1.1 * 0.5)
	if not await assert_pixel("retail exponential fog in authored space", fogged):
		return
	# calculateLegacyFog end fade: 1.42857 * (1 - 2 / (2 / 0.65)) leaves half the colour.
	RenderingServer.global_shader_parameter_set("fog_density", 0.0)
	RenderingServer.global_shader_parameter_set("fog_range", Vector2(0.0, 2.0 / 0.65))
	if not await assert_pixel("retail fog end fade", fogged):
		return
	# Fog starts at fog_range.x: nothing is fogged before it.
	RenderingServer.global_shader_parameter_set("fog_density", log(2.0))
	RenderingServer.global_shader_parameter_set("fog_range", Vector2(2.0, 100.0))
	if not await assert_pixel("no terrain fog before fog start", Color(0.25 * tint * 1.1, 0.25 * tint * 1.1, 0.25 * tint * 1.1)):
		return
	RenderingServer.global_shader_parameter_set("fog_range", Vector2(1.0, 100.0))
	material.set_shader_parameter("fog_mode", 0)
	material.set_shader_parameter("ground_0", striped_texture())
	material.set_shader_parameter("animation_params_0", Vector4(0.5, 0.0, 0.0, 0.0))
	material.set_shader_parameter("animation_time", 0.0)
	if not await assert_pixel("animated UV start", Color(0.2 * tint * 1.1, 0.2 * tint * 1.1, 0.2 * tint * 1.1)):
		return
	material.set_shader_parameter("animation_time", 1.0)
	if not await assert_pixel("animated UV offset", Color(0.8 * tint * 1.1, 0.8 * tint * 1.1, 0.8 * tint * 1.1)):
		return
	material.set_shader_parameter("animation_time", 0.0)
	material.set_shader_parameter("animation_params_0", Vector4.ZERO)
	material.set_shader_parameter("config", Vector4(1.0, 0.0, 3.0, 0.0))
	if not await assert_pixel("UV repeat", Color(0.8 * tint * 1.1, 0.8 * tint * 1.1, 0.8 * tint * 1.1)):
		return
	material.set_shader_parameter("config", Vector4(1.0, 0.0, 1.0, 0.0))
	material.set_shader_parameter("ground_0", solid_texture(Color(0.25, 0.25, 0.25, 0.0)))
	material.set_shader_parameter("ambient", Vector3.ZERO)
	material.set_shader_parameter("horizon_ambient", Vector3.ZERO)
	material.set_shader_parameter("ground_ambient", Vector3.ZERO)
	material.set_shader_parameter("direct", Vector3(0.5, 0.5, 0.5))
	if not await assert_pixel("unshadowed directional direct", Color(0.25 * tint * 0.5, 0.25 * tint * 0.5, 0.25 * tint * 0.5)):
		return
	material.set_shader_parameter("ambient", Vector3.ONE)
	material.set_shader_parameter("horizon_ambient", Vector3.ONE)
	material.set_shader_parameter("ground_ambient", Vector3.ONE)
	material.set_shader_parameter("direct", Vector3.ZERO)
	material.set_shader_parameter("animation_params_0", Vector4(0.0, 0.0, 1.0, 0.0))
	material.set_shader_parameter("environment_map", constant_float_cubemap(Color(0.8, 0.8, 0.8)))
	var camera := viewport.get_camera_3d()
	camera.look_at_from_position(Vector3(0.0, 0.4, 2.0), Vector3.ZERO)
	var n_dot_view := 0.4 / Vector2(0.4, 2.0).length()
	var fresnel := pow(1.0 - n_dot_view, 4.0)
	var reflected := 0.25 * tint * (1.0 - fresnel) + Color(0.8, 0.8, 0.8).linear_to_srgb().r * fresnel
	if not await assert_pixel("linear float cubemap weighted Fresnel", Color(reflected * 1.1, reflected * 1.1, reflected * 1.1)):
		return
	quit(0)
