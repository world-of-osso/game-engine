extends "res://tests/render_scale_buffers.gd"

const UI_PIXEL := Vector2i(12, 12)
const UI_COLOR := Color(1.0, 0.0, 0.0, 1.0)
const SAMPLE_Y := 112
const FIRST_X := 80
const LAST_X := 350
const INPUT_TOLERANCE := 0.012
const OUTPUT_TOLERANCE := 0.014
const MIN_SHARPENING := 0.018
const SHARPNESS := 0.6

func run_test() -> void:
	root.size = SIZE
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if config.is_empty() or not config.contains("/data/diagnostics/"):
		fail("RCAS fixture requires owned XDG_CONFIG_HOME under data/diagnostics")
		return
	var path := config.path_join("world-of-osso/options_settings.ron")
	if not FileAccess.file_exists(path):
		fail("RCAS fixture requires copied canonical options in owned XDG_CONFIG_HOME")
		return
	var saved := FileAccess.get_file_as_string(path)
	if not saved.contains("renderScale: 0.75") or not saved.contains("uiScale: 1.0"):
		fail("RCAS fixture requires startup renderScale: 0.75 and uiScale: 1.0")
		return
	if DisplayServer.get_name() == "headless" or RenderingServer.get_rendering_device() == null:
		fail("RCAS pixels require Vulkan RenderingDevice and visible offscreen display")
		return

	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	var reference := SubViewport.new()
	reference.size = SIZE
	reference.own_world_3d = true
	reference.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(reference)
	add_pattern_camera(root)
	add_pattern_camera(reference)
	var ui_layer := CanvasLayer.new()
	ui_layer.layer = 100
	var marker := ColorRect.new()
	marker.color = UI_COLOR
	marker.position = Vector2(8, 8)
	marker.size = Vector2(24, 24)
	marker.mouse_filter = Control.MOUSE_FILTER_IGNORE
	ui_layer.add_child(marker)
	root.add_child(ui_layer)

	if not await expect_scale(reference, 0.75, "startup"):
		return
	var startup := await capture_pair(reference, "startup")
	if not expect_images(startup, "startup"):
		return
	if not expect_ui_pixel(startup.root_image, marker, "startup"):
		return
	await click_menu_action(client, "MenuBtnOptions")
	if not await wait_for_graphics(client):
		return
	var slider := option_control(client, "Sliderrender_scale")
	if slider == null or not slider.is_visible_in_tree():
		fail("Authored Graphics Render Scale slider missing")
		return
	await drag_scale(slider, true)
	if not expect_saved(path, "1.0") or not await expect_scale(reference, 1.0, "live 1.0"):
		return
	var full_scale := await capture_pair(reference, "live 1.0")
	if not expect_images(full_scale, "live 1.0"):
		return
	if not expect_unfiltered(full_scale.root_image, full_scale.reference_image):
		return
	if not expect_ui_pixel(full_scale.root_image, marker, "live 1.0"):
		return
	await drag_scale(slider, false)
	if not expect_saved(path, "0.5") or not await expect_scale(reference, 0.5, "live 0.5"):
		return
	var half_scale := await capture_pair(reference, "live 0.5")
	if not expect_images(half_scale, "live 0.5"):
		return
	if not expect_ui_pixel(half_scale.root_image, marker, "live 0.5"):
		return
	if not expect_rcas(half_scale.root_image, half_scale.reference_image, "live 0.5"):
		return
	if not expect_rcas(startup.root_image, startup.reference_image, "startup 0.75"):
		return
	print("PASS: real viewport RCAS at startup/live scales, full-scale bypass, untouched UI")
	quit(0)

