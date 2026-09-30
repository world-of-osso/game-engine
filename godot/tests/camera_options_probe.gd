extends RefCounted

const LOOK_POINT := Vector2(640, 360)
const VERTICAL_MOTION := Vector2(0, 4)
# Original authored ranges/defaults, not values read back from native options.
const LOOK_MIN := 0.002
const LOOK_MAX := 0.03
const LOOK_DEFAULT := 0.01
const FOV_MIN := 90.0
const FOV_MAX := 120.0
const FOV_DEFAULT := 90.0
const PITCH_TOLERANCE := 0.00001
const FOV_TOLERANCE := 0.001
# Independent CameraState::default (src/camera_control_data.rs): logical
# distance/target 15, bounds 2..40, zoom_speed 8 (no rate assertion here).
const DISTANCE_DEFAULT := 15.0
const DISTANCE_MIN_DEFAULT := 2.0
const DISTANCE_MAX_DEFAULT := 40.0
# Original authored Camera slider ranges.
const DISTANCE_MIN_RANGE := Vector2(1.0, 10.0)
const DISTANCE_MAX_RANGE := Vector2(10.0, 60.0)
const DISTANCE_MIN := 10.0
const DISTANCE_MAX := 12.0
const DISTANCE_NUMERIC_TOLERANCE := 0.00001
const DISTANCE_ENDPOINT_TOLERANCE := 0.01
const DISTANCE_SETTLE_FRAMES := 120
# Shared apply_camera_input scroll factor, independent of native readback.
const SCROLL_DISTANCE_STEP := 2.0

var fixture

func run(flow: SceneTree, client: Node) -> bool:
	fixture = flow
	if client.get_node_or_null("GameMenuUI") != null or client.target_state().target != null:
		return reject("Camera probe requires menu-closed/target-none")
	for inverted in [false, true]:
		if not await set_invert_y(client, inverted):
			return false
		if not await expect_vertical_look(client, inverted):
			return false
	if not await set_invert_y(client, false):
		return false
	for requested in [Vector2(0.02, 105.0), Vector2(LOOK_DEFAULT, FOV_DEFAULT)]:
		if not await set_camera_sliders(client, requested.x, requested.y):
			return false
		if not await expect_vertical_look(client, false, -VERTICAL_MOTION.y * requested.x):
			return false
		if not expect_physical_fov(client, requested.y):
			return false
	if not await expect_distance_bounds(client):
		return false
	if not await clear_mouse_selection(client):
		return false
	if client.get_node_or_null("GameMenuUI") != null or not client.account_state().gameplay_input_allowed:
		return reject("Camera probe did not return to menu-closed gameplay")
	print("PASS: authored look sensitivity/FOV persist numerically and reach gameplay/physical camera; slider defaults saved")
	print("PASS: authored invert-Y Off/On persists and reverses gameplay pitch sign; default Off restored, input released")
	return true

func clear_mouse_selection(client: Node) -> bool:
	# A real RMB press can select the unit under the pointer. Original Escape
	# clears that selection before it can open the menu.
	if client.target_state().target != null:
		await tap_escape()
		if client.target_state().target != null or client.get_node_or_null("GameMenuUI") != null:
			return reject("Escape did not clear mouse-look selection before menu")
	return true

func open_camera_options(client: Node) -> bool:
	if not await clear_mouse_selection(client):
		return false
	await tap_escape()
	if not await fixture.wait_menu(client) or not fixture.menu_authored(client):
		return false
	for name in ["MenuBtnOptions", "OptionsTabcamera"]:
		if not await click_option(client, name):
			return false
	return true

