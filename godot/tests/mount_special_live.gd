extends "res://tests/player_emote_live.gd"
# Real private-server journal, native jump/chat input, and read-only peer observations.
# Inherits bounded login/input helpers; never creates replicas or synthetic emote events.
var proof_dir := OS.get_environment("MOUNT_SPECIAL_PROOF")
var role := OS.get_environment("MOUNT_SPECIAL_ROLE")
var actor_name := OS.get_environment("MOUNT_SPECIAL_ACTOR")
var proof_skin := OS.get_environment("MOUNT_SPECIAL_SKIN")
var mounts := [[18, "Chestnut Mare"], [129, "Golden Gryphon"], [1532, "Soaring Spelltome"]]

func run_test() -> void:
	print("MOUNT_SPECIAL_BEGIN ", role, " ", proof_skin)
	root.size = Vector2i(1920, 1080)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if server.is_empty() or server.ends_with(":5000") or proof_dir.is_empty() or not OS.get_environment("EMOTE_ACCOUNT").begins_with("fb_mount_"):
		fail("Explicit private server, fb_mount_* account and proof directory required")
		return
	shots = OS.get_environment("GODOT_TEST_CAPTURE_DIR")
	if shots.is_empty() or actor_name.is_empty() or role not in ["actor", "observer"]:
		fail("Capture directory, actor name and actor/observer role required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await until(func(): return not client.account_state().assets_starting, "asset startup", 180000): return
	print("MOUNT_SPECIAL_ASSETS_READY ", client.account_state())
	var error: String = client.connect_account(server, OS.get_environment("EMOTE_ACCOUNT"), PASSWORD, false)
	if not error.is_empty(): fail(error); return
	print("MOUNT_SPECIAL_CONNECT ", error)
	if not await enter_world(): return
	print("MOUNT_SPECIAL_WORLD_READY ", client.account_state())
	var skin_error: String = client.preview_world_skin(proof_skin == "forever")
	if not skin_error.is_empty(): fail(skin_error); return
	var expected_skin := "Modern" if proof_skin == "modern" else "Forever"
	if client.account_state().ui_skin != expected_skin: fail("Unexpected skin"); return
	client.set_world_minutes(720.0)
	client.set_camera_orbit(PI, -0.3, 12.0)
	write_record(role + "-ready", client.account_state())
	if not await until(func(): return FileAccess.file_exists(path("setup-go")), "private setup", 180000): return
	if role == "observer":
		await observe_actor()
	else:
		await drive_mounts()

func drive_mounts() -> void:
	if not await until(func(): return FileAccess.file_exists(path("observer-ready")), "observer login", 120000): return
	for mount in mounts:
		await open_mounts()
		if not await until(func(): return control("MountJournalSearchBox") != null, "mount journal"): return
		await click_control(control("MountJournalSearchBox"))
		# Ctrl+A uses native input, not widget/model mutation.
		push_control_a()
		await process_frame
		await type_text(mount[1])
		if not await until(func(): return int(client.mount_journal_state().selected) == mount[0] and client.mount_journal_state().preview_ready, "authentic mount selection", 120000): return
		await click_control(control("MountJournalMountButton"))
		if not await until(func(): return mount_animation() != null and rider_skeleton() != null, "loaded mounted models", 120000): return
		await tap(KEY_ESCAPE)
		if not await until(func(): return mount_animation().current_animation_id() == 0, "mount idle"): return
		var prefix := "mount-%d" % mount[0]
		write_record(prefix + "-stage", {"mount_id": mount[0], "skin": proof_skin})
		if not await until(func(): return FileAccess.file_exists(path(prefix + "-observer-mounted")), "observer mounted model", 120000): return
		var before: Vector3 = client.account_state().local_player_position
		await tap(KEY_SPACE)
		if not await until(func(): return mount_animation().current_animation_id() == 94, "stationary jump MountSpecial"): return
		await capture(prefix + "-stationary-special.png")
		var samples := []
		for frame in range(45):
			await process_frame
			var position: Vector3 = client.account_state().local_player_position
			if abs(position.y - before.y) > 0.05: fail("Stationary special left ground"); return
			if frame % 10 == 0: samples.append(pose_record())
		write_record(prefix + "-stationary", {"before": before, "after": client.account_state().local_player_position, "poses": samples})
		await capture(prefix + "-rider-special.png")
		if not await until(func(): return mount_animation().current_animation_id() == 0, "special returns to idle", 30000): return
		if not await send_line("/mountspecial"): return
		if not await until(func(): return mount_animation().current_animation_id() == 94, "slash same MountSpecial"): return
		await capture(prefix + "-slash-special.png")
		if not await until(func(): return mount_animation().current_animation_id() == 0, "slash returns to idle", 30000): return
		push_key(KEY_W, 0, true)
		for frame in range(6): await process_frame
		var moving_start: Vector3 = client.account_state().local_player_position
		push_key(KEY_SPACE, 0, true)
		var rose := false
		for frame in range(20):
			await process_frame
			var position: Vector3 = client.account_state().local_player_position
			rose = rose or position.y > moving_start.y + 0.1
		push_key(KEY_SPACE, 0, false)
		push_key(KEY_W, 0, false)
		if not rose: fail("Moving mounted jump did not jump"); return
		write_record(prefix + "-moving-jump", {"rose": rose, "start": moving_start, "end": client.account_state().local_player_position})
		if not await until(func(): return FileAccess.file_exists(path(prefix + "-observer-special")), "replicated peer special", 30000): return
		await open_mounts()
		await click_control(control("MountJournalMountButton"))
		if not await until(func(): return mount_animation() == null, "dismount"): return
		await tap(KEY_ESCAPE)
	write_record("actor-done", {"skin": proof_skin, "mounts": mounts})
	print("MOUNT_SPECIAL_ACTOR PASS ", proof_skin)
	quit(0)

func observe_actor() -> void:
	for mount in mounts:
		var prefix := "mount-%d" % mount[0]
		if not await until(func(): return FileAccess.file_exists(path(prefix + "-stage")) and mount_animation() != null, "actor mounted", 180000): return
		write_record(prefix + "-observer-mounted", pose_record())
		if not await until(func(): return mount_animation().current_animation_id() == 94, "real replicated special", 120000): return
		await capture(prefix + "-observer-special.png")
		write_record(prefix + "-observer-special", pose_record())
		if not await until(func(): return mount_animation() == null, "actor dismount", 120000): return
	print("MOUNT_SPECIAL_OBSERVER PASS ", proof_skin)
	quit(0)

func actor_unit() -> Node:
	var units := client.find_child("WorldUnits", true, false)
	return units.find_child(actor_name, false, false) if units != null else null

func mount_animation() -> WowAnimationPlayer:
	var unit := actor_unit()
	if unit == null: return null
	for animation_node in unit.find_children("M2Animation", "", true, false):
		if animation_node.get_parent().name == "NpcModel": return animation_node as WowAnimationPlayer
	return null

func rider_skeleton() -> Skeleton3D:
	var unit := actor_unit()
	var model := unit.find_child("PlayerModel", true, false) if unit != null else null
	return model.get_node_or_null("Skeleton3D") as Skeleton3D if model != null else null

func pose_record() -> Dictionary:
	var poses := []
	var skeleton := rider_skeleton()
	if skeleton != null:
		for bone in range(skeleton.get_bone_count()):
			poses.append(str(skeleton.get_bone_pose(bone)))
	return {"mount_clip": mount_animation().current_animation_id() if mount_animation() != null else -1, "rider_bones": poses, "account": client.account_state()}

func control(name: String) -> Control:
	return client.find_child(name, true, false) as Control

func open_mounts() -> void:
	await click_control(control("MinimapLauncherButton"))
	if not await until(func(): return control("LauncherEntryCollectionsMicroButton") != null, "Collections entry"): return
	await click_control(control("LauncherEntryCollectionsMicroButton"))

func push_control_a() -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_A
		event.physical_keycode = KEY_A
		event.ctrl_pressed = true
		event.pressed = pressed
		root.push_input(event, true)

func path(label: String) -> String:
	return proof_dir.path_join(proof_skin + "-" + label + ".json")

func write_record(label: String, data: Variant) -> void:
	var file := FileAccess.open(path(label), FileAccess.WRITE)
	if file == null: fail("Cannot write proof receipt " + path(label)); return
	file.store_string(JSON.stringify(data, "\t"))

func until(predicate: Callable, what: String, timeout_ms: int = 15000) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		if predicate.call(): return true
		await process_frame
	fail("Timed out: " + what)
	return false
