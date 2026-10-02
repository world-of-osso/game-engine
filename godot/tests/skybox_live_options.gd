extends "res://tests/skybox_debug_screen.gd"

# MAIN supplies an isolated XDG_CONFIG_HOME with canonical CameraOptions
# fovDegrees: 105.0, mouseSensitivity: 0.007. Never writes config directly.
# godot --path godot -s res://tests/skybox_live_options.gd --
#   --screen skyboxdebug --skybox-fdid 525142 --skybox-verify --skybox-time-ms 100000
# Reuse production mounting/source/physical-camera/input observers, not pixel tests.
const MENU_WAIT_MS := 3000
const BASELINE_FOV := 105.0
const BASELINE_SENSITIVITY := 0.007
const EDIT_FOV := 110.0
const EDIT_SENSITIVITY := 0.004
# Original options_menu_active_sections and client_options_data ranges.
const FOV_RANGE := Vector2(90.0, 120.0)
const SENSITIVITY_RANGE := Vector2(0.001, 0.01)
const NUMBER_EPSILON := 0.0000001
const HANDLE_PIXEL_EPSILON := 0.05

var options_path := ""
var legacy_path := ""
var legacy_exists := false
var legacy_bytes := PackedByteArray()
var scene_id := 0
var camera_id := 0
var sky_id := 0


func run_test() -> void:
	root.size = Vector2i(1280, 720)
	if not prepare_owned_baseline():
		return
	if not await mount_production() or not observe_assets():
		return
	scene_id = scene.get_instance_id()
	camera_id = camera.get_instance_id()
	sky_id = sky.get_instance_id()
	if not expect_live_values(BASELINE_FOV, BASELINE_SENSITIVITY):
		return
	if not await open_camera_options():
		return
	if not expect_slider_values(BASELINE_FOV, BASELINE_SENSITIVITY):
		return
	if not await edit_camera_values(EDIT_FOV, EDIT_SENSITIVITY):
		return
	if not await close_options_with_escape():
		return
	if not await expect_saved_orbit():
		return
	if not await open_camera_options():
		return
	if not expect_slider_values(EDIT_FOV, EDIT_SENSITIVITY):
		return
	if not await edit_camera_values(BASELINE_FOV, BASELINE_SENSITIVITY):
		return
	if not await close_options_with_done():
		return
	if not await expect_saved_orbit():
		return
	if not await open_main_menu() or not await click_authored("MenuBtnResume"):
		return
	if not await expect_menu_closed():
		return
	print("PASS: offline SkyboxDebug authored live FOV/sensitivity; canonical save, retained sliders, physical orbit, baseline restored")
	quit(0)


func prepare_owned_baseline() -> bool:
	var args := OS.get_cmdline_user_args()
	if first_value(args, "--screen") != "skyboxdebug" or first_value(args, "--skybox-fdid") != "525142":
		return reject("Requires --screen skyboxdebug --skybox-fdid 525142")
	if first_value(args, "--skybox-time-ms") != "100000" or not args.has("--skybox-verify"):
		return reject("Requires --skybox-verify --skybox-time-ms 100000")
	if args.has("--server") or args.has("--light-skybox-id"):
		return reject("Offline coastal fixture excludes server and LightSkybox selection")
	expected_file = "costalislandskybox.m2"
	verify_only = true
	fixed_time = true
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if config.is_empty() or not config.is_absolute_path() or config.simplify_path() == OS.get_environment("HOME").path_join(".config").simplify_path():
		return reject("Requires fixture-owned absolute XDG_CONFIG_HOME, not user's .config")
	options_path = config.path_join("world-of-osso/options_settings.ron")
	legacy_path = ProjectSettings.globalize_path("res://../data/ui/options_settings.ron")
	legacy_exists = FileAccess.file_exists(legacy_path)
	if legacy_exists:
		legacy_bytes = FileAccess.get_file_as_bytes(legacy_path)
	return expect_saved_values(BASELINE_FOV, BASELINE_SENSITIVITY)


