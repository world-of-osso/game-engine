extends SceneTree

## Dev-server regression: WMO walls stop the player whether or not portal culling draws them.
## The character stands on the Stockade entrance stairs (WoW -8774, 838, 92.1; place it there
## with `game-server-admin teleport <name> 0 -8774 838 92.1` while offline), turns with the real
## turn keys to face across the stairwell (the original client's stairs test direction), and
## holds W for 1.5 s (about 10 yd at run speed). The stairwell wall is 6.2 yd away: the client
## and the server must stop the player short of it. Without WMO walls it walked through.
## Requires GODOT_TEST_SERVER=127.0.0.1:5000, WMO_WALL_ACCOUNT, WMO_WALL_PASSWORD,
## WMO_WALL_CHARACTER (card 0), and --fixed-fps 60. WMO_WALL_SHOTS names a screenshot directory.
const WORLD_WAIT_MS := 180000
const RUN_FRAMES := 90
const YAW_TOLERANCE := 0.05
const WALL_DISTANCE := 6.3
## Bevy (x, z) of WoW (-0.622, 0.783): across the stairwell, WMO-local -Y of `sw_magicdistrict`.
const ACROSS := Vector2(-0.622, -0.783)

var NAME := OS.get_environment("WMO_WALL_CHARACTER")
var turn_sign := 0.0

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	await process_frame
	if not is_equal_approx(root.get_process_delta_time(), 1.0 / 60.0):
		fail("Run with --fixed-fps 60")
		return
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server != "127.0.0.1:5000":
		fail("GODOT_TEST_SERVER must explicitly select 127.0.0.1:5000")
		return
	var account := OS.get_environment("WMO_WALL_ACCOUNT")
	var password := OS.get_environment("WMO_WALL_PASSWORD")
	if account == "" or password == "" or NAME == "":
		fail("WMO_WALL_ACCOUNT, WMO_WALL_PASSWORD and WMO_WALL_CHARACTER are required")
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
	# Let the nearest WMO wall shapes build.
	for _frame in 240:
		await process_frame
	if not await face(player, atan2(ACROSS.x, ACROSS.y)):
		return
	var start := Vector2(player.position.x, player.position.z)
	print("START client=", player.position, " server=", client.account_state().local_server_position, " yaw=%.3f" % facing(player))
	push_key(KEY_W, true)
	for _frame in RUN_FRAMES:
		await process_frame
	push_key(KEY_W, false)
	for _frame in 120:
		await process_frame
	await capture("wall")
	var state: Dictionary = client.account_state()
	var walked := Vector2(player.position.x, player.position.z).distance_to(start)
	var server_pos: Vector3 = state.local_server_position
	var server_walked := Vector2(server_pos.x, server_pos.z).distance_to(start)
	print("END client=", player.position, " server=", server_pos, " walked=%.2f server_walked=%.2f" % [walked, server_walked])
	if walked < 1.0:
		fail("Held W did not move the player: %.2f yd" % walked)
		return
	if walked > WALL_DISTANCE or server_walked > WALL_DISTANCE:
		fail("Walked through the stairwell wall: client %.2f, server %.2f yd (wall at 6.2)" % [walked, server_walked])
		return
	print("PASS: the stairwell wall stops the player on the client and the server")
	client.free()
	quit(0)

func facing(player: Node3D) -> float:
	return player.rotation.y + PI / 2.0

## Turn with the real turn keys until the player's forward (sin, cos) has yaw `target`.
func face(player: Node3D, target: float) -> bool:
	for _attempt in 600:
		var error := angle_difference(facing(player), target)
		if abs(error) < YAW_TOLERANCE:
			release_turns()
			await process_frame
			return true
		if turn_sign == 0.0:
			var before := facing(player)
			await hold_key(KEY_RIGHT, 5)
			turn_sign = sign(angle_difference(before, facing(player)))
			if turn_sign == 0.0:
				fail("Turn keys do not change facing")
				return false
			continue
		var right: bool = sign(error) == turn_sign
		push_key(KEY_RIGHT, right)
		push_key(KEY_LEFT, not right)
		await process_frame
		release_turns()
	fail("Could not face %.3f" % target)
	return false

func release_turns() -> void:
	push_key(KEY_RIGHT, false)
	push_key(KEY_LEFT, false)


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
	var dir := OS.get_environment("WMO_WALL_SHOTS")
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
