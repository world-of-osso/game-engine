extends SceneTree

# MAIN GPU invocation (no new flags):
# godot --path godot -s res://tests/skybox_static_sampler.gd -- --screen skyboxdebug
#   --skybox-fdid 525142 --skybox-verify --skybox-time-ms 100000
# Synthetic sampler witnesses using the actual authored Fog21 shader object.
# Not a coastal image-parity test; sampler success falsifies only this lead.
const SIZE: int = 64
const SETUP_MS: int = 30000
const GPU_SECONDS: float = 20.0
const TOLERANCE: int = 2
const RED_CODES: Vector4i = Vector4i(128, 0, 0, 255)
const BLUE_CODES: Vector4i = Vector4i(0, 0, 128, 255)
const LEVEL_SIZES: Array[int] = [64, 32, 16, 8, 4, 2, 1]
const CLAMP_POINTS: Array[Vector2i] = [
	Vector2i(20, 30), Vector2i(22, 34), Vector2i(41, 30), Vector2i(43, 34),
]
const MIP_POINTS: Array[Vector2i] = [Vector2i(28, 28), Vector2i(32, 32), Vector2i(36, 36)]

var client: Node
var production_shader: Shader
var selected_path: String = ""
var shader_hash: String = ""
var viewport: SubViewport
var quad: MeshInstance3D
var finished: bool = false
var setup_complete: bool = false
var controls_passed: bool = false
var shots_directory: String = ""

func _initialize() -> void:
	create_timer(float(SETUP_MS) / 1000.0).timeout.connect(on_setup_timeout)
	call_deferred("run_cases")

func fail(category: String, message: String) -> void:
	if finished:
		return
	finished = true
	push_error("skybox_static_sampler %s: %s" % [category, message])
	quit(1)

func on_setup_timeout() -> void:
	if not setup_complete:
		fail("SETUP", "30s production source/camera/SkyBatch21 deadline")

func on_gpu_timeout() -> void:
	fail("SETUP", "20s GPU-case deadline; incomplete real draws/readbacks are not sampler RED")

func first_value(args: PackedStringArray, flag: String) -> String:
	var index: int = args.find(flag)
	if index < 0 or index + 1 >= args.size():
		return ""
	return args[index + 1]

func mount_production() -> bool:
	var args: PackedStringArray = OS.get_cmdline_user_args()
	if (first_value(args, "--screen") != "skyboxdebug"
		or first_value(args, "--skybox-fdid") != "525142"
		or first_value(args, "--skybox-time-ms") != "100000"
		or not args.has("--skybox-verify") or args.has("--light-skybox-id")):
		fail("SETUP", "requires --screen skyboxdebug --skybox-fdid 525142 --skybox-verify --skybox-time-ms 100000")
		return false
	if not ClassDB.class_exists("GameClient"):
		fail("SETUP", "actual GameClient native class unavailable")
		return false
	var packed: PackedScene = load("res://scenes/client.tscn") as PackedScene
	if packed == null:
		fail("SETUP", "production client.tscn absent")
		return false
	client = packed.instantiate()
	if client == null or not client.is_class("GameClient"):
		fail("SETUP", "production scene root is not GameClient")
		return false
	root.add_child(client)
	return true