func add_pattern_camera(viewport: Viewport) -> void:
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color.BLACK
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_DISABLED
	environment.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	environment.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	viewport.add_child(world_environment)
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 2.0
	camera.position = Vector3(0, 0, 3)
	viewport.add_child(camera)
	camera.current = true
	var mesh := MeshInstance3D.new()
	var quad := QuadMesh.new()
	quad.size = Vector2(4, 2.25)
	mesh.mesh = quad
	var material := ShaderMaterial.new()
	material.shader = load("res://tests/render_scale_rcas_pattern.gdshader")
	mesh.material_override = material
	camera.add_child(mesh)
	mesh.position = Vector3(0, 0, -3)

func expect_scale(reference: SubViewport, scale: float, stage: String) -> bool:
	if not is_equal_approx(root.scaling_3d_scale, scale):
		fail("%s: root 3D scale=%f, expected %f" % [stage, root.scaling_3d_scale, scale])
		return false
	reference.scaling_3d_scale = scale
	return true

func capture_pair(reference: SubViewport, stage: String) -> Dictionary:
	var menu := root.find_child("GameMenuUI", true, false) as CanvasLayer
	if menu == null:
		fail("RCAS capture requires the real GameMenuUI layer")
		return {}
	var was_visible := menu.visible
	menu.hide()
	await process_frame
	await RenderingServer.frame_post_draw
	var images := {"root_image": root.get_texture().get_image(), "reference_image": reference.get_texture().get_image()}
	menu.visible = was_visible
	if not save_capture_pair(images, stage):
		return {}
	return images

func save_capture_pair(images: Dictionary, stage: String) -> bool:
	var directory := OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	if directory.is_empty():
		return true
	if not directory.contains("/data/diagnostics/"):
		fail("RCAS captures must stay under owned data/diagnostics")
		return false
	var error := DirAccess.make_dir_recursive_absolute(directory)
	if error != OK:
		fail("Create RCAS capture directory: " + error_string(error))
		return false
	for kind in ["root", "reference"]:
		var image: Image = images[kind + "_image"]
		var filename: String = "rcas-" + stage.replace(" ", "-") + "-" + kind + ".png"
		error = image.save_png(directory.path_join(filename))
		if error != OK:
			fail("Save RCAS " + filename + ": " + error_string(error))
			return false
	return true

func expect_images(images: Dictionary, stage: String) -> bool:
	var actual: Image = images.root_image
	var reference: Image = images.reference_image
	if actual == null or reference == null or actual.get_size() != SIZE or reference.get_size() != SIZE:
		fail("%s: root/reference must both be full-resolution images" % stage)
		return false
	return true

func expect_ui_pixel(image: Image, marker: ColorRect, stage: String) -> bool:
	if marker.get_global_rect() != Rect2(8, 8, 24, 24) or image.get_pixelv(UI_PIXEL) != UI_COLOR:
		fail("%s: higher-layer red UI geometry/pixel changed" % stage)
		return false
	return true

func expect_unfiltered(actual: Image, reference: Image) -> bool:
	print_reference_signal(reference, "live 1.0")
	var edges := 0
	for x in range(FIRST_X, LAST_X):
		var position := Vector2i(x, SAMPLE_Y)
		var expected := reference.get_pixelv(position)
		if channel_error(actual.get_pixelv(position), expected) > INPUT_TOLERANCE:
			fail("full-scale root/reference 3D input mismatch at %s: root=%s reference=%s (UI occlusion or mismatched scene)" % [position, actual.get_pixelv(position), expected])
			return false
		if channel_error(rcas(reference, position), expected) > MIN_SHARPENING:
			edges += 1
	if edges < 4:
		fail("Unfiltered reference lacks four strong 3D pattern edges; RCAS proof cannot run")
		return false
	return true

