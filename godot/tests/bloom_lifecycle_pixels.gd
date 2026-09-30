extends "res://tests/bloom_options_pixels.gd"

# Main owns Vulkan RED/GREEN. Run with --screen gamemenu and an owned canonical
# options copy: bloomEnabled=true, bloomIntensity=0.08, renderScale=uiScale=1.0.
# XDG_CONFIG_HOME and GODOT_TEST_CAPTURE_DIR must be under data/diagnostics/.
# No RCAS/MSAA combination or exact Bevy-kernel parity claim.
const RESIZED := Vector2i(1440, 720)
const RESIZE_OFFSET := Vector2i(80, 0)


class ErrorObserver:
	extends Logger
	var mutex := Mutex.new()
	var errors := 0

	func _log_error(
		_function: String,
		_file: String,
		_line: int,
		_code: String,
		_rationale: String,
		_editor_notify: bool,
		error_type: int,
		_script_backtraces: Array[ScriptBacktrace]
	) -> void:
		if error_type != ERROR_TYPE_WARNING:
			mutex.lock()
			errors += 1
			mutex.unlock()

	func count() -> int:
		mutex.lock()
		var result := errors
		mutex.unlock()
		return result


var observer := ErrorObserver.new()
var failed := false
var owned_nodes: Array[Node] = []
var owner_camera: Camera3D
var ui_layer: CanvasLayer
var isolated: SubViewport


func _initialize() -> void:
	OS.add_logger(observer)
	Engine.max_fps = 0
	call_deferred("run_test")


func run_test() -> void:
	await run_lifecycle()
	# Free fixture client, cameras, scene, UI and SubViewport before acceptance.
	# The viewport-owned bloom controller remains until SceneTree shutdown.
	for node in owned_nodes:
		if is_instance_valid(node):
			node.queue_free()
	for frame in range(3):
		await process_frame
		await RenderingServer.frame_post_draw
	for node in owned_nodes:
		if is_instance_valid(node):
			fail("Lifecycle teardown retained fixture node: " + str(node))
	owned_nodes.clear()
	owner_camera = null
	ui_layer = null
	isolated = null
	var engine_errors := observer.count()
	OS.remove_logger(observer)
	if not failed and engine_errors == 0:
		print(
			"PASS: native bloom camera replacement, SubViewport isolation, resized halo/emission/UI and fixture teardown"
		)
	else:
		print("BLOOM_LIFECYCLE failed=", failed, " engine_errors=", engine_errors)
	quit(1 if failed or engine_errors != 0 else 0)


