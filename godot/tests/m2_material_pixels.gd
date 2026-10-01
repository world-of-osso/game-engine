extends SceneTree

const Oracle := preload("res://tests/m2_wwv_oracle.gd")
const SHADER_PATH := "res://shaders/m2.gdshader"
const TEX := [Color(0.4, 0.3, 0.2, 0.6), Color(0.2, 0.5, 0.7, 0.4), Color(0.9, 0.6, 0.3, 0.7), Color(0.5, 0.5, 0.5, 0.8)]
const WEIGHTS := Vector3(0.9, 0.6, 0.3)
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
	material.set_shader_parameter("pixel_shader", 1)
	material.set_shader_parameter("vertex_shader", 0)
	material.set_shader_parameter("base_texture", texture_color(TEX[0]))
	material.set_shader_parameter("second_texture", texture_color(TEX[1]))
	material.set_shader_parameter("third_texture", texture_color(TEX[2]))
	material.set_shader_parameter("fourth_texture", texture_color(TEX[3]))
	material.set_shader_parameter("texture_wrap", 255)
	material.set_shader_parameter("texture_matrix_1", Basis.IDENTITY)
	material.set_shader_parameter("texture_matrix_2", Basis.IDENTITY)
	material.set_shader_parameter("texture_weights", Vector3.ONE)
	material.set_shader_parameter("mesh_color", Vector3.ONE)
	material.set_shader_parameter("transparency", 1.0)
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

# A matrix translating texture coordinates by `offset` (the shader's (u, v, 1) mat3).
func offset_matrix(offset: Vector2) -> Basis:
	return Basis(Vector3(1.0, 0.0, 0.0), Vector3(0.0, 1.0, 0.0), Vector3(offset.x, offset.y, 1.0))

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

func assert_unlit_without_sun() -> bool:
	viewport.remove_child(sun)
	material.set_shader_parameter("base_texture", texture_color(Color(0.4, 0.3, 0.2)))
	if not await assert_pixel("unlit without sun", Color(0.4, 0.3, 0.2)):
		return false
	material.set_shader_parameter("render_flags", 1)
	material.set_shader_parameter("fog_mode", 1)
	# Half fog at the fixture distance of 2: exp(-(2 - 1) * ln 2).
	material.set_shader_parameter("fog_range", Vector2(1.0, 100.0))
	material.set_shader_parameter("fog_density", log(2.0))
	material.set_shader_parameter("fog_opacity", 1.0)
	material.set_shader_parameter("fog_color", Color(0.0, 0.0, 1.0).srgb_to_linear())
	if not await assert_pixel("unlit fogged without sun", Color(0.2, 0.15, 0.6)):
		return false
	viewport.add_child(sun)
	return true

# Every calcM2FragMaterial combiner over four known texels, unlit and opaque:
# matDiffuse + specular (m2shader_text.slang adds specular after calcLight).
func assert_combiners() -> bool:
	material.set_shader_parameter("texture_weights", WEIGHTS)
	material.set_shader_parameter("mesh_color", Vector3(0.8, 0.9, 1.0))
	for pixel_shader in 37:
		material.set_shader_parameter("pixel_shader", pixel_shader)
		var frag := Oracle.fragment(pixel_shader, TEX[0], TEX[1], TEX[2], TEX[3], Vector3(0.8, 0.9, 1.0), WEIGHTS)
		var rgb: Vector3 = frag[0] + frag[1] * Vector3(0.8, 0.9, 1.0)
		var expected := Color(clampf(rgb.x, 0.0, 1.0), clampf(rgb.y, 0.0, 1.0), clampf(rgb.z, 0.0, 1.0))
		if not await assert_pixel("pixel shader %d" % pixel_shader, expected):
			return false
	material.set_shader_parameter("texture_weights", Vector3.ONE)
	material.set_shader_parameter("mesh_color", Vector3.ONE)
	return true

