extends RefCounted

# Measured from cached azeroth_32_48 ADT; movement and swim flags stay production-owned.
const START := Vector3(-8558.0, 144.96008, 522.0)
const WATER_LEVEL := 143.98892
const DEEP_Z := 500.0
const SHORE_Z := 522.0
const FRAMES_LIMIT := 540
const STILL_FRAMES := 45
const POSE_BLEND_MS := 150

func check(flow, client: Node, player: Node3D) -> String:
	var animation = load("res://tests/player_locomotion_probe.gd").new()
	var error := ""
	var model_deadline := Time.get_ticks_msec() + 30000
	while Time.get_ticks_msec() < model_deadline:
		error = animation.bind(player)
		if error == "" and animation.animation.current_animation_id() == 0:
			break
		await flow.process_frame
	if error != "" or animation.animation.current_animation_id() != 0:
		return "Timed out waiting for native swimming player Stand 0: " + error
	if player.position.distance_to(START) > 0.5:
		return "Shore fixture did not begin grounded in Stand 0 at authored dry start: " + str(player.position)
	error = check_ground(client, player, false)
	if error != "":
		return error
	print("FIXTURE SWIM_WORLD_READY")
	var stand_pose: Array[Transform3D] = animation.capture_pose()
	flow.push_key(KEY_W, true)
	var run_seen := false
	var swim_at := -1
	var swim_pose_changed := false
	for frame in range(FRAMES_LIMIT):
		await flow.process_frame
		if client.account_state().screen != "InWorld":
			flow.push_key(KEY_W, false)
			return "World exited on shore crossing at frame " + str(frame)
		var current_id: int = animation.animation.current_animation_id()
		run_seen = run_seen or current_id == 5
		if current_id == 42 and swim_at < 0:
			swim_at = Time.get_ticks_msec()
		if swim_at >= 0 and current_id != 42:
			flow.push_key(KEY_W, false)
			return "W left Swim 42 after entering water: " + str(current_id)
		if swim_at >= 0 and Time.get_ticks_msec() - swim_at >= POSE_BLEND_MS:
			swim_pose_changed = swim_pose_changed or animation.changed_from(stand_pose)
		if player.position.z <= DEEP_Z and swim_pose_changed:
			break
	if not run_seen or swim_at < 0 or not swim_pose_changed or player.position.z > DEEP_Z:
		flow.push_key(KEY_W, false)
		return "W did not traverse Run 5 -> Swim 42 into deep water with changed bones: " + str(player.position)
	error = check_ground(client, player, true)
	if error != "":
		flow.push_key(KEY_W, false)
		return error
	if DisplayServer.get_name() == "headless":
		flow.push_key(KEY_W, false)
		return "Swimming screenshot requires rendered Godot display"
	flow.push_key(KEY_W, false)
	print("FIXTURE SWIM_RELEASED")
	var water_pixels = load("res://tests/adt_water_pixels.gd").new()
	var water_error: String = await water_pixels.check(flow, client)
	if water_error != "":
		flow.push_key(KEY_W, false)
		return water_error
	await RenderingServer.frame_post_draw
	var screenshot: Image = flow.root.get_texture().get_image()
	if screenshot == null or screenshot.is_empty() or screenshot.save_png("res://../data/diagnostics/godot-conversion/swimming.png") != OK:
		flow.push_key(KEY_W, false)
		return "Could not capture deep-water swimming frame"
	for frame in range(STILL_FRAMES):
		await flow.process_frame
		if frame >= 10 and animation.animation.current_animation_id() != 41:
			return "Released W did not settle to SwimIdle 41 at frame " + str(frame)
	if animation.animation.current_animation_id() != 41:
		return "Released W never selected SwimIdle 41"
	var idle_at := player.position
	for frame in range(STILL_FRAMES):
		await flow.process_frame
		if animation.animation.current_animation_id() != 41 or player.position.distance_to(idle_at) > 0.05:
			return "Deep-water idle drifted or left SwimIdle 41"
	print("FIXTURE SWIM_IDLE_DONE")
	flow.push_key(KEY_SPACE, true)
	var stationary := player.position
	for frame in range(STILL_FRAMES):
		await flow.process_frame
		if animation.animation.current_animation_id() != 41 or player.position.distance_to(stationary) > 0.05:
			flow.push_key(KEY_SPACE, false)
			return "Stationary Space jumped, rose, or left SwimIdle 41 at frame " + str(frame)
	flow.push_key(KEY_SPACE, false)
	print("FIXTURE SWIM_SPACE_IDLE_DONE")
	flow.push_key(KEY_W, true)
	flow.push_key(KEY_SPACE, true)
	var moving := player.position
	for frame in range(STILL_FRAMES):
		await flow.process_frame
		if (frame >= 10 and animation.animation.current_animation_id() != 42) or player.position.y > stationary.y + 0.1:
			flow.push_key(KEY_W, false)
			flow.push_key(KEY_SPACE, false)
			return "Moving W+Space jumped, rose, or left Swim 42 at frame " + str(frame)
	flow.push_key(KEY_W, false)
	flow.push_key(KEY_SPACE, false)
	if player.position.z >= moving.z - 0.5:
		return "Wet W+Space did not advance through water"
	print("FIXTURE SWIM_SPACE_FORWARD_DONE")
	for frame in range(STILL_FRAMES):
		await flow.process_frame
		if animation.animation.current_animation_id() != 41:
			return "Wet forward release did not return to SwimIdle 41"
	error = await check_lateral(flow, client, player, animation, KEY_A, 43, "LEFT")
	if error != "":
		return error
	error = await check_lateral(flow, client, player, animation, KEY_D, 44, "RIGHT")
	if error != "":
		return error
	if absf(player.position.x - START.x) > 0.3:
		return "Lateral return missed original X before S: " + str(player.position)
	print("FIXTURE SWIM_BACKWARD_START")
	flow.push_key(KEY_S, true)
	var backward_seen := false
	var dry_seen := false
	for frame in range(FRAMES_LIMIT):
		await flow.process_frame
		var current_id: int = animation.animation.current_animation_id()
		backward_seen = backward_seen or current_id == 45
		if backward_seen and current_id == 13:
			dry_seen = true
		if dry_seen and current_id != 13:
			flow.push_key(KEY_S, false)
			return "Reverse dry crossing left WalkBackwards 13: " + str(current_id)
		if dry_seen and player.position.z >= SHORE_Z:
			break
	flow.push_key(KEY_S, false)
	if not backward_seen or not dry_seen or player.position.z < SHORE_Z:
		return "S did not cross SwimBackwards 45 -> WalkBackwards 13 to dry shore: " + str(player.position)
	error = check_ground(client, player, false)
	if error != "":
		return error
	await RenderingServer.frame_post_draw
	var shoreline: Image = flow.root.get_texture().get_image()
	if shoreline == null or shoreline.is_empty() or shoreline.save_png("res://../data/diagnostics/godot-conversion/swimming-shoreline.png") != OK:
		return "Could not capture authored shoreline frame"
	print("FIXTURE SWIM_BACKWARD_RELEASED")
	for frame in range(STILL_FRAMES * 2):
		await flow.process_frame
		if frame >= 10 and animation.animation.current_animation_id() != 0:
			return "Dry S release did not remain Stand 0"
	print("FIXTURE SWIM_DONE")
	return ""

