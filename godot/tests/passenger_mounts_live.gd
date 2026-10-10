extends SceneTree

## Two real clients on a private realm. Roles driver/passenger, accounts fb_*, password
## fbtest. MOUNT_SYNC is unique per run; MOUNT_SHOTS is a private diagnostics directory.
## Main prepares accounts, riding spells and the selected skin before creating MOUNT_READY.
## First boarding uses the party-frame Ride menu; exit uses the actual Retail leave button.

var client: Node
var role := ""
var partner := ""
var character := ""
var sync_dir := ""
var shots := ""
var skin := ""

func _initialize() -> void:
	Engine.max_fps = 30
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	role = OS.get_environment("MOUNT_ROLE")
	partner = OS.get_environment("MOUNT_PARTNER")
	character = OS.get_environment("MOUNT_CHARACTER")
	sync_dir = OS.get_environment("MOUNT_SYNC")
	shots = OS.get_environment("MOUNT_SHOTS")
	skin = OS.get_environment("MOUNT_SKIN")
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("MOUNT_ACCOUNT")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":5000") or not account.begins_with("fb_"):
		fail("Private server and fb_* account required")
		return
	DirAccess.make_dir_recursive_absolute(sync_dir)
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error: String = client.connect_account(server, account, "fbtest", false)
	if error != "" or not await enter_world():
		fail("World entry: " + error)
		return
	var in_world := FileAccess.open(sync_dir + "/" + role + "-inworld", FileAccess.WRITE)
	if in_world == null:
		fail("Cannot announce world entry")
		return
	in_world.store_string("ready")
	in_world.close()
	if not await wait_until(func(): return FileAccess.file_exists(OS.get_environment("MOUNT_READY")), "main preparation"):
		return
	client.set_world_minutes(720.0)
	client.set_camera_orbit(1.2, -0.3, 16.0)
	if not await barrier("world"):
		return
	if role == "driver":
		await send_line("/invite " + partner)
	else:
		if not await wait_until(func(): return shown("StaticPopup1Button1"), "party invitation"):
			return
		await click(control("StaticPopup1Button1"))
	if not await barrier("group"):
		return
	await wait_frames(60)
	if not await mount_and_board("first", true):
		return
	if role == "passenger":
		if not await assert_seated():
			return
		if not await capture(skin + "-passenger-mammoth"):
			return
		var before := local_position()
		push_key(KEY_W, 0, true)
		await wait_seconds(0.7)
		push_key(KEY_W, 0, false)
		if local_position().distance_to(before) > 0.25:
			fail("Passenger input moved the player")
			return
	if not await barrier("seated"):
		return
	var before_move := local_position()
	if role == "driver":
		push_key(KEY_W, 0, true)
		await wait_seconds(1.2)
		push_key(KEY_W, 0, false)
		if local_position().distance_to(before_move) < 2.0:
			fail("Driver did not move")
			return
	if not await barrier("moved"):
		return
	await wait_seconds(1.0)
	if role == "passenger":
		if local_position().distance_to(before_move) < 2.0 or not await assert_seated():
			fail("Passenger did not follow the driver")
			return
		if not await capture(skin + "-driver-moved-passenger-mammoth"):
			return
		var leave := control("MainMenuBarVehicleLeaveButton") as Button
		print("LEAVE_INPUT path=", leave.get_path(), " rect=", leave.get_global_rect(), " disabled=", leave.disabled)
		leave.pressed.connect(func(): print("LEAVE_BUTTON_PRESSED"))
		await click(leave)
		print("LEAVE_INPUT_AFTER ", client.vehicle_state())
		if not await wait_until(func(): return not seated(), "leave-seat request"):
			return
	if not await barrier("exited"):
		return
	if role == "driver" and not await wait_until(func(): return client.vehicle_state().get("passengers", -1) == 0, "empty seat after exit"):
		return
	if not await mount_and_board("reboard", false):
		return
	if role == "driver":
		client.use_spell(61425)
	if not await wait_until(func(): return not seated() if role == "passenger" else not client.vehicle_state().has("vehicle_id"), "dismount ejection"):
		return
	if not await barrier("dismounted") or not await mount_and_board("disconnect", false):
		return
	if not await barrier("before-disconnect"):
		return
	if role == "passenger":
		print("PASSENGER_MOUNTS_LIVE PASS passenger ", skin)
		client.free()
		quit(0)
		return
	if not await wait_until(func(): return client.vehicle_state().get("passengers", -1) == 0, "passenger disconnect frees occupancy"):
		return
	print("PASSENGER_MOUNTS_LIVE PASS driver ", skin)
	client.free()
	quit(0)