func expect_saved_values(fov: float, mouse: float) -> bool:
	if not FileAccess.file_exists(options_path):
		return reject("Canonical owned options missing: " + options_path)
	var raw := FileAccess.get_file_as_string(options_path)
	if FileAccess.get_open_error() != OK:
		return reject("Cannot read canonical owned options: " + options_path)
	var regex := RegEx.new()
	if regex.compile("(?s)\\bcamera\\s*:\\s*\\((.*?)\\)") != OK:
		return reject("Cannot compile camera-block observer")
	var block := regex.search(raw)
	if block == null:
		return reject("Canonical CameraOptions block missing")
	var saved_fov := saved_number(block.get_string(1), "fovDegrees", NAN)
	var saved_mouse := saved_number(block.get_string(1), "mouseSensitivity", NAN)
	if failed:
		return false
	if not is_finite(saved_fov) or not is_finite(saved_mouse) or absf(saved_fov - fov) > 0.001 or absf(saved_mouse - mouse) > NUMBER_EPSILON:
		return reject("Canonical camera expected=%s/%s observed=%s/%s" % [fov, mouse, saved_fov, saved_mouse])
	print("FIXTURE SKYBOX_CANONICAL ", options_path, " fov=", saved_fov, " sensitivity=", saved_mouse)
	return true


func expect_retained_offline() -> bool:
	if not is_instance_valid(scene) or not is_instance_valid(camera) or not is_instance_valid(sky):
		return reject("Options freed underlying sky scene/camera/M2")
	if client.get_node_or_null("SkyboxDebug") != scene or scene.get_instance_id() != scene_id or camera.get_instance_id() != camera_id or sky.get_instance_id() != sky_id or root.get_camera_3d() != camera or not scene.is_ancestor_of(sky) or not scene.is_ancestor_of(camera):
		return reject("Options replaced underlying production sky scene/camera/M2")
	if not check_offline():
		return false
	var state: Dictionary = client.call("account_state")
	if state.screen != "Login" or state.reply_received or state.character_count != 0 or state.unit_count != 0 or state.world_attached or client.get_node_or_null("WorldUnits") != null:
		return reject("Skybox Options authenticated/attached world or changed session: " + str(state))
	if FileAccess.file_exists(legacy_path) != legacy_exists:
		return reject("Options created/deleted legacy options file")
	if legacy_exists and FileAccess.get_file_as_bytes(legacy_path) != legacy_bytes:
		return reject("Options changed legacy options instead of owned canonical file")
	return true


func expect_live_values(fov: float, mouse: float) -> bool:
	expected_fov = fov
	sensitivity = mouse
	if not expect_saved_values(fov, mouse) or not expect_retained_offline():
		return false
	if not is_finite(camera.fov):
		return reject("Actual Camera3D FOV is nonfinite")
	# Independent original orbit eye/basis at yaw 0: menu input must not orbit.
	return check_pose(0.0, START_DISTANCE)


func open_main_menu() -> bool:
	await tap_escape()
	if not await wait_visible_control("MenuBtnOptions"):
		return false
	for name in ["GameMenuRoot", "MenuBtnResume", "MenuBtnExit"]:
		if visible_control(name) == null:
			return reject("Offline authored menu missing: " + name)
	var menu := client.get_node("GameMenuUI")
	if menu.find_child("MenuBtnLogout", true, false) != null:
		return reject("Unauthenticated SkyboxDebug menu exposes Log Out")
	return expect_live_values(expected_fov, sensitivity)


func open_camera_options() -> bool:
	if not await open_main_menu():
		return false
	if not await click_authored("MenuBtnOptions") or not await wait_visible_control("OptionsDoneButton"):
		return false
	if not await click_authored("OptionsTabcamera"):
		return false
	return expect_live_values(expected_fov, sensitivity)


func wait_visible_control(name: String) -> bool:
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if visible_control(name) != null:
			return true
	return reject("Missing authored %s after physical Escape/click from offline SkyboxDebug (bounded %dms); InWorld-only access is missing-menu RED" % [name, MENU_WAIT_MS])


func visible_control(name: String) -> Control:
	var menu := client.get_node_or_null("GameMenuUI")
	var control := menu.find_child(name, true, false) as Control if menu != null else null
	return control if control != null and control.is_visible_in_tree() else null


func click_authored(name: String) -> bool:
	var control := visible_control(name)
	if control == null:
		return reject("Authored visible control missing: " + name)
	var point := control.get_global_rect().get_center()
	push_motion(point, Vector2.ZERO, false)
	await process_frame
	for pressed in [true, false]:
		push_left(point, pressed)
		await process_frame
	await process_frame
	return expect_live_values(expected_fov, sensitivity)


