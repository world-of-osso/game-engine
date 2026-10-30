extends SceneTree

## Exercise changing glyph atlases while the renderer consumes earlier uploads.
func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	var font := FontFile.new()
	var font_path := ProjectSettings.globalize_path("res://../data/fonts/ARIALN.ttf")
	if font.load_dynamic_font(font_path) != OK:
		push_error("Cannot load atlas fixture font: " + font_path)
		quit(1)
		return
	var characters := font.get_supported_chars()
	if characters.length() < 400:
		push_error("Atlas fixture font has too few glyphs")
		quit(1)
		return
	var labels: Array[Label] = []
	for index in range(12):
		var label := Label.new()
		label.position = Vector2(20, 20 + index * 45)
		label.size = Vector2(1200, 45)
		label.add_theme_font_override("font", font)
		label.add_theme_font_size_override("font_size", 18 + index * 4)
		root.add_child(label)
		labels.append(label)
	for offset in range(0, characters.length(), 8):
		for label in labels:
			label.text = characters.substr(offset, 8) + " Atlas rendering"
		await process_frame
	for label in labels:
		label.text = "Atlas rendering: readable text 1234567890"
	await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image == null or image.is_empty():
		push_error("Atlas fixture produced no rendered image")
		quit(1)
		return
	var bright_pixels := 0
	for y in range(20, mini(image.get_height(), 100)):
		for x in range(20, mini(image.get_width(), 1000)):
			var pixel := image.get_pixel(x, y)
			if pixel.r > 0.8 and pixel.g > 0.8 and pixel.b > 0.8:
				bright_pixels += 1
	if bright_pixels < 100:
		push_error("Atlas fixture did not draw the final text")
		quit(1)
		return
	print("FONT_ATLAS_PASS glyphs=", characters.length(), " bright_pixels=", bright_pixels)
	quit(0)
