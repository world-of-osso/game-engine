extends "res://tests/liquid_material_pixels.gd"

## BLS magma/procmagma colour chains against the production DB2 material.
## detail 0 is the Magma definition: t0(TC1.xy).rgb with alpha 1 and no noise layer.
## detail >= 1 is ProcMagma: the noise RGB supplies three successive lerp weights.
var client: Node3D

func run() -> void:
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if client.account_state().ui_skin != "Forever":
		fail("Magma proof requires ui_skin == Forever")
		return
	root.get_node("M2MaterialClock").set_process(false)
	make_viewport()
	var loader = ClassDB.instantiate("WowTerrainLoader")
	var wow := Vector3(CENTRE.x, -CENTRE.z, CENTRE.y)
	var result: Dictionary = loader.load_liquid_material(7, 413, 0, wow, NOON)
	if result.has("error"):
		fail(str(result.error))
		return
	var material: ShaderMaterial = result.material
	quad.set_surface_override_material(0, material)
	backing.albedo_color = Color(0.1, 0.1, 0.9)
	RenderingServer.global_shader_parameter_set("liquid_animation_time_ms", 1000.0)
	# Distinct probes: green base layer 0, red everything else, blue noise volume.
	material.set_shader_parameter("texture_0", probe(Color(0.0, 1.0, 0.0, 1.0)))
	material.set_shader_parameter("texture_1", probe(Color(1.0, 0.0, 0.0, 1.0)))
	material.set_shader_parameter("texture_2", probe(Color(1.0, 0.0, 0.0, 1.0)))
	material.set_shader_parameter("texture_3", probe(Color(1.0, 0.0, 0.0, 1.0)))
	material.set_shader_parameter("noise_volume", probe(Color(0.0, 0.0, 1.0, 1.0)))

	material.set_shader_parameter("liquid_detail", 0)
	var low := await capture()
	var low_mean := mean(low)
	print("MAGMA_DETAIL0 ", low_mean)
	if low_mean.g < 0.5 or low_mean.r > 0.25 or low_mean.b > 0.25:
		fail("detail 0 must be the single t0 layer, got " + str(low_mean))
		return

	material.set_shader_parameter("liquid_detail", 2)
	var high := await capture()
	var high_mean := mean(high)
	print("MAGMA_PROCMAGMA ", high_mean)
	if changed_pixels(low, high, 0.02) < 1000:
		fail("procmagma must add the noise-weighted layers")
		return
	if high_mean.r <= low_mean.r:
		fail("procmagma weights must mix the other layers in: " + str(high_mean))
		return
	print("PASS: MAGMA_LAYERS magma is one layer at detail 0, procmagma adds noise-weighted layers")
	client.free()
	quit(0)

func probe(color: Color) -> ImageTexture:
	var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	image.fill(color)
	return ImageTexture.create_from_image(image)

func capture() -> Image:
	if client.account_state().ui_skin != "Forever":
		fail("ui_skin must be Forever before capture")
		return Image.new()
	return await super.capture()
