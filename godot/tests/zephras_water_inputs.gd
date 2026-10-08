extends SceneTree

const OUTPUT := "res://../data/diagnostics/zephras-water-white/"
const CASES := {
	"retail_ocean": [947, 42, 1643, Vector3(7733.333, 6133.333, 0.0)],
	"zephras_lake": [1251, 18420, 2991, Vector3(3065.0, 1380.0, 756.5897)],
	"zephras_river": [1279, 21229, 2991, Vector3(3065.0, 1380.0, 756.5897)],
}
const UNIFORMS := ["floats_0", "floats_4", "floats_8", "floats_12", "floats_16", "depth_coefficients", "liquid_color_0", "liquid_color_1", "liquid_color_2", "liquid_ints", "flow", "color_source", "wave_periods", "river_close", "river_far", "ocean_close", "ocean_far", "specular_color", "underwater_fog_color", "underwater_fog"]

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var report := {}
	var loader = ClassDB.instantiate("WowTerrainLoader")
	for name in CASES:
		var c: Array = CASES[name]
		var result: Dictionary = loader.load_liquid_material(c[0], c[1], c[2], c[3], 1440.0)
		if result.has("error"):
			push_error(name + ": " + str(result.error))
			quit(1)
			return
		var material: ShaderMaterial = result.material
		var values := {"case": str(c), "scene_light": result.scene_light}
		for uniform in UNIFORMS:
			values[uniform] = str(material.get_shader_parameter(uniform))
		for slot in 6:
			var texture = material.get_shader_parameter("texture_%d" % slot)
			if texture is Texture2D:
				var image: Image = texture.get_image()
				values["texture_%d" % slot] = image_statistics(image)
				image.resize(256, 256)
				image.save_png(OUTPUT + name + "-texture-%d.png" % slot)
		report[name] = values
		print("WATER_INPUT ", name, " ", JSON.stringify(values))
	var file := FileAccess.open(OUTPUT + "bound-inputs.json", FileAccess.WRITE)
	file.store_string(JSON.stringify(report, "\t"))
	print("PASS: water bound-input audit")
	quit(0)

func image_statistics(image: Image) -> Dictionary:
	image.convert(Image.FORMAT_RGBA8)
	var data := image.get_data()
	var sum := Vector4.ZERO
	var minimum := Vector4(255, 255, 255, 255)
	var maximum := Vector4.ZERO
	var count := image.get_width() * image.get_height()
	for pixel in count:
		for channel in 4:
			var value := float(data[pixel * 4 + channel])
			sum[channel] += value
			minimum[channel] = minf(minimum[channel], value)
			maximum[channel] = maxf(maximum[channel], value)
	return {"size": str(image.get_size()), "mean_rgba": str(sum / float(count) / 255.0), "min_rgba": str(minimum), "max_rgba": str(maximum)}