func select_authored_shader() -> bool:
	var deadline: int = Time.get_ticks_msec() + SETUP_MS
	while Time.get_ticks_msec() < deadline and not finished:
		await process_frame
		if finished:
			return false
		if not is_instance_valid(client) or client.is_queued_for_deletion():
			fail("SETUP", "production client disappeared")
			return false
		var scene: Node3D = client.get_node_or_null("SkyboxDebug") as Node3D
		if scene == null or not scene.is_node_ready():
			continue
		var sources: Array[Node3D] = []
		for node in scene.find_children("*", "Node3D", true, false):
			if node.has_meta("m2_source_path"):
				sources.append(node as Node3D)
		if sources.size() > 1:
			fail("SETUP", "ambiguous authored M2 source roots")
			return false
		if sources.is_empty():
			continue
		var sky: Node3D = sources[0]
		var camera: Camera3D = root.get_camera_3d()
		if camera == null:
			continue
		if not scene.find_children("*", "Camera3D", true, false).has(camera):
			fail("SETUP", "active camera is not the production SkyboxDebug camera")
			return false
		var source: String = str(sky.get_meta("m2_source_path"))
		if (not source.replace("\\", "/").ends_with("/models/skyboxes/costalislandskybox.m2")
			or not FileAccess.file_exists(source)):
			fail("SETUP", "actual coastal M2 source absent/wrong: " + source)
			return false
		var batch: MeshInstance3D = sky.get_node_or_null("SkyBatch21") as MeshInstance3D
		if batch == null or batch.mesh == null or batch.mesh.get_surface_count() == 0:
			continue
		var material: ShaderMaterial = batch.get_active_material(0) as ShaderMaterial
		if material == null or material.shader == null or material.shader.code.is_empty():
			fail("SETUP", "SkyBatch21 actual ShaderMaterial/shader absent")
			return false
		var arrays: Array = batch.mesh.surface_get_arrays(0)
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var uvs: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV]
		if vertices.is_empty() or uvs.size() != vertices.size():
			fail("SETUP", "SkyBatch21 actual vertex/UV arrays absent")
			return false
		if not material.get_shader_parameter("base_texture") is Texture2D:
			fail("SETUP", "SkyBatch21 production base texture absent")
			return false
		if material.get_shader_parameter("has_second_texture") != false:
			fail("SETUP", "SkyBatch21 is not the single-stage Fog witness")
			return false
		production_shader = material.shader
		selected_path = str(batch.get_path())
		shader_hash = production_shader.code.sha256_text()
		var minimum: Vector2 = uvs[0]
		var maximum: Vector2 = uvs[0]
		for uv in uvs:
			minimum = minimum.min(uv)
			maximum = maximum.max(uv)
		print("SceneInputs argv=", Array(OS.get_cmdline_user_args()),
			" scene=", scene.get_path(), " sky=", sky.get_path(),
			" source=", source, " source_sha256=", FileAccess.get_sha256(source),
			" camera=", camera.get_path(), " camera_transform=", camera.global_transform,
			" camera_fov=", camera.fov, " selected_production_path=", selected_path,
			" surface=0 shader_resource=", production_shader.resource_path,
			" shader_sha256=", shader_hash, " vertices=", vertices.size(),
			" actual_uv_min=", minimum, " actual_uv_max=", maximum,
			" source_base_color=", material.get_shader_parameter("base_color"),
			" source_transparency=", material.get_shader_parameter("transparency"),
			" source_secondary=", material.get_shader_parameter("has_second_texture"),
			" source_uv_mode=", material.get_shader_parameter("uv_mode_1"),
			" source_uv_offset=", material.get_shader_parameter("uv_offset_1"))
		return true
	fail("SETUP", "30s production source/camera/SkyBatch21 readiness bound")
	return false

func build_viewport() -> void:
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
	quad = MeshInstance3D.new()
	quad.position = Vector3(0.0, 0.0, -2.0)
	viewport.add_child(quad)

func set_quad_uvs(minified: bool) -> void:
	var source_quad: QuadMesh = QuadMesh.new()
	source_quad.size = Vector2(2.0, 2.0)
	var arrays: Array = source_quad.get_mesh_arrays()
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var uvs: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV]
	if vertices.is_empty() or uvs.size() != vertices.size():
		fail("SETUP", "QuadMesh actual vertex/UV arrays absent")
		return
	for index in range(uvs.size()):
		uvs[index] = uvs[index] * 64.0 if minified else uvs[index] * 2.0 - Vector2(0.5, 0.5)
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	var mesh: ArrayMesh = ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	quad.mesh = mesh
	print("QUAD vertices=", vertices.size(), " all_actual_vertex_uvs=", uvs,
		" formula=", "UV*64" if minified else "UV*2-.5", " fov=90 z=-2 extent=64x64")