func set_invert_y(client: Node, inverted: bool) -> bool:
	if not await open_camera_options(client):
		return false
	var hit := "ToggleSwitchinvert_yRightHit" if inverted else "ToggleSwitchinvert_yLeftHit"
	var menu := client.get_node("GameMenuUI")
	# Authored toggle_widget emits a hitbox only for the inactive segment.
	# If already selected (initial Off), round-trip through the opposite
	# segment so this case still exercises an authored edit and persistence.
	if menu.find_child(hit, true, false) == null:
		var opposite := "ToggleSwitchinvert_yLeftHit" if inverted else "ToggleSwitchinvert_yRightHit"
		if not await click_option(client, opposite):
			return false
	if not await click_option(client, hit):
		return false
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if config.is_empty():
		return reject("Camera probe requires fixture-owned XDG_CONFIG_HOME")
	var path := config.path_join("world-of-osso/options_settings.ron")
	if not FileAccess.file_exists(path):
		return reject("Authored camera edit did not create canonical settings file: " + path)
	var saved := FileAccess.get_file_as_string(path).replace(" ", "").replace("\t", "").replace("\n", "").replace("\r", "")
	var expected := "invert_y:true" if inverted else "invert_y:false"
	if not saved.contains(expected):
		return reject("Authored camera edit did not persist %s before closing Options" % expected)
	return await close_camera_options(client)

func close_camera_options(client: Node) -> bool:
	if not await click_option(client, "OptionsDoneButton"):
		return false
	if client.get_node_or_null("GameMenuUI") != null:
		if not await click_option(client, "MenuBtnResume"):
			return false
	return await fixture.wait_menu_closed(client, null)

func set_camera_sliders(client: Node, look: float, fov: float) -> bool:
	if not await open_camera_options(client):
		return false
	if not await click_slider(client, "Sliderlook_sensitivity", (look - LOOK_MIN) / (LOOK_MAX - LOOK_MIN)):
		return false
	if not await click_slider(client, "Sliderfov_degrees", (fov - FOV_MIN) / (FOV_MAX - FOV_MIN)):
		return false
	if not expect_saved_number("look_sensitivity", look, 0.0000001) or not expect_saved_number("fovDegrees", fov, FOV_TOLERANCE):
		return false
	return await close_camera_options(client)

func expect_distance_bounds(client: Node) -> bool:
	if not expect_logical_distance(client, DISTANCE_DEFAULT, DISTANCE_NUMERIC_TOLERANCE):
		return false
	# Max first: configure clamps both logical distance and target from 15
	# to 12 before Min changes. No physical/collision-adjusted pose oracle.
	if not await set_distance_sliders(client, DISTANCE_MIN, DISTANCE_MAX, true):
		return false
	if not expect_logical_distance(client, DISTANCE_MAX, DISTANCE_NUMERIC_TOLERANCE):
		return false
	if not await expect_wheel_distance(client, MOUSE_BUTTON_WHEEL_UP, 3.0, DISTANCE_MIN, DISTANCE_MIN, DISTANCE_MAX):
		return false
	if not await expect_wheel_distance(client, MOUSE_BUTTON_WHEEL_DOWN, 3.0, DISTANCE_MAX, DISTANCE_MIN, DISTANCE_MAX):
		return false
	if not await set_distance_sliders(client, DISTANCE_MIN_DEFAULT, DISTANCE_MAX_DEFAULT):
		return false
	# Target remains exactly 12 after settling; continuous wheel factor 1.5
	# adds 1.5 * shared step 2 = 3, restoring the independent default 15.
	var restore_factor := (DISTANCE_DEFAULT - DISTANCE_MAX) / SCROLL_DISTANCE_STEP
	if not await expect_wheel_distance(client, MOUSE_BUTTON_WHEEL_DOWN, restore_factor, DISTANCE_DEFAULT, DISTANCE_MIN_DEFAULT, DISTANCE_MAX_DEFAULT):
		return false
	print("PASS: authored Max/Min clamp logical distance; real wheel reaches both bounds; bounds 2/40 and logical default 15 restored (not physical pose or zoom/follow rate proof)")
	return true

func set_distance_sliders(client: Node, minimum: float, maximum: float, expect_max_clamp: bool = false) -> bool:
	if not await open_camera_options(client):
		return false
	if not await click_slider(client, "Slidermax_distance", (maximum - DISTANCE_MAX_RANGE.x) / (DISTANCE_MAX_RANGE.y - DISTANCE_MAX_RANGE.x)):
		return false
	if not expect_saved_number("max_distance", maximum, DISTANCE_NUMERIC_TOLERANCE):
		return false
	if expect_max_clamp and not expect_logical_distance(client, DISTANCE_MAX, DISTANCE_NUMERIC_TOLERANCE):
		return false
	if not await click_slider(client, "Slidermin_distance", (minimum - DISTANCE_MIN_RANGE.x) / (DISTANCE_MIN_RANGE.y - DISTANCE_MIN_RANGE.x)):
		return false
	if not expect_saved_number("max_distance", maximum, DISTANCE_NUMERIC_TOLERANCE) or not expect_saved_number("min_distance", minimum, DISTANCE_NUMERIC_TOLERANCE):
		return false
	return await close_camera_options(client)

