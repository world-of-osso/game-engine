extends "res://tests/display_options.gd"

# Run with --screen gamemenu, an owned options_settings.ron, and AA_TEST_MODE.
# None/MSAA coverage only. Taa requires a separate temporal oracle; Godot's
# stock use_taa flag cannot establish the legacy filter contract.
const SIZE := Vector2i(1280, 720)
const CENTER := Vector2(640, 360)
const QUAD_PIXELS := Vector2(324, 216)
const ANGLE := 0.31
const SCENE_RECT := Rect2i(420, 180, 440, 360)
const UI_RECT := Rect2i(620, 345, 48, 24)
const EDGE_BAND := 2.0
const BYTE_TOLERANCE := 1.0 / 255.0

var aa_mode := "Msaa4x"


func run_test() -> void:
	root.size = SIZE
	var selected := OS.get_environment("AA_TEST_MODE")
	if not selected.is_empty():
		aa_mode = selected
	if aa_mode not in ["None", "Msaa4x"]:
		fail("AA_TEST_MODE must be None|Msaa4x; Taa requires separate temporal acceptance")
		return
	var config := OS.get_environment("XDG_CONFIG_HOME")
	var directory := OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	if not config.contains("/data/diagnostics/") or not directory.contains("/data/diagnostics/"):
		fail("AA fixture requires owned config and captures under data/diagnostics")
		return
	var path := config.path_join("world-of-osso/options_settings.ron")
	if not FileAccess.file_exists(path):
		fail("AA fixture requires saved options in owned " + path)
		return
	if not expect_saved_aa(path, false, "startup input"):
		return
	if DisplayServer.get_name() == "headless" or RenderingServer.get_rendering_device() == null:
		fail("AA pixels require Vulkan RenderingDevice and visible offscreen display")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	var error := inspect_standalone_menu(client)
	if error != "":
		fail(error)
		return
	# Observe production startup application; never assign msaa_3d or use_taa.
	if not expect_renderer_aa("saved startup"):
		return
	add_aa_scene()
	add_aa_ui()
	var startup := await capture_options_pixels(client, directory, "aa-" + aa_mode + "-startup.png")
	if not expect_aa_pixels(startup, "saved startup"):
		return
	await click_menu_action(client, "MenuBtnOptions")
	if not await wait_for_graphics(client):
		return
	var enable_cap := option_control(client, "ToggleSwitchframe_rate_limit_enabledRightHit")
	if enable_cap == null or not enable_cap.is_visible_in_tree():
		fail("Authored inactive Frame Rate Limit On segment missing")
		return
	# An unrelated real Graphics commit must preserve the hidden saved AA mode.
	# Unlike bloom/render scale, the frame cap does not alter spatial edge pixels.
	await click_option(client, "ToggleSwitchframe_rate_limit_enabledRightHit")
	if Engine.max_fps != DEFAULT_FPS:
		fail("Unrelated authored Frame Rate Limit edit did not apply 144 FPS")
		return
	if not expect_saved_aa(path, true, "unrelated Graphics commit"):
		return
	if not expect_renderer_aa("unrelated Graphics commit"):
		return
	var committed := await capture_options_pixels(
		client, directory, "aa-" + aa_mode + "-committed.png"
	)
	if not expect_aa_pixels(committed, "unrelated Graphics commit"):
		return
	print(
		"PASS: saved AA ",
		aa_mode,
		" startup, geometric coverage and unrelated Graphics preservation"
	)
	quit(0)


func expect_saved_aa(path: String, cap_enabled: bool, stage: String) -> bool:
	var expected := {
		"antiAlias": aa_mode,
		"frameRateLimitEnabled": str(cap_enabled).to_lower(),
		"frameRateLimit": "144",
		"bloomEnabled": "false",
		"ssaoEnabled": "false",
	}
	for key in expected:
		var actual := saved_option_value(path, key)
		if actual != expected[key]:
			fail("%s: saved %s=%s expected %s in %s" % [stage, key, actual, expected[key], path])
			return false
	for key in ["renderScale", "uiScale"]:
		var actual := saved_option_value(path, key)
		if not actual.is_valid_float() or not is_equal_approx(actual.to_float(), 1.0):
			fail("%s: saved %s=%s expected 1.0" % [stage, key, actual])
			return false
	return true


func expect_renderer_aa(stage: String) -> bool:
	var expected_msaa := Viewport.MSAA_4X if aa_mode == "Msaa4x" else Viewport.MSAA_DISABLED
	var expected_taa := false
	print(
		"AA_STATE ", stage, " saved=", aa_mode, " msaa_3d=", root.msaa_3d, " use_taa=", root.use_taa
	)
	if root.msaa_3d != expected_msaa or root.use_taa != expected_taa:
		fail(
			(
				"%s: saved antiAlias=%s observed msaa_3d=%s use_taa=%s expected %s/%s"
				% [stage, aa_mode, root.msaa_3d, root.use_taa, expected_msaa, expected_taa]
			)
		)
		return false
	return true