func create_texture(minified: bool) -> ImageTexture:
	var bytes: PackedByteArray = PackedByteArray()
	var levels: Array[int] = LEVEL_SIZES if minified else [SIZE]
	for level in range(levels.size()):
		var size: int = levels[level]
		for y in range(size):
			for x in range(size):
				var codes: Vector4i = RED_CODES
				if minified and level > 0:
					codes = Vector4i(0, 128, 0, 255)
				elif not minified and x >= (size >> 1):
					codes = BLUE_CODES
				bytes.append_array(PackedByteArray([codes.x, codes.y, codes.z, codes.w]))
	if minified and bytes.size() != 21844:
		fail("SETUP", "incorrect complete authored mip chain byte count")
		return null
	var image: Image = Image.create_from_data(SIZE, SIZE, minified, Image.FORMAT_RGBA8, bytes)
	if image == null or image.is_empty() or (minified and image.get_mipmap_count() != 6):
		fail("SETUP", "RGBA8 authored texture construction failed")
		return null
	var texture: ImageTexture = ImageTexture.create_from_image(image)
	if texture == null:
		fail("SETUP", "synthetic texture upload failed")
		return null
	var uploaded: Image = texture.get_image()
	if uploaded == null or uploaded.get_data() != bytes:
		fail("SETUP", "uploaded texture differs from authored RGBA8/mip bytes")
		return null
	print("TEXTURE minified=", minified, " levels=", levels, " bytes=", bytes.size(),
		" base=", RED_CODES, " other_codes=", "smaller_levels=green128" if minified else "right_half=blue128",
		" alpha=255; no generated mipmaps")
	return texture

func create_control(texture: ImageTexture) -> StandardMaterial3D:
	var material: StandardMaterial3D = StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR
	material.texture_repeat = false
	material.transparency = BaseMaterial3D.TRANSPARENCY_DISABLED
	material.albedo_color = Color.WHITE
	material.albedo_texture = texture
	return material

func create_production_material(texture: ImageTexture) -> ShaderMaterial:
	var material: ShaderMaterial = ShaderMaterial.new()
	# Share the selected live Shader itself; never load a substitute or rewrite code.
	material.shader = production_shader
	for parameter in ["base_texture", "second_texture", "third_texture", "fourth_texture"]:
		material.set_shader_parameter(parameter, texture)
	for parameter in ["has_second_texture", "has_third_texture", "has_fourth_texture"]:
		material.set_shader_parameter(parameter, false)
	for parameter in ["uv_mode_1", "uv_mode_2", "uv_mode_3", "uv_mode_4"]:
		material.set_shader_parameter(parameter, 0)
	for parameter in ["uv_offset_1", "uv_offset_2"]:
		material.set_shader_parameter(parameter, Vector2.ZERO)
	material.set_shader_parameter("combine_mode", 0)
	material.set_shader_parameter("base_color", Color.WHITE)
	material.set_shader_parameter("transparency", 1.0)
	material.set_shader_parameter("alpha_test", 0.0)
	print("BINDINGS selected_production_path=", selected_path, " shader_sha256=", shader_hash,
		" same_shader_object=", material.shader == production_shader,
		" all_textures=synthetic all_secondary=false all_uv_modes=0 offsets=0",
		" combine=0 base_color=white transparency=1 alpha_test=0; no sampler override")
	return material

func encoded_pixel(image: Image, point: Vector2i) -> Vector4i:
	var color: Color = image.get_pixelv(point)
	return Vector4i(int(round(color.r * 255.0)), int(round(color.g * 255.0)),
		int(round(color.b * 255.0)), int(round(color.a * 255.0)))

