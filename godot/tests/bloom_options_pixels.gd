extends "res://tests/display_options.gd"

# Run real GameClient with --screen gamemenu and an owned canonical options copy:
# bloomEnabled: true, bloomIntensity: 0.08, renderScale: 1.0, uiScale: 1.0.
# BLOOM_TEST_START_DISABLED=1 requires bloomEnabled: false in that owned copy.
# BLOOM_TEST_COMBINED=1 requires renderScale: 0.5 for production startup RCAS.
# Combined MSAA 4X is fixture viewport state, not persisted Options coverage.
# GODOT_TEST_CAPTURE_DIR and XDG_CONFIG_HOME must be under data/diagnostics/.
# Qualitative wiring proof only; no claim of exact Bevy bloom kernel fidelity.
const SIZE := Vector2i(1280, 720)
const CENTER := Vector2i(640, 360)
const UI_RECT := Rect2i(616, 306, 48, 24)
const HALO_RECT := Rect2i(668, 344, 16, 32)
const BLACK_TOLERANCE := 1.0 / 255.0
const MIN_HALO := 2.0 / 255.0
const MIN_INCREASE := 1.0 / 255.0

var combined := OS.get_environment("BLOOM_TEST_COMBINED") == "1"

func expected_render_scale() -> float:
	return 0.5 if combined else 1.0

func run_test() -> void:
	root.size = SIZE
	if combined:
		root.msaa_3d = Viewport.MSAA_4X
	var start_enabled := OS.get_environment("BLOOM_TEST_START_DISABLED") != "1"
	var config := OS.get_environment("XDG_CONFIG_HOME")
	var directory := OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	if not config.contains("/data/diagnostics/") or not directory.contains("/data/diagnostics/"):
		fail("Bloom fixture requires owned config and capture directories under data/diagnostics")
		return
	var path := config.path_join("world-of-osso/options_settings.ron")
	if not FileAccess.file_exists(path):
		fail("Bloom fixture requires canonical options copy in owned " + path)
		return
	if not expect_saved_bloom(path, start_enabled, 0.08, "startup input"):
		return
	if DisplayServer.get_name() == "headless" or RenderingServer.get_rendering_device() == null:
		fail("Bloom pixels require Vulkan RenderingDevice and visible offscreen display")
		return
	var client: Node = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_for_startup_menu(client):
		return
	if inspect_standalone_menu(client) != "":
		fail("Bloom fixture requires actual standalone GameMenu: " + inspect_standalone_menu(client))
		return
	add_bloom_scene()
	add_contrasting_ui()
	# Preserve startup pixels before any Options action. Validate halo only after
	# proving the disabled baseline, scene signal, and actual persisted/UI state.
	var startup := await capture_options_pixels(client, directory, "bloom-startup-enabled.png" if start_enabled else "bloom-startup-disabled.png")
	if startup == null:
		return
	if not start_enabled:
		if not expect_capture(startup, "saved disabled startup") or not expect_disabled_scene(startup):
			return
	await click_menu_action(client, "MenuBtnOptions")
	if not await wait_for_graphics(client):
		return
	if not expect_saved_bloom(path, start_enabled, 0.08, "loaded startup") or not expect_bloom_controls(client, start_enabled, "0.08", "loaded startup"):
		return
	if start_enabled:
		await click_option(client, "ToggleSwitchbloom_enabledLeftHit")
	if not expect_saved_bloom(path, false, 0.08, "live disabled") or not expect_bloom_controls(client, false, "0.08", "live disabled"):
		return
	var disabled := await capture_options_pixels(client, directory, "bloom-live-disabled.png")
	if not expect_capture(disabled, "live disabled") or not expect_disabled_scene(disabled):
		return
	if not expect_capture(startup, "saved startup") or not expect_emission_center(startup, disabled, "saved startup"):
		return
	var baseline := halo_mean(disabled)
	var startup_halo := halo_mean(startup)
	print("BLOOM_BASELINE disabled=", baseline, " startup=", startup_halo)
	if start_enabled and startup_halo - baseline < MIN_HALO:
		fail("Saved bloomEnabled=true bloomIntensity=0.08 produced no startup halo outside fixed emissive quad: enabled=%f disabled=%f required delta>=%f; disabled scene, persisted state and authored controls established" % [startup_halo, baseline, MIN_HALO])
		return
	await click_option(client, "ToggleSwitchbloom_enabledRightHit")
	if not expect_saved_bloom(path, true, 0.08, "live enabled") or not expect_bloom_controls(client, true, "0.08", "live enabled"):
		return
	var enabled := await capture_options_pixels(client, directory, "bloom-live-enabled.png")
	if not expect_capture(enabled, "live enabled") or not expect_emission_center(enabled, disabled, "live enabled"):
		return
	var enabled_halo := halo_mean(enabled)
	if enabled_halo - baseline < MIN_HALO:
		fail("Authored bloom On failed to restore halo: enabled=%f disabled=%f" % [enabled_halo, baseline])
		return
	if not await set_option_slider_end(client, "Sliderbloom_intensity", true):
		return
	if not expect_saved_bloom(path, true, 1.0, "live intensity 1.0") or not expect_bloom_controls(client, true, "1.00", "live intensity 1.0"):
		return
	var stronger := await capture_options_pixels(client, directory, "bloom-live-intensity-1.png")
	if not expect_capture(stronger, "live intensity 1.0") or not expect_emission_center(stronger, disabled, "live intensity 1.0"):
		return
	var stronger_halo := halo_mean(stronger)
	print("BLOOM_LIVE low=", enabled_halo, " high=", stronger_halo)
	if stronger_halo - enabled_halo < MIN_INCREASE:
		fail("Authored Bloom Intensity 0.08->1.0 did not increase halo: low=%f high=%f" % [enabled_halo, stronger_halo])
		return
	# Disable at high intensity as well: emission must remain, halo must vanish.
	await click_option(client, "ToggleSwitchbloom_enabledLeftHit")
	if not expect_saved_bloom(path, false, 1.0, "high intensity disabled") or not expect_bloom_controls(client, false, "1.00", "high intensity disabled"):
		return
	var final_disabled := await capture_options_pixels(client, directory, "bloom-high-intensity-disabled.png")
	if not expect_capture(final_disabled, "high intensity disabled") or not expect_disabled_scene(final_disabled) or not expect_emission_center(final_disabled, disabled, "high intensity disabled"):
		return
	if start_enabled:
		print("PASS: saved startup bloom halo, authored live Off/On/intensity, preserved emission and exact higher-layer contrasting UI")
	else:
		print("PASS: saved disabled startup emission without halo, authored first On after existing camera/intensity/Off, preserved emission and exact higher-layer contrasting UI")
	quit(0)

