extends SceneTree

## Dev-server regression: WMO walls bound the follow camera. The character stands in a WMO
## interior (placed there with `game-server-admin` while offline); the camera turns a full circle
## in eight real turn-key steps. At every step the camera must stay where the eye still sees it
## (no WMO collision face between them), and where the 15 yd orbit would pass a wall the walls
## must pull it in. Before WMO physics bodies existed the camera sat at its full orbit behind the
## walls, showing the room from outside.
## Requires GODOT_TEST_SERVER=127.0.0.1:5000, WMO_CAMERA_ACCOUNT, WMO_CAMERA_PASSWORD,
## WMO_CAMERA_CHARACTER (card 0), and --fixed-fps 60. WMO_CAMERA_SHOTS names a directory for one
## screenshot per step.
const WORLD_WAIT_MS := 180000
const SETTLE_LIMIT_FRAMES := 600
const STEPS := 8
## KEY_ROTATE_SPEED 2.5 rad/s: 19 frames at 60 Hz turn the camera about 45°.
const TURN_FRAMES := 19
const WMO_LAYER := 2
const EYE_HEIGHT := 1.8

var NAME := OS.get_environment("WMO_CAMERA_CHARACTER")

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	await process_frame
	if not is_equal_approx(root.get_process_delta_time(), 1.0 / 60.0):
		fail("Run with --fixed-fps 60 so turn steps are 45°")
		return
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5000":
		fail("GODOT_TEST_SERVER must explicitly select 127.0.0.1:5000")
		return
	var account := OS.get_environment("WMO_CAMERA_ACCOUNT")
	var password := OS.get_environment("WMO_CAMERA_PASSWORD")
	if account == "" or password == "" or NAME == "":
		fail("WMO_CAMERA_ACCOUNT, WMO_CAMERA_PASSWORD and WMO_CAMERA_CHARACTER are required")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, password, false)
	if error != "":
		fail("connect: " + error)
		return
	var player := await enter_world(client)
	if player == null:
		return
	var camera := client.get_node("WorldCamera") as Camera3D
	var walls: Node = client.get_node_or_null("WmoCollision")
	print("WMO collision bodies: ", walls.get_child_count() if walls != null else 0)
	var distances := []
	var obstructed := []
	for step in STEPS:
		if step > 0:
			await hold_key(KEY_LEFT, TURN_FRAMES)
		if not await settle(camera):
			return
		var eye := player.global_position + Vector3.UP * EYE_HEIGHT
		var distance := camera.global_position.distance_to(eye)
		var wall := wall_between(client, eye, camera.global_position)
		print("STEP %d player=%s camera=%s distance=%.2f wall_between=%s" % [step, player.global_position, camera.global_position, distance, wall])
		distances.append(distance)
		if wall:
			obstructed.append(step)
		await capture("step%d" % step)
	var eye_now := player.global_position + Vector3.UP * EYE_HEIGHT
	for mask in [1, 1 | WMO_LAYER]:
		print("RAY %.1f us per camera ray, mask %d" % [ray_cost(client, eye_now, camera.global_position, mask), mask])
	# Paired frame-rate cost of the WMO bodies at the last pose: with, detached, with again.
	await measure_fps("with WMO bodies")
	if walls != null:
		client.remove_child(walls)
		await measure_fps("without WMO bodies")
		client.add_child(walls)
		await measure_fps("with WMO bodies")
	var longest: float = distances.max()
	var shortest: float = distances.min()
	print("SUMMARY distances=", distances, " obstructed=", obstructed)
	if walls == null or walls.get_child_count() == 0:
		fail("No WMO collision bodies in the world")
		return
	if not obstructed.is_empty():
		fail("Camera behind a WMO wall at steps %s" % [obstructed])
		return
	if shortest > longest - 1.0:
		fail("No step pulled the camera in: %s" % [distances])
		return
	print("PASS: WMO walls keep the camera in sight of the player at every yaw")
	client.free()
	quit(0)

## Uncapped wall-clock frame rate at the last pose.
func measure_fps(label: String) -> void:
	Engine.max_fps = 0
	for _frame in 60:
		await process_frame
	var start := Time.get_ticks_usec()
	for _frame in 300:
		await process_frame
	var fps := 300.0 / ((Time.get_ticks_usec() - start) / 1000000.0)
	print("FPS %.1f over 300 frames %s" % [fps, label])
	Engine.max_fps = 60

## Mean wall-clock cost of one camera-length ray query against `mask`.
func ray_cost(client: Node3D, from: Vector3, to: Vector3, mask: int) -> float:
	var space := client.get_world_3d().direct_space_state
	var query := PhysicsRayQueryParameters3D.create(from, to, mask)
	var start := Time.get_ticks_usec()
	for _ray in 2000:
		space.intersect_ray(query)
	return (Time.get_ticks_usec() - start) / 2000.0

func wall_between(client: Node3D, eye: Vector3, camera: Vector3) -> bool:
	var query := PhysicsRayQueryParameters3D.create(eye, camera, WMO_LAYER)
	return not client.get_world_3d().direct_space_state.intersect_ray(query).is_empty()

func enter_world(client: Node) -> Node3D:
	var deadline := Time.get_ticks_msec() + 20000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "CharacterSelect" and state.reply_received:
			break
	var ui = client.get_node("CharacterSelectUI")
	await click_control(ui.find_child("CharCard_0", true, false))
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != NAME:
		fail("Card 0 is %s, not %s" % [selected.text, NAME])
		return null
	await click_control(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen != "InWorld" or state.terrain.pending_count != 0:
			continue
		var player := client.get_node_or_null("WorldUnits/" + NAME) as Node3D
		var camera := client.get_node_or_null("WorldCamera") as Camera3D
		if player == null or camera == null or not camera.current:
			continue
		# Let the physics bodies register and the first server snapshots arrive.
		for _frame in 60:
			await process_frame
		print("WORLD READY map=", state.terrain.map, " player=", player.global_position)
		return player
	fail("Timed out waiting for world: " + str(client.account_state()))
	return null

func settle(camera: Camera3D) -> bool:
	var last := camera.global_position
	for _frame in SETTLE_LIMIT_FRAMES:
		await process_frame
		if camera.global_position.distance_to(last) < 0.0005:
			return true
		last = camera.global_position
	fail("Camera did not settle")
	return false

func hold_key(keycode: Key, frames: int) -> void:
	push_key(keycode, true)
	for _frame in frames:
		await process_frame
	push_key(keycode, false)
	await process_frame

func push_key(keycode: Key, pressed: bool) -> void:
	var key := InputEventKey.new()
	key.physical_keycode = keycode
	key.pressed = pressed
	root.push_input(key, true)

func capture(label: String) -> void:
	var dir := OS.get_environment("WMO_CAMERA_SHOTS")
	if dir == "":
		return
	await RenderingServer.frame_post_draw
	var path := dir.path_join(label + ".png")
	root.get_texture().get_image().save_png(path)
	print("SHOT ", path)

func click_control(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
