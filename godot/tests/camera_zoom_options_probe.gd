extends RefCounted

# Independent originals: CameraState::default, authored zoom_speed slider,
# apply_camera_input's wheel step 2, and camera_follow_data::follow_camera.
const SPEED_MIN := 2.0
const SPEED_MAX := 20.0
const SPEED_DEFAULT := 8.0
const DISTANCE_DEFAULT := 15.0
const WHEEL_TARGET := 17.0
const SAVED_TOLERANCE := 0.00001
const RATE_TOLERANCE := 0.0001
const SETTLE_TOLERANCE := 0.01
const SAMPLE_FRAMES := 24
const SETTLE_FRAMES := 360
const WHEEL_POINT := Vector2(640, 360)

var fixture
var options

func run(flow: SceneTree, client: Node, camera_options: RefCounted) -> bool:
	fixture = flow
	options = camera_options
	if not gameplay_ready(client):
		return false
	var defaults := {"zoom_speed": SPEED_DEFAULT, "min_distance": 2.0, "max_distance": 40.0}
	for field in defaults:
		if not options.expect_saved_number(field, defaults[field], SAVED_TOLERANCE):
			return false
	if not options.expect_logical_distance(client, DISTANCE_DEFAULT, SETTLE_TOLERANCE):
		return false
	for speed in [SPEED_MIN, SPEED_MAX]:
		if not await set_zoom_speed(client, speed):
			return false
		if not await expect_logical_rate(client, speed):
			return false
		if not await restore_distance(client):
			return false
	if not await set_zoom_speed(client, SPEED_DEFAULT):
		return false
	if not options.expect_logical_distance(client, DISTANCE_DEFAULT, SETTLE_TOLERANCE):
		return false
	if not gameplay_ready(client):
		return false
	print("PASS: authored Zoom Speed 2/20 canonical saves and consecutive post-draw logical f32 histories; target/distance 15 and speed 8 restored (no physical/collision/Follow Rate proof)")
	return true

func set_zoom_speed(client: Node, speed: float) -> bool:
	# Reuse real authored hitboxes, captured endpoint drag, and target-aware Escape.
	if not await options.open_camera_options(client):
		return false
	if not await options.click_slider(client, "Sliderzoom_speed", (speed - SPEED_MIN) / (SPEED_MAX - SPEED_MIN)):
		return false
	if not options.expect_saved_number("zoom_speed", speed, SAVED_TOLERANCE):
		return false
	return await options.close_camera_options(client)

func expect_logical_rate(client: Node, speed: float) -> bool:
	if not gameplay_ready(client):
		return false
	# SceneTree.process_frame precedes native process callbacks. Start AFTER
	# post-draw, then inject once; no shifted observations or skipped frames.
	await RenderingServer.frame_post_draw
	var previous_frame := Engine.get_process_frames()
	var origin := float(client.account_state().camera_distance)
	if not is_finite(origin) or absf(origin - DISTANCE_DEFAULT) > SETTLE_TOLERANCE:
		return reject("Zoom rate baseline not fresh logical 15: " + str(origin))
	var predicted := f32(origin)
	print("CAMERA_ZOOM_PROBE ", JSON.stringify({"phase": "baseline", "speed": speed, "frame": previous_frame, "origin": origin, "target": WHEEL_TARGET, "tolerance": RATE_TOLERANCE}))
	push_wheel(MOUSE_BUTTON_WHEEL_DOWN)
	for sample in range(SAMPLE_FRAMES):
		await RenderingServer.frame_post_draw
		var frame := Engine.get_process_frames()
		var dt := float(client.get_process_delta_time())
		var actual := float(client.account_state().camera_distance)
		if frame - previous_frame != 1 or not is_finite(dt) or dt <= 0.0 or not is_finite(f32(dt)) or f32(dt) <= 0.0:
			print("CAMERA_ZOOM_PROBE ", JSON.stringify({"phase": "clock_failure", "speed": speed, "sample": sample, "previous_frame": previous_frame, "frame": frame, "dt": dt, "actual": actual}))
			return reject("Zoom rate clock inconsistent; require consecutive process frames and positive finite delta, without observation shifts")
		predicted = predict_distance(predicted, speed, dt, WHEEL_TARGET)
		print("CAMERA_ZOOM_PROBE ", JSON.stringify({"phase": "sample", "speed": speed, "sample": sample, "frame": frame, "dt": dt, "dt_f32": f32(dt), "predicted": predicted, "actual": actual}))
		if not is_finite(actual) or absf(actual - predicted) > RATE_TOLERANCE:
			return reject("Zoom logical rate mismatch speed=%s frame=%s dt=%s predicted=%s actual=%s fixed_tolerance=%s" % [speed, frame, dt, predicted, actual, RATE_TOLERANCE])
		# Only oracle-owned history advances; never feed actual back into it.
		previous_frame = frame
	return true

func predict_distance(previous: float, speed: float, dt: float, target: float) -> float:
	# glam FloatExt::lerp is self + (rhs - self) * t, not a weighted sum.
	# Native process casts delta to f32 before speed * delta; round EVERY op.
	var zoom_t := minf(f32(f32(speed) * f32(dt)), f32(1.0))
	var difference := f32(f32(target) - previous)
	var step := f32(difference * zoom_t)
	return f32(previous + step)

func f32(value: float) -> float:
	return PackedFloat32Array([value])[0]

func restore_distance(client: Node) -> bool:
	if not gameplay_ready(client):
		return false
	# Last rate observation is already post-draw. Wheel Up factor 1 changes
	# independent target 17 back to 15 under the CURRENT speed, no reset.
	push_wheel(MOUSE_BUTTON_WHEEL_UP)
	var previous_frame := Engine.get_process_frames()
	for sample in range(SETTLE_FRAMES):
		await RenderingServer.frame_post_draw
		var frame := Engine.get_process_frames()
		var dt := float(client.get_process_delta_time())
		var actual := float(client.account_state().camera_distance)
		if frame - previous_frame != 1 or not is_finite(dt) or dt <= 0.0 or not is_finite(actual):
			return reject("Zoom restore clock/state inconsistent frame=%s previous=%s dt=%s actual=%s" % [frame, previous_frame, dt, actual])
		if absf(actual - DISTANCE_DEFAULT) <= SETTLE_TOLERANCE:
			print("CAMERA_ZOOM_PROBE ", JSON.stringify({"phase": "restored", "frame": frame, "frames": sample + 1, "target": DISTANCE_DEFAULT, "actual": actual}))
			return true
		previous_frame = frame
	return reject("Zoom restore did not settle to logical 15 within %s post-draw frames" % SETTLE_FRAMES)

func push_wheel(button: int) -> void:
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = button
		event.factor = 1.0
		event.position = WHEEL_POINT
		event.global_position = WHEEL_POINT
		event.pressed = pressed
		fixture.root.push_input(event, true)

func gameplay_ready(client: Node) -> bool:
	if client.get_node_or_null("GameMenuUI") != null or client.target_state().target != null or not client.account_state().gameplay_input_allowed:
		return reject("Zoom probe requires menu-closed/target-none gameplay")
	return true

func reject(message: String) -> bool:
	fixture.fail(message)
	return false
