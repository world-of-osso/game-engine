extends SceneTree

# Real-server account-scene cameras: left-drag orbits the character-select and
# character-creation cameras, and the creation camera buttons reset, zoom and rotate.

var client: Node

func _initialize() -> void:
	call_deferred("run")

func fail(message: String) -> void:
	push_error(message)
	client.queue_free()
	quit(1)

func frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func settle() -> void:
	await create_timer(1.5).timeout

func press(ui: Node, name: String) -> bool:
	var button = ui.find_child(name, true, false)
	if button == null:
		return false
	button.emit_signal("pressed")
	await frames(2)
	return true

func left_drag(relative: Vector2) -> void:
	var down := InputEventMouseButton.new()
	down.position = Vector2(640, 300)
	down.button_index = MOUSE_BUTTON_LEFT
	down.pressed = true
	root.push_input(down, true)
	var motion := InputEventMouseMotion.new()
	motion.position = down.position
	motion.relative = relative
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	root.push_input(motion, true)
	await frames(2)
	var up := InputEventMouseButton.new()
	up.position = down.position
	up.button_index = MOUSE_BUTTON_LEFT
	up.pressed = false
	root.push_input(up, true)
	await frames(2)

func wait_for(path: String) -> Node:
	var deadline = Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		var node = client.get_node_or_null(path)
		if node != null:
			return node
		await process_frame
	return null

func character_distance(camera: Camera3D) -> float:
	var character := client.get_node("CharacterCreateScene/CreationCharacter") as Node3D
	return camera.global_position.distance_to(character.global_position)

func run() -> void:
	root.size = Vector2i(1280, 720)
	var server = OS.get_environment("GODOT_TEST_SERVER")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if server.is_empty():
		fail("GODOT_TEST_SERVER must select a local server")
		return
	var error = client.connect_account(server, "admin", "admin", false)
	if error != "":
		fail(error)
		return
	if await wait_for("CharacterSelectScene/SelectedCharacter") == null:
		fail("Character selection must show the selected roster character")
		return
	var select_camera := client.get_node("CharacterSelectScene/Camera") as Camera3D
	await frames(5)
	var select_before := select_camera.global_position
	await left_drag(Vector2(60, 0))
	if select_before.distance_to(select_camera.global_position) < 0.05:
		fail("Left-drag must orbit the character-select camera")
		return
	await press(client.get_node("CharacterSelectUI"), "CreateChar")
	var camera := await wait_for("CharacterCreateScene/Camera") as Camera3D
	if camera == null or await wait_for("CharacterCreateScene/CreationCharacter") == null:
		fail("Creation must show its scene camera and character")
		return
	await settle()
	var home := camera.global_position
	await left_drag(Vector2(80, 0))
	if home.distance_to(camera.global_position) < 0.05:
		fail("Left-drag must orbit the creation camera on the race page")
		return
	var ui = client.get_node("CharacterCreateUI")
	await press(ui, "CharCreateNext")
	if not await press(ui, "Camera_reset"):
		fail("Customize must show the original camera controls")
		return
	await settle()
	if home.distance_to(camera.global_position) > 0.05:
		fail("Camera reset must restore the authored shot")
		return
	var distance := character_distance(camera)
	await press(ui, "Camera_zoom_in")
	await settle()
	if character_distance(camera) > distance - 0.3:
		fail("Zoom in must move the camera toward the character")
		return
	await press(ui, "Camera_zoom_out")
	await press(ui, "Camera_zoom_out")
	await settle()
	if character_distance(camera) < distance + 0.3:
		fail("Zoom out must move the camera away from the character")
		return
	var before_rotate := camera.global_position
	await press(ui, "Camera_rotate_left")
	await frames(2)
	var left := camera.global_position
	await press(ui, "Camera_rotate_right")
	await press(ui, "Camera_rotate_right")
	await frames(2)
	if before_rotate.distance_to(left) < 0.05 or left.distance_to(camera.global_position) < 0.05:
		fail("Rotate buttons must orbit the creation camera")
		return
	print("PASS: account camera drag on select/create and creation reset, zoom and rotate controls")
	client.queue_free()
	quit(0)