func add_bloom_scene() -> void:
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color.BLACK
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_DISABLED
	environment.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	environment.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	# Do not turn on Godot glow here: only production Options may enable bloom.
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	root.add_child(world_environment)
	var camera := Camera3D.new()
	camera.name = "BloomFixtureCamera"
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.keep_aspect = Camera3D.KEEP_HEIGHT
	camera.size = 2.0
	camera.position = Vector3(0, 0, 3)
	root.add_child(camera)
	camera.current = true
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE * (48.0 * 2.0 / SIZE.y)
	var mesh := MeshInstance3D.new()
	mesh.mesh = quad
	var material := ShaderMaterial.new()
	material.shader = load("res://tests/bloom_options_pattern.gdshader")
	mesh.material_override = material
	camera.add_child(mesh)
	mesh.position = Vector3(0, 0, -3)

func add_contrasting_ui() -> void:
	var layer := CanvasLayer.new()
	layer.layer = 100
	# Adjacent white/black UI edges near the emitter discriminate whole-screen
	# bloom applied after UI. Ignore mouse so authored Options still receive input.
	for side in range(2):
		var rect := ColorRect.new()
		rect.color = Color.WHITE if side == 0 else Color.BLACK
		rect.position = Vector2(UI_RECT.position + Vector2i(side * 24, 0))
		rect.size = Vector2(24, 24)
		rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
		layer.add_child(rect)
	root.add_child(layer)

func expect_saved_bloom(path: String, enabled: bool, intensity: float, stage: String) -> bool:
	for key in ["bloomEnabled", "bloomIntensity", "renderScale", "uiScale"]:
		var actual := saved_option_value(path, key)
		var expected := str(enabled).to_lower() if key == "bloomEnabled" else str(
			intensity if key == "bloomIntensity" else
			expected_render_scale() if key == "renderScale" else 1.0
		)
		var matches := actual == expected if key == "bloomEnabled" else actual.is_valid_float() and is_equal_approx(actual.to_float(), expected.to_float())
		if not matches:
			fail("%s: persisted %s=%s expected %s in owned %s" % [stage, key, actual, expected, path])
			return false
	return true

