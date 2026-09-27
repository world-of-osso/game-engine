extends SceneTree

const SHADER_PATH := "res://shaders/m2.gdshader"
const SIZE := 64
const TOLERANCE := 0.018

var viewport: SubViewport
var material: ShaderMaterial
var sun: DirectionalLight3D

func _initialize() -> void:
	call_deferred("run_cases")

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func texture_color(color: Color) -> ImageTexture:
	var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	image.fill(color)
	return ImageTexture.create_from_image(image)

func stripe(left: Color, right: Color) -> ImageTexture:
	var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	for row in 2:
		image.set_pixel(0, row, left)
		image.set_pixel(1, row, right)
	return ImageTexture.create_from_image(image)

func fixture() -> void:
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
	sun = DirectionalLight3D.new()
	sun.rotation.x = -PI / 2.0
	viewport.add_child(sun)
	var arrays := PlaneMesh.new().surface_get_arrays(0)
	var colors := PackedColorArray()
	var uv1 := PackedVector2Array()
	var uv2 := PackedVector2Array()
	for vertex in arrays[Mesh.ARRAY_VERTEX].size():
		colors.append(Color(0.5, 0.75, 1.0, 1.0))
		uv1.append(Vector2(0.25, 0.5))
		uv2.append(Vector2(0.75, 0.5))
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_TEX_UV] = uv1
	arrays[Mesh.ARRAY_TEX_UV2] = uv2
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var instance := MeshInstance3D.new()
	instance.mesh = mesh
	instance.material_override = material
	viewport.add_child(instance)

func base_inputs() -> void:
	material.set_shader_parameter("effect_mode", 1)
	material.set_shader_parameter("base_texture", texture_color(Color(0.4, 0.3, 0.2, 0.6)))
	material.set_shader_parameter("second_texture", texture_color(Color(0.2, 0.5, 0.7, 0.4)))
	material.set_shader_parameter("base_color", Color.WHITE)
	material.set_shader_parameter("uv_mode_1", 0)
	material.set_shader_parameter("uv_mode_2", 0)
	material.set_shader_parameter("uv_offset_1", Vector2.ZERO)
	material.set_shader_parameter("uv_offset_2", Vector2.ZERO)
	material.set_shader_parameter("transparency", 1.0)
	material.set_shader_parameter("alpha_test", 0.0)
	material.set_shader_parameter("render_flags", 3)
	material.set_shader_parameter("gx_blend", 0)
	material.set_shader_parameter("ambient", Vector3.ONE)
	material.set_shader_parameter("horizon_ambient", Vector3.ONE)
	material.set_shader_parameter("ground_ambient", Vector3.ONE)
	material.set_shader_parameter("direct", Vector3.ZERO)
	material.set_shader_parameter("sun_direction", Vector3.DOWN)
	material.set_shader_parameter("fog_mode", 0)
	material.set_shader_parameter("fog_color", Vector3.ZERO)
	material.set_shader_parameter("fog_range", Vector2.ZERO)
	material.set_shader_parameter("fog_opacity", 0.0)

func pixel() -> Color:
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
	var actual := await pixel()
	if absf(actual.r - expected.r) > TOLERANCE or absf(actual.g - expected.g) > TOLERANCE or absf(actual.b - expected.b) > TOLERANCE:
		fail("%s: expected %s, got %s" % [label, expected, actual])
		return false
	print("PASS: ", label, " ", actual)
	return true

func over_black(authored: Color, alpha: float) -> Color:
	return (authored.srgb_to_linear() * alpha).linear_to_srgb()

