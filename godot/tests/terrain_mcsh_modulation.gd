extends "res://tests/terrain_mcsh_pixels.gd"

# Fixed test operand for the supplied historical m_colors[2].r. This value is
# deliberately not relabeled as a modern LightData column without evidence.
const AMBIENT_BYTES := Vector3i(24, 90, 180)
const ALPHA_BYTE := 128

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	output = OS.get_environment("GODOT_MCSH_SCREENSHOTS")
	if output.is_empty() or not DirAccess.dir_exists_absolute(output):
		fail("GODOT_MCSH_SCREENSHOTS required")
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
	for mode in [0, 1, 2, 3, 4]:
		if not await check_modulation(mode):
			return
	stage.free()
	probe = null
	print("PASS: supplied S3 byte modulation and0x10 gate; alpha-channel production mapping remains separate")
	quit(0)

func check_modulation(mode: int) -> bool:
	var result: Dictionary = probe.chunk(mode)
	if result.has("error"):
		fail(str(result.error))
		return false
	var node: MeshInstance3D = result.node
	stage.add_child(node)
	var material := node.material_override as ShaderMaterial
	material.set_shader_parameter("ambient", Vector3(AMBIENT_BYTES) / 255.0)
	material.set_shader_parameter("horizon_ambient", Vector3(AMBIENT_BYTES) / 255.0)
	material.set_shader_parameter("ground_ambient", Vector3(AMBIENT_BYTES) / 255.0)
	material.set_shader_parameter("direct", Vector3.ZERO)
	material.set_shader_parameter("sun_direction", Vector3.DOWN)
	material.set_shader_parameter("fog_opacity", 0.0)
	var bounds := node.get_aabb()
	camera3d.size = maxf(bounds.size.x, bounds.size.z)
	camera3d.look_at_from_position(Vector3(0.0, bounds.size.length()*2.0, 0.0), Vector3.ZERO, Vector3.FORWARD)
	var overlay := material.next_pass as ShaderMaterial
	if overlay == null:
		fail("Authored MCSH has no surface modulation pass")
		return false
	overlay.set_shader_parameter("ambient", Vector3(AMBIENT_BYTES)/255.0)
	overlay.set_shader_parameter("shadow_alpha", float(ALPHA_BYTE)/255.0)
	overlay.set_shader_parameter("world_shadow_enable_bits", 0)
	var before := await capture("s3-modulation-mode%d-before.png" % mode)
	overlay.set_shader_parameter("world_shadow_enable_bits", 16)
	var after := await capture("s3-modulation-mode%d-after.png" % mode)
	# Each channel applies exactly (85*(ambient_byte+3))>>8, then ordinary alpha blend.
	var color := Color(float((85*(AMBIENT_BYTES.x+3))>>8)/255.0, float((85*(AMBIENT_BYTES.y+3))>>8)/255.0, float((85*(AMBIENT_BYTES.z+3))>>8)/255.0)
	var mask: PackedByteArray = result.mask
	var arrays := node.mesh.surface_get_arrays(0)
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var uvs: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV]
	var origin := find_corner(vertices, uvs, Vector2.ZERO)
	var across := find_corner(vertices, uvs, Vector2.RIGHT)-origin
	var down := find_corner(vertices, uvs, Vector2.DOWN)-origin
	for row in 64:
		for col in 64:
			var uv := Vector2((col+0.5)/64.0, (row+0.5)/64.0)
			var pixel := camera3d.unproject_position(node.to_global(origin+across*uv.x+down*uv.y))
			var x := roundi(pixel.x)
			var y := roundi(pixel.y)
			var base := before.get_pixel(x,y)
			var expected := base
			if not mask.is_empty() and mask[row*8+col/8] & (1 << (col%8)):
				expected = base.srgb_to_linear().lerp(color.srgb_to_linear(), float(ALPHA_BYTE)/255.0).linear_to_srgb()
			var actual := after.get_pixel(x,y)
			if maxf(absf(actual.r-expected.r),maxf(absf(actual.g-expected.g),absf(actual.b-expected.b))) > 2.0/255.0:
				fail("S3 modulation mismatch mode%d row%d col%d expected%s actual%s" % [mode,row,col,expected,actual])
				return false
	overlay.set_shader_parameter("world_shadow_enable_bits", 1)
	var disabled := await capture("s3-modulation-mode%d-wrongbit.png" % mode)
	if disabled.get_data() != before.get_data():
		fail("Wrong shadow enable bit changed pixels")
		return false
	print("PASS S3 modulation mode",mode," cells4096")
	node.free()
	return true