func check_case(label: String, material: Material, minified: bool) -> bool:
	set_quad_uvs(minified)
	if finished:
		return false
	quad.material_override = material
	for _draw in range(2):
		await RenderingServer.frame_post_draw
		if finished:
			return false
	var image: Image = viewport.get_texture().get_image()
	if image == null or image.is_empty() or image.get_size() != Vector2i(SIZE, SIZE):
		fail("SETUP", label + ": missing actual 64x64 GPU image")
		return false
	if not shots_directory.is_empty():
		var path: String = shots_directory.path_join(label + ".png")
		var error: Error = image.save_png(path)
		if error != OK:
			fail("SETUP", "%s: capture failed %s: %s" % [label, error, path])
			return false
		print("CAPTURE path=", path)
	var background: Vector4i = encoded_pixel(image, Vector2i(2, 2))
	if not quad.is_visible_in_tree() or background != Vector4i(0, 0, 0, 255):
		fail("SETUP", label + ": quad visibility/opaque black background invalid")
		return false
	var passed: bool = true
	var points: Array[Vector2i] = MIP_POINTS if minified else CLAMP_POINTS
	for point in points:
		var expected: Vector4i = RED_CODES if minified or point.x < (SIZE >> 1) else BLUE_CODES
		var actual: Vector4i = encoded_pixel(image, point)
		var matches: bool = (absi(actual.x - expected.x) <= TOLERANCE
			and absi(actual.y - expected.y) <= TOLERANCE
			and absi(actual.z - expected.z) <= TOLERANCE and actual.w == expected.w)
		passed = passed and matches
		# The quad projects to [16,48); pixel centers supply independent witness UVs.
		var quad_uv: Vector2 = (Vector2(point) + Vector2(0.5, 0.5) - Vector2(16, 16)) / 32.0
		var witness_uv: Vector2 = quad_uv * 64.0 if minified else quad_uv * 2.0 - Vector2(0.5, 0.5)
		print(label, " pixel=", point, " witness_uv=", witness_uv, " actual=", actual,
			" expected=", expected, " tolerance=2 codes alpha_exact=255 ", "PASS" if matches else "FAIL",
			" selected_production_path=", selected_path, " shader_sha256=", shader_hash)
	return passed

func run_cases() -> void:
	if not mount_production():
		return
	if not await select_authored_shader():
		return
	setup_complete = true
	create_timer(GPU_SECONDS).timeout.connect(on_gpu_timeout)
	shots_directory = OS.get_environment("SKYBOX_STATIC_SAMPLER_SHOTS")
	if not shots_directory.is_empty():
		var error: Error = DirAccess.make_dir_recursive_absolute(shots_directory)
		if error != OK:
			fail("SETUP", "cannot create optional capture directory: " + shots_directory)
			return
	print("LIMITS setup=30s GPU_cases=20s; isolated synthetic clamp/mip0 only;",
		" no production actor/material/clock mutation; no coastal parity/all-mismatch attribution")
	build_viewport()
	var gradient: ImageTexture = create_texture(false)
	var mips: ImageTexture = create_texture(true)
	if finished:
		return
	var clamp_control: bool = await check_case("control_clamp_linear_no_mip", create_control(gradient), false)
	if finished:
		return
	var mip_control: bool = await check_case("control_static_mip0", create_control(mips), true)
	if finished:
		return
	controls_passed = clamp_control and mip_control
	if not controls_passed:
		fail("SETUP", "linear NO-MIP repeat=false controls failed; production not evaluated")
		return
	var clamp_passed: bool = await check_case("production_static_clamp", create_production_material(gradient), false)
	if finished:
		return
	var mip_passed: bool = await check_case("production_static_mip0", create_production_material(mips), true)
	if finished:
		return
	print("RESULT controls=PASS clamp=", clamp_passed, " mip0=", mip_passed)
	if not clamp_passed or not mip_passed:
		fail("RED", "controls passed; actual authored Fog21 shader violates static clamp and/or mip0 contract")
		return
	finished = true
	print("PASS: actual Fog21 shader satisfies synthetic static clamp/mip0 witnesses; falsifies this sampler lead only, not coastal parity")
	quit(0)
