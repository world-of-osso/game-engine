extends SceneTree

# Dev-server regression: the follow camera must track the local player every frame while held W
# moves it, not only after movement stops. Covers an unobstructed run and a run during which a
# real physics obstruction brushes the camera for one frame (as WMOs, doodads and terrain do).
# Requires GODOT_TEST_SERVER=127.0.0.1:5000 (admin/admin); GODOT_TEST_CARD selects the roster card.
# Requires --fixed-fps 60 so process delta matches a real 60 Hz display at any headless speed.
const HOLD_FRAMES := 120
const WORLD_WAIT_MS := 120000
const SETTLE_LIMIT_FRAMES := 600
# Brush once the run reaches steady follow lag, leaving frames to observe the recovery.
const BRUSH_FRAME := 40
# Follow smoothing trails the orbit point by about run speed / follow speed (7 / 10 m); a camera
# that stops following falls behind by the whole run distance.
const MAX_FOLLOW_LAG := 1.5
const MIN_RUN_DISTANCE := 3.0

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	await process_frame
	if not is_equal_approx(root.get_process_delta_time(), 1.0 / 60.0):
		fail("Run with --fixed-fps 60 so frame delta matches a 60 Hz display")
		return
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5000":
		fail("GODOT_TEST_SERVER must explicitly select 127.0.0.1:5000")
		return
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, "admin", "admin", false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await wait_for_character_select(client):
		return
	var card := OS.get_environment("GODOT_TEST_CARD")
	var ui = client.get_node("CharacterSelectUI")
	await click_control(ui.find_child("CharCard_" + (card if card != "" else "0"), true, false))
	await click_control(ui.find_child("EnterWorld", true, false))
	var player := await wait_for_world(client)
	if player == null:
		return
	var camera := client.get_node("WorldCamera") as Camera3D
	var rest_distance := await settle_camera(player, camera)
	if rest_distance < 0.0:
		return
	if not await run_and_track(client, player, camera, rest_distance, "OPEN", false):
		return
	rest_distance = await settle_camera(player, camera)
	if rest_distance < 0.0:
		return
	if not await run_and_track(client, player, camera, rest_distance, "BRUSHED", true):
		return
	print("PASS: world camera tracks the moving local player every frame, open and after a brief obstruction")
	client.free()
	quit(0)

func eye(player: Node3D) -> Vector3:
	return player.global_position + Vector3.UP * 1.8

func settle_camera(player: Node3D, camera: Camera3D) -> float:
	var last := camera.global_position
	for _frame in range(SETTLE_LIMIT_FRAMES):
		await process_frame
		if camera.global_position.distance_to(last) < 0.0005:
			var distance := camera.global_position.distance_to(eye(player))
			print("REST camera distance %.3f" % distance)
			return distance
		last = camera.global_position
	fail("World camera did not settle behind the idle player")
	return -1.0

# One frame of real physics obstruction just inside the running camera's orbit, as a wall, doodad
# or terrain lip passing between player and camera does.
func brush_camera(client: Node3D, player: Node3D, camera: Camera3D) -> void:
	var orbit := (eye(player) - camera.global_position).normalized()
	var distance := camera.global_position.distance_to(eye(player))
	var mesh := MeshInstance3D.new()
	mesh.mesh = BoxMesh.new()
	var body := StaticBody3D.new()
	var shape := CollisionShape3D.new()
	shape.shape = BoxShape3D.new()
	body.add_child(shape)
	mesh.add_child(body)
	client.add_child(mesh)
	mesh.global_position = eye(player) - orbit * (distance - 2.0)
	await physics_frame
	await process_frame
	mesh.free()
	print("BRUSHED camera distance %.3f" % camera.global_position.distance_to(eye(player)))

func run_and_track(client: Node3D, player: Node3D, camera: Camera3D, rest_distance: float, phase: String, brush: bool) -> bool:
	var start := player.global_position
	var model := player.find_child("PlayerModel", true, false) as Node3D
	var skeleton := model.find_child("Skeleton3D", true, false) as Node3D if model != null else null
	var last_player := player.global_position
	var last_camera := camera.global_position
	var max_lag := 0.0
	var moving_frames := 0
	var frozen_moving_frames := 0
	push_key(KEY_W, true)
	for frame in range(1, HOLD_FRAMES + 1):
		if brush and frame == BRUSH_FRAME:
			await brush_camera(client, player, camera)
		else:
			await process_frame
		var player_position := player.global_position
		var camera_position := camera.global_position
		var lag := camera_position.distance_to(eye(player)) - rest_distance
		max_lag = maxf(max_lag, lag)
		var player_step := player_position.distance_to(last_player)
		var camera_step := camera_position.distance_to(last_camera)
		if player_step > 0.01:
			moving_frames += 1
			if camera_step < player_step * 0.1:
				frozen_moving_frames += 1
		print("%s %d player=%s model=%s skeleton=%s camera=%s player_step=%.4f camera_step=%.4f lag=%.3f" % [
			phase, frame, player_position,
			model.global_position if model != null else Vector3.ZERO,
			skeleton.global_position if skeleton != null else Vector3.ZERO,
			camera_position, player_step, camera_step, lag])
		last_player = player_position
		last_camera = camera_position
	push_key(KEY_W, false)
	var ran := player.global_position.distance_to(start)
	print("SUMMARY %s moving=%d frozen_while_moving=%d ran=%.3f max_lag=%.3f" % [phase, moving_frames, frozen_moving_frames, ran, max_lag])
	if ran < MIN_RUN_DISTANCE:
		fail("%s: held W did not move the local player: ran %.3f" % [phase, ran])
		return false
	if frozen_moving_frames > 0:
		fail("%s: world camera stood still on %d of %d frames the player moved" % [phase, frozen_moving_frames, moving_frames])
		return false
	if max_lag > MAX_FOLLOW_LAG:
		fail("%s: world camera fell %.3f m behind its orbit distance (bound %.1f)" % [phase, max_lag, MAX_FOLLOW_LAG])
		return false
	return true

func wait_for_character_select(client: Node) -> bool:
	var deadline := Time.get_ticks_msec() + 15000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count > 0:
			return true
	fail("Timed out waiting for character selection: " + str(client.account_state()))
	return false

func wait_for_world(client: Node) -> Node3D:
	var deadline := Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen != "InWorld" or state.terrain.pending_count != 0 or state.terrain.parsed_tiles.is_empty():
			continue
		var player := client.get_node_or_null("WorldUnits/" + str(state.selected_character_name)) as Node3D
		var camera := client.get_node_or_null("WorldCamera") as Camera3D
		if player == null or camera == null or not camera.current:
			continue
		print("WORLD READY player=", state.selected_character_name, " at ", player.global_position)
		return player
	fail("Timed out waiting for world: " + str(client.account_state()))
	return null

func push_key(keycode: Key, pressed: bool) -> void:
	var key := InputEventKey.new()
	key.physical_keycode = keycode
	key.pressed = pressed
	root.push_input(key, true)

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
