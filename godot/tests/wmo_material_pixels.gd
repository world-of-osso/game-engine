extends SceneTree

const SHADER_PATH := "res://shaders/wmo.gdshader"
const SIZE := 64
const TOLERANCE := 0.019
const TEXEL := Color(0.4, 0.3, 0.2, 1.0)

var viewport: SubViewport
var environment: Environment
var material: ShaderMaterial
var surface: MeshInstance3D
var sun: DirectionalLight3D

func _initialize() -> void:
	call_deferred("run_cases")

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func solid_texture(color: Color) -> ImageTexture:
	var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	image.fill(color)
	return ImageTexture.create_from_image(image)

func stripe(left: Color, right: Color) -> ImageTexture:
	var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	for row in 2:
		image.set_pixel(0, row, left)
		image.set_pixel(1, row, right)
	return ImageTexture.create_from_image(image)

func fixture_scene() -> void:
	viewport = SubViewport.new()
	viewport.size = Vector2i(SIZE, SIZE)
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	viewport.own_world_3d = true
	root.add_child(viewport)
	environment = Environment.new()
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
	surface = MeshInstance3D.new()
	surface.mesh = wmo_quad(0.5, true)
	surface.material_override = material
	viewport.add_child(surface)

const UV3 := Vector2(0.25, 0.5)
const UV4 := Vector2(0.75, 0.5)
var quad_flags := 0

# UV/UV2 like authored MOTV1/2; CUSTOM0 = (MOTV3, MOTV4), CUSTOM1 = MOC2 RGBA8,
# CUSTOM2.x = second MOCV alpha, matching the native WMO scene binding.
func wmo_quad(second_alpha: float, with_custom: bool, moc2 := Color(0.0, 0.0, 0.0, 1.0), uv2_gradient := false) -> ArrayMesh:
	var arrays := PlaneMesh.new().surface_get_arrays(0)
	var colors := PackedColorArray()
	var uvs := PackedVector2Array()
	var uv2s := PackedVector2Array()
	var uv34 := PackedFloat32Array()
	var moc2_bytes := PackedByteArray()
	var alphas := PackedFloat32Array()
	for vertex in arrays[Mesh.ARRAY_VERTEX].size():
		var position: Vector3 = arrays[Mesh.ARRAY_VERTEX][vertex]
		colors.append(Color(0.1, 0.05, 0.025, 0.5))
		uvs.append(Vector2(0.25, 0.5))
		uv2s.append(Vector2(position.x, position.z) * 0.5 + Vector2(0.5, 0.5) if uv2_gradient else Vector2(0.75, 0.5))
		uv34.append_array(PackedFloat32Array([UV3.x, UV3.y, UV4.x, UV4.y]))
		moc2_bytes.append_array(PackedByteArray([moc2.r8, moc2.g8, moc2.b8, moc2.a8]))
		alphas.append(second_alpha)
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = uv2s
	quad_flags = 0
	if with_custom:
		arrays[Mesh.ARRAY_CUSTOM0] = uv34
		arrays[Mesh.ARRAY_CUSTOM1] = moc2_bytes
		arrays[Mesh.ARRAY_CUSTOM2] = alphas
		quad_flags = (Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT) \
			| (Mesh.ARRAY_CUSTOM_RGBA8_UNORM << Mesh.ARRAY_FORMAT_CUSTOM1_SHIFT) \
			| (Mesh.ARRAY_CUSTOM_R_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM2_SHIFT)
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays, [], {}, quad_flags)
	return mesh

# One mesh: near layer (y=0.5) indexed before far layer (y=0), like WMO group
# triangles in arbitrary authored order. Only depth writes keep the near layer.
func layered_quad(near_color: Color, far_color: Color) -> ArrayMesh:
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var colors := PackedColorArray()
	var uvs := PackedVector2Array()
	for layer in [[0.5, near_color], [0.0, far_color]]:
		for corner in [Vector2(-1, -1), Vector2(1, -1), Vector2(1, 1), Vector2(-1, -1), Vector2(1, 1), Vector2(-1, 1)]:
			vertices.append(Vector3(corner.x, layer[0], corner.y))
			normals.append(Vector3.UP)
			colors.append(layer[1])
			uvs.append(Vector2(0.25, 0.5))
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh

