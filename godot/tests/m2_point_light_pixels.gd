extends SceneTree

# M2 point-light falloff on the opaque M2, WMO and terrain shaders: an OmniLight3D set up
# as assets/m2_lights.rs does (range = attenuation end, exponent 0, specular = start /
# end) must light a floor with retail's squared linear ramp (WebWowViewerCpp
# pointLight.frag.slang:51-58), not Godot's (1 - (d / range)^4)^2 window.

const SIZE := 64
const HEIGHT := 0.25
const START := 0.3
const END := 0.9
const TOLERANCE := 0.02

var viewport: SubViewport
var floor_instance: MeshInstance3D
var light: OmniLight3D

func _initialize() -> void:
	call_deferred("run_cases")

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func solid_texture(color: Color) -> ImageTexture:
	var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	image.fill(color)
	return ImageTexture.create_from_image(image)

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
	var arrays := PlaneMesh.new().surface_get_arrays(0)
	var colors := PackedColorArray()
	var uv1 := PackedVector2Array()
	var uv2 := PackedVector2Array()
	for vertex in arrays[Mesh.ARRAY_VERTEX].size():
		colors.append(Color(0.5, 0.5, 0.5, 1.0))
		uv1.append(Vector2(0.25, 0.5))
		uv2.append(Vector2(0.75, 0.5))
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_TEX_UV] = uv1
	arrays[Mesh.ARRAY_TEX_UV2] = uv2
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	floor_instance = MeshInstance3D.new()
	floor_instance.mesh = mesh
	viewport.add_child(floor_instance)
	light = OmniLight3D.new()
	light.position = Vector3(0.0, HEIGHT, 0.0)
	light.omni_range = END
	light.omni_attenuation = 0.0
	light.light_specular = START / END
	light.shadow_enabled = false
	viewport.add_child(light)

func m2_material() -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = load("res://shaders/m2.gdshader")
	material.set_shader_parameter("effect_mode", 0)
	material.set_shader_parameter("base_texture", solid_texture(Color(0.5, 0.4, 0.3, 1.0)))
	material.set_shader_parameter("base_color", Color.WHITE)
	material.set_shader_parameter("transparency", 1.0)
	material.set_shader_parameter("render_flags", 2)
	material.set_shader_parameter("gx_blend", 0)
	for name in ["ambient", "horizon_ambient", "ground_ambient", "direct"]:
		material.set_shader_parameter(name, Vector3.ZERO)
	material.set_shader_parameter("sun_direction", Vector3.DOWN)
	return material

func wmo_material() -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = load("res://shaders/wmo.gdshader")
	material.set_shader_parameter("base_texture", solid_texture(Color(0.5, 0.4, 0.3, 1.0)))
	material.set_shader_parameter("base_color", Color.WHITE)
	material.set_shader_parameter("emissive", Vector3.ZERO)
	material.set_shader_parameter("unlit", false)
	material.set_shader_parameter("unfogged", true)
	for name in ["ambient", "horizon_ambient", "ground_ambient", "direct", "interior_ambient"]:
		material.set_shader_parameter(name, Vector3.ZERO)
	material.set_shader_parameter("sun_direction", Vector3.DOWN)
	return material

func terrain_material() -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = load("res://shaders/terrain.gdshader")
	material.set_shader_parameter("config", Vector4(1.0, 0.0, 1.0, 0.0))
	material.set_shader_parameter("layer_params_0", Vector4(1.0, 0.0, 0.0, 1.0))
	material.set_shader_parameter("height_0", solid_texture(Color(0.0, 0.0, 0.0, 0.5)))
	material.set_shader_parameter("ground_0", solid_texture(Color(0.5, 0.4, 0.3, 0.0)))
	material.set_shader_parameter("alpha_packed", solid_texture(Color(0.0, 0.0, 0.0, 1.0)))
	for name in ["ambient", "horizon_ambient", "ground_ambient", "direct"]:
		material.set_shader_parameter(name, Vector3.ZERO)
	material.set_shader_parameter("sun_direction", Vector3.DOWN)
	return material

func capture() -> Image:
	for frame in 2:
		await process_frame
		await RenderingServer.frame_post_draw
	return viewport.get_texture().get_image()

# Floor position of `pixel`'s centre (orthographic, 2 yd across, camera -Z up).
func floor_position(pixel: Vector2i) -> Vector3:
	var scale := 2.0 / SIZE
	return Vector3((pixel.x + 0.5) * scale - 1.0, 0.0, (pixel.y + 0.5) * scale - 1.0)

# Retail: squared ramp x N.L, relative to a light straight overhead.
func retail_relative(pixel: Vector2i) -> float:
	var distance := (floor_position(pixel) - light.position).length()
	var ramp := 1.0 - clampf((distance - START) / (END - START), 0.0, 1.0)
	return ramp * ramp * HEIGHT / distance

func godot_relative(pixel: Vector2i) -> float:
	var distance := (floor_position(pixel) - light.position).length()
	var window := maxf(1.0 - pow(distance / END, 4.0), 0.0)
	return window * window * HEIGHT / distance

func light_added(lit: Image, dark: Image, pixel: Vector2i) -> float:
	return lit.get_pixelv(pixel).srgb_to_linear().r - dark.get_pixelv(pixel).srgb_to_linear().r

func assert_falloff(label: String, material: ShaderMaterial) -> bool:
	floor_instance.material_override = material
	light.visible = true
	var lit := await capture()
	light.visible = false
	var dark := await capture()
	if lit == null or lit.is_empty() or dark == null or dark.is_empty():
		fail("GPU pixel readback unavailable")
		return false
	var centre := Vector2i(SIZE / 2, SIZE / 2)
	var full := light_added(lit, dark, centre) / retail_relative(centre)
	if full < 0.1 or lit.get_pixelv(centre).r > 0.97:
		fail("%s: overhead light adds %f (lit %s)" % [label, full, lit.get_pixelv(centre)])
		return false
	# 0.35-0.9 yd from the light: inside the start, on the ramp, and past the end.
	for column in [38, 44, 48, 52, 54, 62]:
		var pixel := Vector2i(column, SIZE / 2)
		var actual := light_added(lit, dark, pixel) / full
		var expected := retail_relative(pixel)
		if absf(actual - expected) > TOLERANCE:
			fail("%s at %s: relative light %f, retail %f, Godot window %f" % [label, pixel, actual, expected, godot_relative(pixel)])
			return false
		print("PASS: %s at %s relative %f (retail %f, Godot window %f)" % [label, pixel, actual, expected, godot_relative(pixel)])
	return true

func run_cases() -> void:
	fixture_scene()
	for case in [["M2", m2_material()], ["WMO", wmo_material()], ["terrain", terrain_material()]]:
		if not await assert_falloff(case[0], case[1]):
			return
	quit(0)
