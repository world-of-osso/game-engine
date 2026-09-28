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

func wmo_quad(second_alpha: float, with_custom: bool) -> ArrayMesh:
	var arrays := PlaneMesh.new().surface_get_arrays(0)
	var colors := PackedColorArray()
	var uvs := PackedVector2Array()
	var uv2s := PackedVector2Array()
	var custom := PackedFloat32Array()
	for vertex in arrays[Mesh.ARRAY_VERTEX].size():
		colors.append(Color(0.1, 0.05, 0.025, 0.5))
		uvs.append(Vector2(0.25, 0.5))
		uv2s.append(Vector2(0.75, 0.5))
		custom.append_array(PackedFloat32Array([second_alpha, 0.0, 0.0, 0.0]))
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = uv2s
	var flags := 0
	if with_custom:
		arrays[Mesh.ARRAY_CUSTOM0] = custom
		flags = Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays, [], {}, flags)
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
	var flags := 0
	if arrays[Mesh.ARRAY_CUSTOM0] != null:
		flags = Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays, [], {}, flags)
	surface.mesh = mesh

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
	material.set_shader_parameter("two_layer_shader", 0)
	material.set_shader_parameter("blend_mode", 0)
	material.set_shader_parameter("ambient", Vector3(0.4, 0.4, 0.4))
	material.set_shader_parameter("horizon_ambient", Vector3(0.4, 0.4, 0.4))
	material.set_shader_parameter("ground_ambient", Vector3(0.4, 0.4, 0.4))
	material.set_shader_parameter("direct", Vector3(0.3, 0.3, 0.3))
	material.set_shader_parameter("sun_direction", Vector3.DOWN)
	material.set_shader_parameter("fog_mode", 0)
	material.set_shader_parameter("fog_color", Vector3.ZERO)
	material.set_shader_parameter("fog_range", Vector2.ZERO)
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
	material.set_shader_parameter("two_layer_shader", 6)
	if not await assert_pixel("MOMT6 second texture alpha then MOCV2 half", Color(0.75, 0.0, 0.25)):
		return false
	material.set_shader_parameter("two_layer_shader", 13)
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
	material.set_shader_parameter("two_layer_shader", 0)
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
	material.set_shader_parameter("fog_range", Vector2(1.0, 3.0))
	material.set_shader_parameter("fog_opacity", 1.0)
	material.set_shader_parameter("fog_color", Color(0.0, 0.0, 1.0).srgb_to_linear())
	material.set_shader_parameter("unfogged", false)
	if not await assert_pixel("unlit still fogged halfway", Color(0.2, 0.15, 0.6)):
		return false
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
	quit(0)