func mount_and_board(tag: String, use_menu: bool) -> bool:
	if role == "driver":
		if not client.vehicle_state().has("vehicle_id"):
			var error: String = client.use_spell(61425)
			if error != "":
				return fail(error)
		if not await wait_until(func(): return client.vehicle_state().get("vehicle_id", 0) == 312 and client.get_node("WorldUnits/" + character).find_child("NpcVisualRoot", true, false) != null, "mammoth loaded"):
			return false
	if not await barrier(tag + "-mounted"):
		return false
	if role == "passenger":
		if use_menu:
			if not await ride_menu():
				return false
		else:
			var error: String = client.ride_player_mount(partner)
			if error != "":
				return fail(error)
		if not await wait_until(func(): return seated(), "authoritative passenger ownership"):
			return false
	if not await barrier(tag + "-boarded"):
		return false
	await wait_frames(30)
	return true

func ride_menu() -> bool:
	for prefix in ["CompactPartyFrameMember", "PartyMemberFrame"]:
		for index in range(1, 6):
			var frame := control(prefix + str(index))
			if frame == null or not frame.is_visible_in_tree():
				continue
			await click_at(frame.get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
			var title := control("UnitFrameContextMenuTitle") as Label
			if title != null and title.text == partner and shown("UnitFrameContextMenuRide"):
				await click(control("UnitFrameContextMenuRide"))
				return true
	return fail("Party member menu has no Ride entry")

func assert_seated() -> bool:
	var state: Dictionary = client.vehicle_state()
	if state.get("seat_id", 0) != 2764 or not shown("MainMenuBarVehicleLeaveButton"):
		return fail("Wrong seat or missing exit control: " + str(state))
	var player := client.get_node("WorldUnits/" + character) as Node3D
	var driver := client.get_node("WorldUnits/" + partner) as Node3D
	var visual := player.find_child("PlayerModel", true, false) as Node3D
	var point := driver.get_node_or_null("NpcVisualRoot/NpcModel/Skeleton3D/AttachmentBone40/Attachment40") as Node3D
	if visual == null or point == null:
		return fail("Passenger visual or actual attachment40 absent")
	var animation := visual.get_node("M2Animation") as WowAnimationPlayer
	if animation.current_animation_id() != 91:
		return fail("Passenger does not hold seat animation91")
	if visual.global_position.distance_to(point.global_position) > 0.35:
		return fail("Passenger is not on the authored animated seat")
	print("SEATED ", state, " visual=", visual.global_position, " attachment=", point.global_position, " logical=", player.global_position)
	return true

func seated() -> bool:
	return client.vehicle_state().has("seat_id")

func local_position() -> Vector3:
	return client.account_state().local_player_position

func enter_world() -> bool:
	if not await wait_until(func(): var state: Dictionary = client.account_state(); return state.get("reply_received", false) and state.screen == "CharacterSelect" and state.character_count > 0 and client.get_node_or_null("CharacterSelectUI") != null and not state.assets_starting, "character select"):
		return false
	await click(control("CharCard_0"))
	await click(control("EnterWorld"))
	return await wait_until(func(): var state: Dictionary = client.account_state(); return state.screen == "InWorld" and state.local_player_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty(), "in world")

func control(name: String) -> Control:
	return client.find_child(name, true, false) as Control

func shown(name: String) -> bool:
	var frame := control(name)
	return frame != null and frame.is_visible_in_tree()

func wait_until(predicate: Callable, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + 180000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	return fail("Timeout " + what + ": " + str(client.account_state()) + " " + str(client.vehicle_state()))

func barrier(tag: String) -> bool:
	var marker := FileAccess.open(sync_dir + "/" + role + "-" + tag, FileAccess.WRITE)
	if marker == null:
		return fail("Cannot write barrier " + tag)
	marker.store_string("ready")
	marker.close()
	var other := "passenger" if role == "driver" else "driver"
	return await wait_until(func(): return FileAccess.file_exists(sync_dir + "/" + other + "-" + tag), "barrier " + tag)

func click(target: Control) -> void:
	if target == null:
		fail("Missing input control")
		return
	await click_at(target.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)

func click_at(point: Vector2, button: int) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(2)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func send_line(text: String) -> void:
	await tap(KEY_ENTER)
	for letter in text:
		var code := OS.find_keycode_from_string(letter.to_upper())
		if letter == "/":
			code = KEY_SLASH
		push_key(code, letter.unicode_at(0), true)
		await process_frame
		push_key(code, 0, false)
		await process_frame
	await tap(KEY_ENTER)

func tap(code: Key) -> void:
	push_key(code, 0, true)
	await process_frame
	push_key(code, 0, false)
	await wait_frames(2)

func push_key(code: Key, unicode: int, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.unicode = unicode
	event.pressed = pressed
	root.push_input(event, true)

func wait_frames(count: int) -> void:
	for _index in range(count):
		await process_frame

func wait_seconds(seconds: float) -> void:
	var deadline := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < deadline:
		await process_frame

func capture(stem: String) -> bool:
	await RenderingServer.frame_post_draw
	var path := shots + "/" + stem + ".png"
	var saved := root.get_texture().get_image().save_png(path)
	if saved != OK:
		return fail("Screenshot write failed: " + path)
	print("PASSENGER_CAPTURE ", path)
	return true

func fail(message: String) -> bool:
	push_error(message)
	if client != null:
		print("FAIL_STATE ", client.account_state(), " ", client.vehicle_state())
	quit(1)
	return false