func blended_shader() -> Shader:
	var shader := Shader.new()
	shader.code = load(SHADER_PATH).code.replace("shader_type spatial;", "shader_type spatial;\n#define WMO_BLENDED")
	return shader

func assert_opaque_depth() -> bool:
	var interior_dot := 0.9 / Vector3(-0.30822, -0.9, 0.30822).length()
	var interior_scale := lerpf(0.7, 1.1, 0.5 + 0.5 * interior_dot)
	var near := Color(0.4 * 0.4, 0.3 * 0.3, 0.2 * 0.25) * interior_scale
	var previous := surface.mesh
	surface.mesh = layered_quad(Color(0.1, 0.05, 0.025, 0.0), Color(0.0, 0.0, 0.0, 0.0))
	if not await assert_pixel("opaque WMO near layer occludes later far layer", near):
		return false
	surface.mesh = previous
	return true

func set_vertex_color(color: Color) -> void:
	var arrays := surface.mesh.surface_get_arrays(0)
	var colors := PackedColorArray()
	for vertex in arrays[Mesh.ARRAY_VERTEX].size():
		colors.append(color)
	arrays[Mesh.ARRAY_COLOR] = colors
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays, [], {}, quad_flags)
	surface.mesh = mesh

# WebWowViewerCpp iWmoApi.h wmoMaterialShader: MOMT id -> [vertex, pixel].
const RETAIL_SHADERS := [
	[0, 0], [3, 1], [3, 2], [1, 3], [0, 4], [1, 5], [4, 6], [0, 7], [6, 8], [4, 9],
	[-1, -1], [2, 10], [2, 11], [4, 12], [-1, -1], [4, 13], [0, 0], [2, 14], [7, 15],
	[4, 16], [7, 17], [0, 18], [8, 19], [0, 20],
]

func set_retail_shader(momt: int) -> void:
	material.set_shader_parameter("vertex_shader", RETAIL_SHADERS[momt][0])
	material.set_shader_parameter("pixel_shader", RETAIL_SHADERS[momt][1])

func base_inputs() -> void:
	material.set_shader_parameter("base_texture", solid_texture(TEXEL))
	material.set_shader_parameter("second_texture", solid_texture(Color.BLUE))
	material.set_shader_parameter("base_color", Color.WHITE)
	material.set_shader_parameter("emissive", Vector3.ZERO)
	material.set_shader_parameter("interior_ambient", Vector3(0.2, 0.2, 0.2))
	material.set_shader_parameter("exterior_lit", false)
	material.set_shader_parameter("unlit", false)
	material.set_shader_parameter("unfogged", true)
	material.set_shader_parameter("has_mocv", true)
	material.set_shader_parameter("has_second_mocv", true)
	material.set_shader_parameter("has_uv2", true)
	material.set_shader_parameter("has_uv3", true)
	material.set_shader_parameter("has_uv4", true)
	material.set_shader_parameter("has_moc2", true)
	for slot in ["third_texture", "texture_4", "texture_5", "texture_6", "texture_7", "texture_8", "texture_9"]:
		material.set_shader_parameter(slot, solid_texture(Color(0.0, 0.0, 0.0, 0.0)))
	set_retail_shader(0)
	material.set_shader_parameter("blend_mode", 0)
	material.set_shader_parameter("ambient", Vector3(0.4, 0.4, 0.4))
	material.set_shader_parameter("horizon_ambient", Vector3(0.4, 0.4, 0.4))
	material.set_shader_parameter("ground_ambient", Vector3(0.4, 0.4, 0.4))
	material.set_shader_parameter("direct", Vector3(0.3, 0.3, 0.3))
	material.set_shader_parameter("sun_direction", Vector3.DOWN)
	material.set_shader_parameter("fog_mode", 0)
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

