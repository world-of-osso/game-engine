extends SceneTree
## Main runs rendered Vulkan with -- --screen charcreate, owned XDG_CONFIG_HOME
## and GODOT_TEST_CAPTURE_DIR under data/diagnostics/. No config writes/auth.
## Proves preview visibility under saved Taa, not temporal filter/Retail parity.

const TaaEffect = preload("res://rendering/taa_effect.gd")
const ErrorObserver = preload("res://tests/bloom_lifecycle_pixels.gd").ErrorObserver
const SIZE := Vector2i(1280, 720)
const WAIT_MS := 60000
const BYTE := 1.0 / 255.0

var client: Node
var preview: Node3D
var camera: Camera3D
var projection := {}
var observer := ErrorObserver.new()
var failed := false
var original_time_scale := 1.0


func _initialize() -> void:
	OS.add_logger(observer)
	call_deferred("run_test")


func run_test() -> void:
	original_time_scale = Engine.time_scale
	await run_smoke()
	if is_instance_valid(preview):
		preview.show()
	if is_instance_valid(client):
		client.queue_free()
	for frame in range(8):
		await process_frame
		await RenderingServer.frame_post_draw
	if is_instance_valid(client):
		fail("Client retained after settled teardown")
	preview = null
	camera = null
	client = null
	Engine.time_scale = original_time_scale
	var errors := observer.count()
	OS.remove_logger(observer)
	if not failed and errors == 0:
		print("PASS: no-auth saved Taa authored creation, controlled preview pixels, teardown")
	else:
		printerr("TAA_CHARACTER_CREATE failed=", failed, " engine_errors=", errors)
	quit(1 if failed or errors != 0 else 0)


func run_smoke() -> void:
	root.size = SIZE
	var args := OS.get_cmdline_user_args()
	var screen_index := args.find("--screen")
	if (
		screen_index < 0
		or screen_index + 1 >= args.size()
		or args[screen_index + 1] != "charcreate"
	):
		fail("Pass --screen charcreate after Godot's -- separator")
		return
	for arg in args:
		if arg == "--server" or arg.begins_with("--server="):
			fail("Offline fixture forbids --server")
			return
	var config := OS.get_environment("XDG_CONFIG_HOME")
	var directory := OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	if not config.contains("/data/diagnostics/") or not directory.contains("/data/diagnostics/"):
		fail("Require owned diagnostic config and captures")
		return
	var path := config.path_join("world-of-osso/options_settings.ron")
	if not expect_policy(path):
		return
	if DisplayServer.get_name() == "headless" or RenderingServer.get_rendering_device() == null:
		fail("Authored pixels require rendered Vulkan")
		return
	if RenderingServer.get_current_rendering_driver_name().to_lower() != "vulkan":
		fail("Authored TAA fixture requires Vulkan")
		return
	var error := DirAccess.make_dir_recursive_absolute(directory)
	if error != OK:
		fail("Create capture directory: " + error_string(error))
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		camera = client.get_node_or_null("CharacterCreateScene/Camera") as Camera3D
		preview = client.get_node_or_null("CharacterCreateScene/CreationCharacter") as Node3D
		if client.account_state().screen != "CharacterCreate" or camera == null or preview == null:
			continue
		if root.get_camera_3d() == camera and preview.is_visible_in_tree():
			if not preview.find_children("*", "MeshInstance3D", true, false).is_empty():
				break
	if camera == null or preview == null or root.get_camera_3d() != camera:
		fail(
			"Missing actual creation camera/preview; need authored model route, not a pixel threshold"
		)
		return
	if not expect_offline():
		return
	# Native bone/material clocks advance by delta. Freeze simulation, not drawing;
	# retain authored UI/background and production TAA attachment/history handling.
	Engine.time_scale = 0.0
	await process_frame
	await RenderingServer.frame_post_draw
	if camera.projection != Camera3D.PROJECTION_PERSPECTIVE:
		fail("Authored camera projection not restored after drawing")
		return
	for property in [
		"projection",
		"fov",
		"near",
		"far",
		"size",
		"frustum_offset",
		"keep_aspect",
		"h_offset",
		"v_offset",
		"global_transform"
	]:
		projection[property] = camera.get(property)
	var region := projected_model_rect()
	if region.size.x <= 0 or region.size.y <= 0:
		fail("No projectable preview mesh bounds; need authored model visibility route")
		return
	var visible := await capture(directory, "taa-charcreate-visible.png")
	var visible_control := await capture(directory, "taa-charcreate-visible-control.png")
	preview.hide()
	var hidden := await capture(directory, "taa-charcreate-hidden.png")
	var hidden_control := await capture(directory, "taa-charcreate-hidden-control.png")
	preview.show()
	var restored := await capture(directory, "taa-charcreate-restored.png")
	if failed:
		return
	if not expect_offline() or not expect_policy(path) or not expect_renderer():
		return
	if not expect_model_pixels(visible, visible_control, hidden, hidden_control, restored, region):
		return
	print(
		"AUTHORED_PREVIEW branch=", preview.get_path(), " region=", region, " captures=", directory
	)


func expect_offline() -> bool:
	var state: Dictionary = client.account_state()
	var ui := client.get_node_or_null("CharacterCreateUI") as CanvasLayer
	if (
		state.screen != "CharacterCreate"
		or state.reply_received
		or state.character_count != 0
		or state.unit_count != 0
	):
		fail(
			(
				"Creation startup authenticated, fabricated state, or left CharacterCreate: "
				+ str(state)
			)
		)
		return false
	if ui == null or not ui.visible:
		fail("Authored CharacterCreateUI missing/hidden")
		return false
	return true


