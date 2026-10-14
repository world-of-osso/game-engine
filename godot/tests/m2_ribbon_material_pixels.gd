extends SceneTree
# Synthetic material inputs; real production ribbon shader, mesh and draw ordering.
var failed := false
var receipts := []
var output := OS.get_environment("RIBBON_MATERIAL_OUTPUT")

func _initialize() -> void:
	call_deferred("prove_materials")

func prove_materials() -> void:
	if output.is_empty() or DirAccess.make_dir_recursive_absolute(output) != OK:
		push_error("Ribbon raster output directory unavailable")
		quit(1)
		return
	var scene := Node3D.new()
	root.add_child(scene)
	var environment := WorldEnvironment.new()
	environment.environment = Environment.new()
	environment.environment.background_mode = Environment.BG_COLOR
	environment.environment.background_color = Color.BLACK
	environment.environment.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	scene.add_child(environment)
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 3.0
	camera.position = Vector3(0.0, 0.0, 3.0)
	scene.add_child(camera)
	camera.make_current()
	var strip = ClassDB.instantiate("RibbonMaterialFixture")
	scene.add_child(strip)
	assert_input(strip.add_strip(0, 0, Color(1.0, 0.0, 0.0, 0.25), 0.0))
	await settle()
	var alpha_pixel := center_pixel()
	save_capture("alpha", alpha_pixel, [{"blend": 0, "priority": 0, "rgba": [1.0, 0.0, 0.0, 0.25], "depth": 0.0}])
	if alpha_pixel.r >= 0.8 or alpha_pixel.r <= 0.1:
		push_error("Opaque-source ribbon must alpha blend 0.25, got %s" % alpha_pixel)
		failed = true
	strip.queue_free()
	await process_frame
	strip = ClassDB.instantiate("RibbonMaterialFixture")
	scene.add_child(strip)
	assert_input(strip.add_strip(2, -3, Color.RED, 0.3))
	assert_input(strip.add_strip(2, 7, Color.GREEN, -0.3))
	await settle()
	var ordered_pixel := center_pixel()
	save_capture("priority", ordered_pixel, [{"blend": 2, "priority": -3, "rgba": [1.0, 0.0, 0.0, 1.0], "depth": 0.3}, {"blend": 2, "priority": 7, "rgba": [0.0, 1.0, 0.0, 1.0], "depth": -0.3}])
	if ordered_pixel.g < 0.9 or ordered_pixel.r > 0.1:
		push_error("Higher authored priority must draw farther green over nearer red, got %s" % ordered_pixel)
		failed = true
	assert_input(strip.add_strip(2, 128, Color.BLUE, 0.0), true)
	strip.queue_free()
	await process_frame
	strip = ClassDB.instantiate("RibbonMaterialFixture")
	scene.add_child(strip)
	assert_input(strip.add_strip(0, 0, Color.WHITE, 0.0))
	assert_input(strip.sample_real_uv(0))
	await settle()
	var uv_start := center_pixel()
	assert_input(strip.sample_real_uv(633))
	await settle()
	var uv_middle := center_pixel()
	if uv_start.g < 0.9 or uv_start.r > 0.1 or uv_middle.r < 0.9 or uv_middle.g > 0.1:
		push_error("Real1865728 UV translation must move green to red: %s → %s" % [uv_start, uv_middle])
		failed = true
	print("Real ribbon UV raster source1865728 global0=", uv_start, " global633=", uv_middle)
	print("Ribbon material raster alpha=", alpha_pixel, " priority=", ordered_pixel)
	if not failed: print("PASS native ribbon source opaque-alpha and priority raster")
	quit(1 if failed else 0)

func assert_input(error: String, expect_error := false) -> void:
	if error.is_empty() == expect_error:
		push_error("Ribbon material fixture unexpected result: %s" % error)
		failed = true

func settle() -> void:
	for _frame in range(4):
		await process_frame
		await RenderingServer.frame_post_draw

func save_capture(phase: String, pixel: Color, inputs: Array) -> void:
	var image := root.get_texture().get_image()
	var path := output.path_join("native-ribbon-%s.png" % phase)
	if image.save_png(path) != OK:
		push_error("Ribbon PNG save failed: %s" % path)
		failed = true
		return
	receipts.append({"phase": phase, "image": path, "size": [image.get_width(), image.get_height()], "pixel": [pixel.r, pixel.g, pixel.b, pixel.a], "inputs": inputs, "revision": OS.get_environment("RIBBON_MATERIAL_REVISION"), "source": "production ribbon material, shader and mesh path; synthetic material inputs", "unix_time": Time.get_unix_time_from_system()})
	var file := FileAccess.open(output.path_join("material-raster-receipts.json"), FileAccess.WRITE)
	if file == null:
		push_error("Ribbon raster receipts save failed")
		failed = true
		return
	file.store_string(JSON.stringify(receipts, "\t"))

func center_pixel() -> Color:
	var image := root.get_texture().get_image()
	return image.get_pixel(image.get_width() / 2, image.get_height() / 2)
