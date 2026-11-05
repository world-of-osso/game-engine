extends "res://tests/liquid_material_pixels.gd"

## Real1192 DB2 textures and production shader. Retail70688 G10: the shore fade is
## physical water depth from a per-surface baked map, so it must not follow the camera.
var client: Node3D

const MAP_DEPTH_YD := 5.0

func run() -> void:
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if client.account_state().ui_skin != "Forever":
		fail("Forge proof requires ui_skin == Forever")
		return
	root.get_node("M2MaterialClock").set_process(false)
	make_viewport()
	backing.albedo_color = Color(1.0, 0.0, 0.0)
	for object_id in [0, 11187, 10949]:
		var material := load_forge(object_id)
		if material == null:
			return
		RenderingServer.global_shader_parameter_set("liquid_animation_time_ms", 1000.1)
		var first := await capture()
		RenderingServer.global_shader_parameter_set("liquid_animation_time_ms", 1000.9)
		var same_tick := await capture()
		if changed_pixels(first, same_tick, 0.0001) != 0:
			fail("Fel UV clock must use integer milliseconds")
			return
		RenderingServer.global_shader_parameter_set("liquid_animation_time_ms", LATER_MS)
		var later := await capture()
		var moved := changed_pixels(first, later, 0.004)
		print("FORGE_FLOW object=", object_id, " moved=", moved)
		if (object_id == 10949 and moved < 200) or (object_id != 10949 and moved != 0):
			fail("Fel motion must follow the row's FlowSpeed; object=" + str(object_id))
			return
	var material := load_forge(0)
	if material == null:
		return
	# The specular probe must run with the production world-space UV: the mesh UV is only
	# bound for the baked-map checks below.
	RenderingServer.global_shader_parameter_set("specular_color", Vector3.ONE)
	if not await check_layer_gate(material, "texture_2", 3, Color(0,0,0,1), Color(1,1,1,1)):
		return
	bind_depth_map(material)
	if not await check_layer_gate(material, "texture_3", 2, Color(1,0,0,0), Color(0,0,1,0)):
		return
	if not await check_view_independence(material):
		return
	if not await check_base_without_map(material):
		return
	print("PASS: FORGE_LIQUID_PIXELS object-zero/static, row10949/animated, integer clock, " +
		"shore2/specular3, camera-independent depth-fade")
	client.free()
	quit(0)

func load_forge(object_id: int) -> ShaderMaterial:
	var loader = ClassDB.instantiate("WowTerrainLoader")
	var wow := Vector3(CENTRE.x, -CENTRE.z, CENTRE.y)
	var result: Dictionary = loader.load_liquid_material(1192, object_id, 2519, wow, NOON)
	if result.has("error"):
		fail(str(result.error))
		return null
	quad.set_surface_override_material(0, result.material)
	return result.material

func solid_texture(color: Color) -> ImageTexture:
	var image := Image.create(1,1,false,Image.FORMAT_RGBA8)
	image.fill(color)
	return ImageTexture.create_from_image(image)

## A synthetic bottom-depth map: 0 yd at u=0 rising to MAP_DEPTH_YD at u=1.
func bind_depth_map(material: ShaderMaterial, flat_depth := -1.0) -> void:
	var width := 128
	var height := 4
	var bytes := PackedByteArray()
	for _row in height:
		for column in width:
			var depth := flat_depth if flat_depth >= 0.0 else MAP_DEPTH_YD * float(column) / float(width - 1)
			bytes.append_array(PackedFloat32Array([depth]).to_byte_array())
	var image := Image.create_from_data(width, height, false, Image.FORMAT_RF, bytes)
	var texture := ImageTexture.create_from_image(image)
	print("FORGE_MAP_TEXELS ", image.get_pixel(0,0).r, " ", image.get_pixel(64,0).r, " ", image.get_pixel(127,0).r)
	material.set_shader_parameter("shore_depth_map", texture)
	# The quad's local VERTEX spans CENTRE +/- 100 on X and Z; map that onto 0..1.
	var span := 200.0
	material.set_shader_parameter("shore_map_transform", Vector4(
		1.0 / span, 1.0 / span,
		-(CENTRE.x - 100.0) / span, -(CENTRE.z - 100.0) / span))
	material.set_shader_parameter("shore_map_enabled", true)
	material.set_shader_parameter("liquid_detail", 2)