func expect_bloom_controls(client: Node, enabled: bool, value: String, stage: String) -> bool:
	# toggle_widget emits a hit control ONLY for the inactive segment.
	var inactive := option_control(client, "ToggleSwitchbloom_enabledLeftHit" if enabled else "ToggleSwitchbloom_enabledRightHit")
	var active := option_control(client, "ToggleSwitchbloom_enabledRightHit" if enabled else "ToggleSwitchbloom_enabledLeftHit")
	var label := option_control(client, "SliderValuebloom_intensity") as Label
	var slider := option_control(client, "Sliderbloom_intensity")
	if inactive == null or not inactive.is_visible_in_tree() or active != null or label == null or label.text != value or slider == null or not slider.is_visible_in_tree():
		fail("%s: authored Graphics bloom toggle/value do not reflect enabled=%s intensity=%s" % [stage, enabled, value])
		return false
	return true

func expect_capture(image: Image, stage: String) -> bool:
	if image == null or image.get_size() != SIZE:
		fail(stage + ": full-resolution bloom capture missing")
		return false
	if not is_equal_approx(root.scaling_3d_scale, expected_render_scale()):
		fail("%s: renderScale=%f expected %f" % [
			stage, root.scaling_3d_scale, expected_render_scale()
		])
		return false
	if combined and root.msaa_3d != Viewport.MSAA_4X:
		fail(stage + ": combined fixture viewport MSAA is not 4X (not Options coverage)")
		return false
	for y in range(UI_RECT.position.y, UI_RECT.end.y):
		for x in range(UI_RECT.position.x, UI_RECT.end.x):
			var expected := Color.WHITE if x < UI_RECT.position.x + 24 else Color.BLACK
			if image.get_pixel(x, y) != expected:
				fail("%s: higher-layer contrasting UI changed at (%d,%d): actual=%s expected=%s" % [stage, x, y, image.get_pixel(x, y), expected])
				return false
	for rect in [HALO_RECT, Rect2i(CENTER - Vector2i(8, 8), Vector2i(16, 16))]:
		for y in range(rect.position.y, rect.end.y):
			for x in range(rect.position.x, rect.end.x):
				var color := image.get_pixel(x, y)
				if not is_finite(color.r) or not is_finite(color.g) or not is_finite(color.b) or not is_finite(color.a):
					fail("%s: nonfinite scene pixel at (%d,%d)" % [stage, x, y])
					return false
	return true

func expect_disabled_scene(image: Image) -> bool:
	for y in range(HALO_RECT.position.y, HALO_RECT.end.y):
		for x in range(HALO_RECT.position.x, HALO_RECT.end.x):
			var color := image.get_pixel(x, y)
			if maxf(color.r, maxf(color.g, color.b)) > BLACK_TOLERANCE:
				fail("Disabled bloom left halo on plain black background at (%d,%d): %s" % [x, y, color])
				return false
	for offset in [Vector2i(-8, -8), Vector2i(8, 8), Vector2i.ZERO]:
		var color := image.get_pixelv(CENTER + offset)
		if minf(color.r, minf(color.g, color.b)) < 0.95:
			fail("Disabled baseline lacks authored bright emission center at %s: %s" % [CENTER + offset, color])
			return false
	return true

func expect_emission_center(image: Image, baseline: Image, stage: String) -> bool:
	for y in range(CENTER.y - 8, CENTER.y + 8):
		for x in range(CENTER.x - 8, CENTER.x + 8):
			var actual := image.get_pixel(x, y)
			var source := baseline.get_pixel(x, y)
			# Additive postprocessing may brighten; disabling may not remove or
			# recolor the authored emission. Saturated source bounds this allowance.
			if actual.r < source.r - BLACK_TOLERANCE or actual.g < source.g - BLACK_TOLERANCE or actual.b < source.b - BLACK_TOLERANCE:
				fail("%s: authored emission center dimmed/changed at (%d,%d): actual=%s disabled=%s" % [stage, x, y, actual, source])
				return false
	return true

func halo_mean(image: Image) -> float:
	var total := 0.0
	for y in range(HALO_RECT.position.y, HALO_RECT.end.y):
		for x in range(HALO_RECT.position.x, HALO_RECT.end.x):
			var color := image.get_pixel(x, y)
			total += (color.r + color.g + color.b) / 3.0
	return total / float(HALO_RECT.size.x * HALO_RECT.size.y)
