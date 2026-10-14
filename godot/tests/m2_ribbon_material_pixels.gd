extends SceneTree
# Synthetic material inputs; real production ribbon shader, mesh and draw ordering.
var failed := false

func _initialize() -> void:
	call_deferred("prove_materials")

func prove_materials() -> void:
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
	if ordered_pixel.g < 0.9 or ordered_pixel.r > 0.1:
		push_error("Higher authored priority must draw farther green over nearer red, got %s" % ordered_pixel)
		failed = true
	assert_input(strip.add_strip(2, 128, Color.BLUE, 0.0), true)
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

func center_pixel() -> Color:
	var image := root.get_texture().get_image()
	return image.get_pixel(image.get_width() / 2, image.get_height() / 2)