func run_cases() -> void:
	if not ResourceLoader.exists(SHADER_PATH):
		fail("M2 shader missing: " + SHADER_PATH)
		return
	material = ShaderMaterial.new()
	material.shader = load(SHADER_PATH)
	fixture()
	base_inputs()
	if not await assert_unlit_without_sun():
		return
	base_inputs()
	if not await assert_combiners():
		return
	material.set_shader_parameter("pixel_shader", 1)
	# AlphaKey discards a combiner alpha below 128/255 (calcM2FragMaterial blendMode 1).
	material.set_shader_parameter("gx_blend", 1)
	material.set_shader_parameter("base_texture", texture_color(Color(0.4, 0.3, 0.2, 0.49)))
	if not await assert_pixel("alpha key below 128/255 discarded", Color.BLACK):
		return
	material.set_shader_parameter("base_texture", texture_color(Color(0.4, 0.3, 0.2, 0.51)))
	if not await assert_pixel("alpha key at 0.51 retained", Color(0.4, 0.3, 0.2)):
		return
	# Alpha blending: finalOpacity = discardAlpha * meshOpacity.
	material.set_shader_parameter("gx_blend", 2)
	material.set_shader_parameter("base_texture", texture_color(Color(0.4, 0.3, 0.2, 0.6)))
	material.set_shader_parameter("transparency", 0.5)
	if not await assert_pixel("mesh opacity multiplies texture alpha", over_black(Color(0.4, 0.3, 0.2), 0.3)):
		return
	material.set_shader_parameter("transparency", 0.00005)
	if not await assert_pixel("near-zero mesh opacity skips the batch", Color.BLACK):
		return
	material.set_shader_parameter("transparency", 1.0)
	material.set_shader_parameter("gx_blend", 0)
	material.set_shader_parameter("pixel_shader", 0)
	material.set_shader_parameter("base_texture", stripe(Color.RED, Color.GREEN))
	material.set_shader_parameter("vertex_shader", 10)
	if not await assert_pixel("Diffuse_T2 samples the first texture at UV2", Color(0.0, 1.0, 0.0)):
		return
	material.set_shader_parameter("vertex_shader", 10)
	material.set_shader_parameter("texture_matrix_2", offset_matrix(Vector2(0.5, 0.0)))
	if not await assert_pixel("Diffuse_T2 uses texture matrix 2", Color(1.0, 0.0, 0.0)):
		return
	material.set_shader_parameter("texture_matrix_2", Basis.IDENTITY)
	material.set_shader_parameter("vertex_shader", 0)
	material.set_shader_parameter("texture_matrix_1", offset_matrix(Vector2(0.5, 0.0)))
	if not await assert_pixel("texture matrix 1 moves the first texture", Color(0.0, 1.0, 0.0)):
		return
	material.set_shader_parameter("texture_matrix_1", Basis.IDENTITY)
	# Without its wrap flag, U clamps: 0.25 + 1.0 stays on the right texel, not wrapping left.
	material.set_shader_parameter("texture_matrix_1", offset_matrix(Vector2(1.0, 0.0)))
	material.set_shader_parameter("texture_wrap", 255 & ~1)
	if not await assert_pixel("clamped U holds the edge texel", Color(0.0, 1.0, 0.0)):
		return
	material.set_shader_parameter("texture_wrap", 255)
	if not await assert_pixel("wrapped U repeats", Color(1.0, 0.0, 0.0)):
		return
	material.set_shader_parameter("texture_matrix_1", Basis.IDENTITY)
	material.set_shader_parameter("pixel_shader", 5)
	material.set_shader_parameter("base_texture", texture_color(Color.WHITE))
	material.set_shader_parameter("second_texture", stripe(Color.RED, Color.GREEN))
	material.set_shader_parameter("vertex_shader", 2)
	if not await assert_pixel("Diffuse_T1_T2 samples the second texture at UV2", Color(0.0, 1.0, 0.0)):
		return
	material.set_shader_parameter("vertex_shader", 7)
	if not await assert_pixel("Diffuse_T1_T1 samples the second texture at UV1", Color(1.0, 0.0, 0.0)):
		return
	material.set_shader_parameter("pixel_shader", 1)
	material.set_shader_parameter("vertex_shader", 0)
	material.set_shader_parameter("base_texture", texture_color(Color(0.4, 0.3, 0.2)))
	material.set_shader_parameter("second_texture", texture_color(Color.WHITE))
	material.set_shader_parameter("render_flags", 2)
	material.set_shader_parameter("ambient", Vector3(0.4, 0.4, 0.4))
	material.set_shader_parameter("horizon_ambient", Vector3(0.4, 0.4, 0.4))
	material.set_shader_parameter("ground_ambient", Vector3(0.4, 0.4, 0.4))
	material.set_shader_parameter("direct", Vector3(0.3, 0.3, 0.3))
	if not await assert_pixel("lit hemisphere and direct", Color(0.4, 0.3, 0.2) * 0.74):
		return
	# The specular term is added after lighting: Opaque_AddAlpha's tex2 * tex2.a.
	material.set_shader_parameter("pixel_shader", 13)
	material.set_shader_parameter("second_texture", texture_color(Color(0.2, 0.1, 0.0, 0.5)))
	if not await assert_pixel("specular after lighting", Color(0.4, 0.3, 0.2) * 0.74 + Color(0.1, 0.05, 0.0)):
		return
	material.set_shader_parameter("pixel_shader", 1)
	material.set_shader_parameter("second_texture", texture_color(Color.WHITE))
	material.set_shader_parameter("render_flags", 3)
	if not await assert_pixel("unlit", Color(0.4, 0.3, 0.2)):
		return
	material.set_shader_parameter("fog_mode", 1)
	# Half fog at the fixture distance of 2: exp(-(2 - 1) * ln 2).
	material.set_shader_parameter("fog_range", Vector2(1.0, 100.0))
	material.set_shader_parameter("fog_density", log(2.0))
	material.set_shader_parameter("fog_opacity", 1.0)
	material.set_shader_parameter("fog_color", Color(0.0, 0.0, 1.0).srgb_to_linear())
	material.set_shader_parameter("render_flags", 1)
	if not await assert_pixel("unlit fogged", Color(0.2, 0.15, 0.6)):
		return
	# calculateLegacyFog end fade: 1.42857 * (1 - 2 / (2 / 0.65)) leaves half the colour.
	material.set_shader_parameter("fog_density", 0.0)
	material.set_shader_parameter("fog_range", Vector2(0.0, 2.0 / 0.65))
	if not await assert_pixel("unlit end fade", Color(0.2, 0.15, 0.6)):
		return
	# Fog starts at fog_range.x: nothing is fogged before it.
	material.set_shader_parameter("fog_density", log(2.0))
	material.set_shader_parameter("fog_range", Vector2(2.0, 100.0))
	if not await assert_pixel("unlit before fog start", Color(0.4, 0.3, 0.2)):
		return
	material.set_shader_parameter("fog_range", Vector2(1.0, 100.0))
	material.set_shader_parameter("render_flags", 2)
	if not await assert_pixel("lit unfogged", Color(0.4, 0.3, 0.2) * 0.74):
		return
	# The batch colour multiplies the texel in authored space.
	material.set_shader_parameter("mesh_color", Vector3(0.5, 0.75, 1.0))
	material.set_shader_parameter("render_flags", 3)
	material.set_shader_parameter("fog_mode", 0)
	var single_gamma := Color(0.4 * 0.5, 0.3 * 0.75, 0.2)
	if not await assert_pixel("mesh colour", single_gamma):
		return
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
	# Mod2x also doubles its output for the DST_COLOR, SRC_COLOR blend.
	if not await assert_pixel("modulate2 GX fog grey", (lit_single * 0.5 + Color(0.25, 0.25, 0.25)) * 2.0):
		return
	material.set_shader_parameter("gx_blend", 10)
	if not await assert_pixel("GX fog black variant 10", lit_single * 0.5):
		return
	material.set_shader_parameter("gx_blend", 13)
	if not await assert_pixel("GX fog black variant 13", lit_single * 0.5):
		return
	material.set_shader_parameter("fog_mode", 0)
	material.set_shader_parameter("render_flags", 2)
	material.set_shader_parameter("mesh_color", Vector3.ONE)
	material.set_shader_parameter("base_texture", texture_color(Color(0.4, 0.4, 0.4)))
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
	# WMO doodad interior light (commonLightFunctions.slang calcLight): at blend 0 the
	# interior ambient and direct light along the personal sun replace the exterior
	# light, ignoring the sun's shadow; at 0.5 the two mix.
	material.set_shader_parameter("exterior_blend", 0.0)
	material.set_shader_parameter("interior_ambient", Vector3(0.2, 0.2, 0.2))
	material.set_shader_parameter("interior_horizon_ambient", Vector3(0.2, 0.2, 0.2))
	material.set_shader_parameter("interior_ground_ambient", Vector3(0.2, 0.2, 0.2))
	material.set_shader_parameter("interior_direct", Vector3(0.5, 0.5, 0.5))
	material.set_shader_parameter("interior_sun_direction", Vector3.DOWN)
	var interior := 0.4 * (0.2 * 1.1 + 0.5)
	if not await assert_pixel("interior doodad light under the caster", Color(interior, interior, interior)):
		return
	block.free()
	material.set_shader_parameter("exterior_blend", 0.5)
	var mixed := 0.5 * interior + 0.5 * unshadowed
	if not await assert_pixel("half interior, half exterior", Color(mixed, mixed, mixed)):
		return
	quit(0)
