extends "res://tests/display_options.gd"

const SIZE := Vector2i(1280, 720)
const RESIZED := Vector2i(1600, 900)
const PIXEL := Vector2i(12, 12)
const PROBE_COLOR := Color(1.0, 0.0, 0.0, 1.0)

func run_test() -> void:
	root.size = SIZE
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if config.is_empty() or not config.contains("/data/diagnostics/"):
		fail("Render-scale fixture requires owned XDG_CONFIG_HOME under data/diagnostics")
		return
	var path := config.path_join("world-of-osso/options_settings.ron")
	if not FileAccess.file_exists(path):
		fail("Render-scale fixture requires a copied canonical options file in owned XDG_CONFIG_HOME")
		return
	var saved := FileAccess.get_file_as_string(path)
	if not saved.contains("renderScale: 0.75") or not saved.contains("uiScale: 1.0"):
		fail("Render-scale fixture requires startup renderScale: 0.75 and uiScale: 1.0")
		return
	if DisplayServer.get_name() == "headless" or RenderingServer.get_rendering_device() == null:
		fail("Render-scale buffer proof requires Vulkan RenderingDevice and visible offscreen display")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	var probe = load("res://tests/render_scale_buffer_probe.gd").new()
	var camera := Camera3D.new()
	camera.name = "TestRenderScaleCamera"
	camera.position = Vector3(0, 0, 3)
	camera.current = true
	var mesh := MeshInstance3D.new()
	mesh.mesh = BoxMesh.new()
	mesh.position = Vector3(0, 0, -3)
	camera.add_child(mesh)
	var compositor := Compositor.new()
	compositor.compositor_effects = [probe]
	camera.compositor = compositor
	root.add_child(camera)
	var layer := CanvasLayer.new()
	layer.layer = 100
	var marker := ColorRect.new()
	marker.color = PROBE_COLOR
	marker.position = Vector2(8, 8)
	marker.size = Vector2(24, 24)
	marker.mouse_filter = Control.MOUSE_FILTER_IGNORE
	layer.add_child(marker)
	root.add_child(layer)
	if not await expect_buffers(probe, SIZE, Vector2i(960, 540), "startup 0.75"):
		return
	if not await expect_ui(marker, SIZE, "startup"):
		return
	await click_menu_action(client, "MenuBtnOptions")
	if not await wait_for_graphics(client):
		return
	var slider := option_control(client, "Sliderrender_scale")
	if slider == null or not slider.is_visible_in_tree():
		fail("Authored Graphics Render Scale slider missing")
		return
	await drag_scale(slider, false)
	if not expect_saved(path, "0.5") or not await expect_buffers(probe, SIZE, Vector2i(640, 360), "live 0.5"):
		return
	if not await expect_ui(marker, SIZE, "live 0.5"):
		return
	await drag_scale(slider, true)
	if not expect_saved(path, "1.0") or not await expect_buffers(probe, SIZE, SIZE, "live 1.0"):
		return
	if not await expect_ui(marker, SIZE, "live 1.0"):
		return
	root.size = RESIZED
	if not await expect_buffers(probe, RESIZED, RESIZED, "resized 1.0"):
		return
	if not await expect_ui(marker, RESIZED, "resized 1.0"):
		return
	await drag_scale(slider, false)
	if not expect_saved(path, "0.5") or not await expect_buffers(probe, RESIZED, Vector2i(800, 450), "resized 0.5"):
		return
	if not await expect_ui(marker, RESIZED, "resized 0.5"):
		return
	print("PASS: startup, authored Graphics slider, resize, 3D internal buffer and unscaled 2D pixel")
	quit(0)

func drag_scale(slider: Control, maximum: bool) -> void:
	var rect := slider.get_global_rect()
	var start := rect.get_center()
	var end := Vector2(rect.end.x + 10.0 if maximum else rect.position.x - 10.0, start.y)
	var down := InputEventMouseButton.new()
	down.button_index = MOUSE_BUTTON_LEFT
	down.position = start
	down.global_position = start
	down.pressed = true
	root.push_input(down, true)
	await process_frame
	var motion := InputEventMouseMotion.new()
	motion.position = end
	motion.global_position = end
	motion.relative = end - start
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	root.push_input(motion, true)
	await process_frame
	down.position = end
	down.global_position = end
	down.pressed = false
	root.push_input(down, true)
	for frame in range(3):
		await process_frame

func expect_saved(path: String, value: String) -> bool:
	var saved := FileAccess.get_file_as_string(path)
	if not saved.contains("renderScale: " + value) or not saved.contains("uiScale: 1.0"):
		fail("Authored Render Scale edit not persisted independently of UI scale: " + saved)
		return false
	return true

func expect_buffers(probe, target: Vector2i, internal: Vector2i, stage: String) -> bool:
	var previous: Dictionary = probe.snapshot()
	var serial: int = previous.get("serial", 0)
	var deadline := Time.get_ticks_msec() + 5000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var sample: Dictionary = probe.snapshot()
		if sample.get("serial", 0) <= serial or sample.get("target") != target:
			continue
		if sample.get("internal") != internal:
			fail("%s: 3D internal=%s target=%s, expected internal=%s target=%s" % [stage, sample.get("internal"), sample.get("target"), internal, target])
			return false
		print("RENDER_SCALE_BUFFER ", stage, " internal=", internal, " target=", target)
		return true
	fail("%s: no current 3D compositor callback for target=%s; last=%s" % [stage, target, probe.snapshot()])
	return false

func expect_ui(marker: ColorRect, size: Vector2i, stage: String) -> bool:
	if root.size != size or marker.get_global_rect() != Rect2(8, 8, 24, 24):
		fail("%s: UI geometry scaled or root size changed: %s %s" % [stage, marker.get_global_rect(), root.size])
		return false
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.get_size() != size or not image.get_pixelv(PIXEL).is_equal_approx(PROBE_COLOR):
		fail("%s: unscaled UI pixel or final target missing: image=%s pixel=%s" % [stage, image.get_size(), image.get_pixelv(PIXEL)])
		return false
	var directory := OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	if not directory.is_empty():
		if not directory.contains("/data/diagnostics/"):
			fail("Captures must stay under owned data/diagnostics")
			return false
		var error := DirAccess.make_dir_recursive_absolute(directory)
		if error != OK:
			fail("Create capture directory: " + error_string(error))
			return false
		error = image.save_png(directory.path_join("render-scale-" + stage.replace(" ", "-") + ".png"))
		if error != OK:
			fail("Save capture: " + error_string(error))
			return false
	return true
