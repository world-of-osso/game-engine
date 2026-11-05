extends "res://tests/liquid_material_pixels.gd"

## Production DB2/material loader and real Forge1192 textures. This is a shader-contract
## test, not a claim that the MLLD authored shoreline or Retail pixel equations are ported.
var client: Node3D

func run() -> void:
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if client.account_state().ui_skin != "Forever":
		fail("Forge proof requires ui_skin == Forever")
		return
	root.get_node("M2MaterialClock").set_process(false)
	make_viewport()
	var error := await check_case("forge", [1192, 0, 2519])
	if error != "":
		fail(error)
		return
	var material := quad.get_surface_override_material(0) as ShaderMaterial
	material.set_shader_parameter("liquid_detail", 0)
	RenderingServer.global_shader_parameter_set("liquid_animation_time_ms", 1000.0)
	var low := await capture()
	material.set_shader_parameter("liquid_detail", 1)
	var medium := await capture()
	if changed_pixels(low, medium, 0.004) < 200:
		fail("Forge detail1 must add emissive/procedural/shore layers")
		return
	material.set_shader_parameter("liquid_detail", 0)
	var low_again := await capture()
	if changed_pixels(low, low_again, 0.004) != 0:
		fail("Fixed Forge clock/detail must reproduce pixels")
		return
	print("PASS: FORGE_LIQUID_PIXELS zero-object-flow scroll, detail layers, fixed-clock repeat")
	client.free()
	quit(0)

func capture() -> Image:
	if client.account_state().ui_skin != "Forever":
		fail("ui_skin must be Forever before capture")
		return Image.new()
	return await super.capture()
