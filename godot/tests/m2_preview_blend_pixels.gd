extends SceneTree
# Actual Ozruk material, transparent 3D target and premultiplied preview compositor.
# Compare rendered pixels, including world RGB preservation and normal coverage.
const SIZE := 64
const BACKGROUND := Color(0.25, 0.35, 0.45)
const TOLERANCE := 0.015
var model: Node
var viewport: SubViewport
var composition: SubViewport
var view: TextureRect
var geometry: MeshInstance3D

func _initialize() -> void:
	call_deferred("run_cases")

func fail(reason: String) -> void:
	push_error("M2_PREVIEW_BLEND_FAIL " + reason)
	cleanup()
	quit(1)

func run_cases() -> void:
	var loader = ClassDB.instantiate("WowAssetLoader")
	var loaded: Dictionary = loader.load_creature_display(36475)
	if loaded.has("error"):
		fail(str(loaded.error))
		return
	model = loaded.node
	var additive: ShaderMaterial = model.find_child("Batch0", true, false).get_active_material(0)
	var opaque: ShaderMaterial = model.find_child("Batch1", true, false).get_active_material(0)
	var authored := additive.shader
	var preview: Shader = authored.get_meta("preview_additive_shader")
	if preview == null:
		fail("Production additive preview shader missing")
		return
	var image: Image = additive.get_shader_parameter("base_texture").get_image()
	if image.is_compressed() and image.decompress() != OK:
		fail("Cannot observe source texels")
		return
	var black := source_texel(image, false)
	var bright := source_texel(image, true)
	if black.x < 0.0 or bright.x < 0.0:
		fail("Authored black/bright full-opacity texels missing")
		return
	make_viewports()
	additive.shader = preview
	set_geometry(additive, black)
	var pixels := await capture_pixels()
	if not check_pixel("black additive preserves backdrop", pixels.composed, BACKGROUND): return
	if absf(pixels.source.a) > TOLERANCE:
		fail("Pure additive source wrote coverage")
		return
	# Same actual source texel and opacity: preview RGB must equal the world shader.
	additive.set_shader_parameter("transparency", 0.5)
	set_geometry(additive, bright)
	additive.shader = authored
	var world := await capture_pixels()
	additive.shader = preview
	pixels = await capture_pixels()
	if not check_pixel("positive Add RGB unchanged", pixels.source, world.source): return
	if not check_pixel("positive Add retained over backdrop", pixels.composed, over(pixels.source, 0.0)): return
	# The same M2 fragment inputs under native source-alpha blending verify coverage.
	var mixed := Shader.new()
	mixed.code = authored.code.replace("blend_add", "blend_mix")
	additive.shader = mixed
	additive.set_shader_parameter("gx_blend", 2)
	pixels = await capture_pixels()
	if absf(pixels.source.a - 0.5) > TOLERANCE:
		fail("Half-opacity mix did not store half coverage: " + str(pixels.source))
		return
	if not check_pixel("mix coverage preserved", pixels.composed, over(pixels.source, pixels.source.a)): return
	# Actual opaque Ozruk batch: it must cover the UI even if its pixel is black.
	set_geometry(opaque, bright)
	pixels = await capture_pixels()
	if absf(pixels.source.a - 1.0) > TOLERANCE:
		fail("Opaque M2 did not store full coverage")
		return
	if not check_pixel("opaque coverage preserved", pixels.composed, pixels.source): return
	cleanup()
	print("M2_PREVIEW_BLEND_PASS black/positive Add, world RGB, mix and opaque coverage")
	quit(0)

func capture_pixels() -> Dictionary:
	for frame in range(6): await process_frame
	await RenderingServer.frame_post_draw
	return {
		"source": viewport.get_texture().get_image().get_pixel(SIZE / 2, SIZE / 2),
		"composed": composition.get_texture().get_image().get_pixel(SIZE / 2, SIZE / 2),
	}

func check_pixel(label: String, actual: Color, expected: Color) -> bool:
	var error := maxf(absf(actual.r - expected.r), maxf(absf(actual.g - expected.g), absf(actual.b - expected.b)))
	if error > TOLERANCE:
		fail("%s: expected %s got %s" % [label, expected, actual])
		return false
	print("M2_PREVIEW_BLEND_PIXEL ", label, " actual=", actual)
	return true

func over(source: Color, coverage: float) -> Color:
	return Color(minf(source.r + BACKGROUND.r * (1.0 - coverage), 1.0), minf(source.g + BACKGROUND.g * (1.0 - coverage), 1.0), minf(source.b + BACKGROUND.b * (1.0 - coverage), 1.0))

func source_texel(image: Image, bright: bool) -> Vector2:
	for y in image.get_height():
		for x in image.get_width():
			var color := image.get_pixel(x, y)
			var matches := maxf(color.r, maxf(color.g, color.b)) > 0.7 if bright else color.r == 0.0 and color.g == 0.0 and color.b == 0.0
			if matches and color.a == 1.0:
				return Vector2((x + 0.5) / image.get_width(), (y + 0.5) / image.get_height())
	return Vector2(-1.0, -1.0)

func make_viewports() -> void:
	viewport = SubViewport.new()
	viewport.size = Vector2i(SIZE, SIZE)
	viewport.own_world_3d = true
	viewport.transparent_bg = true
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(viewport)
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 2.0
	camera.position = Vector3(0.0, 2.0, 0.0)
	camera.look_at_from_position(camera.position, Vector3.ZERO, Vector3.FORWARD)
	viewport.add_child(camera)
	camera.current = true
	geometry = MeshInstance3D.new()
	viewport.add_child(geometry)
	composition = SubViewport.new()
	composition.size = Vector2i(SIZE, SIZE)
	composition.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(composition)
	var backdrop := ColorRect.new()
	backdrop.color = BACKGROUND
	backdrop.size = Vector2(SIZE, SIZE)
	composition.add_child(backdrop)
	view = TextureRect.new()
	var composite := CanvasItemMaterial.new()
	composite.blend_mode = CanvasItemMaterial.BLEND_MODE_PREMULT_ALPHA
	view.material = composite
	view.texture = viewport.get_texture()
	view.size = Vector2(SIZE, SIZE)
	composition.add_child(view)

func set_geometry(material: ShaderMaterial, uv: Vector2) -> void:
	var arrays := PlaneMesh.new().surface_get_arrays(0)
	var coords := PackedVector2Array()
	for vertex in arrays[Mesh.ARRAY_VERTEX].size(): coords.append(uv)
	arrays[Mesh.ARRAY_TEX_UV] = coords
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	geometry.mesh = mesh
	geometry.material_override = material

func cleanup() -> void:
	if composition != null: composition.free()
	if viewport != null: viewport.free()
	if model != null: model.free()
