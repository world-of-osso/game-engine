extends SceneTree

## Screenshots of the world around a character, for rendering comparisons against a
## private server. Environment:
##   GODOT_TEST_SERVER               server address (a private test server)
##   VIEW_ACCOUNT / VIEW_CHARACTER   account (password fbtest) and its first character
##   VIEW_SHOTS                      screenshot directory
##   VIEW_PLAN                       shots "name,minutes,yaw,pitch,distance,settle_s" joined
##                                   by ";": the time of day (0..2880), the camera orbit
##                                   (radians, yards) and seconds to let it settle
##   VIEW_HIDE_WMO_LIQUIDS           "1" hides WMO group liquids (Group*_Liquid)
##   VIEW_WAIT_LIQUIDS               "1" starts once a WMO liquid spawned, not every object
##   VIEW_WORLD_TIMEOUT_S            seconds to wait for the world's terrain and objects (300)

const PASSWORD := "fbtest"

var client: Node
var character := ""
var shots := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("VIEW_ACCOUNT")
	character = OS.get_environment("VIEW_CHARACTER")
	shots = OS.get_environment("VIEW_SHOTS")
	var plan := OS.get_environment("VIEW_PLAN")
	if server == "" or account == "" or character == "" or shots == "" or plan == "":
		fail("GODOT_TEST_SERVER, VIEW_ACCOUNT, VIEW_CHARACTER, VIEW_SHOTS and VIEW_PLAN are required")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	for entry in plan.split(";", false):
		var fields := entry.split(",")
		if fields.size() != 6:
			fail("Shot %s is not name,minutes,yaw,pitch,distance,settle_s" % entry)
			return
		client.set_world_minutes(float(fields[1]))
		client.set_camera_orbit(float(fields[2]), float(fields[3]), float(fields[4]))
		await wait_real(float(fields[5]))
		var hidden := hide_wmo_liquids() if OS.get_environment("VIEW_HIDE_WMO_LIQUIDS") == "1" else 0
		await wait_frames(3)
		var camera := client.get_node("WorldCamera") as Camera3D
		print("FIXTURE SHOT %s camera=%s liquids_hidden=%d wmo_liquids=%d nearest=%s" % [fields[0], camera.global_transform, hidden, wmo_liquids().size(), nearest_liquids(camera.global_position)])
		await capture(fields[0] + ".png")
	print("FIXTURE CAPTURE_WORLD_VIEW_DONE")
	client.free()
	quit(0)

func wmo_liquids() -> Array:
	return client.find_children("Group*_Liquid", "MeshInstance3D", true, false)

## The three WMO liquids nearest `point`: name, surface centre and distance.
func nearest_liquids(point: Vector3) -> Array:
	var found := []
	for liquid in wmo_liquids():
		var box: AABB = liquid.global_transform * liquid.get_aabb()
		var centre := box.get_center()
		found.append([point.distance_to(centre), liquid.get_parent().name + "/" + liquid.name, centre])
	found.sort_custom(func(a, b): return a[0] < b[0])
	return found.slice(0, 3)

func hide_wmo_liquids() -> int:
	var liquids := wmo_liquids()
	for liquid in liquids:
		liquid.visible = false
	return liquids.size()

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 20000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1:
			break
	var ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await click(ui.find_child("CharCard_0", true, false))
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != character:
		fail("Card 0 is %s, not %s" % [selected.text, character])
		return false
	await click(ui.find_child("EnterWorld", true, false))
	var timeout := int(OS.get_environment("VIEW_WORLD_TIMEOUT_S")) if OS.get_environment("VIEW_WORLD_TIMEOUT_S") != "" else 300
	deadline = Time.get_ticks_msec() + timeout * 1000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD at ", state.local_player_position)
			return await wait_objects(deadline)
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

## Waits until the terrain objects (doodads, WMOs and their liquids) have spawned.
func wait_objects(deadline: int) -> bool:
	while Time.get_ticks_msec() < deadline:
		await wait_frames(30)
		var objects: Dictionary = client.account_state().world_objects
		var liquids_ready := OS.get_environment("VIEW_WAIT_LIQUIDS") == "1" and not wmo_liquids().is_empty()
		if liquids_ready or (objects.pending == 0 and objects.spawned > 0):
			print("FIXTURE OBJECTS ", objects)
			return true
	fail("Timed out spawning world objects: " + str(client.account_state().world_objects))
	return false

func click(control: Control) -> void:
	var point := control.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await wait_frames(2)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(shots.path_join(file))
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func wait_real(seconds: float) -> void:
	var deadline := Time.get_ticks_msec() + int(seconds * 1000)
	while Time.get_ticks_msec() < deadline:
		await process_frame

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
