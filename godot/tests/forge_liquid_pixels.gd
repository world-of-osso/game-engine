extends "res://tests/liquid_material_pixels.gd"

## Real1192 DB2 textures and the production shader. The shore fade reads the opaque view
## depth behind each pixel, so D is water thickness along the view ray.
var client: Node3D

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
	if not await check_layer_gate(material, "texture_3", 3, Color(1,0,0,0), Color(0,0,1,0)):
		return
	if not await check_depth_gap(material):
		return
	print("PASS: FORGE_LIQUID_PIXELS object-zero/static, row10949/animated, integer clock, " +
		"shore3/specular3, view-depth shore fade")
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

## D is the view-depth gap to the opaque surface behind each pixel, over Float_11. Moving the
## opaque backing further below the liquid must widen the gap and shrink the shore band, and a
## grazing view must see a bigger gap than a steep one at the same separation.
func check_depth_gap(material: ShaderMaterial) -> bool:
	var original_texture = material.get_shader_parameter("texture_3")
	material.set_shader_parameter("texture_3", solid_texture(Color(0,0,1,1)))
	# G14: ProcFel binds the depth register and its shore layer at liquid detail >= 3, where
	# its specular layer is also active. The specular gate is proven separately, so mute it
	# here to isolate the shore term this probe measures.
	var specular_before = RenderingServer.global_shader_parameter_get("specular_color")
	RenderingServer.global_shader_parameter_set("specular_color", Vector3.ZERO)
	material.set_shader_parameter("liquid_detail", 3)
	place_camera(120.0, 300.0)
	var measured: Array[float] = []
	for gap in [0.2, 0.5, 12.0]:
		set_backing_gap(gap)
		var counts := await shore_counts()
		var value: float = counts[0] / maxf(counts[1], 1.0)
		measured.append(value)
		print("FORGE_GAP backing=", gap, " shore_fraction=", value, " shore/covered=", counts)
	set_backing_gap(2.0)
	# A 0.2 yd gap is far inside the 2.6 yd fade; 12 yd is far outside it.
	if measured[0] < 0.5:
		fail("A 0.2 yd view gap must read as shore: " + str(measured))
		return false
	if measured[2] > 0.02:
		fail("A 12 yd view gap must read as open water: " + str(measured))
		return false
	if not (measured[0] > measured[1] and measured[1] >= measured[2]):
		fail("The shore band must shrink as the view gap grows: " + str(measured))
		return false
	# Same separation, different view angle: a grazing view has the larger gap.
	set_backing_gap(0.5)
	place_camera(120.0, 300.0)
	var steep := await shore_fraction()
	place_camera(12.0, 220.0)
	var grazing := await shore_fraction()
	print("FORGE_ANGLE steep=", steep, " grazing=", grazing)
	if grazing >= steep:
		fail("A grazing view must see a larger gap and less shore: " + str([steep, grazing]))
		return false
	set_backing_gap(2.0)
	place_camera(120.0, 300.0)
	material.set_shader_parameter("texture_3", original_texture)
	RenderingServer.global_shader_parameter_set("specular_color", specular_before)
	return true

## Move the opaque plane behind the liquid to `gap` yards below its surface.
func set_backing_gap(gap: float) -> void:
	for child in viewport.get_children():
		if child is MeshInstance3D and child != quad:
			child.position = CENTRE - Vector3(0.0, gap, 0.0)

## Share of the covered quad that took the blue shore probe.
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