func expect_rcas(actual: Image, reference: Image, stage: String) -> bool:
	print_reference_signal(reference, stage)
	var edges := 0
	for x in range(FIRST_X, LAST_X):
		var position := Vector2i(x, SAMPLE_Y)
		var input := reference.get_pixelv(position)
		var expected := rcas(reference, position)
		if channel_error(input, expected) <= MIN_SHARPENING:
			continue
		edges += 1
		var output := actual.get_pixelv(position)
		if channel_error(output, expected) > OUTPUT_TOLERANCE:
			fail("%s: RCAS missing/wrong at %s: root=%s expected=%s unfiltered=%s" % [stage, position, output, expected, input])
			return false
	if edges < 4:
		fail("%s: fewer than four sharpenable reference pixels; input pattern missing" % stage)
		return false
	return true

func print_reference_signal(image: Image, stage: String) -> void:
	var darkest := 1.0
	var brightest := 0.0
	var max_delta := 0.0
	var max_neighbor_delta := 0.0
	var sharpenable := 0
	var invalid := 0
	for x in range(FIRST_X, LAST_X):
		var position := Vector2i(x, SAMPLE_Y)
		var input := image.get_pixelv(position)
		darkest = minf(darkest, input.r)
		brightest = maxf(brightest, input.r)
		if x > FIRST_X:
			max_neighbor_delta = maxf(max_neighbor_delta, channel_error(input, image.get_pixel(x - 1, SAMPLE_Y)))
		var delta := channel_error(rcas(image, position), input)
		if is_nan(delta) or is_inf(delta):
			invalid += 1
			continue
		max_delta = maxf(max_delta, delta)
		if delta > MIN_SHARPENING:
			sharpenable += 1
	var samples := [image.get_pixel(FIRST_X, SAMPLE_Y), image.get_pixel(floori((FIRST_X + LAST_X) / 2.0), SAMPLE_Y), image.get_pixel(LAST_X - 1, SAMPLE_Y)]
	print("RCAS_REFERENCE ", stage, " y=", SAMPLE_Y, " red_range=", Vector2(darkest, brightest), " max_neighbor_delta=", max_neighbor_delta, " max_oracle_delta=", max_delta, " sharpenable=", sharpenable, " invalid=", invalid, " samples=", samples)

func rcas(image: Image, position: Vector2i) -> Color:
	var center := image.get_pixelv(position).srgb_to_linear()
	var neighbors: Array[Vector3] = [
		linear_pixel(image, position + Vector2i(0, -1)),
		linear_pixel(image, position + Vector2i(-1, 0)),
		linear_pixel(image, position + Vector2i(1, 0)),
		linear_pixel(image, position + Vector2i(0, 1)),
	]
	var minimum := Vector3(INF, INF, INF)
	var maximum := Vector3(-INF, -INF, -INF)
	var total := Vector3.ZERO
	for neighbor in neighbors:
		minimum = minimum.min(neighbor)
		maximum = maximum.max(neighbor)
		total += neighbor
	var lobe_rgb := Vector3.ZERO
	for channel in range(3):
		var hit_min := minimum[channel] / (4.0 * maximum[channel])
		var hit_max := (10.0 - maximum[channel]) / (-40.0 + 4.0 * minimum[channel])
		lobe_rgb[channel] = maxf(-hit_min, hit_max)
	var lobe := clampf(maxf(lobe_rgb.x, maxf(lobe_rgb.y, lobe_rgb.z)), -0.1875, 0.0) * SHARPNESS
	var rgb := (total * lobe + Vector3(center.r, center.g, center.b)) / (4.0 * lobe + 1.0)
	var encoded := Color(rgb.x, rgb.y, rgb.z, center.a).linear_to_srgb()
	encoded.a = center.a
	return encoded

func linear_pixel(image: Image, position: Vector2i) -> Vector3:
	var bounds := image.get_size() - Vector2i.ONE
	var sample := image.get_pixelv(position.clamp(Vector2i.ZERO, bounds)).srgb_to_linear()
	return Vector3(sample.r, sample.g, sample.b)

func channel_error(a: Color, b: Color) -> float:
	return maxf(absf(a.r - b.r), maxf(absf(a.g - b.g), absf(a.b - b.b)))