func expect_slider_values(fov: float, mouse: float) -> bool:
	if not expect_slider_fraction("fov_degrees", (fov - FOV_RANGE.x) / (FOV_RANGE.y - FOV_RANGE.x)):
		return false
	if not expect_slider_fraction("mouse_sensitivity", (mouse - SENSITIVITY_RANGE.x) / (SENSITIVITY_RANGE.y - SENSITIVITY_RANGE.x)):
		return false
	var label := visible_control("SliderValuefov_degrees") as Label
	if label == null or label.text != "%.1f" % fov:
		return reject("Authored FOV label did not retain saved value")
	# Mouse caption rounds to .2 decimals; handle position proves numeric retention.
	return expect_live_values(fov, mouse)


func expect_slider_fraction(key: String, fraction: float) -> bool:
	var slider := visible_control("Slider" + key)
	var handle := visible_control("Slider" + key + "Handle")
	if slider == null or handle == null:
		return reject("Authored camera slider/handle missing: " + key)
	var rect := slider.get_global_rect()
	var thumb := handle.get_global_rect()
	# ui-toolkit slider_visuals: thumb left = (track width - thumb width) * pct.
	var expected_x := rect.position.x + (rect.size.x - thumb.size.x) * fraction
	if rect.size.x <= thumb.size.x or absf(thumb.position.x - expected_x) > HANDLE_PIXEL_EPSILON:
		return reject("Retained %s handle expected_x=%s observed_x=%s" % [key, expected_x, thumb.position.x])
	return true


func edit_camera_values(fov: float, mouse: float) -> bool:
	if not await drag_slider("fov_degrees", FOV_RANGE, fov):
		return false
	if not expect_live_values(fov, sensitivity):
		return false
	if not await drag_slider("mouse_sensitivity", SENSITIVITY_RANGE, mouse):
		return false
	if visible_control("OptionsDoneButton") == null:
		return reject("Slider drag closed Options before live-value assertion")
	return expect_slider_values(fov, mouse)


func drag_slider(key: String, bounds: Vector2, value: float) -> bool:
	var slider := visible_control("Slider" + key)
	if slider == null:
		return reject("Authored slider missing: " + key)
	var rect := slider.get_global_rect()
	var start := rect.get_center()
	var end := rect.position + Vector2(rect.size.x * (value - bounds.x) / (bounds.y - bounds.x), rect.size.y * 0.5)
	push_motion(start, Vector2.ZERO, false)
	await process_frame
	push_left(start, true)
	await process_frame
	push_motion(end, end - start, true)
	await process_frame
	push_left(end, false)
	for frame in range(3):
		await process_frame
	return true


func close_options_with_escape() -> bool:
	await tap_escape()
	if visible_control("OptionsDoneButton") != null or visible_control("MenuBtnResume") == null:
		return reject("Options Escape did not return to existing main menu")
	if not expect_live_values(expected_fov, sensitivity):
		return false
	await tap_escape()
	return await expect_menu_closed()


func close_options_with_done() -> bool:
	if not await click_authored("OptionsDoneButton"):
		return false
	return await expect_menu_closed()


func expect_menu_closed() -> bool:
	var deadline := Time.get_ticks_msec() + MENU_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if client.get_node_or_null("GameMenuUI") == null:
			return expect_live_values(expected_fov, sensitivity)
	return reject("Done/Escape did not close authored skybox menu")


func expect_saved_orbit() -> bool:
	# Original yaw = -dx * saved mouseSensitivity; check actual eye and basis.
	await drag(Vector2(4, 0))
	var moved := check_pose(-4.0 * sensitivity, START_DISTANCE)
	await drag(Vector2(-4, 0))
	if not moved:
		return false
	if not expect_live_values(expected_fov, sensitivity):
		return false
	print("FIXTURE SKYBOX_LIVE_ORBIT sensitivity=", sensitivity, " expected_yaw=", -4.0 * sensitivity)
	return true


func tap_escape() -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_ESCAPE
		event.physical_keycode = KEY_ESCAPE
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	for frame in range(3):
		await process_frame


func push_left(point: Vector2, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = point
	event.global_position = point
	event.button_index = MOUSE_BUTTON_LEFT
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
	event.pressed = pressed
	root.push_input(event, true)


func push_motion(point: Vector2, relative: Vector2, held: bool) -> void:
	var event := InputEventMouseMotion.new()
	event.position = point
	event.global_position = point
	event.relative = relative
	event.screen_relative = relative
	event.button_mask = MOUSE_BUTTON_MASK_LEFT if held else 0
	root.push_input(event, true)
