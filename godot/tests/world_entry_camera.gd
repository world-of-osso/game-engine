extends SceneTree

## Live regression: the world on entry. The character stands at Northshire Abbey (WoW
## -8966.63, -194.0, 80.0, set with `game-server-admin set-position` while offline), on
## azeroth_32_48 next to the Northshire oak (MDDF 10452).
## 1. The first rendered in-world frame already shows the center tile's objects: the oak and
##    the abbey WMO are attached, the oak with its M2 collision body. Before, loading ended
##    with ~10% of the tile's placements attached and an empty valley on screen.
## 2. That frame shows the camera on its 15 yd orbit from the eye, not flying in from the
##    world origin, and no doodad collision lies between the eye and the camera.
## 3. A doodad collision body added after the camera settled, across its orbit, pulls the
##    camera in front of it on the next frame (objects off the center tile still stream in
##    after loading).
## Requires GODOT_TEST_SERVER, ENTRY_CAMERA_ACCOUNT, ENTRY_CAMERA_PASSWORD and
## ENTRY_CAMERA_CHARACTER (card 0). ENTRY_CAMERA_SHOTS names a directory for screenshots.
const CAMERA_DISTANCE := 15.0
const EYE_HEIGHT := 1.8
const DOODAD_LAYER := 8
const TOLERANCE := 0.5
const OAK := "Doodad10452"
const SETTLE_LIMIT_FRAMES := 600

var NAME := OS.get_environment("ENTRY_CAMERA_CHARACTER")
var client: Node3D

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("ENTRY_CAMERA_ACCOUNT")
	var password := OS.get_environment("ENTRY_CAMERA_PASSWORD")
	if server == "" or account == "" or password == "" or NAME == "":
		fail("GODOT_TEST_SERVER, ENTRY_CAMERA_ACCOUNT, ENTRY_CAMERA_PASSWORD and ENTRY_CAMERA_CHARACTER are required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, password, false)
	if error != "":
		fail("connect: " + error)
		return
	if not await select_and_enter():
		return
	if not await first_frame():
		return
	if not await late_obstacle_pulls_camera_in():
		return
	print("PASS: center tile objects and an on-orbit camera from the first in-world frame; a late doodad body pulls the camera in")
	client.free()
	quit(0)

## Waits for the first in-world frame to be drawn and checks what it was drawn with.
func first_frame() -> bool:
	var loading_started := -1
	var deadline := Time.get_ticks_msec() + 300000
	while Time.get_ticks_msec() < deadline:
		await RenderingServer.frame_post_draw
		var state: Dictionary = client.account_state()
		if state.screen == "Loading" and loading_started < 0:
			loading_started = Time.get_ticks_msec()
		if state.screen != "InWorld":
			continue
		print("LOADING_MS ", Time.get_ticks_msec() - loading_started)
		await capture("first-frame")
		var camera := client.get_node_or_null("WorldCamera") as Camera3D
		var player := player_node()
		if camera == null or player == null:
			fail("InWorld frame without camera or player: " + str(state))
			return false
		var oak := client.find_child(OAK, true, false) as Node3D
		var wmos := client.find_children("Wmo*", "Node3D", true, false).filter(
			func(node): return node.global_position.distance_to(player.global_position) < 100.0)
		var eye := player.global_position + Vector3.UP * EYE_HEIGHT
		var expected := orbit_point(eye, state.camera_yaw, state.camera_pitch, CAMERA_DISTANCE)
		var offset := camera.global_position.distance_to(expected)
		print("FIRST_FRAME eye=%s camera=%s expected=%s offset=%.2f oak=%s wmos=%s objects=%s" % [eye, camera.global_position, expected, offset, oak != null, wmos.map(func(node): return node.name), state.world_objects])
		if oak == null or oak.get_node_or_null("M2Collision") == null:
			fail("First in-world frame lacks the oak or its collision body")
			return false
		if wmos.is_empty():
			fail("First in-world frame lacks the abbey WMO")
			return false
		if offset > TOLERANCE:
			fail("First in-world frame camera %.2f yd off its %.0f yd orbit" % [offset, CAMERA_DISTANCE])
			return false
		if doodad_between(eye, camera.global_position):
			fail("First in-world frame camera behind doodad collision")
			return false
		return true
	fail("Timed out waiting for the world: " + str(client.account_state()))
	return false

## A 4 x 4 x 0.2 yd slab on the doodad layer, across the orbit 8 yd from the eye.
func late_obstacle_pulls_camera_in() -> bool:
	var camera := client.get_node("WorldCamera") as Camera3D
	if not await settle(camera):
		return false
	var eye := player_node().global_position + Vector3.UP * EYE_HEIGHT
	var before := camera.global_position.distance_to(eye)
	var direction := (camera.global_position - eye).normalized()
	var box := BoxShape3D.new()
	box.size = Vector3(4.0, 4.0, 0.2)
	var shape := CollisionShape3D.new()
	shape.shape = box
	var body := StaticBody3D.new()
	body.name = "LateObstacle"
	body.collision_layer = DOODAD_LAYER
	body.collision_mask = 0
	body.add_child(shape)
	client.add_child(body)
	body.look_at_from_position(eye + direction * 8.0, eye)
	await RenderingServer.frame_post_draw
	var after := camera.global_position.distance_to(eye)
	var blocked := doodad_between(eye, camera.global_position)
	print("LATE_OBSTACLE before=%.2f after=%.2f doodad_between=%s" % [before, after, blocked])
	await capture("late-obstacle")
	if blocked or after > 8.0:
		fail("Late doodad body did not pull the camera in on the next frame: %.2f yd" % after)
		return false
	return true

## The follow camera's position: `eye - orbit_dir * distance`, with orbit_dir the
## `Quat::from_euler(YXZ, yaw, pitch, 0) * -Z` of `camera_follow_data::follow_camera`.
func orbit_point(eye: Vector3, yaw: float, pitch: float, distance: float) -> Vector3:
	var direction := Basis.from_euler(Vector3(pitch, yaw, 0.0), EULER_ORDER_YXZ) * Vector3.FORWARD
	return eye - direction * distance

func doodad_between(eye: Vector3, point: Vector3) -> bool:
	var query := PhysicsRayQueryParameters3D.create(eye, point, DOODAD_LAYER)
	query.hit_back_faces = true
	return not client.get_world_3d().direct_space_state.intersect_ray(query).is_empty()

func player_node() -> Node3D:
	return client.get_node_or_null("WorldUnits/" + NAME) as Node3D

func select_and_enter() -> bool:
	var ui = null
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		ui = client.get_node_or_null("CharacterSelectUI")
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 and ui != null:
			break
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await click_control(ui.find_child("CharCard_0", true, false))
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != NAME:
		fail("Card 0 is %s, not %s" % [selected.text, NAME])
		return false
	await click_control(ui.find_child("EnterWorld", true, false))
	return true

## Waits until the camera and the player (the server corrects the spawn height) are still.
func settle(camera: Camera3D) -> bool:
	var last := [camera.global_position, player_node().global_position]
	var still := 0
	for _frame in SETTLE_LIMIT_FRAMES:
		await process_frame
		var now := [camera.global_position, player_node().global_position]
		still = still + 1 if now[0].distance_to(last[0]) < 0.0005 and now[1].distance_to(last[1]) < 0.0005 else 0
		if still >= 30:
			return true
		last = now
	fail("Camera and player did not settle")
	return false

## The frame last drawn.
func capture(label: String) -> void:
	var dir := OS.get_environment("ENTRY_CAMERA_SHOTS")
	if dir == "":
		return
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