func assert_lighting() -> bool:
	# Root retail_shade with top-facing normal: exterior N.L=1, interior N.L≈0.9.
	var interior_dot := 0.9 / Vector3(-0.30822, -0.9, 0.30822).length()
	var interior_scale := lerpf(0.7, 1.1, 0.5 + 0.5 * interior_dot)
	var interior := Color(0.4 * 0.4, 0.3 * 0.3, 0.2 * 0.25) * interior_scale
	var exterior := Color(0.4 * (0.6 * 1.1 + 0.3), 0.3 * (0.5 * 1.1 + 0.3), 0.2 * (0.45 * 1.1 + 0.3))
	set_vertex_color(Color(0.1, 0.05, 0.025, 0.0))
	if not await assert_pixel("interior double MOCV, fixed sun, no direct", interior):
		return false
	set_vertex_color(Color(0.1, 0.05, 0.025, 0.5))
	if not await assert_pixel("authored midpoint of interior and exterior", (interior + exterior) * 0.5):
		return false
	set_vertex_color(Color(0.1, 0.05, 0.025, 1.0))
	if not await assert_pixel("exterior doubled MOCV and direct", exterior):
		return false
	set_vertex_color(Color(0.1, 0.05, 0.025, 0.0))
	material.set_shader_parameter("exterior_lit", true)
	if not await assert_pixel("exterior group overrides MOCV alpha", exterior):
		return false
	material.set_shader_parameter("exterior_lit", false)
	material.set_shader_parameter("has_mocv", false)
	if not await assert_pixel("missing MOCV defaults to black alpha zero", TEXEL * (0.2 * interior_scale)):
		return false
	material.set_shader_parameter("has_mocv", true)
	return true

func assert_layers() -> bool:
	material.set_shader_parameter("unlit", true)
	material.set_shader_parameter("base_texture", solid_texture(Color.RED))
	material.set_shader_parameter("second_texture", solid_texture(Color(0.0, 0.0, 1.0, 0.5)))
	set_retail_shader(6)
	if not await assert_pixel("MOMT6 second texture alpha then MOCV2 half", Color(0.75, 0.0, 0.25)):
		return false
	set_retail_shader(13)
	if not await assert_pixel("MOMT13 second MOCV alpha opaque", Color(0.5, 0.0, 0.5)):
		return false
	material.set_shader_parameter("has_second_mocv", false)
	if not await assert_pixel("missing MOCV2 alpha defaults one", Color.RED):
		return false
	material.set_shader_parameter("has_second_mocv", true)
	surface.mesh = wmo_quad(0.0, true)
	material.set_shader_parameter("second_texture", stripe(Color.RED, Color.BLUE))
	material.set_shader_parameter("has_uv2", true)
	if not await assert_pixel("MOMT13 samples second UV", Color.BLUE):
		return false
	material.set_shader_parameter("has_uv2", false)
	# At the (1,1) repeat seam, the root WMO linear sampler averages both texels.
	if not await assert_pixel("missing UV2 samples repeat-wrapped (1,1)", Color(0.5, 0.0, 0.5).linear_to_srgb()):
		return false
	material.set_shader_parameter("has_uv2", true)
	set_retail_shader(0)
	material.set_shader_parameter("base_texture", solid_texture(TEXEL))
	material.set_shader_parameter("unlit", false)
	return true

