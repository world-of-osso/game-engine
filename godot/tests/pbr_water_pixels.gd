extends SceneTree

## Forever Material130 PBR water GPU fixture. The material is the authored Forever row
## (LiquidType 1251, LiquidObject 18420, map 2991) with its six LiquidTypeXTexture masks
## and 38 floats. Checks that the PBR path is selected, all six authored textures are
## bound, and the row foam weights and colours change the pixels.

const WOW_POSITION := Vector3(3065.0, 756.58972, -1380.0)
const WATER := Vector3(3065.0, 756.58972, -1380.0)
const NOON := 1440.0
const SIZE := Vector2i(128, 64)

var material: ShaderMaterial
var scene_light: Dictionary
var viewport: SubViewport
var water_root: Node3D

func _initialize() -> void:
	call_deferred("run")

func run() -> void:

	if DisplayServer.get_name() == "headless":
		fail("PBR water pixels require a display")
		return
	root.get_node("M2MaterialClock").set_process(false)
	var loader = ClassDB.instantiate("WowTerrainLoader")
	var result: Dictionary = loader.load_liquid_material(1279, 21229, 2991, WOW_POSITION, NOON)
	if result.has("error"):
		fail("load_liquid_material: " + str(result.error))
		return
	material = result.material
	material.set_shader_parameter("mesh_uv", true)
	scene_light = result.scene_light
	var checks := [check_pbr_bindings, check_mask_coverage, check_row_colours, check_layer_uv]
	for check in checks:
		var error: String = await check.call()
		if error != "":
			fail(error)
			return
	print("PASS: PBR_WATER_PIXELS")
	quit(0)

func check_pbr_bindings() -> String:
	if material.get_shader_parameter("pbr_water") != true:
		return "Material130 did not select the PBR water path"
	for slot in 6:
		var texture = material.get_shader_parameter("texture_%d" % slot)
		if texture == null:
			return "Material130 slot %d has no authored texture" % slot
		if texture.get_width() < 2 or texture.get_height() < 2:
			return "Material130 slot %d is not the authored mask" % slot
	var batch_0: Vector4 = material.get_shader_parameter("pbr_batch_0")
	var batch_2: Vector4 = material.get_shader_parameter("pbr_batch_2")
	print("PBR batch0=", batch_0, " batch2=", batch_2)
	if batch_0.w <= 0.0 or batch_2.w <= 0.0:
		return "Material130 row floats were not bound: %s %s" % [batch_0, batch_2]
	return ""

## The pixel program turns the masks into coverage: 2*(t0*t1) + t2.wxyz, whose alpha drives
## the water mask. Replacing the mid mask's alpha must change the frame.
func check_mask_coverage() -> String:
	make_viewport()
	build()
	# A bright tint makes the mask's blend visible against the sub-viewport's black scene.
	var original_tint: Vector4 = material.get_shader_parameter("pbr_batch_2")
	material.set_shader_parameter("pbr_batch_2", Vector4(1.0, 1.0, 1.0, original_tint.w))
	var original = material.get_shader_parameter("foam_mid")
	material.set_shader_parameter("foam_mid", solid_texture(Color(1, 1, 1, 0)))
	var clear := await capture()
	material.set_shader_parameter("foam_mid", solid_texture(Color(1, 1, 1, 1)))
	var covered := await capture()
	material.set_shader_parameter("foam_mid", original)
	material.set_shader_parameter("pbr_batch_2", original_tint)
	var distance := color_distance(mean(clear), mean(covered))
	print("PBR mask coverage clear=", mean(clear), " covered=", mean(covered), " delta=", distance)
	if distance < 0.02:
		return "The authored mask coverage did not drive the PBR water"
	return ""

## The program samples its own scrolled UV transforms from the batch; changing a layer's
## scale must move the sampled mask, otherwise the masks are not varying across the surface.
func check_layer_uv() -> String:
	build()
	var original: Vector4 = material.get_shader_parameter("pbr_batch_1")
	var scaled := await capture()
	material.set_shader_parameter("pbr_batch_1", Vector4(original.x * 3.0, original.y, original.z, original.w))
	var moved := await capture()
	material.set_shader_parameter("pbr_batch_1", original)
	var distance := frame_distance(scaled, moved)
	print("PBR layer uv delta=", distance)
	if distance < 0.0005:
		return "The layer UV transform did not move the sampled masks"
	return ""

