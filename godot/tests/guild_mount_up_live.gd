extends "res://tests/guild_perks_live.gd"

var officer_dir := ""
var phase := 0
var samples: Array[Dictionary] = []

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	shots = OS.get_environment("BANK_SHOTS")
	diagnostics = OS.get_environment("GUILD_PERKS_DIAGNOSTICS")
	officer_dir = OS.get_environment("GUILD_PERKS_OFFICER_DIR")
	assert(OS.get_environment("GODOT_TEST_SERVER") == "127.0.0.1:5891", "Private5891 only")
	assert(OS.get_environment("BANK_ACCOUNT") == "fb_guildcompletion_perkspeer", "Private ordinary member only")
	assert(not officer_dir.is_empty(), "Real officer coordination directory required")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_frames(30)
	role = "mount-up-entry"
	if not await connect_private_account() or not await enter_world("Gcperkspeer"):
		return
	if not mark_officer("member-ready.go"):
		return
	if not await wait_until(func(): return FileAccess.file_exists(officer_dir.path_join("riding-ready.go")), "actual riding/mount fixture setup"):
		return
	if not await wait_until(func(): return 32235 in Array(client.spells_state().known), "actual learned Golden Gryphon spell"):
		return
	client.set_world_minutes(720.0)
	client.set_camera_orbit(PI, -0.3, 14.0)
	for skin in ["Modern", "Forever"]:
		phase += 1
		samples.clear()
		if not await select_skin(skin):
			return
		role = skin.to_lower() + "-mount-up"
		await close_guild()
		if known_perks().size() != 0:
			fail("Native control must start as a real nonmember")
			return
		if not await prove_mount_cycle():
			return
		write_json(role + "-movement.json", {"samples": samples})
	mark_officer("done.go")
	print("PASS GUILD_MOUNT_UP_NATIVE both_skins=true real_invite_accept=true real_officer_remove=true ground=14/15.4/14 flight=22.4/24.64/22.4")
	client.free()
	quit(0)

func prove_mount_cycle() -> bool:
	if not await mount_gryphon() or not await sample_movement(14.0, "nonmember-ground"):
		return false
	if not await lift_off() or not await sample_movement(22.4, "nonmember-flight"):
		return false
	if not await dismount_and_land():
		return false
	if not mark_officer("invite-%d.go" % phase):
		return false
	if not await wait_until(func(): return shown("GuildInviteFrameJoinButton"), "actual officer invitation"):
		return false
	await press("GuildInviteFrameJoinButton")
	if not await wait_until(func(): return known_perks().size() == 5, "accepted membership grants"):
		return false
	await close_guild()
	if not await mount_gryphon() or not await sample_movement(15.4, "member-ground"):
		return false
	if not await lift_off() or not await sample_movement(24.64, "member-flight"):
		return false
	if not mark_officer("remove-%d.go" % phase):
		return false
	if not await wait_until(func(): return known_perks().is_empty(), "real officer removal revokes grants"):
		return false
	if perk_observation().mount_id != 129 or not perk_observation().flying:
		return fail("Guild removal must preserve the actual mounted flight")
	if not await sample_movement(22.4, "removed-flight") or not await dismount_and_land():
		return false
	if not await mount_gryphon() or not await sample_movement(14.0, "removed-ground"):
		return false
	if not await dismount_and_land():
		return false
	return await sample_movement(7.0, "unmounted")

func mount_gryphon() -> bool:
	var error: String = client.use_spell(32235)
	if not error.is_empty():
		return fail("Actual Golden Gryphon cast: " + error)
	return await wait_until(func(): return perk_observation().mount_id == 129 and perk_observation().can_fly, "actual expert-riding Golden Gryphon")

func dismount_and_land() -> bool:
	var error: String = client.use_spell(32235)
	if not error.is_empty():
		return fail("Actual dismount: " + error)
	if not await wait_until(func(): return perk_observation().mount_id == null and not perk_observation().flying, "actual dismount"):
		return false
	await wait_frames(5)
	return await wait_until(func(): return not perk_observation().falling, "actual landing after dismount")

func lift_off() -> bool:
	release_widget_focus()
	# dispatch_jump_action routes stationary mounted Space to MountSpecial.
	# Move first so actual Space takes the normal Jump/Ascend path.
	move_key(KEY_W, true)
	await wait_frames(1)
	move_key(KEY_SPACE, true)
	await wait_frames(3)
	write_json(role + "-takeoff-first-space.json", perk_observation())
	move_key(KEY_SPACE, false)
	await wait_frames(3)
	write_json(role + "-takeoff-released.json", perk_observation())
	move_key(KEY_SPACE, true)
	var lifted: bool = await wait_until(func(): return perk_observation().flying, "physical steady-flight takeoff")
	move_key(KEY_SPACE, false)
	move_key(KEY_W, false)
	return lifted

func sample_movement(expected: float, label: String) -> bool:
	release_widget_focus()
	move_key(KEY_W, true)
	var matched: bool = await wait_until(func():
		var value: Variant = perk_observation().movement_speed
		return value != null and abs(float(value) - expected) < 0.02,
		"actual replicated movement speed %.2f: %s" % [expected, label])
	if matched:
		var sample: Dictionary = perk_observation()
		sample["phase"] = label
		sample["expected_speed"] = expected
		samples.append(sample)
		write_json(role + "-" + label + "-sample.json", sample)
		if not sample.forward_held:
			move_key(KEY_W, false)
			return fail("Physical forward input was not captured")
		await capture(label)
	move_key(KEY_W, false)
	await wait_frames(3)
	return matched

func release_widget_focus() -> void:
	var focused := root.gui_get_focus_owner()
	if focused != null:
		focused.release_focus()

func move_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

func capture(label: String) -> void:
	# Reuse the established mount fixture's camera: view-only, no physics changes.
	var previous := root.get_camera_3d()
	var camera := load("res://tests/mount_special_observer_camera.gd").new() as Camera3D
	camera.set("actor", client.get_node("WorldUnits/Gcperkspeer"))
	camera.process_priority = 1000000
	camera.fov = previous.fov
	camera.near = previous.near
	camera.far = previous.far
	root.add_child(camera)
	await process_frame
	await super.capture(label)
	camera.queue_free()
	previous.make_current()

func mark_officer(name: String) -> bool:
	var file := FileAccess.open(officer_dir.path_join(name), FileAccess.WRITE)
	if file == null:
		return fail("Cannot publish officer coordination marker " + name)
	file.store_string("go")
	file.close()
	return true