func assert_alpha_and_fog() -> bool:
	material.set_shader_parameter("unlit", true)
	material.set_shader_parameter("base_texture", solid_texture(Color(1.0, 0.0, 0.0, 60.0 / 255.0)))
	if not await assert_pixel("opaque ignores low texture alpha", Color.RED):
		return false
	material.set_shader_parameter("blend_mode", 1)
	material.set_shader_parameter("base_texture", solid_texture(Color(1.0, 0.0, 0.0, 127.0 / 255.0)))
	if not await assert_pixel("AlphaKey 127 discarded", Color.BLACK):
		return false
	material.set_shader_parameter("base_texture", solid_texture(Color(1.0, 0.0, 0.0, 128.0 / 255.0)))
	if not await assert_pixel("AlphaKey 128 retained as opaque", Color.RED):
		return false
	var opaque_shader := material.shader
	material.shader = blended_shader()
	for mode in [2, 3]:
		material.set_shader_parameter("blend_mode", mode)
		if not await assert_pixel("GxBlend %d retains source alpha" % mode, Color(128.0 / 255.0, 0.0, 0.0).linear_to_srgb()):
			return false
	material.shader = opaque_shader
	material.set_shader_parameter("blend_mode", 0)
	material.set_shader_parameter("base_texture", solid_texture(TEXEL))
	material.set_shader_parameter("unlit", false)
	material.set_shader_parameter("exterior_lit", true)
	material.set_shader_parameter("emissive", Vector3(0.05, 0.0, 0.0))
	var lit := Color(0.4 * 0.96 + Color(0.05, 0.0, 0.0).linear_to_srgb().r, 0.3 * 0.85, 0.2 * 0.795)
	if not await assert_pixel("SIDN/emissive adds after shading in authored space", lit):
		return false
	material.set_shader_parameter("unlit", true)
	if not await assert_pixel("unlit skips ambient/direct, retains emissive", TEXEL + Color(0.05, 0.0, 0.0).linear_to_srgb()):
		return false
	material.set_shader_parameter("emissive", Vector3.ZERO)
	material.set_shader_parameter("fog_mode", 1)
	# Half fog at the fixture distance of 2: exp(-(2 - 1) * ln 2).
	RenderingServer.global_shader_parameter_set("fog_range", Vector2(1.0, 100.0))
	RenderingServer.global_shader_parameter_set("fog_density", log(2.0))
	material.set_shader_parameter("fog_opacity", 1.0)
	RenderingServer.global_shader_parameter_set("fog_color", Color(0.0, 0.0, 1.0).srgb_to_linear())
	material.set_shader_parameter("unfogged", false)
	if not await assert_pixel("unlit still fogged halfway", Color(0.2, 0.15, 0.6)):
		return false
	# calculateLegacyFog end fade: 1.42857 * (1 - 2 / (2 / 0.65)) leaves half the colour.
	RenderingServer.global_shader_parameter_set("fog_density", 0.0)
	RenderingServer.global_shader_parameter_set("fog_range", Vector2(0.0, 2.0 / 0.65))
	if not await assert_pixel("end fade fogs halfway", Color(0.2, 0.15, 0.6)):
		return false
	# Fog starts at fog_range.x: nothing is fogged before it.
	RenderingServer.global_shader_parameter_set("fog_density", log(2.0))
	RenderingServer.global_shader_parameter_set("fog_range", Vector2(2.0, 100.0))
	if not await assert_pixel("no fog before fog start", TEXEL):
		return false
	RenderingServer.global_shader_parameter_set("fog_range", Vector2(1.0, 100.0))
	material.set_shader_parameter("blend_mode", 3)
	if not await assert_pixel("GxBlend 3 fogs to black", Color(0.2, 0.15, 0.1)):
		return false
	material.set_shader_parameter("blend_mode", 4)
	if not await assert_pixel("GxBlend 4 fogs to white", Color(0.7, 0.65, 0.6)):
		return false
	material.set_shader_parameter("blend_mode", 5)
	if not await assert_pixel("GxBlend 5 fogs to grey", Color(0.45, 0.4, 0.35)):
		return false
	material.set_shader_parameter("blend_mode", 0)
	material.set_shader_parameter("unfogged", true)
	if not await assert_pixel("unfogged retains unlit texel", TEXEL):
		return false
	return true

# Exterior-lit fixture light in authored space: MOCV (0.1,0.05,0.025) doubled on
# ambient 0.4 (x1.1 at N.L=1) plus direct 0.3; calcLight adds emissive afterwards.
func lit(diffuse: Color, emissive: Color) -> Color:
	return Color(diffuse.r * 0.96 + emissive.r, diffuse.g * 0.85 + emissive.g, diffuse.b * 0.795 + emissive.b)

func rgb_mix(a: Color, b: Color, t: float) -> Color:
	return Color(lerpf(a.r, b.r, t), lerpf(a.g, b.g, t), lerpf(a.b, b.b, t), lerpf(a.a, b.a, t))

func rgb_mul(a: Color, b: Color) -> Color:
	return Color(a.r * b.r, a.g * b.g, a.b * b.b, a.a * b.a)

func rgb_scale(a: Color, k: float) -> Color:
	return Color(a.r * k, a.g * k, a.b * k, a.a * k)

