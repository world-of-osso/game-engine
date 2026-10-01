extends SceneTree

# Stockade instance portal sheet (instanceportal.m2, FDID 197007, batch 0): blend 7
# GxBlend_BlendAdd, shader 0x8022, colour 0 (108, 133, 203), transparency 0.7.
# WebWowViewerCpp: matDiffuse = meshColor * tex.rgb, finalOpacity = tex.a * meshOpacity,
# blended ONE, ONE_MINUS_SRC_ALPHA; Godot blends in its linear framebuffer.
const MODEL := "res://../data/models/197007.m2"
const VIEW_SIZE := 64
const TOLERANCE := 3.0 / 255.0
# A texel inside the portal texture's range (7361559: grey 0-206, alpha 0-186).
const TEXEL := Color8(160, 160, 160, 140)
const BACKGROUND := Color(0.2, 0.6, 0.1)
# WebWowViewer shading and blend factors over BACKGROUND, lit by ambient 1 (x1.1 facing the sun).
const EXPECTED := Color8(85, 149, 142)

var viewport: SubViewport

func _initialize() -> void:
	call_deferred("run_cases")

func fail(message: String) -> void:
	push_error(message)
	quit(1)

func add_plane(material: Material, height: float) -> void:
	var instance := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(2.0, 2.0) if height > 0.0 else Vector2(8.0, 8.0)
	instance.mesh = plane
	instance.position = Vector3(0.0, height, 0.0)
	instance.material_override = material
	viewport.add_child(instance)

func make_viewport() -> void:
	viewport = SubViewport.new()
	viewport.size = Vector2i(VIEW_SIZE, VIEW_SIZE)
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
	camera.size = 4.0
	camera.look_at_from_position(Vector3(0.0, 2.0, 0.0), Vector3.ZERO, Vector3(0.0, 0.0, -1.0))
	viewport.add_child(camera)
	camera.current = true
	var sun := DirectionalLight3D.new()
	sun.rotation.x = -PI / 2.0
	viewport.add_child(sun)

func background_material() -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = BACKGROUND
	return material

func texel_texture() -> ImageTexture:
	var image := Image.create(1, 1, false, Image.FORMAT_RGBA8)
	image.fill(TEXEL)
	return ImageTexture.create_from_image(image)

func portal_material() -> ShaderMaterial:
	if not ClassDB.class_exists("WowAssetLoader"):
		fail("WowAssetLoader not registered")
		return null
	var loader: Object = ClassDB.instantiate("WowAssetLoader")
	var result: Dictionary = loader.load_m2(MODEL)
	if result.has("error") or not result.get("node") is Node3D:
		fail("Portal load: " + str(result.get("error", "no node")))
		return null
	var model: Node3D = result.node
	var batch := model.get_node_or_null("Batch0") as MeshInstance3D
	if batch == null:
		model.free()
		fail("Portal model has no Batch0")
		return null
	var material := batch.get_active_material(0) as ShaderMaterial
	model.free()
	if material == null:
		fail("Portal Batch0 has no shader material")
	return material

func read_pixel(x: int, y: int) -> Color:
	for frame in 2:
		await process_frame
		await RenderingServer.frame_post_draw
	var image := viewport.get_texture().get_image()
	if image == null or image.is_empty():
		return Color(-1.0, -1.0, -1.0)
	return image.get_pixel(x, y)

func near(actual: Color, expected: Color) -> bool:
	return absf(actual.r - expected.r) <= TOLERANCE and absf(actual.g - expected.g) <= TOLERANCE \
		and absf(actual.b - expected.b) <= TOLERANCE

func run_cases() -> void:
	var material := portal_material()
	if material == null:
		return
	# The authored batch material with a known texel in place of the portal texture.
	material.set_shader_parameter("base_texture", texel_texture())
	make_viewport()
	add_plane(background_material(), 0.0)
	add_plane(material, 0.5)
	var corner := await read_pixel(2, 2)
	if not near(corner, BACKGROUND):
		fail("background: expected %s, got %s" % [BACKGROUND, corner])
		return
	var actual := await read_pixel(VIEW_SIZE / 2, VIEW_SIZE / 2)
	if not near(actual, EXPECTED):
		fail("portal sheet: expected %s (WebWowViewer), got %s" % [EXPECTED, actual])
		return
	print("PASS: portal sheet adds its blue tint over the room: ", actual)
	quit(0)