func add_aa_scene() -> void:
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color.BLACK
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_DISABLED
	environment.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	environment.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	root.add_child(world_environment)
	var camera := Camera3D.new()
	camera.name = "AntiAliasFixtureCamera"
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.keep_aspect = Camera3D.KEEP_HEIGHT
	camera.size = 2.0
	camera.position = Vector3(0, 0, 3)
	root.add_child(camera)
	camera.current = true
	var quad := QuadMesh.new()
	quad.size = QUAD_PIXELS * (2.0 / SIZE.y)
	var mesh := MeshInstance3D.new()
	mesh.mesh = quad
	mesh.rotation.z = ANGLE
	var material := ShaderMaterial.new()
	material.shader = load("res://tests/anti_alias_options_pattern.gdshader")
	mesh.material_override = material
	root.add_child(mesh)


func add_aa_ui() -> void:
	var layer := CanvasLayer.new()
	layer.layer = 100
	for side in range(2):
		var rect := ColorRect.new()
		rect.color = Color.WHITE if side == 0 else Color.BLACK
		rect.position = Vector2(UI_RECT.position + Vector2i(side * 24, 0))
		rect.size = Vector2(24, 24)
		rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
		layer.add_child(rect)
	root.add_child(layer)


func quad_corners() -> PackedVector2Array:
	var corners := PackedVector2Array()
	for sign_pair in [Vector2(-1, -1), Vector2(1, -1), Vector2(1, 1), Vector2(-1, 1)]:
		# World +Y projects upward: reflect after applying the mesh's Z rotation.
		var offset: Vector2 = (sign_pair * QUAD_PIXELS * 0.5).rotated(ANGLE)
		corners.append(CENTER + Vector2(offset.x, -offset.y))
	return corners


func edge_distance(point: Vector2, corners: PackedVector2Array) -> float:
	var nearest := INF
	for index in range(4):
		var start := corners[index]
		var end := corners[(index + 1) % 4]
		var closest := Geometry2D.get_closest_point_to_segment(point, start, end)
		nearest = minf(nearest, point.distance_to(closest))
	return nearest


func expect_aa_pixels(image: Image, stage: String) -> bool:
	if image == null or image.get_size() != SIZE or not is_equal_approx(root.scaling_3d_scale, 1.0):
		fail(stage + ": AA capture must be full 1280x720 with renderScale=1.0")
		return false
	for y in range(SIZE.y):
		for x in range(SIZE.x):
			var color := image.get_pixel(x, y)
			if (
				not is_finite(color.r)
				or not is_finite(color.g)
				or not is_finite(color.b)
				or not is_finite(color.a)
			):
				fail("%s: nonfinite pixel at (%d,%d)" % [stage, x, y])
				return false
	for y in range(UI_RECT.position.y, UI_RECT.end.y):
		for x in range(UI_RECT.position.x, UI_RECT.end.x):
			var expected := Color.WHITE if x < UI_RECT.position.x + 24 else Color.BLACK
			if image.get_pixel(x, y) != expected:
				fail("%s: higher-layer black/white UI changed at (%d,%d)" % [stage, x, y])
				return false
	# Saturated unshaded albedo avoids lighting/tonemap noise in the interior.
	if (
		image.get_pixel(640, 400).r < 1.0 - BYTE_TOLERANCE
		or image.get_pixel(420, 180).r > BYTE_TOLERANCE
	):
		fail(stage + ": missing white quad center or black background")
		return false
	var corners := quad_corners()
	var partial := 0
	var samples := 0
	for y in range(SCENE_RECT.position.y, SCENE_RECT.end.y):
		for x in range(SCENE_RECT.position.x, SCENE_RECT.end.x):
			if edge_distance(Vector2(x + 0.5, y + 0.5), corners) > EDGE_BAND:
				continue
			samples += 1
			var color := image.get_pixel(x, y)
			var lowest := minf(color.r, minf(color.g, color.b))
			var highest := maxf(color.r, maxf(color.g, color.b))
			if lowest > BYTE_TOLERANCE and highest < 1.0 - BYTE_TOLERANCE:
				partial += 1
	print(
		"AA_EDGE ",
		stage,
		" mode=",
		aa_mode,
		" intermediate=",
		partial,
		" band_samples=",
		samples,
		" fraction=",
		float(partial) / samples
	)
	if aa_mode == "None" and partial != 0:
		fail(
			"%s: None produced %d intermediate opaque-edge pixels; expected zero" % [stage, partial]
		)
		return false
	if aa_mode in ["Msaa4x", "Taa"] and partial == 0:
		fail(stage + ": " + aa_mode + " produced no intermediate opaque-edge coverage")
		return false
	return true