func rgb_add(a: Color, b: Color) -> Color:
	return Color(a.r + b.r, a.g + b.g, a.b + b.b, a.a + b.a)

# commonWMOMaterial.slang caclWMOFragMat for solid layers t1..t3 and MOCV2 alpha a2,
# returning [matDiffuse, emissive].
func reference_fragment(pixel: int, t1: Color, t2: Color, t3: Color, a2: float) -> Array:
	var none := Color(0, 0, 0, 0)
	match pixel:
		-1: return [rgb_mul(t1, t2), none]
		0, 1, 2, 4, 18: return [t1, none]
		3: return [t1, rgb_scale(t2, t1.a)]
		5: return [t1, rgb_mul(rgb_scale(t1, t1.a), t2)]
		6: return [rgb_mix(rgb_mix(t1, t2, t2.a), t1, a2), none]
		7:
			var color_mix := rgb_mix(t1, t2, 1.0 - a2)
			return [color_mix, rgb_mul(rgb_scale(color_mix, color_mix.a), t3)]
		8, 12: return [rgb_mix(t2, t1, a2), none]
		9: return [t1, rgb_scale(t2, t2.a * a2)]
		10:
			var mix_factor := clampf(t3.a * a2, 0.0, 1.0)
			return [rgb_mix(rgb_mix(rgb_scale(rgb_mul(t1, t2), 2.0), t3, mix_factor), t1, t1.a), none]
		11: return [t1, rgb_add(rgb_mul(rgb_scale(t1, t1.a), t2), rgb_scale(t3, t3.a * a2))]
		13: return [rgb_mix(rgb_scale(t2, 1.0 - t2.a), t1, a2), rgb_scale(t2, t2.a * (1.0 - a2))]
		14: return [rgb_mix(rgb_add(rgb_scale(rgb_mul(t1, t2), 2.0), rgb_scale(t3, clampf(t3.a * a2, 0.0, 1.0))), t1, t1.a), none]
		15: return [rgb_scale(rgb_mul(rgb_mix(rgb_mix(t1, t2, t2.a), t1, a2), t3), 2.0), none]
		16: return [rgb_mix(t1, rgb_scale(rgb_mul(t1, t2), 2.0), a2), none]
		17: return [rgb_scale(rgb_mul(rgb_mix(rgb_mix(t1, t2, t2.a), t1, t3.a), t3), 2.0), none]
	fail("no reference for pixel shader %d" % pixel)
	return [none, none]

func assert_retail_formulas() -> bool:
	surface.mesh = wmo_quad(0.5, true)
	var t1 := Color(0.4, 0.3, 0.2, 0.5)
	var t2 := Color(0.2, 0.4, 0.6, 0.75)
	var t3 := Color(0.6, 0.2, 0.4, 1.0)
	material.set_shader_parameter("exterior_lit", true)
	material.set_shader_parameter("base_texture", solid_texture(t1))
	material.set_shader_parameter("second_texture", solid_texture(t2))
	material.set_shader_parameter("third_texture", solid_texture(t3))
	for momt in range(22):
		set_retail_shader(momt)
		var pixel: int = RETAIL_SHADERS[momt][1]
		var reference := reference_fragment(pixel, t1, t2, t3, 0.5)
		if not await assert_pixel("MOMT %d pixel shader %d" % [momt, pixel], lit(reference[0], reference[1])):
			return false
	return true