func run_lifecycle() -> void:
	root.size = SIZE
	var config := OS.get_environment("XDG_CONFIG_HOME")
	var directory := OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	if not config.contains("/data/diagnostics/") or not directory.contains("/data/diagnostics/"):
		fail("Bloom lifecycle requires owned config/capture directories under data/diagnostics")
		return
	var path := config.path_join("world-of-osso/options_settings.ron")
	if not FileAccess.file_exists(path):
		fail("Bloom lifecycle requires canonical options copy in owned " + path)
		return
	if not expect_saved_bloom(path, true, 0.08, "startup input"):
		return
	if (
		DisplayServer.get_name() == "headless"
		or RenderingServer.get_rendering_device() == null
		or RenderingServer.get_current_rendering_driver_name().to_lower() != "vulkan"
	):
		fail("Bloom lifecycle pixels require Vulkan and visible offscreen display")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	owned_nodes.append(client)
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	var menu_error := inspect_standalone_menu(client)
	if menu_error != "":
		fail(menu_error)
		return
	var before := root.get_children()
	add_bloom_scene()
	add_contrasting_ui()
	for node in root.get_children():
		if not before.has(node):
			owned_nodes.append(node)
			if node is CanvasLayer:
				ui_layer = node
	owner_camera = root.get_node("BloomFixtureCamera") as Camera3D
	add_isolated_scene()
	var startup := await capture_options_pixels(client, directory, "lifecycle-startup-enabled.png")
	if not expect_capture(startup, "startup enabled"):
		return
	await click_menu_action(client, "MenuBtnOptions")
	if failed or not await wait_for_graphics(client):
		return
	if not expect_bloom_controls(client, true, "0.08", "loaded startup"):
		return
	if not await set_bloom(client, path, false, "initial baseline"):
		return
	var disabled := await capture_options_pixels(
		client, directory, "lifecycle-original-disabled.png"
	)
	if not expect_halo_pair(startup, disabled, "startup enabled"):
		return
	var isolated_baseline := await capture_isolated(directory, "lifecycle-subviewport-disabled.png")
	if isolated_baseline == null or not expect_disabled_scene(isolated_baseline):
		return
	if not await set_bloom(client, path, true, "before replacement"):
		return
	var original := await capture_options_pixels(
		client, directory, "lifecycle-original-enabled.png"
	)
	if not expect_halo_pair(original, disabled, "before replacement"):
		return
	if not await expect_isolated_unchanged(directory, isolated_baseline, "original-enabled"):
		return
	# New camera is added while owner bloom is already active. Transfer only the
	# emissive fixture, not the old compositor or camera resource.
	replace_owner_camera()
	var replaced := await capture_options_pixels(
		client, directory, "lifecycle-replacement-enabled.png"
	)
	if not await set_bloom(client, path, false, "replacement baseline"):
		return
	var replacement_baseline := await capture_options_pixels(
		client, directory, "lifecycle-replacement-disabled.png"
	)
	if not expect_halo_pair(replaced, replacement_baseline, "active camera replacement"):
		return
	if not expect_emission_center(
		replacement_baseline, disabled, "replacement versus original source"
	):
		return
	if not await set_bloom(client, path, true, "before resize"):
		return
	var replacement_restored := await capture_options_pixels(
		client, directory, "lifecycle-replacement-restored.png"
	)
	if not expect_halo_pair(replacement_restored, replacement_baseline, "replacement restored"):
		return
	if not await expect_isolated_unchanged(directory, isolated_baseline, "replacement-enabled"):
		return
	# Keep height fixed: orthographic KEEP_HEIGHT preserves the 48px emitter.
	# Width/aspect change reallocates the production bloom pyramid. Center and
	# adjacent UI move +80px; crop only translates sample coordinates, not pixels.
	root.size = RESIZED
	ui_layer.offset = Vector2(RESIZE_OFFSET)
	var resized := await capture_resized(client, directory, "lifecycle-resized-enabled.png")
	if resized == null:
		return
	if not await set_bloom(client, path, false, "resized baseline"):
		return
	var resized_baseline := await capture_resized(
		client, directory, "lifecycle-resized-disabled.png"
	)
	if (
		resized_baseline == null
		or not expect_halo_pair(resized, resized_baseline, "production resize")
	):
		return
	if not expect_emission_center(resized_baseline, disabled, "resize versus original source"):
		return
	if not await set_bloom(client, path, true, "resized restored"):
		return
	var restored := await capture_resized(client, directory, "lifecycle-resized-restored.png")
	if restored == null or not expect_halo_pair(restored, resized_baseline, "resized restored"):
		return
	if not await expect_isolated_unchanged(directory, isolated_baseline, "resized-enabled"):
		return
	if not expect_saved_bloom(path, true, 0.08, "final owned options"):
		return


func set_bloom(client: Node, path: String, enabled: bool, stage: String) -> bool:
	await click_option(
		client,
		"ToggleSwitchbloom_enabledRightHit" if enabled else "ToggleSwitchbloom_enabledLeftHit"
	)
	return (
		not failed
		and expect_saved_bloom(path, enabled, 0.08, stage)
		and expect_bloom_controls(client, enabled, "0.08", stage)
	)


func expect_halo_pair(enabled: Image, baseline: Image, stage: String) -> bool:
	if not expect_capture(baseline, stage + " disabled") or not expect_disabled_scene(baseline):
		return false
	if (
		not expect_capture(enabled, stage + " enabled")
		or not expect_emission_center(enabled, baseline, stage)
	):
		return false
	var delta := halo_mean(enabled) - halo_mean(baseline)
	print(
		"BLOOM_LIFECYCLE ",
		stage,
		" disabled=",
		halo_mean(baseline),
		" enabled=",
		halo_mean(enabled),
		" delta=",
		delta
	)
	if delta < MIN_HALO:
		fail("%s: halo delta %f below unchanged minimum %f" % [stage, delta, MIN_HALO])
		return false
	return true


