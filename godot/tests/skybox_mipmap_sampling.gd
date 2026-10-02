extends SceneTree

# Native authored-level regression: explicit control and production sky sampler
# consume uploaded smaller mips. No cross-engine pixel-parity requirement.
# MAIN runs with a real GPU; optional SKYBOX_MIPMAP_SHOTS saves both 64x64 PNGs.
const SHADER_PATH: String = "res://shaders/sky_m2.gdshader"
const SIZE: int = 64
const LEVEL_SIZES: Array[int] = [64, 32, 16, 8, 4, 2, 1]
const BASE_CODES: Vector3i = Vector3i(128, 0, 0)
const MIP_CODES: Vector3i = Vector3i(0, 128, 0)
const CODE_TOLERANCE: int = 2
const DEADLINE_SECONDS: float = 20.0
const SAMPLE_POINTS: Array[Vector2i] = [
	Vector2i(28, 28), Vector2i(32, 32), Vector2i(36, 36),
]

var viewport: SubViewport
var quad: MeshInstance3D
var texture: ImageTexture
var finished: bool = false
var control_passed: bool = false
var shots_directory: String = ""

func _initialize() -> void:
	create_timer(DEADLINE_SECONDS).timeout.connect(_on_deadline)
	call_deferred("run_cases")

func _on_deadline() -> void:
	if not finished:
		fail("RED" if control_passed else "SETUP", "20s deadline waiting for actual GPU output")

func fail(category: String, message: String) -> void:
	finished = true
	push_error("skybox_mipmap_sampling %s: %s" % [category, message])
	quit(1)

func create_authored_texture() -> ImageTexture:
	var bytes: PackedByteArray = PackedByteArray()
	for level in range(LEVEL_SIZES.size()):
		var size: int = LEVEL_SIZES[level]
		var codes: Vector3i = BASE_CODES if level == 0 else MIP_CODES
		var level_bytes: PackedByteArray = PackedByteArray()
		level_bytes.resize(size * size * 4)
		for pixel in range(size * size):
			var offset: int = pixel * 4
			level_bytes[offset] = codes.x
			level_bytes[offset + 1] = codes.y
			level_bytes[offset + 2] = codes.z
			level_bytes[offset + 3] = 255
		bytes.append_array(level_bytes)
	if bytes.size() != 21844:
		fail("SETUP", "incorrect complete RGBA8 mip-chain byte count: %s" % bytes.size())
		return null
	# Upload authored levels verbatim: never generate/replace their green contents.
	var image: Image = Image.create_from_data(SIZE, SIZE, true, Image.FORMAT_RGBA8, bytes)
	if image == null or image.is_empty() or not image.has_mipmaps() or image.get_mipmap_count() != 6:
		fail("SETUP", "cannot construct all seven authored mip levels")
		return null
	var uploaded: ImageTexture = ImageTexture.create_from_image(image)
	if uploaded == null:
		fail("SETUP", "cannot upload authored mip texture")
		return null
	var uploaded_image: Image = uploaded.get_image()
	if uploaded_image == null or uploaded_image.is_empty() or not uploaded_image.has_mipmaps():
		fail("SETUP", "uploaded texture readback lacks authored mipmaps")
		return null
	print("SYNTHETIC_TEXTURE extent=", uploaded_image.get_size(),
		" format=RGBA8 bytes=", bytes.size(), " authored_levels=", LEVEL_SIZES,
		" image.has_mipmaps=", image.has_mipmaps(),
		" texture.get_image().has_mipmaps=", uploaded_image.has_mipmaps(),
		" mipmap_count=", uploaded_image.get_mipmap_count(),
		" base=", BASE_CODES, " smaller_levels=", MIP_CODES, " alpha=255")
	return uploaded

func build_fixture() -> void:
	viewport = SubViewport.new()
	viewport.size = Vector2i(SIZE, SIZE)
	viewport.own_world_3d = true
	viewport.transparent_bg = false
	viewport.msaa_3d = Viewport.MSAA_DISABLED
	viewport.use_taa = false
	viewport.screen_space_aa = Viewport.SCREEN_SPACE_AA_DISABLED
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(viewport)

	var environment: Environment = Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color.BLACK
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_DISABLED
	environment.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	environment.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	environment.tonemap_exposure = 1.0
	environment.glow_enabled = false
	environment.fog_enabled = false
	environment.volumetric_fog_enabled = false
	var world_environment: WorldEnvironment = WorldEnvironment.new()
	world_environment.environment = environment
	viewport.add_child(world_environment)

	var camera: Camera3D = Camera3D.new()
	camera.fov = 90.0
	camera.near = 0.1
	camera.far = 100.0
	viewport.add_child(camera)
	camera.current = true

	var source_quad: QuadMesh = QuadMesh.new()
	source_quad.size = Vector2(2.0, 2.0)
	var arrays: Array = source_quad.get_mesh_arrays()
	var uvs: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV]
	for index in range(uvs.size()):
		uvs[index] *= 64.0
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	var mesh: ArrayMesh = ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	quad = MeshInstance3D.new()
	quad.mesh = mesh
	quad.position = Vector3(0.0, 0.0, -2.0)
	viewport.add_child(quad)

