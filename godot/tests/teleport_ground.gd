extends SceneTree

# Private-server, real-asset regression. Start at least 500 yd from a supported
# destination on the same map. Start with --screen inworld --server ... --char ...,
# and supply TEST_CHARACTER, TEST_ADMIN, GAME_SERVER_ADMIN_SOCKET, TEST_MAP and
# TEST_X/Y/Z (server/WoW coordinates). The destination tile must not already be loaded.
const WORLD_WAIT_MS := 300000
var client: Node
var character := OS.get_environment("TEST_CHARACTER")

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var admin := OS.get_environment("TEST_ADMIN")
	var server := OS.get_environment("TEST_SERVER")
	var args := OS.get_cmdline_user_args()
	var server_index := args.find("--server")
	if not server.begins_with("127.0.0.1:") or server_index < 0 or server_index + 1 >= args.size() or args[server_index + 1] != server or admin.is_empty() or character.is_empty() or OS.get_environment("GAME_SERVER_ADMIN_SOCKET").is_empty():
		fail("Requires the explicit private server, admin socket, binary and character")
		return
	var destination := Vector3(float(OS.get_environment("TEST_X")), float(OS.get_environment("TEST_Z")), -float(OS.get_environment("TEST_Y")))
	var startup_deadline := Time.get_ticks_msec() + WORLD_WAIT_MS
	var ready_file := OS.get_environment("TEST_READY_FILE")
	while not ready_file.is_empty() and not FileAccess.file_exists(ready_file) and Time.get_ticks_msec() < startup_deadline:
		await process_frame
	if not ready_file.is_empty() and not FileAccess.file_exists(ready_file):
		fail("Private server was not made ready")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var player := await enter_world()
	if player == null:
		return
	if client.account_state().ui_skin != "Forever":
		fail("Teleport fixture requires the Forever skin")
		return
	if player.position.distance_to(destination) < 500.0:
		fail("Destination must be at least 500 yd away")
		return
	var old_epoch = client.account_state().local_player_epoch
	var output := []
	var code := OS.execute(admin, ["teleport", character, OS.get_environment("TEST_MAP"), OS.get_environment("TEST_X"), OS.get_environment("TEST_Y"), OS.get_environment("TEST_Z")], output, true)
	if code != 0:
		fail("Teleport failed: " + str(output))
		return
	var deadline := Time.get_ticks_msec() + 90000
	var saw_wait := false
	var arrived := false
	var arrived_at := 0
	var arrival_position := Vector3.ZERO
	push_key(true)
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.local_player_epoch == old_epoch:
			continue
		if not arrived:
			arrived = true
			arrived_at = Time.get_ticks_msec()
			arrival_position = state.local_server_position
			if Vector2(arrival_position.x - destination.x, arrival_position.z - destination.z).length() > 0.1 or absf(arrival_position.y - destination.y) > 3.0:
				push_key(false)
				fail("Server did not place the player at the destination: " + str(arrival_position))
				return
		if state.teleport_ground_waiting:
			saw_wait = true
			if player.position.distance_to(arrival_position) > 0.1:
				push_key(false)
				fail("Player moved or fell while destination loaded: " + str(player.position))
				return
		elif saw_wait:
			break
	if not arrived or not saw_wait or client.account_state().teleport_ground_waiting:
		push_key(false)
		fail("Did not observe a complete destination wait: " + str(client.account_state()))
		return
	if Time.get_ticks_msec() - arrived_at >= 59000:
		push_key(false)
		fail("Destination wait ended at the timeout, not on collision readiness")
		return
	var resumed_at := player.position
	for _frame in 30:
		await process_frame
	push_key(false)
	if Vector2(player.position.x - resumed_at.x, player.position.z - resumed_at.z).length() < 0.1:
		fail("Movement did not resume after collision loaded")
		return
	for _frame in 30:
		await process_frame
	if absf(player.position.y - destination.y) > 3.0:
		fail("Player did not remain on destination ground: " + str(player.position))
		return
	print("TELEPORT_GROUND PASS destination=", destination, " final=", player.position)
	quit(0)

func enter_world() -> Node3D:
	var deadline := Time.get_ticks_msec() + WORLD_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		var player := client.get_node_or_null("WorldUnits/" + character) as Node3D
		if state.screen == "InWorld" and player != null:
			return player
	fail("World entry timed out: " + str(client.account_state()))
	return null

func push_key(pressed: bool) -> void:
	var event := InputEventKey.new()
	event.physical_keycode = KEY_W
	event.pressed = pressed
	root.push_input(event, true)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
