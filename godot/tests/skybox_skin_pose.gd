extends SceneTree

# Native bone pose must deform skinned geometry for authored animation.
# Isolated GPU boundary diagnostic; not M2, coastal, or whole-scene parity.
# Optional owned captures: SKYBOX_SKIN_POSE_SHOTS=/fixture/owned/directory.
const SHADER_PATH: String = "res://shaders/sky_m2.gdshader"
const SIZE: int = 64
const CODE_TOLERANCE: int = 2
const DEADLINE_SECONDS: float = 20.0
const COLORED: Vector4i = Vector4i(128, 64, 32, 255)
const BACKGROUND: Vector4i = Vector4i(0, 0, 0, 255)
const WITNESSES: Array[Vector2i] = [Vector2i(20, 32), Vector2i(52, 32)]

var viewport: SubViewport
var skeleton: Skeleton3D
var quad: MeshInstance3D
var texture: ImageTexture
var finished: bool = false
var failure_kind: String = "SETUP"
var shots_directory: String = ""

func _initialize() -> void:
	create_timer(DEADLINE_SECONDS).timeout.connect(_on_deadline)
	call_deferred("run_cases")

func _on_deadline() -> void:
	if not finished:
		fail("deadline exceeded while waiting for actual GPU output")

func fail(message: String) -> void:
	finished = true
	push_error("skybox_skin_pose %s: %s" % [failure_kind, message])
	quit(1)

func constant_texture(codes: Vector4i) -> ImageTexture:
	var bytes: PackedByteArray = PackedByteArray([codes.x, codes.y, codes.z, codes.w])
	var image: Image = Image.create_from_data(1, 1, false, Image.FORMAT_RGBA8, bytes)
	return ImageTexture.create_from_image(image)

func build_skinned_mesh() -> ArrayMesh:
	var geometry: QuadMesh = QuadMesh.new()
	geometry.size = Vector2(2.0, 2.0)
	# Keep Godot's vertices, normals, UVs and winding; no M2 offset assumptions.
	var arrays: Array = geometry.surface_get_arrays(0)
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var bones: PackedInt32Array = PackedInt32Array()
	var weights: PackedFloat32Array = PackedFloat32Array()
	for _vertex in range(vertices.size()):
		bones.append_array(PackedInt32Array([0, 0, 0, 0]))
		weights.append_array(PackedFloat32Array([1.0, 0.0, 0.0, 0.0]))
	arrays[Mesh.ARRAY_BONES] = bones
	arrays[Mesh.ARRAY_WEIGHTS] = weights
	var mesh: ArrayMesh = ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh

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

	var model: Node3D = Node3D.new()
	model.position = Vector3(0.0, 0.0, -2.0)
	viewport.add_child(model)
	skeleton = Skeleton3D.new()
	skeleton.name = "Skeleton3D"
	skeleton.add_bone("root")
	skeleton.set_bone_rest(0, Transform3D.IDENTITY)
	model.add_child(skeleton)
	var skin: Skin = Skin.new()
	skin.add_bind(0, Transform3D.IDENTITY)
	quad = MeshInstance3D.new()
	quad.mesh = build_skinned_mesh()
	quad.skin = skin
	quad.skeleton = NodePath("../Skeleton3D")
	model.add_child(quad)
	# Both siblings retain identity local transforms for the entire diagnostic.
	texture = constant_texture(COLORED)

func control_material() -> StandardMaterial3D:
	var material: StandardMaterial3D = StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = Color.WHITE
	material.albedo_texture = texture
	material.transparency = BaseMaterial3D.TRANSPARENCY_DISABLED
	return material

func sky_material(shader: Shader) -> ShaderMaterial:
	var material: ShaderMaterial = ShaderMaterial.new()
	material.shader = shader
	var white: ImageTexture = constant_texture(Vector4i(255, 255, 255, 255))
	material.set_shader_parameter("base_texture", texture)
	material.set_shader_parameter("second_texture", white)
	material.set_shader_parameter("third_texture", white)
	material.set_shader_parameter("fourth_texture", white)
	material.set_shader_parameter("combine_mode", 0)
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
	return material

func encoded_pixel(image: Image, point: Vector2i) -> Vector4i:
	var color: Color = image.get_pixelv(point)
	return Vector4i(
		int(round(color.r * 255.0)), int(round(color.g * 255.0)),
		int(round(color.b * 255.0)), int(round(color.a * 255.0)))

func pixel_matches(actual: Vector4i, expected: Vector4i) -> bool:
	return (
		absi(actual.x - expected.x) <= CODE_TOLERANCE
		and absi(actual.y - expected.y) <= CODE_TOLERANCE
		and absi(actual.z - expected.z) <= CODE_TOLERANCE
		and actual.w == expected.w)