func replace_owner_camera() -> void:
	var replacement := Camera3D.new()
	replacement.name = "BloomReplacementCamera"
	replacement.projection = owner_camera.projection
	replacement.keep_aspect = owner_camera.keep_aspect
	replacement.size = owner_camera.size
	replacement.transform = owner_camera.transform
	owned_nodes.append(replacement)
	root.add_child(replacement)
	var emitter := owner_camera.get_child(0) as MeshInstance3D
	emitter.reparent(replacement, false)
	replacement.make_current()
	owner_camera.queue_free()
	owner_camera = replacement


func add_isolated_scene() -> void:
	isolated = SubViewport.new()
	isolated.name = "BloomIsolatedViewport"
	isolated.size = SIZE
	isolated.own_world_3d = true
	isolated.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	owned_nodes.append(isolated)
	root.add_child(isolated)
	var camera := Camera3D.new()
	camera.projection = owner_camera.projection
	camera.keep_aspect = owner_camera.keep_aspect
	camera.size = owner_camera.size
	camera.transform = owner_camera.transform
	camera.environment = owner_camera.get_world_3d().environment
	isolated.add_child(camera)
	camera.make_current()
	var emitter := owner_camera.get_child(0).duplicate() as MeshInstance3D
	camera.add_child(emitter)


func capture_isolated(directory: String, filename: String) -> Image:
	for frame in range(3):
		await process_frame
	await RenderingServer.frame_post_draw
	var image := isolated.get_texture().get_image()
	if image == null or image.get_size() != SIZE:
		fail("SubViewport capture missing or wrong dimensions: " + filename)
		return null
	var error := DirAccess.make_dir_recursive_absolute(directory)
	if error == OK:
		error = image.save_png(directory.path_join(filename))
	if error != OK:
		fail("Save SubViewport capture: " + error_string(error))
		return null
	return image


func expect_isolated_unchanged(directory: String, baseline: Image, stage: String) -> bool:
	var image := await capture_isolated(directory, "lifecycle-subviewport-" + stage + ".png")
	if (
		image == null
		or not expect_disabled_scene(image)
		or not expect_emission_center(image, baseline, stage + " SubViewport")
	):
		return false
	# Whole independently rendered image must remain unchanged, including black
	# outside the inherited halo sample. A leaked weak halo cannot hide elsewhere.
	for y in range(SIZE.y):
		for x in range(SIZE.x):
			var actual := image.get_pixel(x, y)
			var source := baseline.get_pixel(x, y)
			for channel in range(4):
				if (
					not is_finite(actual[channel])
					or not is_finite(source[channel])
					or absf(actual[channel] - source[channel]) > BLACK_TOLERANCE
				):
					fail(
						(
							"%s: owner bloom changed SubViewport pixel (%d,%d): actual=%s baseline=%s"
							% [stage, x, y, actual, source]
						)
					)
					return false
	print("BLOOM_LIFECYCLE isolated unchanged: ", stage)
	return true


func capture_resized(client: Node, directory: String, filename: String) -> Image:
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if root.size == RESIZED and Vector2i(root.get_visible_rect().size) == RESIZED:
			var image := await capture_options_pixels(client, directory, filename)
			if image == null:
				return null
			if image.get_size() != RESIZED:
				fail("Resized production capture has dimensions " + str(image.get_size()))
				return null
			print(
				"BLOOM_LIFECYCLE resize root=",
				root.size,
				" viewport=",
				root.get_visible_rect().size,
				" capture=",
				image.get_size()
			)
			return image.get_region(Rect2i(RESIZE_OFFSET, SIZE))
	fail("Production viewport did not resize to " + str(RESIZED))
	return null


func fail(message: String) -> void:
	failed = true
	push_error(message)