func expect_policy(path: String) -> bool:
	if not FileAccess.file_exists(path):
		fail("Missing owned saved options: " + path)
		return false
	var source := FileAccess.get_file_as_string(path)
	if FileAccess.get_open_error() != OK:
		fail("Read saved options: " + error_string(FileAccess.get_open_error()))
		return false
	var expected := {
		"antiAlias": "Taa",
		"bloomEnabled": "false",
		"ssaoEnabled": "false",
		"renderScale": "1",
		"uiScale": "1"
	}
	for key in expected:
		var pattern := RegEx.new()
		pattern.compile("(?m)^\\s*" + key + ":\\s*([^,\\n]+)\\s*,")
		var matches := pattern.search_all(source)
		if matches.size() != 1:
			fail("Require exactly one saved " + key)
			return false
		var value: String = matches[0].get_string(1).strip_edges()
		var matches_value: bool = value == expected[key]
		if key in ["renderScale", "uiScale"]:
			matches_value = value.is_valid_float() and value.to_float() == 1.0
		if not matches_value:
			fail("Saved " + key + "=" + value + " expected " + expected[key])
			return false
	return true


func expect_renderer() -> bool:
	if root.use_taa or root.msaa_3d != Viewport.MSAA_DISABLED or root.scaling_3d_scale != 1.0:
		fail("Saved Taa must leave stock TAA/MSAA off and renderScale=1")
		return false
	if camera.compositor != null:
		for effect in camera.compositor.compositor_effects:
			if effect.get_script() == TaaEffect:
				var state: Dictionary = effect.snapshot()
				if effect.enabled and not state.disposed and state.error.is_empty():
					return true
	fail("Production custom TAA consumer absent/disabled/failed on authored camera")
	return false


func projected_model_rect() -> Rect2i:
	var bounds := Rect2()
	var started := false
	for node in preview.find_children("*", "MeshInstance3D", true, false):
		var mesh := node as MeshInstance3D
		if not mesh.is_visible_in_tree() or mesh.mesh == null:
			continue
		var box := mesh.get_aabb()
		for corner in range(8):
			var point := mesh.global_transform * box.get_endpoint(corner)
			if camera.is_position_behind(point):
				fail("Preview bounds cross camera; need a reliable authored projection route")
				return Rect2i()
			var pixel := camera.unproject_position(point)
			bounds = bounds.expand(pixel) if started else Rect2(pixel, Vector2.ZERO)
			started = true
	if not started:
		return Rect2i()
	var start := Vector2i(bounds.position.floor())
	var end := Vector2i(bounds.end.ceil())
	return Rect2i(start, end - start).intersection(Rect2i(Vector2i.ZERO, SIZE))


func capture(directory: String, filename: String) -> Image:
	if failed:
		return null
	# 64 draws settle temporal history; phase 0 pairs the controller's eight-entry
	# Halton cycle across controls/hide/restore without modifying the controller.
	var deadline := Time.get_ticks_msec() + WAIT_MS
	var drawn := Engine.get_frames_drawn()
	while Time.get_ticks_msec() < deadline:
		await process_frame
		await RenderingServer.frame_post_draw
		if root.get_camera_3d() != camera:
			fail("Authored camera replaced during capture")
			return null
		for property in projection:
			if camera.get(property) != projection[property]:
				fail("Camera field not restored outside draw: " + property)
				return null
		if Engine.get_frames_drawn() - drawn >= 64 and Engine.get_frames_drawn() % 8 == 0:
			if not expect_renderer():
				return null
			var image := root.get_texture().get_image()
			if image == null or image.is_empty() or image.get_size() != SIZE:
				fail("Missing actual authored rendered image")
				return null
			var error := image.save_png(directory.path_join(filename))
			if error != OK:
				fail("Save authored capture: " + error_string(error))
				return null
			return image
	fail("Timed out waiting for settled same-phase rendered capture")
	return null


func contrast(a: Color, b: Color) -> float:
	return maxf(absf(a.r - b.r), maxf(absf(a.g - b.g), absf(a.b - b.b)))


func expect_model_pixels(a: Image, b: Image, h: Image, hc: Image, r: Image, region: Rect2i) -> bool:
	var attributable := 0
	var varying := 0
	var model_contrast := 0.0
	var noise := 0.0
	for y in range(region.position.y, region.end.y):
		for x in range(region.position.x, region.end.x):
			var control := maxf(
				contrast(a.get_pixel(x, y), b.get_pixel(x, y)),
				contrast(h.get_pixel(x, y), hc.get_pixel(x, y))
			)
			control = maxf(control, contrast(b.get_pixel(x, y), r.get_pixel(x, y)))
			var difference := minf(
				contrast(b.get_pixel(x, y), h.get_pixel(x, y)),
				contrast(r.get_pixel(x, y), hc.get_pixel(x, y))
			)
			noise += control
			model_contrast += difference
			if control > BYTE:
				varying += 1
			if difference > control + BYTE:
				attributable += 1
	print(
		"MODEL_PIXELS attributable=",
		attributable,
		" varying=",
		varying,
		" hide_restore_contrast=",
		model_contrast,
		" control_contrast=",
		noise
	)
	# Relational attribution, not an invented character golden/area threshold.
	if attributable == 0 or attributable <= varying or model_contrast <= noise:
		fail(
			"Preview hide/restore did not dominate unchanged-scene variation; visible model unproven"
		)
		return false
	return true


func fail(message: String) -> void:
	failed = true
	printerr(message)