func expect_logical_distance(client: Node, expected: float, tolerance: float) -> bool:
	var observed := float(client.account_state().camera_distance)
	if not is_finite(observed) or absf(observed - expected) > tolerance:
		return reject("Logical camera distance expected=%s observed=%s tolerance=%s" % [expected, observed, tolerance])
	print("CAMERA_DISTANCE_PROBE expected=", expected, " logical=", observed)
	return true

func expect_wheel_distance(client: Node, button: int, factor: float, expected: float, minimum: float, maximum: float) -> bool:
	if client.get_node_or_null("GameMenuUI") != null or not client.account_state().gameplay_input_allowed:
		return reject("Logical wheel probe requires menu-closed gameplay input")
	var previous := float(client.account_state().camera_distance)
	if not is_finite(previous) or previous < minimum - DISTANCE_NUMERIC_TOLERANCE or previous > maximum + DISTANCE_NUMERIC_TOLERANCE:
		return reject("Logical wheel baseline outside finite bounds: " + str(previous))
	var samples: Array[float] = [previous]
	# Real input only: native PhysicalInput consumes signed factor on press.
	# Release the wheel as well; never mutate camera/state or fake its pose.
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = button
		event.factor = factor
		event.position = LOOK_POINT
		event.global_position = LOOK_POINT
		event.pressed = pressed
		fixture.root.push_input(event, true)
	for frame in range(DISTANCE_SETTLE_FRAMES):
		await fixture.process_frame
		var observed := float(client.account_state().camera_distance)
		if not is_finite(observed) or observed < minimum - DISTANCE_NUMERIC_TOLERANCE or observed > maximum + DISTANCE_NUMERIC_TOLERANCE:
			return reject("Logical wheel sample outside finite bounds: frame=%s distance=%s bounds=%s..%s" % [frame, observed, minimum, maximum])
		var wrong_direction := observed > previous + DISTANCE_NUMERIC_TOLERANCE if button == MOUSE_BUTTON_WHEEL_UP else observed < previous - DISTANCE_NUMERIC_TOLERANCE
		if wrong_direction:
			return reject("Logical wheel sample not monotonic: frame=%s previous=%s observed=%s" % [frame, previous, observed])
		samples.append(observed)
		previous = observed
	if not expect_logical_distance(client, expected, DISTANCE_ENDPOINT_TOLERANCE):
		return false
	print("CAMERA_WHEEL_DISTANCE_PROBE button=", button, " factor=", factor, " expected=", expected, " bounds=", Vector2(minimum, maximum), " samples=", samples)
	return true

func click_slider(client: Node, name: String, fraction: float) -> bool:
	var menu := client.get_node_or_null("GameMenuUI")
	var slider := menu.find_child(name, true, false) as Control if menu != null else null
	if slider == null or not slider.is_visible_in_tree():
		return reject("Authored camera slider absent/hidden: " + name)
	# Same physical click geometry as world_sound_flow; no readback calibration.
	var rect := slider.get_global_rect()
	var point := rect.position + Vector2(rect.size.x * fraction, rect.size.y * 0.5)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = point
		event.global_position = point
		event.pressed = pressed
		fixture.root.push_input(event, true)
		await fixture.process_frame
	return true

