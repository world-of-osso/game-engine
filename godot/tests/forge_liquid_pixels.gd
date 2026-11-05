extends "res://tests/liquid_material_pixels.gd"

## Real1192 DB2 textures and production shader. G4/Fel motion and G8 shore:
## objectless WMO flow is zero (static, not a bug) and the shore fade is
## D = saturate((linear scene depth - liquid surface depth) / Float_11).
var client: Node3D

func run() -> void:
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if client.account_state().ui_skin != "Forever":
		fail("Forge proof requires ui_skin == Forever")
		return
	root.get_node("M2MaterialClock").set_process(false)
	make_viewport()
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
	if not await check_layer_gate(material, "texture_3", 2, Color(1,0,0,0), Color(0,0,1,0)):
		return
	RenderingServer.global_shader_parameter_set("specular_color", Vector3.ONE)
	if not await check_layer_gate(material, "texture_2", 3, Color(0,0,0,1), Color(1,1,1,1)):
		return
	if not await check_shore_fade(material):
		return
	print("PASS: FORGE_LIQUID_PIXELS object-zero/static, row10949/animated, integer clock, " +
		"shore2/specular3, depth-fade width from Float_11")
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
	return true

## D = saturate((scene - surface) / Float_11): a larger 1/Float_11 reaches the open
## surface sooner, so the shore-coloured area must shrink as the scale grows.
func check_shore_fade(material: ShaderMaterial) -> bool:
	var params: Vector4 = material.get_shader_parameter("fel_params_0")
	# The row supplies 1/Float_11 = 1/2.6; the scene behind this quad sits 2-5 yd lower.
	if absf(params.y - 1.0 / 2.6) > 0.0001:
		fail("fel_params_0.y must be 1/Float_11 from the row, got " + str(params.y))
		return false
	var original_texture = material.get_shader_parameter("texture_3")
	material.set_shader_parameter("liquid_detail", 2)
	material.set_shader_parameter("texture_3", solid_texture(Color(0,0,1,1)))
	var counts := {}
	for scale in [0.0, 0.05, 0.3846, 4.0]:
		material.set_shader_parameter("fel_params_0", Vector4(params.x, scale, params.z, params.w))
		var image := await capture()
		counts[scale] = shore_pixels(image)
		print("FORGE_SHORE scale=", scale, " shore_pixels=", counts[scale])
	var ordered: bool = counts[0.0] >= counts[0.05] and counts[0.05] >= counts[0.3846] and counts[0.3846] >= counts[4.0]
	if not ordered:
		fail("Shore area must shrink as 1/Float_11 grows: " + str(counts))
		return false
	if counts[0.0] < 1000 or counts[4.0] > counts[0.0] / 4:
		fail("Depth fade did not move the shore edge: " + str(counts))
		return false
	material.set_shader_parameter("fel_params_0", params)
	material.set_shader_parameter("texture_3", original_texture)
	return true

## Pixels dominated by the blue shore probe rather than the orange lava layers.
func shore_pixels(image: Image) -> int:
	var count := 0
	for y in SIZE.y:
		for x in SIZE.x:
			var pixel := image.get_pixel(x, y)
			if pixel.b > pixel.r + 0.15:
				count += 1
	return count

func capture() -> Image:
	if client.account_state().ui_skin != "Forever":
		fail("ui_skin must be Forever before capture")
		return Image.new()
	return await super.capture()
