extends "res://tests/water_material_pixels.gd"

func run() -> void:
	var loader = ClassDB.instantiate("WowTerrainLoader")
	var retail: Dictionary = loader.load_liquid_material(947, 42, 1643, Vector3(7733.333, 6133.333, 0), 1440.0)
	if retail.has("error"):
		fail(str(retail.error))
		return
	material = retail.material
	scene_light = retail.scene_light
	make_viewport()
	var errors: Array[String] = []
	var alpha_error: String = await check_negative_opacity_gain()
	if alpha_error != "":
		errors.append(alpha_error)
	for pair in [[1251, 18420], [1279, 21229]]:
		var result: Dictionary = loader.load_liquid_material(pair[0], pair[1], 2991, Vector3(3065, 1380, 756.5897), 1440.0)
		if result.has("error"):
			errors.append(str(result.error))
			continue
		var borrowed: ShaderMaterial = result.material
		var floats: Vector4 = borrowed.get_shader_parameter("floats_0")
		if absf(floats.z - 1.0) > 0.000001:
			errors.append("LiquidType%d fed PBR Float2=%s to legacy Water" % [pair[0], floats.z])
		var normal: Texture2D = borrowed.get_shader_parameter("texture_2")
		var expected: Texture2D = retail.material.get_shader_parameter("texture_2")
		if normal.get_image().get_data() != expected.get_image().get_data():
			errors.append("LiquidType%d fed foam pixels rather than the borrowed legacy normal" % pair[0])
	if not errors.is_empty():
		fail("; ".join(errors))
		return
	print("PASS: legacy borrowed inputs and non-negative water opacity")
	quit(0)

func check_negative_opacity_gain() -> String:
	# Exact wrong-contract values observed on Zephras: zero RGB, alpha .75 -> 0,
	# Float2=7.85. Control all other color contributions to isolate amplification.
	material.set_shader_parameter("floats_0", Vector4(1, 0, 7.85, 1))
	material.set_shader_parameter("floats_4", Vector4(2.5, 90, 0.7, 0))
	material.set_shader_parameter("ocean_close", Vector4(0, 0, 0, 0.75))
	material.set_shader_parameter("ocean_far", Vector4.ZERO)
	material.set_shader_parameter("specular_color", Vector3.ZERO)
	material.set_shader_parameter("underwater_fog", Vector3(1000, 100000000, 0))
	material.set_shader_parameter("fog_mode", 0)
	material.set_shader_parameter("animation_time_ms", 0.0)
	build(Vector2.ONE, Vector2(5, 5), Color(0.02, 0.02, 0.02))
	var first: Image = await capture()
	set_backing_color(Color(0.04, 0.04, 0.04))
	var second: Image = await capture()
	var gain := (mean(second, 0).r - mean(first, 0).r) / 0.02
	print("WATER_NEGATIVE_ALPHA backing_gain=", gain, " first=", mean(first, 0), " second=", mean(second, 0))
	if gain > 1.1:
		return "Negative legacy water opacity amplified the backing by %s" % gain
	return ""