# Hand-derived values for the shaders the campsite, portal room and farm use.
func assert_retail_emissive_cases() -> bool:
	surface.mesh = wmo_quad(0.5, true)
	material.set_shader_parameter("exterior_lit", true)
	var t1 := Color(0.4, 0.3, 0.2, 1.0)
	material.set_shader_parameter("base_texture", solid_texture(t1))
	# MOMT 9 MapObjDiffuseEmissive: emissive = t2.rgb * t2.a * MOCV2.a = (0.2,0.4,0.6)*0.5.
	material.set_shader_parameter("second_texture", solid_texture(Color(0.2, 0.4, 0.6, 1.0)))
	set_retail_shader(9)
	if not await assert_pixel("MOMT 9 adds second texture by MOCV2 alpha", Color(0.484, 0.455, 0.459)):
		return false
	# MOMT 12 MapObjEnvMetalEmissive: t2 at the sphere-env UV, t3 at MOTV2 (stripe right
	# texel), emissive = t1*t1.a*t2 + t3*t3.a*MOCV2.a = (0.08,0.12,0.12) + (0.3,0.1,0.2).
	material.set_shader_parameter("third_texture", stripe(Color(0.0, 1.0, 0.0, 1.0), Color(0.6, 0.2, 0.4, 1.0)))
	set_retail_shader(12)
	if not await assert_pixel("MOMT 12 env metal plus MOTV2 emissive", Color(0.764, 0.475, 0.479)):
		return false
	# MOMT 16 MapObjDiffuseTerrain is MapObjDiffuse: t1 lit, no second layer.
	set_retail_shader(16)
	if not await assert_pixel("MOMT 16 diffuse terrain ignores extra layers", lit(t1, Color.BLACK)):
		return false
	return true

# MOMT 23 MapObjDFShader: MOC2 (0.5,0.5,0,0.25) weights layers 1/2; heights 0.8/0.4
# (textures 6/7 alpha) give alphaVec (0.4,0.2,0,0) -> (0.4,0.16,0,0) -> (5/7, 2/7).
# mixed = t2*5/7 + t3*2/7 = (0.62857,0.2,0.37143, a 0.85714); diffuse = mixed*(1-0.25);
# emissive = mixed.a * env(0.5) * mixed.rgb.
func assert_df_shader() -> bool:
	material.set_shader_parameter("exterior_lit", true)
	surface.mesh = wmo_quad(0.5, true, Color(0.5, 0.5, 0.0, 0.25))
	material.set_shader_parameter("base_texture", solid_texture(Color(0.5, 0.5, 0.5, 1.0)))
	material.set_shader_parameter("second_texture", solid_texture(Color(0.8, 0.2, 0.2, 1.0)))
	material.set_shader_parameter("third_texture", solid_texture(Color(0.2, 0.2, 0.8, 0.5)))
	material.set_shader_parameter("texture_6", solid_texture(Color(0.0, 0.0, 0.0, 0.8)))
	material.set_shader_parameter("texture_7", solid_texture(Color(0.0, 0.0, 0.0, 0.4)))
	set_retail_shader(23)
	if not await assert_pixel("MOMT 23 height-blends MOC2-weighted layers", Color(0.72196, 0.21321, 0.38064)):
		return false
	# MOC2 (0,0,0,0): all weight on layer 4, texture 5 at MOTV4 (stripe right texel).
	surface.mesh = wmo_quad(0.5, true, Color(0.0, 0.0, 0.0, 0.0))
	material.set_shader_parameter("texture_5", stripe(Color(1.0, 0.0, 0.0, 1.0), Color(0.1, 0.5, 0.3, 1.0)))
	material.set_shader_parameter("texture_9", solid_texture(Color(0.0, 0.0, 0.0, 1.0)))
	if not await assert_pixel("MOMT 23 fourth layer samples MOTV4", Color(0.146, 0.675, 0.3885)):
		return false
	surface.mesh = wmo_quad(0.5, true)
	return true

# MOMT 22 MapObjParallax with solid layers (offsets cannot change a solid sample):
# t6=(r 0.5, g 0.5, b 0.5), t3=t_3=(0.2,0.4,0.6,1), t4=(0.4,0.2,0.2,0.5), t5=(0.2,0.2,0.4,0.5),
# MOCV2.a=0.5: diffuse_result=(0.4,0.3,0.5), result2=(0.3,0.35,0.55), diffuse=(0.35,0.325,0.375);
# mix3 = t3*0.5 + t5*0.5*(1-0.6) = (0.14,0.24,0.38); tex_2 = t3 -> mult (0.2,0.4,0.6);
# fake = (t1*t1.a - mult)*0.5 + mult = (0.3,0.35,0.4); emissive = mix3*0.5 + fake*t3
# = (0.13,0.26,0.43).
func assert_parallax() -> bool:
	material.set_shader_parameter("exterior_lit", true)
	surface.mesh = wmo_quad(0.5, true, Color(0.0, 0.0, 0.0, 1.0), true)
	material.set_shader_parameter("base_texture", solid_texture(Color(0.4, 0.3, 0.2, 1.0)))
	material.set_shader_parameter("third_texture", solid_texture(Color(0.2, 0.4, 0.6, 1.0)))
	material.set_shader_parameter("texture_4", solid_texture(Color(0.4, 0.2, 0.2, 0.5)))
	material.set_shader_parameter("texture_5", solid_texture(Color(0.2, 0.2, 0.4, 0.5)))
	material.set_shader_parameter("texture_6", solid_texture(Color(0.5, 0.5, 0.5, 1.0)))
	set_retail_shader(22)
	var diffuse := Color(0.35, 0.325, 0.375)
	if not await assert_pixel("MOMT 22 parallax layers and fake specular", lit(diffuse, Color(0.13, 0.26, 0.43))):
		return false
	surface.mesh = wmo_quad(0.5, true)
	return true