## Mean absolute per-pixel difference over the centre window: catches mask movement that a
## mean-colour comparison cancels out.
func frame_distance(a: Image, b: Image) -> float:
	var total := 0.0
	var centre := Vector2i(SIZE.x / 2, SIZE.y / 2)
	for y in range(centre.y - 12, centre.y + 12):
		for x in range(centre.x - 12, centre.x + 12):
			total += color_distance(a.get_pixel(x, y), b.get_pixel(x, y))
	return total / 576.0

func solid_texture(color: Color) -> ImageTexture:
	var image := Image.create(1, 1, false, Image.FORMAT_RGBA8)
	image.fill(color)
	return ImageTexture.create_from_image(image)

func check_row_colours() -> String:
	build()
	var lit := await capture()
	material.set_shader_parameter("pbr_batch_2", Vector4.ZERO)
	material.set_shader_parameter("pbr_batch_3", Vector4.ZERO)
	var dark := await capture()
	var distance := color_distance(mean(lit), mean(dark))
	print("PBR row colours lit=", mean(lit), " zero=", mean(dark), " delta=", distance)
	if distance < 0.02:
		return "LiquidType.Color did not drive the PBR water colour"
	return ""

func make_viewport() -> void:
	viewport = SubViewport.new()
	viewport.size = SIZE
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	viewport.own_world_3d = true
	root.add_child(viewport)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color(1, 1, 1)
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	viewport.add_child(world_environment)
	var camera := Camera3D.new()
	camera.fov = 30.0
	camera.near = 0.5
	camera.far = 200.0
	viewport.add_child(camera)
	camera.look_at_from_position(WATER + Vector3(0.0, 8.0, 0.0), WATER, Vector3.FORWARD)
	camera.current = true
	water_root = Node3D.new()
	viewport.add_child(water_root)

func build() -> void:
	for child in water_root.get_children():
		child.free()
	var mesh := water_quad()
	var quad := MeshInstance3D.new()
	quad.mesh = mesh
	quad.set_surface_override_material(0, material)
	water_root.add_child(quad)

func water_quad() -> ArrayMesh:
	var z0 := WATER.z - 20.0
	var z1 := WATER.z + 20.0
	var vertices := PackedVector3Array([
		Vector3(WATER.x - 8.0, WATER.y, z0), Vector3(WATER.x + 8.0, WATER.y, z0),
		Vector3(WATER.x - 8.0, WATER.y, z1), Vector3(WATER.x + 8.0, WATER.y, z1)])
	var colors := PackedColorArray()
	colors.resize(4)
	colors.fill(Color.WHITE)
	# I2 B.4: depth 1 means authored coverage is on.
	var depths := PackedVector2Array()
	depths.resize(4)
	depths.fill(Vector2(0.0, 1.0))
	var uvs := PackedVector2Array([Vector2(0, 0), Vector2(1, 0), Vector2(0, 1), Vector2(1, 1)])
	var normals := PackedVector3Array([Vector3.UP, Vector3.UP, Vector3.UP, Vector3.UP])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = depths
	arrays[Mesh.ARRAY_INDEX] = PackedInt32Array([0, 1, 2, 1, 3, 2])
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh

func capture() -> Image:
	for frame in 3:
		await process_frame
		await RenderingServer.frame_post_draw
	return viewport.get_texture().get_image()

func mean(image: Image) -> Color:
	var total := Color(0, 0, 0, 0)
	var centre := Vector2i(SIZE.x / 2, SIZE.y / 2)
	for y in range(centre.y - 12, centre.y + 12):
		for x in range(centre.x - 12, centre.x + 12):
			total += image.get_pixel(x, y)
	return total / 576.0

func color_distance(a: Color, b: Color) -> float:
	return absf(a.r - b.r) + absf(a.g - b.g) + absf(a.b - b.b)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