func expected_pixel(point: Vector2i, shift: int) -> Vector4i:
	# FOV 90, z=-2, size=2: fixed [16,48) square; x=1 moves it 16 pixels.
	# Pixel centers are strictly inside/outside these integer edges; AA is off.
	if point.x >= 16 + shift and point.x < 48 + shift and point.y >= 16 and point.y < 48:
		return COLORED
	return BACKGROUND

func observe_pose(label: String, position: Vector3, shift: int) -> bool:
	skeleton.set_bone_pose_position(0, position)
	# Real native pose/material updates, never an engine animation callback.
	for _frame in range(2):
		await process_frame
		await RenderingServer.frame_post_draw
	if finished:
		return false
	var image: Image = viewport.get_texture().get_image()
	if image == null or image.is_empty():
		failure_kind = "SETUP"
		fail(label + ": missing actual GPU image")
		return false
	if image.get_size() != Vector2i(SIZE, SIZE):
		failure_kind = "SETUP"
		fail("%s: expected 64x64 output, got %s" % [label, image.get_size()])
		return false
	if not shots_directory.is_empty():
		var path: String = shots_directory.path_join(label + ".png")
		var error: Error = image.save_png(path)
		if error != OK:
			failure_kind = "SETUP"
			fail("%s: capture failed (%s): %s" % [label, error, path])
			return false

	print("%s: bone_position=%s expected_position=%s extent=%s visible=%s" % [
		label, skeleton.get_bone_pose_position(0), position,
		image.get_size(), quad.is_visible_in_tree()])
	for point in WITNESSES:
		print("%s pixel=%s expected=%s actual=%s" % [
			label, point, expected_pixel(point, shift), encoded_pixel(image, point)])
	var mismatches: int = 0
	var colored_pixels: int = 0
	var background_pixels: int = 0
	for y in range(SIZE):
		for x in range(SIZE):
			var point: Vector2i = Vector2i(x, y)
			var expected: Vector4i = expected_pixel(point, shift)
			var actual: Vector4i = encoded_pixel(image, point)
			if expected == COLORED:
				colored_pixels += 1
			else:
				background_pixels += 1
			if not pixel_matches(actual, expected):
				if mismatches == 0:
					print("%s first_mismatch pixel=%s expected=%s actual=%s" % [
						label, point, expected, actual])
				mismatches += 1
	print("%s: fixed_rect=[%d,16,%d,48) colored_checks=%d background_checks=%d mismatches=%d tolerance=2 codes" % [
		label, 16 + shift, 48 + shift, colored_pixels, background_pixels, mismatches])
	return (
		mismatches == 0 and quad.is_visible_in_tree()
		and skeleton.get_bone_pose_position(0) == position)

func observe_material(label: String, material: Material) -> bool:
	quad.material_override = material
	var before_passed: bool = await observe_pose(label + "_before", Vector3.ZERO, 0)
	if finished:
		return false
	var after_passed: bool = await observe_pose(label + "_after", Vector3(1.0, 0.0, 0.0), 16)
	return before_passed and after_passed

func run_cases() -> void:
	shots_directory = OS.get_environment("SKYBOX_SKIN_POSE_SHOTS")
	if not shots_directory.is_empty():
		var error: Error = DirAccess.make_dir_recursive_absolute(shots_directory)
		if error != OK:
			fail("cannot create capture directory (%s): %s" % [error, shots_directory])
			return
	build_fixture()
	print("LIMIT: isolated one-bone native GPU skinning boundary; not authored animation or whole-scene parity")
	var control_passed: bool = await observe_material("standard_control", control_material())
	if finished:
		return
	if not control_passed:
		fail("StandardMaterial3D control failed; palette/mesh/perception unproven, not behavior RED")
		return
	print("SETUP PASS: StandardMaterial3D confirms actual GPU pose displacement")
	if not ResourceLoader.exists(SHADER_PATH):
		fail("missing actual production shader: " + SHADER_PATH)
		return
	var shader: Shader = ResourceLoader.load(SHADER_PATH) as Shader
	if shader == null:
		fail("cannot load actual production shader: " + SHADER_PATH)
		return
	print("production_shader_sha256=", FileAccess.get_sha256(SHADER_PATH))
	failure_kind = "BEHAVIOR"
	var sky_passed: bool = await observe_material("production_sky", sky_material(shader))
	if finished:
		return
	if not sky_passed:
		fail("production sky GPU pose contract failed after valid native control; cause not inferred")
		return
	finished = true
	print("PASS: native control and production sky deform the one-bone GPU quad; not full-scene parity")
	quit(0)