func check_lateral(flow, client: Node, player: Node3D, animation: RefCounted, keycode: Key, clip: int, label: String) -> String:
	var origin := player.position
	var idle_pose: Array[Transform3D] = animation.capture_pose()
	var selected_at := -1
	var changed_pose := false
	print("FIXTURE SWIM_" + label + "_START")
	flow.push_key(keycode, true)
	var error := ""
	for frame in range(120):
		await flow.process_frame
		if client.account_state().screen != "InWorld":
			error = label + " exited world at frame " + str(frame)
			break
		var current_id: int = animation.animation.current_animation_id()
		if current_id == clip and selected_at < 0:
			selected_at = Time.get_ticks_msec()
		if selected_at >= 0 and current_id != clip:
			error = label + " left Swim clip " + str(clip) + ": " + str(current_id)
			break
		error = check_lateral_ground(client, player, origin)
		if error != "":
			break
		if selected_at >= 0 and Time.get_ticks_msec() - selected_at >= POSE_BLEND_MS:
			changed_pose = changed_pose or animation.changed_from(idle_pose)
		var dx := player.position.x - origin.x
		# This fixture faces PI: left strafe decreases world X, right increases it.
		var arrived: bool = dx <= -1.8 if keycode == KEY_A else absf(player.position.x - START.x) <= 0.3 and dx >= 1.5
		if arrived and changed_pose:
			break
	flow.push_key(keycode, false)
	print("FIXTURE SWIM_" + label + "_END")
	if error != "":
		return error
	var displacement := player.position.x - origin.x
	if selected_at < 0 or not changed_pose or (keycode == KEY_A and displacement > -1.8) or (keycode == KEY_D and (displacement < 1.5 or absf(player.position.x - START.x) > 0.3)):
		return label + " lacked signed lateral displacement, Swim clip " + str(clip) + ", or changed bones after blend: " + str(player.position)
	var stopped := player.position
	for frame in range(STILL_FRAMES + 10):
		await flow.process_frame
		if frame == 10:
			stopped = player.position
		if frame >= 10 and animation.animation.current_animation_id() != 41:
			return label + " release did not select SwimIdle 41"
		if frame > 10 and player.position.distance_to(stopped) > 0.05:
			return label + " release drifted: " + str(player.position)
		error = check_lateral_ground(client, player, origin)
		if error != "":
			return error
	print("FIXTURE SWIM_" + label + "_IDLE_DONE")
	return ""

func check_lateral_ground(client: Node, player: Node3D, origin: Vector3) -> String:
	var position := player.position
	var ground = client.terrain_height_at(position.x, position.z)
	if ground == null or absf(position.y - float(ground)) > 0.3 or WATER_LEVEL - float(ground) < 3.0:
		return "Lateral swim lost grounded deep-water sample: " + str(position) + " ground=" + str(ground)
	if position.x < START.x - 4.0 or position.x > START.x + 4.0 or position.z < 480.0 or position.z > DEEP_Z + 0.5 or absf(position.z - origin.z) > 0.3:
		return "Lateral swim left measured flat corridor: " + str(position)
	return ""

func check_ground(client: Node, player: Node3D, wet: bool) -> String:
	var position := player.position
	var ground = client.terrain_height_at(position.x, position.z)
	if ground == null or absf(position.y - float(ground)) > 0.3 or absf(position.x - START.x) > 0.5:
		return "Player left sampled authored ground or X line: " + str(position) + " ground=" + str(ground)
	var depth := WATER_LEVEL - float(ground)
	if wet and (depth < 3.0 or absf(position.z - DEEP_Z) > 1.0):
		return "Deep-water sample lacks measured water depth at z500: " + str(position) + " depth=" + str(depth)
	if not wet and (depth >= 1.25 or position.z < SHORE_Z - 0.5):
		return "Dry sample remains beneath swim threshold at z522: " + str(position) + " depth=" + str(depth)
	return ""