func expect_saved_number(field: String, expected: float, tolerance: float) -> bool:
	var config := OS.get_environment("XDG_CONFIG_HOME")
	if config.is_empty():
		return reject("Camera probe requires fixture-owned XDG_CONFIG_HOME")
	var path := config.path_join("world-of-osso/options_settings.ron")
	if not FileAccess.file_exists(path):
		return reject("Authored camera slider did not save canonical settings: " + path)
	var pattern := RegEx.new()
	var error := pattern.compile("\\b%s\\s*:\\s*([-+]?(?:[0-9]+(?:\\.[0-9]*)?|\\.[0-9]+)(?:[eE][-+]?[0-9]+)?)\\s*[,)]" % field)
	if error != OK:
		return reject("Camera numeric RON pattern failed: " + str(error))
	var matched := pattern.search(FileAccess.get_file_as_string(path))
	if matched == null:
		return reject("Canonical camera numeric field absent/invalid: " + field)
	var observed := float(matched.get_string(1))
	if not is_finite(observed) or absf(observed - expected) > tolerance:
		return reject("Saved %s expected=%s observed=%s tolerance=%s" % [field, expected, observed, tolerance])
	print("CAMERA_SAVED_PROBE field=", field, " requested=", expected, " observed=", observed)
	return true

func expect_physical_fov(client: Node, expected: float) -> bool:
	var camera := client.get_node_or_null("WorldCamera") as Camera3D
	if camera == null or not is_finite(camera.fov) or absf(camera.fov - expected) > FOV_TOLERANCE:
		return reject("Physical WorldCamera FOV expected=%s observed=%s" % [expected, camera.fov if camera != null else "missing"])
	print("CAMERA_FOV_PROBE requested=", expected, " physical=", camera.fov)
	return true

func expect_vertical_look(client: Node, inverted: bool, expected_delta: float = NAN) -> bool:
	var state: Dictionary = client.account_state()
	if not state.gameplay_input_allowed:
		return reject("Resumed camera probe has gameplay input blocked")
	var before := float(state.camera_pitch)
	# Fresh fixture starts at -0.3 radians. Small motion keeps both cases far
	# from the original +/-88-degree clamp; never reposition the camera.
	if not is_finite(before) or absf(before) >= 1.0:
		return reject("Camera probe pitch is not safely inside clamp: " + str(before))
	push_right(true)
	await fixture.process_frame
	push_vertical_motion()
	for frame in range(3):
		await fixture.process_frame
	push_right(false)
	for frame in range(3):
		await fixture.process_frame
	var after := float(client.account_state().camera_pitch)
	var delta := after - before
	# Original apply_camera_input contract: positive relative Y subtracts
	# pitch with invert false, adds with true. No native-derived golden.
	if not is_finite(after) or absf(after) >= 1.0 or (delta <= 0.0 if inverted else delta >= 0.0):
		return reject("invert_y=%s: positive relative Y expected %s pitch delta, observed before=%s after=%s delta=%s" % [inverted, "positive" if inverted else "negative", before, after, delta])
	if is_finite(expected_delta) and absf(delta - expected_delta) > PITCH_TOLERANCE:
		return reject("RMB vertical 4px pitch expected=%s observed=%s tolerance=%s" % [expected_delta, delta, PITCH_TOLERANCE])
	# Same motion without RMB must not rotate: exercises actual release,
	# rather than trusting that the synthetic release was consumed.
	push_vertical_motion()
	for frame in range(3):
		await fixture.process_frame
	var released := float(client.account_state().camera_pitch)
	if not is_finite(released) or absf(released - after) > 0.000001:
		return reject("Released RMB still changes gameplay pitch: %s -> %s" % [after, released])
	print("CAMERA_INVERT_PROBE invert_y=", inverted, " before=", before, " after=", after, " delta=", delta)
	return true

func push_right(pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_RIGHT
	event.position = LOOK_POINT
	event.pressed = pressed
	fixture.root.push_input(event, true)

func push_vertical_motion() -> void:
	var event := InputEventMouseMotion.new()
	event.position = LOOK_POINT + VERTICAL_MOTION
	event.relative = VERTICAL_MOTION
	fixture.root.push_input(event, true)

func click_option(client: Node, name: String) -> bool:
	var menu := client.get_node_or_null("GameMenuUI")
	var control := menu.find_child(name, true, false) as Control if menu != null else null
	if control == null or not control.is_visible_in_tree():
		return reject("Authored camera Options control absent/hidden: " + name)
	await fixture.click(control)
	return true

func tap_escape() -> void:
	fixture.push_key(KEY_ESCAPE, true)
	await fixture.process_frame
	fixture.push_key(KEY_ESCAPE, false)
	for frame in range(3):
		await fixture.process_frame

func reject(message: String) -> bool:
	fixture.fail(message)
	return false