func assert_missing_retail_streams() -> bool:
	surface.mesh = wmo_quad(0.5, true)
	material.set_shader_parameter("exterior_lit", true)
	material.set_shader_parameter("base_texture", solid_texture(Color(0.5, 0.5, 0.5, 1.0)))
	material.set_shader_parameter("second_texture", solid_texture(Color(0.8, 0.2, 0.2, 1.0)))
	material.set_shader_parameter("texture_5", stripe(Color(0.1, 0.5, 0.3, 1.0), Color(0.3, 0.1, 0.5, 1.0)))
	material.set_shader_parameter("texture_9", solid_texture(Color(0.0, 0.0, 0.0, 1.0)))
	material.set_shader_parameter("has_moc2", false)
	material.set_shader_parameter("has_uv4", false)
	set_retail_shader(23)
	# Missing MOC2 is (0,0,0,1): layer 4 only, then black AO at alpha 1 -> emissive only.
	# Missing MOTV4 is (1,1): the repeat seam averages both stripe texels (0.2,0.3,0.4).
	var layer := Color(0.1, 0.5, 0.3).srgb_to_linear().lerp(Color(0.3, 0.1, 0.5).srgb_to_linear(), 0.5).linear_to_srgb()
	if not await assert_pixel("missing MOC2/MOTV4 use reference defaults", Color(layer.r * 0.5, layer.g * 0.5, layer.b * 0.5)):
		return false
	return true

func assert_directional_shadow() -> bool:
	material.set_shader_parameter("exterior_lit", true)
	var clear := Color(0.4 * 0.96, 0.3 * 0.85, 0.2 * 0.795)
	var shadowed := Color(0.4 * 0.66, 0.3 * 0.55, 0.2 * 0.495)
	if not await assert_pixel("direct visible before caster", clear):
		return false
	var camera := viewport.get_camera_3d()
	camera.near = 0.05
	camera.far = 5.0
	sun.shadow_enabled = true
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	sun.directional_shadow_max_distance = 5.0
	var blocker := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = Vector3(0.4, 0.6, 0.4)
	blocker.mesh = box
	blocker.position.y = 0.7
	blocker.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY
	viewport.add_child(blocker)
	if not await assert_pixel("real shadow removes direct, preserves MOCV ambient", shadowed):
		return false
	blocker.queue_free()
	if not await assert_pixel("direct returns after caster removal", clear):
		return false
	return true

func run_cases() -> void:
	if not ResourceLoader.exists(SHADER_PATH):
		fail("WMO shader missing: " + SHADER_PATH)
		return
	material = ShaderMaterial.new()
	material.shader = load(SHADER_PATH)
	fixture_scene()
	base_inputs()
	if not await assert_opaque_depth():
		return
	base_inputs()
	if not await assert_lighting():
		return
	base_inputs()
	if not await assert_layers():
		return
	base_inputs()
	if not await assert_alpha_and_fog():
		return
	base_inputs()
	if not await assert_directional_shadow():
		return
	for check in [assert_retail_formulas, assert_retail_emissive_cases, assert_df_shader, assert_parallax, assert_missing_retail_streams]:
		base_inputs()
		if not await check.call():
			return
	quit(0)