func check_layer_gate(material: ShaderMaterial, slot: String, threshold: int, first: Color, second: Color) -> bool:
	var original_texture = material.get_shader_parameter(slot)
	var detail_before = material.get_shader_parameter("liquid_detail")
	for detail in [threshold - 1, threshold]:
		material.set_shader_parameter("liquid_detail", detail)
		material.set_shader_parameter(slot, solid_texture(first))
		var before := await capture()
		material.set_shader_parameter(slot, solid_texture(second))
		var after := await capture()
		var changed := changed_pixels(before,after,0.004)
		print("FORGE_GATE slot=",slot," detail=",detail," changed=",changed)
		if (detail < threshold and changed != 0) or (detail == threshold and changed < 200):
			fail("Wrong detail gate for " + slot)
			return false
	material.set_shader_parameter(slot, original_texture)
	material.set_shader_parameter("liquid_detail", detail_before)
	return true

## The depth fade reads the baked map, so the shore fraction must stay put when only the
## camera moves. A scene-depth fade would push the band toward the horizon.
func check_view_independence(material: ShaderMaterial) -> bool:
	var original_texture = material.get_shader_parameter("texture_3")
	material.set_shader_parameter("texture_3", solid_texture(Color(0,0,1,1)))
	# Diagnostic: a flat 0.1 yd map must read as almost entirely shore.
	bind_depth_map(material, 0.1)
	var flat := await shop_mean()
	print("FORGE_FLAT_MAP mean=", flat)
	bind_depth_map(material)
	var measured: Array[float] = []
	for view in [[120.0, 300.0], [200.0, 340.0], [80.0, 320.0]]:
		place_camera(view[0], view[1])
		var counts := await shore_counts()
		measured.append(counts[0] / maxf(counts[1], 1.0))
		print("FORGE_VIEW height=", view[0], " distance=", view[1], " shore=", counts[0], " covered=", counts[1], " mean=", counts[2])
	var first := measured[0]
	for value in measured:
		if absf(value - first) > 0.12:
			fail("Shore fraction must not follow the camera: " + str(measured))
			return false
	if first <= 0.02 or first >= 0.98:
		fail("Shore band missing from the synthetic depth ramp: " + str(first))
		return false
	material.set_shader_parameter("texture_3", original_texture)
	place_camera(5.0, 10.0)
	return true

## Without a baked map the surface keeps its base colour: the shader must not fall back to
## an invented scene-depth fade.
func check_base_without_map(material: ShaderMaterial) -> bool:
	place_camera(120.0, 300.0)
	material.set_shader_parameter("texture_3", solid_texture(Color(0,0,1,1)))
	var mapped := await shore_fraction()
	material.set_shader_parameter("shore_map_enabled", false)
	var unmapped := await shore_fraction()
	print("FORGE_NO_MAP mapped=", mapped, " unmapped=", unmapped)
	if mapped <= 0.02:
		fail("The baked map must produce a shore band")
		return false
	if unmapped > 0.005:
		fail("Without a shore map the shader must not invent a fade")
		return false
	material.set_shader_parameter("shore_map_enabled", true)
	return true

func shop_mean() -> Color:
	return mean(await capture())

## Share of the covered quad that took the blue shore probe.
func shore_fraction() -> float:
	var counts := await shore_counts()
	return counts[0] / maxf(counts[1], 1.0)

## `[shore pixels, covered pixels]` for the blue shore probe over the red backing.
func shore_counts() -> Array:
	var image := await capture()
	var shore := 0
	var covered := 0
	for y in SIZE.y:
		for x in SIZE.x:
			var pixel := image.get_pixel(x, y)
			# The backing plane is pure red; anything else is liquid or scene.
			if pixel.r > 0.6 and pixel.g < 0.25 and pixel.b < 0.25:
				continue
			covered += 1
			if pixel.b > pixel.r + 0.15:
				shore += 1
	return [shore, covered, mean(image)]

func place_camera(height: float, distance: float) -> void:
	var camera := viewport.get_camera_3d()
	camera.look_at_from_position(CENTRE + Vector3(0.0, height, distance), CENTRE, Vector3.UP)

func capture() -> Image:
	if client.account_state().ui_skin != "Forever":
		fail("ui_skin must be Forever before capture")
		return Image.new()
	return await super.capture()
