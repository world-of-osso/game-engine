extends SceneTree

# Standalone GPU stage contract, not a full-scene equivalence test.
# Optional captures: SKYBOX_LINEAR_COMBINE_SHOTS=/fixture/owned/directory.
const SHADER_PATH: String = "res://shaders/sky_m2.gdshader"
const SIZE: int = 64
const CODE_TOLERANCE: int = 2
const DEADLINE_SECONDS: float = 20.0
const FIRST: Vector3i = Vector3i(64, 96, 128)
const SECOND: Vector3i = Vector3i(64, 96, 128)
const SAMPLE_POINTS: Array[Vector2i] = [
	Vector2i(28, 28), Vector2i(32, 32), Vector2i(36, 36),
]

var viewport: SubViewport
var material: ShaderMaterial
var quad: MeshInstance3D
var finished: bool = false
var shots_directory: String = ""

func _initialize() -> void:
	create_timer(DEADLINE_SECONDS).timeout.connect(_on_deadline)
	call_deferred("run_cases")

func _on_deadline() -> void:
	if not finished:
		fail("deadline exceeded while waiting for actual GPU output")

func fail(message: String) -> void:
	finished = true
	push_error("skybox_linear_combine: " + message)
	quit(1)

func constant_texture(codes: Vector3i) -> ImageTexture:
	var bytes: PackedByteArray = PackedByteArray([codes.x, codes.y, codes.z, 255])
	var image: Image = Image.create_from_data(1, 1, false, Image.FORMAT_RGBA8, bytes)
	return ImageTexture.create_from_image(image)

# Independent standard sRGB transfer functions; no production shader helpers.
func decode_srgb(code: int) -> float:
	var encoded: float = float(code) / 255.0
	if encoded <= 0.04045:
		return encoded / 12.92
	return pow((encoded + 0.055) / 1.055, 2.4)

func encode_srgb(linear: float) -> int:
	var encoded: float
	if linear <= 0.0031308:
		encoded = linear * 12.92
	else:
		encoded = 1.055 * pow(linear, 1.0 / 2.4) - 0.055
	return int(round(encoded * 255.0))

func expected_product() -> Vector3i:
	return Vector3i(
		encode_srgb(decode_srgb(FIRST.x) * decode_srgb(SECOND.x)),
		encode_srgb(decode_srgb(FIRST.y) * decode_srgb(SECOND.y)),
		encode_srgb(decode_srgb(FIRST.z) * decode_srgb(SECOND.z))
	)

func build_fixture(shader: Shader) -> void:
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

	material = ShaderMaterial.new()
	material.shader = shader
	material.set_shader_parameter("base_texture", constant_texture(FIRST))
	material.set_shader_parameter("second_texture", constant_texture(SECOND))
	material.set_shader_parameter("third_texture", constant_texture(Vector3i(255, 255, 255)))
	material.set_shader_parameter("fourth_texture", constant_texture(Vector3i(255, 255, 255)))
	material.set_shader_parameter("combine_mode", 0xE)
	material.set_shader_parameter("has_second_texture", false)
	material.set_shader_parameter("has_third_texture", false)
	material.set_shader_parameter("has_fourth_texture", false)
	material.set_shader_parameter("transparency", 1.0)
	material.set_shader_parameter("base_color", Color.WHITE)
	material.set_shader_parameter("alpha_test", 0.0)
	material.set_shader_parameter("uv_mode_1", 0)
	material.set_shader_parameter("uv_mode_2", 0)
	material.set_shader_parameter("uv_mode_3", 0)
	material.set_shader_parameter("uv_mode_4", 0)
	material.set_shader_parameter("uv_offset_1", Vector2.ZERO)
	material.set_shader_parameter("uv_offset_2", Vector2.ZERO)

	var mesh: QuadMesh = QuadMesh.new()
	mesh.size = Vector2(2.0, 2.0)
	quad = MeshInstance3D.new()
	quad.mesh = mesh
	quad.material_override = material
	# QuadMesh's +Z face points toward the camera at the origin.
	quad.position = Vector3(0.0, 0.0, -2.0)
	viewport.add_child(quad)

func encoded_pixel(image: Image, point: Vector2i) -> Vector4i:
	var color: Color = image.get_pixelv(point)
	return Vector4i(
		int(round(color.r * 255.0)), int(round(color.g * 255.0)),
		int(round(color.b * 255.0)), int(round(color.a * 255.0))
	)

func check_case(label: String, has_second: bool, expected: Vector3i) -> bool:
	material.set_shader_parameter("has_second_texture", has_second)
	# Two completed draws avoid observing the preceding material state.
	for _frame in range(2):
		await process_frame
		await RenderingServer.frame_post_draw
	var image: Image = viewport.get_texture().get_image()
	if image == null or image.is_empty():
		fail(label + ": missing actual GPU image")
		return false
	if image.get_size() != Vector2i(SIZE, SIZE):
		fail("%s: expected 64x64 output, got %s" % [label, image.get_size()])
		return false
	if not shots_directory.is_empty():
		var path: String = shots_directory.path_join(label + ".png")
		var error: Error = image.save_png(path)
		if error != OK:
			fail("%s: capture failed (%s): %s" % [label, error, path])
			return false

	var passed: bool = quad.is_visible_in_tree()
	# Background outside the projected quad must remain black and opaque.
	var background: Vector4i = encoded_pixel(image, Vector2i(2, 2))
	passed = passed and background == Vector4i(0, 0, 0, 255)
	print("%s: extent=%s visible=%s background=%s tolerance=2 codes" % [
		label, image.get_size(), quad.is_visible_in_tree(), background])
	for point in SAMPLE_POINTS:
		var actual: Vector4i = encoded_pixel(image, point)
		var pixel_passed: bool = (
			absi(actual.x - expected.x) <= CODE_TOLERANCE
			and absi(actual.y - expected.y) <= CODE_TOLERANCE
			and absi(actual.z - expected.z) <= CODE_TOLERANCE
			and actual.w == 255
		)
		passed = passed and pixel_passed
		print("%s pixel=%s expected=%s actual=%s %s" % [
			label, point, Vector4i(expected.x, expected.y, expected.z, 255),
			actual, "PASS" if pixel_passed else "FAIL"])
	print("%s: %s" % [label, "PASS" if passed else "FAIL"])
	return passed

func run_cases() -> void:
	if not ResourceLoader.exists(SHADER_PATH):
		fail("missing actual production shader: " + SHADER_PATH)
		return
	var shader: Shader = ResourceLoader.load(SHADER_PATH) as Shader
	if shader == null:
		fail("cannot load actual production shader: " + SHADER_PATH)
		return
	shots_directory = OS.get_environment("SKYBOX_LINEAR_COMBINE_SHOTS")
	if not shots_directory.is_empty():
		var error: Error = DirAccess.make_dir_recursive_absolute(shots_directory)
		if error != OK:
			fail("cannot create capture directory (%s): %s" % [error, shots_directory])
			return
	build_fixture(shader)
	var control_passed: bool = await check_case("single_stage", false, FIRST)
	if finished:
		return
	var combine_passed: bool = await check_case("linear_product_0xE", true, expected_product())
	if finished:
		return
	if not control_passed or not combine_passed:
		fail("GPU stage contract failed; control=%s combine=%s" % [control_passed, combine_passed])
		return
	finished = true
	print("PASS: skybox linear-stage contract (not full-scene parity)")
	quit(0)
