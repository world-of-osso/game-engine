extends SceneTree

const SHADER_PATH := "res://shaders/sky_m2.gdshader"
const SIZE := 64
const TOLERANCE := 0.035

var viewport: SubViewport
var sky: MeshInstance3D
var material: ShaderMaterial

func _initialize() -> void:
	call_deferred("run_cases")

func texture_color(color: Color) -> ImageTexture:
	var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	image.fill(color)
	return ImageTexture.create_from_image(image)

func stripe(left: Color, right: Color) -> ImageTexture:
	var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	for y in 2:
		image.set_pixel(0, y, left)
		image.set_pixel(1, y, right)
	return ImageTexture.create_from_image(image)

func make_plane(height: float, shader_material: Material) -> MeshInstance3D:
	var plane := PlaneMesh.new()
	plane.size = Vector2(2.0, 2.0)
	var arrays := plane.surface_get_arrays(0)
	var first_uv := PackedVector2Array()
	var second_uv := PackedVector2Array()
	for vertex in arrays[Mesh.ARRAY_VERTEX].size():
		first_uv.append(Vector2(0.25, 0.5))
		second_uv.append(Vector2(0.75, 0.5))
	arrays[Mesh.ARRAY_TEX_UV] = first_uv
	arrays[Mesh.ARRAY_TEX_UV2] = second_uv
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var instance := MeshInstance3D.new()
	instance.mesh = mesh
	instance.position.y = height
	instance.material_override = shader_material
	viewport.add_child(instance)
	return instance

func setup() -> bool:
	if not ResourceLoader.exists(SHADER_PATH):
		push_error("Sky M2 shader missing: " + SHADER_PATH)
		return false
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
	camera.look_at_from_position(Vector3(0.0, 2.0, 0.0), Vector3.ZERO, Vector3(0.0, 0.0, -1.0))
	viewport.add_child(camera)
	camera.current = true
	material = ShaderMaterial.new()
	material.shader = load(SHADER_PATH)
	material.set_shader_parameter("base_texture", texture_color(Color(0.4, 0.3, 0.2, 0.6)))
	material.set_shader_parameter("second_texture", texture_color(Color(0.2, 0.5, 0.7, 0.4)))
	material.set_shader_parameter("third_texture", texture_color(Color.WHITE))
	material.set_shader_parameter("fourth_texture", texture_color(Color.WHITE))
	material.set_shader_parameter("has_second_texture", true)
	material.set_shader_parameter("transparency", 1.0)
	sky = make_plane(0.0, material)
	return true

func read_center() -> Color:
	await process_frame
	await RenderingServer.frame_post_draw
	await process_frame
	await RenderingServer.frame_post_draw
	var image := viewport.get_texture().get_image()
	if image == null or image.is_empty():
		push_error("Sky GPU readback unavailable")
		return Color(-1.0, -1.0, -1.0)
	return image.get_pixel(SIZE / 2, SIZE / 2)

func pixel_is(label: String, expected: Color) -> bool:
	var actual := await read_center()
	if absf(actual.r - expected.r) > TOLERANCE or absf(actual.g - expected.g) > TOLERANCE or absf(actual.b - expected.b) > TOLERANCE:
		push_error("%s: expected %s, got %s" % [label, expected, actual])
		return false
	print("PASS: ", label, " ", actual)
	return true

func over_black(color: Color, alpha: float) -> Color:
	return (color.srgb_to_linear() * alpha).linear_to_srgb()

func run_cases() -> void:
	if not setup():
		quit(1)
		return
	var passed := await check_pixels()
	viewport.free()
	quit(0 if passed else 1)

func check_pixels() -> bool:
	material.set_shader_parameter("combine_mode", 0x4014)
	if not await pixel_is("modulate2x", over_black(Color(0.16, 0.3, 0.28), 0.48)):
		return false
	material.set_shader_parameter("combine_mode", 0x8015)
	if not await pixel_is("add with alpha", Color(0.48, 0.5, 0.48)):
		return false
	material.set_shader_parameter("combine_mode", 0x10)
	material.set_shader_parameter("transparency", 0.5)
	if not await pixel_is("animated opacity", over_black(Color(0.08, 0.15, 0.14), 0.3)):
		return false
	material.set_shader_parameter("transparency", 1.0)
	material.set_shader_parameter("base_texture", stripe(Color.RED, Color.GREEN))
	material.set_shader_parameter("second_texture", texture_color(Color.WHITE))
	material.set_shader_parameter("uv_mode_1", 1)
	if not await pixel_is("UV2 selection", over_black(Color.GREEN, 1.0)):
		return false
	material.set_shader_parameter("uv_mode_1", 0)
	material.set_shader_parameter("uv_offset_1", Vector2(0.5, 0.0))
	if not await pixel_is("animated UV translation", Color.GREEN):
		return false
	material.set_shader_parameter("uv_offset_1", Vector2.ZERO)
	material.set_shader_parameter("base_texture", texture_color(Color.BLUE))
	material.set_shader_parameter("has_second_texture", false)
	material.set_shader_parameter("alpha_test", 0.0)
	var foreground := StandardMaterial3D.new()
	foreground.albedo_color = Color.RED
	foreground.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	var front := make_plane(0.5, foreground)
	if not await pixel_is("opaque foreground stays over far sky", Color.RED):
		return false
	front.free()
	material.set_shader_parameter("base_texture", texture_color(Color(0.0, 0.0, 1.0, 0.1)))
	material.set_shader_parameter("alpha_test", 0.2)
	return await pixel_is("sky cutout discards", Color.BLACK)
