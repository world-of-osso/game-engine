extends SceneTree

# S3 DATA-COVERAGE diagnostic, not a guessed terrain shadow modulation equation.
# Real cached split chunk: no grass, doodads or UI skin are instantiated.
const COVERAGE_SHADER := """
shader_type spatial;
render_mode unshaded, fog_disabled;
uniform sampler2D mcsh_mask : hint_default_black, filter_nearest, repeat_disable;
uniform bool coverage_enabled = true;
void fragment() { ALBEDO = vec3(coverage_enabled ? texture(mcsh_mask, UV).r : 0.0); }
"""
var stage: Node3D
var camera3d: Camera3D
var probe: Object
var output := ""
var baseline_arrays: Array
var baseline_alpha: PackedByteArray
var baseline_config: Vector4

func _initialize() -> void:
	call_deferred("run_test")

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	output = OS.get_environment("GODOT_MCSH_SCREENSHOTS")
	if output.is_empty() or not DirAccess.dir_exists_absolute(output):
		fail("Existing GODOT_MCSH_SCREENSHOTS directory required")
		return
	probe = ClassDB.instantiate("WowTerrainShadowProbe")
	stage = Node3D.new()
	root.add_child(stage)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color(0.1, 0.0, 0.1)
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_DISABLED
	environment.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	environment.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	var world := WorldEnvironment.new()
	world.environment = environment
	stage.add_child(world)
	camera3d = Camera3D.new()
	camera3d.projection = Camera3D.PROJECTION_ORTHOGONAL
	stage.add_child(camera3d)
	camera3d.current = true
	var sun := DirectionalLight3D.new()
	sun.rotation.x = -PI / 2.0
	sun.shadow_enabled = false
	stage.add_child(sun)
	for sample in [["zero", 0, true], ["full", 1, true], ["half", 2, true], ["missing", 3, true], ["real", 4, true], ["disabled", 4, false]]:
		if not await check_case(sample[0], sample[1], sample[2]):
			return
	stage.free()
	probe = null
	print("PASS: S3 native mask binding and diagnostic coverage; modulation/policy NOT proven")
	quit(0)

func check_case(label: String, mode: int, enabled: bool) -> bool:
	var result: Dictionary = probe.chunk(mode)
	if result.has("error"):
		fail("MCSH fixture asset error: " + str(result.error))
		return false
	var node: MeshInstance3D = result.node
	stage.add_child(node)
	var material := node.material_override as ShaderMaterial
	var mask: PackedByteArray = result.mask
	var texture = material.get_shader_parameter("mcsh_mask")
	if not mask.is_empty() and not texture is Texture2D:
		fail("MCSH payload did not reach native terrain material: " + label)
		return false
	if mask.is_empty() and texture != null:
		fail("Missing MCSH unexpectedly binds a payload")
		return false
	if texture is Texture2D and not check_uploaded_bits(texture.get_image(), mask):
		return false
	if not check_unchanged_inputs(node, material, mode):
		return false
	var aabb := node.get_aabb()
	camera3d.size = maxf(aabb.size.x, aabb.size.z)
	camera3d.look_at_from_position(Vector3(0.0, aabb.size.length() * 2.0, 0.0), Vector3.ZERO, Vector3.FORWARD)
	material.set_shader_parameter("ambient", Vector3.ONE)
	material.set_shader_parameter("horizon_ambient", Vector3.ONE)
	material.set_shader_parameter("ground_ambient", Vector3.ONE)
	material.set_shader_parameter("direct", Vector3.ZERO)
	material.set_shader_parameter("sun_direction", Vector3.DOWN)
	material.set_shader_parameter("fog_opacity", 0.0)
	if label == "real":
		await capture("s3-real-surface-unmodified.png")
	var diagnostic := ShaderMaterial.new()
	var shader := Shader.new()
	shader.code = COVERAGE_SHADER
	diagnostic.shader = shader
	if texture != null:
		diagnostic.set_shader_parameter("mcsh_mask", texture)
	diagnostic.set_shader_parameter("coverage_enabled", enabled)
	node.material_override = diagnostic
	var image := await capture("s3-coverage-" + label + ".png")
	var arrays := node.mesh.surface_get_arrays(0)
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var uvs: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV]
	var origin := find_corner(vertices, uvs, Vector2.ZERO)
	var across := find_corner(vertices, uvs, Vector2(1.0, 0.0)) - origin
	var down := find_corner(vertices, uvs, Vector2(0.0, 1.0)) - origin
	var covered := 0
	for row in 64:
		for col in 64:
			var uv := Vector2((col + 0.5) / 64.0, (row + 0.5) / 64.0)
			var point := node.to_global(origin + across * uv.x + down * uv.y)
			var pixel := camera3d.unproject_position(point)
			var actual := image.get_pixel(roundi(pixel.x), roundi(pixel.y))
			var bit := enabled and not mask.is_empty() and (mask[row * 8 + col / 8] & (1 << (col % 8))) != 0
			var expected := 1.0 if bit else 0.0
			if absf(actual.r - expected) > 1.0 / 255.0 or actual.r != actual.g or actual.r != actual.b:
				fail("Coverage/UV mismatch %s row%d col%d expected%s actual%s" % [label, row, col, bit, actual])
				return false
			if bit:
				covered += 1
	print("S3 coverage ", label, " cells=4096 covered=", covered, " real_source_set_bits=", count_bits(result.source_mask))
	node.free()
	return true

func check_uploaded_bits(image: Image, mask: PackedByteArray) -> bool:
	if image.get_size() != Vector2i(64, 64):
		fail("MCSH texture must contain64x64 coverage texels")
		return false
	for row in 64:
		for col in 64:
			var expected := 1.0 if (mask[row * 8 + col / 8] & (1 << (col % 8))) != 0 else 0.0
			if image.get_pixel(col, row).r != expected:
				fail("MCSH uploaded bit/row orientation mismatch")
				return false
	return true

func check_unchanged_inputs(node: MeshInstance3D, material: ShaderMaterial, mode: int) -> bool:
	var arrays := node.mesh.surface_get_arrays(0)
	var alpha: Texture2D = material.get_shader_parameter("alpha_packed")
	var config: Vector4 = material.get_shader_parameter("config")
	if mode == 0:
		baseline_arrays = arrays
		baseline_alpha = alpha.get_image().get_data()
		baseline_config = config
	elif arrays != baseline_arrays or alpha.get_image().get_data() != baseline_alpha or config != baseline_config:
		fail("Changing only MCSH changed geometry/MCCV/MCAL/layer config")
		return false
	return true

func find_corner(vertices: PackedVector3Array, uvs: PackedVector2Array, uv: Vector2) -> Vector3:
	for index in uvs.size():
		if uvs[index] == uv:
			return vertices[index]
	fail("Chunk UV corner missing: " + str(uv))
	return Vector3.ZERO

func count_bits(mask: PackedByteArray) -> int:
	var total := 0
	for byte in mask:
		for bit in 8:
			if byte & (1 << bit):
				total += 1
	return total

func capture(name: String) -> Image:
	await process_frame
	await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.get_size() != Vector2i(1920, 1080) or image.save_png(output + "/" + name) != OK:
		fail("MCSH capture failed: " + name)
	return image