func run_cases() -> void:
	if not ResourceLoader.exists(SHADER_PATH):
		fail("M2 shader missing: " + SHADER_PATH)
		return
	material = ShaderMaterial.new()
	material.shader = load(SHADER_PATH)
	fixture()
	base_inputs()
	var ids := [0x4014, 0x10, 0x11, 0x4016, 0x8015, 0x8001, 0x8002, 0x8003]
	var expected := [
		Color(0.16, 0.3, 0.28),
		Color(0.08, 0.15, 0.14),
		Color(0.08, 0.15, 0.14),
		Color(0.16, 0.3, 0.28),
		Color(0.48, 0.5, 0.48),
		Color(0.304, 0.3, 0.232),
		Color(0.48, 0.5, 0.48),
		Color(0.448, 0.42, 0.368),
	]
	var alpha := [0.48, 0.6, 0.24, 0.6, 1.0, 1.0, 1.0, 1.0]
	for index in ids.size():
		material.set_shader_parameter("shader_id", ids[index])
		# Blend over black; texture equations run in authored space before output gamma encoding.
		if not await assert_pixel("combiner 0x%x" % ids[index], over_black(expected[index], alpha[index])):
			return
	material.set_shader_parameter("shader_id", 0x9999)
	if not await assert_pixel("original default combiner is first texel", over_black(Color(0.4, 0.3, 0.2), 0.6)):
		return
	material.set_shader_parameter("shader_id", 0x10)
	material.set_shader_parameter("alpha_test", 0.61)
	if not await assert_pixel("alpha below test discarded", Color.BLACK):
		return
	material.set_shader_parameter("alpha_test", 0.6)
	if not await assert_pixel("alpha at test retained", over_black(expected[1], 0.6)):
		return
	material.set_shader_parameter("alpha_test", 0.0)
	material.set_shader_parameter("transparency", 0.5)
	if not await assert_pixel("transparency multiplies alpha", over_black(expected[1], 0.3)):
		return
	material.set_shader_parameter("transparency", 1.0)
	material.set_shader_parameter("base_texture", stripe(Color.RED, Color.GREEN))
	material.set_shader_parameter("second_texture", texture_color(Color.WHITE))
	material.set_shader_parameter("uv_mode_1", 1)
	if not await assert_pixel("first texture UV2", Color(0.0, 1.0, 0.0)):
		return
	material.set_shader_parameter("uv_mode_1", 0)
	material.set_shader_parameter("uv_offset_1", Vector2(0.5, 0.0))
	if not await assert_pixel("first texture offset", Color(0.0, 1.0, 0.0)):
		return
	material.set_shader_parameter("uv_offset_1", Vector2.ZERO)
	material.set_shader_parameter("base_texture", texture_color(Color.WHITE))
	material.set_shader_parameter("second_texture", stripe(Color.RED, Color.GREEN))
	material.set_shader_parameter("uv_mode_2", 1)
	if not await assert_pixel("second texture UV2", Color(0.0, 1.0, 0.0)):
		return
	material.set_shader_parameter("uv_mode_2", 0)
	material.set_shader_parameter("uv_offset_2", Vector2(0.5, 0.0))
	if not await assert_pixel("second texture offset", Color(0.0, 1.0, 0.0)):
		return
	material.set_shader_parameter("uv_offset_2", Vector2.ZERO)
	material.set_shader_parameter("base_texture", texture_color(Color(0.4, 0.3, 0.2)))
	material.set_shader_parameter("second_texture", texture_color(Color.WHITE))
	material.set_shader_parameter("render_flags", 2)
	material.set_shader_parameter("ambient", Vector3(0.4, 0.4, 0.4))
	material.set_shader_parameter("horizon_ambient", Vector3(0.4, 0.4, 0.4))
	material.set_shader_parameter("ground_ambient", Vector3(0.4, 0.4, 0.4))
	material.set_shader_parameter("direct", Vector3(0.3, 0.3, 0.3))
	if not await assert_pixel("effect lit hemisphere and direct", Color(0.4, 0.3, 0.2) * 0.74):
		return
	material.set_shader_parameter("render_flags", 3)
	if not await assert_pixel("effect unlit", Color(0.4, 0.3, 0.2)):
		return
	material.set_shader_parameter("fog_mode", 1)
	material.set_shader_parameter("fog_range", Vector2(1.0, 3.0))
	material.set_shader_parameter("fog_opacity", 1.0)
	material.set_shader_parameter("fog_color", Color(0.0, 0.0, 1.0).srgb_to_linear())
	material.set_shader_parameter("render_flags", 1)
	if not await assert_pixel("effect unlit fogged", Color(0.2, 0.15, 0.6)):
		return
	material.set_shader_parameter("render_flags", 2)
	if not await assert_pixel("effect lit unfogged", Color(0.4, 0.3, 0.2) * 0.74):
		return
	material.set_shader_parameter("effect_mode", 0)
	material.set_shader_parameter("base_color", Color(0.5, 0.5, 0.5))
	material.set_shader_parameter("render_flags", 3)
	material.set_shader_parameter("fog_mode", 0)
	# Single-texture path multiplies texture, base colour and vertex colour in linear space.
	var single_linear := Color(0.4, 0.3, 0.2).srgb_to_linear() * Color(0.5, 0.5, 0.5).srgb_to_linear() * Color(0.5, 0.75, 1.0)
	var single_gamma := single_linear.linear_to_srgb()
	if not await assert_pixel("single base and vertex colour", single_gamma):
		return
	material.set_shader_parameter("base_color", Color(0.5, 0.5, 0.5, 0.5))
	material.set_shader_parameter("transparency", 0.5)
	material.set_shader_parameter("alpha_test", 0.26)
	if not await assert_pixel("single combined alpha below test", Color.BLACK):
		return
	material.set_shader_parameter("alpha_test", 0.25)
	if not await assert_pixel("single combined alpha at test", over_black(single_gamma, 0.25)):
		return
	material.set_shader_parameter("alpha_test", 0.0)
	material.set_shader_parameter("base_color", Color(0.5, 0.5, 0.5))
	material.set_shader_parameter("transparency", 1.0)
	material.set_shader_parameter("render_flags", 2)
	var lit_single := single_gamma * 0.74
	if not await assert_pixel("single lit", lit_single):
		return
	material.set_shader_parameter("render_flags", 1)
	material.set_shader_parameter("fog_mode", 1)
	if not await assert_pixel("single unlit fogged", single_gamma * 0.5 + Color(0.0, 0.0, 0.5)):
		return
	material.set_shader_parameter("render_flags", 0)
	if not await assert_pixel("single lit fogged", lit_single * 0.5 + Color(0.0, 0.0, 0.5)):
		return
	material.set_shader_parameter("gx_blend", 3)
	if not await assert_pixel("additive GX fog black", lit_single * 0.5):
		return
	material.set_shader_parameter("gx_blend", 4)
	if not await assert_pixel("modulate GX fog white", lit_single * 0.5 + Color(0.5, 0.5, 0.5)):
		return
	material.set_shader_parameter("gx_blend", 5)
	if not await assert_pixel("modulate2 GX fog grey", lit_single * 0.5 + Color(0.25, 0.25, 0.25)):
		return
	material.set_shader_parameter("gx_blend", 10)
	if not await assert_pixel("GX fog black variant 10", lit_single * 0.5):
		return
	material.set_shader_parameter("gx_blend", 13)
	if not await assert_pixel("GX fog black variant 13", lit_single * 0.5):
		return
	material.set_shader_parameter("fog_mode", 0)
	material.set_shader_parameter("render_flags", 2)
	material.set_shader_parameter("effect_mode", 1)
	material.set_shader_parameter("base_texture", texture_color(Color(0.4, 0.4, 0.4)))
	material.set_shader_parameter("second_texture", texture_color(Color.WHITE))
	material.set_shader_parameter("shader_id", 0x10)
	material.set_shader_parameter("ambient", Vector3(0.25, 0.25, 0.25))
	material.set_shader_parameter("horizon_ambient", Vector3(0.25, 0.25, 0.25))
	material.set_shader_parameter("ground_ambient", Vector3(0.25, 0.25, 0.25))
	material.set_shader_parameter("direct", Vector3(0.5, 0.5, 0.5))
	material.set_shader_parameter("sun_direction", Vector3(1.0, -1.0, 0.0))
	sun.look_at_from_position(Vector3.ZERO, Vector3(1.0, -1.0, 0.0))
	sun.shadow_enabled = true
	var n_dot_l := sqrt(0.5)
	var ambient_factor := lerpf(0.7, 1.1, 0.5 + 0.5 * n_dot_l) * 0.25
	var unshadowed := 0.4 * (ambient_factor + 0.5 * n_dot_l)
	if not await assert_pixel("unshadowed angled direct", Color(unshadowed, unshadowed, unshadowed)):
		return
	var block := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = Vector3(0.8, 0.15, 0.8)
	block.mesh = box
	block.position = Vector3(-0.5, 0.5, 0.0)
	viewport.add_child(block)
	var ambient_only := 0.4 * ambient_factor
	var shadow_pixel := await pixel()
	if shadow_pixel.r >= unshadowed - 0.008 or shadow_pixel.r < ambient_only - TOLERANCE:
		fail("real caster must attenuate direct, retain ambient: %s (clear %s, ambient %s)" % [shadow_pixel, unshadowed, ambient_only])
		return
	print("PASS: real caster attenuates direct, retains ambient ", shadow_pixel)
	quit(0)