func create_control_material() -> StandardMaterial3D:
	var material: StandardMaterial3D = StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS
	material.texture_repeat = true
	material.albedo_color = Color.WHITE
	material.albedo_texture = texture
	print("CONTROL_BINDING albedo_texture=same synthetic ImageTexture; unshaded; explicit LINEAR_WITH_MIPMAPS; repeat=true")
	return material

func create_production_material(shader: Shader) -> ShaderMaterial:
	var material: ShaderMaterial = ShaderMaterial.new()
	material.shader = shader
	material.set_shader_parameter("base_texture", texture)
	# Bind valid textures even to disabled optional stages; only base is consumed.
	for parameter in ["second_texture", "third_texture", "fourth_texture"]:
		material.set_shader_parameter(parameter, texture)
	for parameter in ["has_second_texture", "has_third_texture", "has_fourth_texture"]:
		material.set_shader_parameter(parameter, false)
	for parameter in ["uv_mode_1", "uv_mode_2", "uv_mode_3", "uv_mode_4"]:
		material.set_shader_parameter(parameter, 0)
	material.set_shader_parameter("uv_offset_1", Vector2.ZERO)
	material.set_shader_parameter("uv_offset_2", Vector2.ZERO)
	material.set_shader_parameter("combine_mode", 0)
	material.set_shader_parameter("transparency", 1.0)
	material.set_shader_parameter("base_color", Color.WHITE)
	material.set_shader_parameter("alpha_test", 0.0)
	print("PRODUCTION_BINDING shader=", SHADER_PATH,
		" base_texture=same synthetic ImageTexture; optional textures bound but all stages=false; no sampler override; uv_modes/offsets=0; base_white; transparency=1")
	return material

func encoded_pixel(image: Image, point: Vector2i) -> Vector4i:
	var color: Color = image.get_pixelv(point)
	return Vector4i(
		int(round(color.r * 255.0)), int(round(color.g * 255.0)),
		int(round(color.b * 255.0)), int(round(color.a * 255.0))
	)

func check_case(label: String, material: Material) -> bool:
	quad.material_override = material
	# Observe two actual completed draws, not sleeps or caller-driven stage advances.
	for _draw in range(2):
		await RenderingServer.frame_post_draw
		if finished:
			return false
	var image: Image = viewport.get_texture().get_image()
	if image == null or image.is_empty():
		fail("SETUP", label + ": missing actual GPU image")
		return false
	if image.get_size() != Vector2i(SIZE, SIZE):
		fail("SETUP", "%s: expected 64x64 output, got %s" % [label, image.get_size()])
		return false
	if not shots_directory.is_empty():
		var path: String = shots_directory.path_join(label + ".png")
		var error: Error = image.save_png(path)
		if error != OK:
			fail("SETUP", "%s: PNG save failed (%s): %s" % [label, error, path])
			return false
		print("CAPTURE path=", path, " png_extent=", image.get_size())
	var background: Vector4i = encoded_pixel(image, Vector2i(2, 2))
	var passed: bool = quad.is_visible_in_tree() and background == Vector4i(0, 0, 0, 255)
	print(label, " output_extent=", image.get_size(), " output.has_mipmaps=", image.has_mipmaps(),
		" visible=", quad.is_visible_in_tree(), " background=", background, " tolerance=2 codes")
	for point in SAMPLE_POINTS:
		var actual: Vector4i = encoded_pixel(image, point)
		var pixel_passed: bool = (
			absi(actual.x - MIP_CODES.x) <= CODE_TOLERANCE
			and absi(actual.y - MIP_CODES.y) <= CODE_TOLERANCE
			and absi(actual.z - MIP_CODES.z) <= CODE_TOLERANCE
			and actual.w == 255
		)
		passed = passed and pixel_passed
		print(label, " pixel=", point, " expected_green=", Vector4i(0, 128, 0, 255),
			" actual=", actual, " ", "PASS" if pixel_passed else "FAIL")
	return passed

func run_cases() -> void:
	shots_directory = OS.get_environment("SKYBOX_MIPMAP_SHOTS")
	if not shots_directory.is_empty():
		var error: Error = DirAccess.make_dir_recursive_absolute(shots_directory)
		if error != OK:
			fail("SETUP", "cannot create capture directory (%s): %s" % [error, shots_directory])
			return
	texture = create_authored_texture()
	if finished:
		return
	build_fixture()
	control_passed = await check_case("explicit_mipmap_control", create_control_material())
	if finished:
		return
	if not control_passed:
		fail("SETUP", "explicit mip-filter control failed; production sampler behavior not evaluated")
		return
	if not ResourceLoader.exists(SHADER_PATH):
		fail("SETUP", "missing actual production shader: " + SHADER_PATH)
		return
	var shader: Shader = ResourceLoader.load(SHADER_PATH) as Shader
	if shader == null:
		fail("SETUP", "cannot load actual production shader: " + SHADER_PATH)
		return
	print("SOURCE shader_sha256=", FileAccess.get_sha256(SHADER_PATH),
		" script_sha256=", FileAccess.get_sha256("res://tests/skybox_mipmap_sampling.gd"))
	var production_passed: bool = await check_case("production_sky_sampler", create_production_material(shader))
	if finished:
		return
	if not production_passed:
		fail("RED", "control passed; actual production sky sampler failed authored-green-mip oracle")
		return
	finished = true
	print("PASS: explicit mipmap control and production sky sampler consume authored smaller mips; native synthetic sampler regression, not cross-engine pixel parity")
	quit(0)
